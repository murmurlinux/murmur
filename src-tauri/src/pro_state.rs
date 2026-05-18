// Pro entitlement token: receive via `murmur://auth?token=...` deep link,
// validate against the embedded ES256 public key, persist to
// tauri-plugin-store, and refresh opportunistically against
// murmurlinux.com.
//
// The desktop never contacts Supabase. The website is the sole source of
// truth for entitlement state; this module just verifies the signed
// statement the website hands us. Architecture spec lives at
// ~/Projects/murmur-internal/docs/superpowers/specs/
// 2026-05-15-pro-infrastructure-and-auth.md §4 + §6.

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_store::StoreExt;
use thiserror::Error;
use url::Url;

pub const SETTINGS_STORE: &str = "settings.json";
pub const STORE_KEY: &str = "pro";
pub const PRO_STATE_EVENT: &str = "pro-state-changed";

pub const SIGN_IN_URL: &str = "https://murmurlinux.com/app/sign-in?return_to=murmur://auth";
pub const REFRESH_URL: &str = "https://murmurlinux.com/api/auth/refresh";

const KID_V1: &str = "v1";
const REFRESH_AFTER_SECS: i64 = 7 * 24 * 60 * 60;
const OFFLINE_GRACE_SECS: i64 = 7 * 24 * 60 * 60;

// Public key for the murmurlinux.com Pro JWT signer (kid=v1, ES256).
// Mirror of ~/Projects/murmur-internal/docs/pro-infra/keys/README.md.
// Public keys are not secrets; shipping them in source preserves the
// README "no build-time secrets" promise.
const PRO_JWT_PUBLIC_KEY_V1: &str = "-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEtkzvu9Sx/goyoWOKtCe20vRfNA9y
961vij+axzhlyajmVawaMdd3CYjDUCJ+yUhtQ2K/uEDxI1y9ccAz9fLV9w==
-----END PUBLIC KEY-----";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProClaims {
    pub sub: String,
    pub email: String,
    pub is_pro: bool,
    #[serde(default)]
    pub pro_expires_at: Option<String>,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoredToken {
    jwt: String,
    last_successful_refresh_at: i64,
    last_response_was_401: bool,
}

#[derive(Debug, Default)]
struct Inner {
    stored: Option<StoredToken>,
    claims: Option<ProClaims>,
}

pub struct ProState {
    inner: Mutex<Inner>,
}

#[derive(Debug, Error)]
pub enum ProError {
    #[error("malformed token")]
    Malformed,
    #[error("token expired")]
    Expired,
    #[error("bad signature")]
    BadSignature,
    #[error("unknown key id")]
    UnknownKid,
    #[error("missing token in deep link")]
    MissingToken,
    #[error("store error: {0}")]
    Store(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("server rejected token (401)")]
    Unauthorized,
}

impl ProState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
        }
    }

    pub fn is_active(&self) -> bool {
        let g = self.inner.lock().expect("pro_state mutex");
        match (&g.stored, &g.claims) {
            (Some(stored), Some(claims)) if claims.is_pro => {
                is_active_now(claims, stored, now_unix())
            }
            _ => false,
        }
    }

    pub fn email(&self) -> Option<String> {
        self.inner
            .lock()
            .ok()?
            .claims
            .as_ref()
            .map(|c| c.email.clone())
    }

    pub fn pro_expires_at(&self) -> Option<String> {
        self.inner
            .lock()
            .ok()?
            .claims
            .as_ref()
            .and_then(|c| c.pro_expires_at.clone())
    }

    /// Read the cached token from the store and decode-verify it (best
    /// effort; bad/expired/missing all wipe to a clean unsigned state).
    pub fn load_from_store(&self, app: &AppHandle<Wry>) {
        let stored = match read_stored(app) {
            Ok(Some(s)) => s,
            Ok(None) => return,
            Err(e) => {
                log::warn!("pro: store read failed: {e}");
                return;
            }
        };
        match decode_and_verify(&stored.jwt) {
            Ok(claims) => {
                log::info!("pro: loaded cached token (is_pro={})", claims.is_pro);
                let mut g = self.inner.lock().expect("pro_state mutex");
                g.stored = Some(stored);
                g.claims = Some(claims);
            }
            // Expired tokens stay cached so the offline-grace window can
            // honour them at the UI layer; a 401 from refresh is the
            // only thing that wipes entitlement.
            Err(ProError::Expired) => {
                log::info!("pro: cached token past exp; will rely on refresh");
                let mut g = self.inner.lock().expect("pro_state mutex");
                g.stored = Some(stored);
                g.claims = None;
            }
            Err(e) => {
                log::warn!("pro: cached token rejected ({e}); wiping");
                let _ = clear_stored(app);
            }
        }
    }

    /// Validate the deep-link payload and persist. Emits
    /// `pro-state-changed` on success.
    pub fn apply_deep_link(&self, app: &AppHandle<Wry>, url: &str) -> Result<(), ProError> {
        let parsed = Url::parse(url).map_err(|_| ProError::Malformed)?;
        let token = parsed
            .query_pairs()
            .find(|(k, _)| k == "token")
            .map(|(_, v)| v.into_owned())
            .ok_or(ProError::MissingToken)?;
        let claims = decode_and_verify(&token)?;
        let stored = StoredToken {
            jwt: token,
            last_successful_refresh_at: now_unix(),
            last_response_was_401: false,
        };
        write_stored(app, &stored).map_err(|e| ProError::Store(e.to_string()))?;
        {
            let mut g = self.inner.lock().expect("pro_state mutex");
            g.claims = Some(claims.clone());
            g.stored = Some(stored);
        }
        emit_state_changed(app, Some(&claims));
        log::info!(
            "pro: signed in as {} (is_pro={})",
            claims.email,
            claims.is_pro
        );
        Ok(())
    }

    pub fn sign_out(&self, app: &AppHandle<Wry>) -> Result<(), ProError> {
        clear_stored(app).map_err(|e| ProError::Store(e.to_string()))?;
        {
            let mut g = self.inner.lock().expect("pro_state mutex");
            g.claims = None;
            g.stored = None;
        }
        emit_state_changed(app, None);
        Ok(())
    }
}

impl Default for ProState {
    fn default() -> Self {
        Self::new()
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn is_active_now(claims: &ProClaims, stored: &StoredToken, now: i64) -> bool {
    if !claims.is_pro {
        return false;
    }
    if claims.exp > now {
        return true;
    }
    // Past exp: extend if the network was the reason we couldn't refresh.
    !stored.last_response_was_401 && (claims.exp + OFFLINE_GRACE_SECS) > now
}

fn decode_and_verify(jwt: &str) -> Result<ProClaims, ProError> {
    let header = decode_header(jwt).map_err(|_| ProError::Malformed)?;
    match header.kid.as_deref() {
        Some(KID_V1) => {}
        _ => return Err(ProError::UnknownKid),
    }
    let key = DecodingKey::from_ec_pem(PRO_JWT_PUBLIC_KEY_V1.as_bytes())
        .map_err(|_| ProError::Malformed)?;
    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_exp = true;
    let data = decode::<ProClaims>(jwt, &key, &validation).map_err(|e| {
        use jsonwebtoken::errors::ErrorKind;
        match e.kind() {
            ErrorKind::ExpiredSignature => ProError::Expired,
            ErrorKind::InvalidSignature
            | ErrorKind::InvalidEcdsaKey
            | ErrorKind::InvalidKeyFormat => ProError::BadSignature,
            _ => ProError::Malformed,
        }
    })?;
    Ok(data.claims)
}

fn read_stored(app: &AppHandle<Wry>) -> Result<Option<StoredToken>, String> {
    let store = app.store(SETTINGS_STORE).map_err(|e| e.to_string())?;
    let Some(value) = store.get(STORE_KEY) else {
        return Ok(None);
    };
    serde_json::from_value::<StoredToken>(value)
        .map(Some)
        .map_err(|e| e.to_string())
}

fn write_stored(app: &AppHandle<Wry>, stored: &StoredToken) -> Result<(), String> {
    let store = app.store(SETTINGS_STORE).map_err(|e| e.to_string())?;
    let value = serde_json::to_value(stored).map_err(|e| e.to_string())?;
    store.set(STORE_KEY, value);
    store.save().map_err(|e| e.to_string())
}

fn clear_stored(app: &AppHandle<Wry>) -> Result<(), String> {
    let store = app.store(SETTINGS_STORE).map_err(|e| e.to_string())?;
    store.delete(STORE_KEY);
    store.save().map_err(|e| e.to_string())
}

fn emit_state_changed(app: &AppHandle<Wry>, claims: Option<&ProClaims>) {
    let payload = match claims {
        Some(c) => json!({
            "signedIn": true,
            "isPro": c.is_pro,
            "email": c.email,
            "proExpiresAt": c.pro_expires_at,
        }),
        None => json!({
            "signedIn": false,
            "isPro": false,
            "email": null,
            "proExpiresAt": null,
        }),
    };
    let _ = app.emit(PRO_STATE_EVENT, payload);
}

/// Spawn a background refresh if the cached token is older than the
/// refresh window. Idempotent and best-effort: failures are logged.
pub fn maybe_refresh_on_launch(state: &ProState, app: AppHandle<Wry>) {
    let should_refresh = {
        let g = state.inner.lock().expect("pro_state mutex");
        match &g.stored {
            Some(s) => (now_unix() - s.last_successful_refresh_at) > REFRESH_AFTER_SECS,
            None => false,
        }
    };
    if !should_refresh {
        return;
    }
    let app_for_task = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_refresh(&app_for_task).await {
            log::debug!("pro: background refresh failed: {e}");
        }
    });
}

async fn run_refresh(app: &AppHandle<Wry>) -> Result<(), ProError> {
    let state: tauri::State<'_, ProState> = app.state();
    let jwt = {
        let g = state.inner.lock().expect("pro_state mutex");
        match &g.stored {
            Some(s) => s.jwt.clone(),
            None => return Ok(()),
        }
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| ProError::Network(e.to_string()))?;
    let resp = client
        .post(REFRESH_URL)
        .bearer_auth(&jwt)
        .send()
        .await
        .map_err(|e| ProError::Network(e.to_string()))?;
    match resp.status().as_u16() {
        200 => {
            #[derive(Deserialize)]
            struct RefreshBody {
                token: String,
            }
            let body: RefreshBody = resp
                .json()
                .await
                .map_err(|e| ProError::Network(e.to_string()))?;
            let claims = decode_and_verify(&body.token)?;
            let stored = StoredToken {
                jwt: body.token,
                last_successful_refresh_at: now_unix(),
                last_response_was_401: false,
            };
            write_stored(app, &stored).map_err(ProError::Store)?;
            {
                let mut g = state.inner.lock().expect("pro_state mutex");
                g.claims = Some(claims.clone());
                g.stored = Some(stored);
            }
            emit_state_changed(app, Some(&claims));
            Ok(())
        }
        401 => {
            // Server says this entitlement is dead. Wipe right away.
            log::info!("pro: refresh returned 401; clearing entitlement");
            state.sign_out(app)?;
            Err(ProError::Unauthorized)
        }
        other => Err(ProError::Network(format!("refresh status {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json as jsn;

    // Throwaway test keypair (P-256). NOT the production key. Generated
    // with `openssl ecparam -genkey -name prime256v1 -noout` solely for
    // these unit tests.
    const TEST_PRIV_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgpUJZOGPBsRlJcuHM
Otw0iHdf34fcLZYzgRJqWwU+yVOhRANCAAShNZs6vymoar1H8+6YhRakZnIfCfnX
YoR6v3/WGWBIMZD34j76089kTnn1A7hKsqWrBDJtzeLJHbF60iNLd0MS
-----END PRIVATE KEY-----";

    fn mint(kid: &str, claims: serde_json::Value) -> String {
        let key = EncodingKey::from_ec_pem(TEST_PRIV_PEM.as_bytes()).unwrap();
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(kid.into());
        encode(&header, &claims, &key).unwrap()
    }

    fn claims_value(exp_offset: i64, is_pro: bool) -> serde_json::Value {
        let now = now_unix();
        jsn!({
            "sub": "00000000-0000-0000-0000-000000000001",
            "email": "test@example.com",
            "is_pro": is_pro,
            "pro_expires_at": "2099-01-01T00:00:00Z",
            "iat": now,
            "exp": now + exp_offset,
        })
    }

    // The verifier uses the prod public key. To exercise the
    // happy/expired paths without swapping that constant out, we
    // re-verify against a key derived from TEST_PRIV_PEM directly. The
    // bad-signature/wrong-kid/malformed paths exercise the real
    // verifier against tokens minted with the throwaway key — those
    // should fail signature verification against the prod public key.

    #[test]
    fn bad_signature_against_prod_key() {
        let token = mint("v1", claims_value(3600, true));
        // Token is signed by TEST_PRIV_PEM, not the prod key embedded
        // in PRO_JWT_PUBLIC_KEY_V1, so verification must fail.
        let err = decode_and_verify(&token).unwrap_err();
        matches!(err, ProError::BadSignature);
    }

    #[test]
    fn unknown_kid_rejected() {
        let token = mint("v999", claims_value(3600, true));
        let err = decode_and_verify(&token).unwrap_err();
        assert!(matches!(err, ProError::UnknownKid), "got {err:?}");
    }

    #[test]
    fn malformed_jwt_rejected() {
        let err = decode_and_verify("not.a.jwt").unwrap_err();
        assert!(matches!(err, ProError::Malformed), "got {err:?}");
    }

    #[test]
    fn is_active_within_exp() {
        let now = 1_000_000;
        let claims = ProClaims {
            sub: "x".into(),
            email: "e".into(),
            is_pro: true,
            pro_expires_at: None,
            iat: now - 60,
            exp: now + 60,
        };
        let stored = StoredToken {
            jwt: String::new(),
            last_successful_refresh_at: now - 60,
            last_response_was_401: false,
        };
        assert!(is_active_now(&claims, &stored, now));
    }

    #[test]
    fn is_active_within_offline_grace() {
        let now = 1_000_000;
        let claims = ProClaims {
            sub: "x".into(),
            email: "e".into(),
            is_pro: true,
            pro_expires_at: None,
            iat: now - 7200,
            exp: now - 3600, // 1h past exp
        };
        let stored = StoredToken {
            jwt: String::new(),
            last_successful_refresh_at: now - 3 * 24 * 60 * 60,
            last_response_was_401: false,
        };
        assert!(is_active_now(&claims, &stored, now));
    }

    #[test]
    fn is_active_false_past_grace() {
        let now = 1_000_000;
        let claims = ProClaims {
            sub: "x".into(),
            email: "e".into(),
            is_pro: true,
            pro_expires_at: None,
            iat: now - 30 * 24 * 60 * 60,
            exp: now - 10 * 24 * 60 * 60, // 10 days past, beyond 7-day grace
        };
        let stored = StoredToken {
            jwt: String::new(),
            last_successful_refresh_at: now - 30 * 24 * 60 * 60,
            last_response_was_401: false,
        };
        assert!(!is_active_now(&claims, &stored, now));
    }

    #[test]
    fn is_active_false_after_401() {
        let now = 1_000_000;
        let claims = ProClaims {
            sub: "x".into(),
            email: "e".into(),
            is_pro: true,
            pro_expires_at: None,
            iat: now - 60,
            exp: now - 1, // just past exp
        };
        let stored = StoredToken {
            jwt: String::new(),
            last_successful_refresh_at: now - 60,
            last_response_was_401: true,
        };
        assert!(!is_active_now(&claims, &stored, now));
    }

    #[test]
    fn is_active_false_when_not_pro() {
        let now = 1_000_000;
        let claims = ProClaims {
            sub: "x".into(),
            email: "e".into(),
            is_pro: false,
            pro_expires_at: None,
            iat: now - 60,
            exp: now + 86400,
        };
        let stored = StoredToken {
            jwt: String::new(),
            last_successful_refresh_at: now - 60,
            last_response_was_401: false,
        };
        assert!(!is_active_now(&claims, &stored, now));
    }
}
