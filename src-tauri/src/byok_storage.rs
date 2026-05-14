// BYOK API key storage.
//
// On Linux desktops with a running secret-service daemon (gnome-keyring,
// kwallet with secret-service plug-in) we store cleanup-provider API
// keys in the OS keyring via the `keyring` crate (sync-secret-service
// backend, pure-Rust crypto). On systems without a secret-service
// daemon (Sway with no portal, headless sessions, minimal WMs) we fall
// back to the existing plaintext `settings.json` location. The mode is
// chosen at startup via a probe and surfaced to the UI so users see
// honest copy about where their key lives.
//
// Storage layout
// - Keyring: service "murmur", user "byok-<provider>" (e.g. "byok-groq").
// - Plaintext fallback: settings.json -> cleanup.keys.<provider>
//
// Per-provider keys mean switching provider in the UI no longer wipes
// the previous key. The active provider is still selected by
// cleanup.provider (unchanged from v0.3.9).

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use tauri::{AppHandle, Wry};
use tauri_plugin_store::StoreExt;

pub const KEYRING_SERVICE: &str = "murmur";
pub const SETTINGS_STORE: &str = "settings.json";
pub const CLEANUP_KEY: &str = "cleanup";
const MIGRATION_FLAG: &str = "storage_migrated_v1";
const LEGACY_API_KEY_FIELD: &str = "apiKey";
const KEYS_MAP_FIELD: &str = "keys";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageMode {
    Keyring,
    Plaintext,
}

impl StorageMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            StorageMode::Keyring => "keyring",
            StorageMode::Plaintext => "plaintext",
        }
    }
}

pub struct ByokStorage {
    mode: StorageMode,
    app: AppHandle<Wry>,
}

impl ByokStorage {
    /// Probe the OS keyring once at startup and cache the result.
    pub fn detect(app: AppHandle<Wry>) -> Self {
        let mode = if keyring_probe_ok() {
            StorageMode::Keyring
        } else {
            StorageMode::Plaintext
        };
        log::info!("byok: storage mode = {}", mode.as_str());
        Self { mode, app }
    }

    pub fn mode(&self) -> StorageMode {
        self.mode
    }

    pub fn set_key(&self, provider: &str, key: &str) -> Result<()> {
        validate_provider(provider)?;
        let key = key.trim();
        if key.is_empty() {
            return self.clear_key(provider);
        }
        match self.mode {
            StorageMode::Keyring => keyring_set(provider, key),
            StorageMode::Plaintext => self.plaintext_set(provider, key),
        }
    }

    pub fn get_key(&self, provider: &str) -> Result<Option<String>> {
        validate_provider(provider)?;
        match self.mode {
            StorageMode::Keyring => keyring_get(provider),
            StorageMode::Plaintext => self.plaintext_get(provider),
        }
    }

    pub fn clear_key(&self, provider: &str) -> Result<()> {
        validate_provider(provider)?;
        match self.mode {
            StorageMode::Keyring => keyring_clear(provider),
            StorageMode::Plaintext => self.plaintext_clear(provider),
        }
    }

    pub fn has_key(&self, provider: &str) -> Result<bool> {
        Ok(self.get_key(provider)?.is_some())
    }

    pub fn key_hint(&self, provider: &str) -> Result<Option<String>> {
        Ok(self.get_key(provider)?.map(|k| key_hint_from(&k)))
    }

    /// One-shot migration from the v0.3.9 plaintext layout
    /// (cleanup.apiKey + cleanup.provider) to the new per-provider layout.
    ///
    /// Idempotent: short-circuits if `cleanup.storage_migrated_v1` is true.
    /// Safety: when in Keyring mode, the legacy plaintext field is erased
    /// only after a successful read-back round-trip. If read-back fails,
    /// the migration flag is NOT set, the plaintext key stays, and we'll
    /// try again next launch.
    pub fn migrate_once(&self) -> Result<()> {
        let store = self
            .app
            .store(SETTINGS_STORE)
            .context("byok migrate: open settings.json")?;
        let cleanup = store.get(CLEANUP_KEY).unwrap_or_else(|| json!({}));
        if cleanup
            .get(MIGRATION_FLAG)
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return Ok(());
        }

        let legacy_key = cleanup
            .get(LEGACY_API_KEY_FIELD)
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let provider = cleanup
            .get("provider")
            .and_then(Value::as_str)
            .unwrap_or("groq")
            .to_string();

        let mut next = cleanup.clone();
        let obj = next
            .as_object_mut()
            .ok_or_else(|| anyhow!("cleanup field is not an object"))?;

        if legacy_key.is_empty() {
            obj.insert(MIGRATION_FLAG.to_string(), Value::Bool(true));
            obj.remove(LEGACY_API_KEY_FIELD);
            store.set(CLEANUP_KEY, Value::Object(obj.clone()));
            store.save().context("byok migrate: save settings.json")?;
            log::info!("byok: migration v1 marker set (no legacy key to move)");
            return Ok(());
        }

        match self.mode {
            StorageMode::Keyring => {
                keyring_set(&provider, &legacy_key)
                    .with_context(|| format!("byok migrate: write keyring for {provider}"))?;
                let readback = keyring_get(&provider)
                    .with_context(|| format!("byok migrate: read-back keyring for {provider}"))?;
                if readback.as_deref() != Some(legacy_key.as_str()) {
                    log::error!("byok: keyring read-back mismatch; leaving plaintext key in place");
                    return Err(anyhow!("keyring round-trip mismatch"));
                }
                obj.remove(LEGACY_API_KEY_FIELD);
                obj.insert(MIGRATION_FLAG.to_string(), Value::Bool(true));
                store.set(CLEANUP_KEY, Value::Object(obj.clone()));
                store.save().context("byok migrate: save settings.json")?;
                log::info!(
                    "byok: migrated plaintext key for provider {} to OS keyring",
                    provider
                );
            }
            StorageMode::Plaintext => {
                // Move the legacy single-slot apiKey into the per-provider map.
                let keys_entry = obj
                    .entry(KEYS_MAP_FIELD.to_string())
                    .or_insert_with(|| json!({}));
                if let Some(keys_obj) = keys_entry.as_object_mut() {
                    keys_obj.insert(provider.clone(), Value::String(legacy_key));
                }
                obj.remove(LEGACY_API_KEY_FIELD);
                obj.insert(MIGRATION_FLAG.to_string(), Value::Bool(true));
                store.set(CLEANUP_KEY, Value::Object(obj.clone()));
                store.save().context("byok migrate: save settings.json")?;
                log::warn!(
                    "byok: no system keyring detected; plaintext key for {} moved into per-provider slot",
                    provider
                );
            }
        }
        Ok(())
    }

    // --- plaintext helpers ---

    fn plaintext_set(&self, provider: &str, key: &str) -> Result<()> {
        let store = self.app.store(SETTINGS_STORE)?;
        let mut cleanup = store.get(CLEANUP_KEY).unwrap_or_else(|| json!({}));
        let cleanup_obj = cleanup
            .as_object_mut()
            .ok_or_else(|| anyhow!("cleanup field is not an object"))?;
        let keys_entry = cleanup_obj
            .entry(KEYS_MAP_FIELD.to_string())
            .or_insert_with(|| json!({}));
        let keys_obj = keys_entry
            .as_object_mut()
            .ok_or_else(|| anyhow!("cleanup.keys is not an object"))?;
        keys_obj.insert(provider.to_string(), Value::String(key.to_string()));
        store.set(CLEANUP_KEY, Value::Object(cleanup_obj.clone()));
        store.save()?;
        Ok(())
    }

    fn plaintext_get(&self, provider: &str) -> Result<Option<String>> {
        let store = self.app.store(SETTINGS_STORE)?;
        let cleanup = store.get(CLEANUP_KEY).unwrap_or_else(|| json!({}));
        let key = cleanup
            .get(KEYS_MAP_FIELD)
            .and_then(|m| m.get(provider))
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        Ok(key)
    }

    fn plaintext_clear(&self, provider: &str) -> Result<()> {
        let store = self.app.store(SETTINGS_STORE)?;
        let mut cleanup = store.get(CLEANUP_KEY).unwrap_or_else(|| json!({}));
        if let Some(cleanup_obj) = cleanup.as_object_mut() {
            if let Some(keys_obj) = cleanup_obj
                .get_mut(KEYS_MAP_FIELD)
                .and_then(Value::as_object_mut)
            {
                keys_obj.remove(provider);
            }
            store.set(CLEANUP_KEY, Value::Object(cleanup_obj.clone()));
            store.save()?;
        }
        Ok(())
    }
}

fn validate_provider(provider: &str) -> Result<()> {
    if matches!(provider, "groq" | "anthropic" | "xai") {
        Ok(())
    } else {
        Err(anyhow!("unknown cleanup provider: {provider}"))
    }
}

fn entry_user_for(provider: &str) -> String {
    format!("byok-{provider}")
}

fn keyring_set(provider: &str, key: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &entry_user_for(provider))
        .context("keyring: new entry")?;
    entry.set_password(key).context("keyring: set_password")
}

fn keyring_get(provider: &str) -> Result<Option<String>> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &entry_user_for(provider))
        .context("keyring: new entry")?;
    match entry.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(anyhow!("keyring: get_password: {err}")),
    }
}

fn keyring_clear(provider: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &entry_user_for(provider))
        .context("keyring: new entry")?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(anyhow!("keyring: delete_credential: {err}")),
    }
}

fn keyring_probe_ok() -> bool {
    let user = format!("__probe__-{}", std::process::id());
    let value = format!("probe-{}", std::process::id());
    let entry = match keyring::Entry::new(KEYRING_SERVICE, &user) {
        Ok(e) => e,
        Err(err) => {
            log::debug!("byok probe: entry create failed: {err}");
            return false;
        }
    };
    if let Err(err) = entry.set_password(&value) {
        log::debug!("byok probe: set failed: {err}");
        return false;
    }
    let read = entry.get_password();
    let _ = entry.delete_credential();
    match read {
        Ok(got) if got == value => true,
        Ok(other) => {
            log::warn!("byok probe: read-back mismatch (got {} bytes)", other.len());
            false
        }
        Err(err) => {
            log::debug!("byok probe: read failed: {err}");
            false
        }
    }
}

fn key_hint_from(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.len() <= 4 {
        "****".to_string()
    } else {
        let tail: String = trimmed
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("****{tail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_validation_accepts_known_providers() {
        for p in ["groq", "anthropic", "xai"] {
            assert!(validate_provider(p).is_ok(), "{p} should be accepted");
        }
    }

    #[test]
    fn provider_validation_rejects_unknown() {
        for p in ["openai", "", "GROQ", "groq ", "../etc"] {
            assert!(validate_provider(p).is_err(), "{p} should be rejected");
        }
    }

    #[test]
    fn entry_user_format_is_stable() {
        assert_eq!(entry_user_for("groq"), "byok-groq");
        assert_eq!(entry_user_for("anthropic"), "byok-anthropic");
        assert_eq!(entry_user_for("xai"), "byok-xai");
    }

    #[test]
    fn key_hint_masks_short_keys_fully() {
        assert_eq!(key_hint_from(""), "****");
        assert_eq!(key_hint_from("abcd"), "****");
        assert_eq!(key_hint_from("ab"), "****");
    }

    #[test]
    fn key_hint_shows_last_four_for_real_keys() {
        assert_eq!(key_hint_from("gsk_AbCdEf1234"), "****1234");
        assert_eq!(key_hint_from("sk-ant-api03-xyz789"), "****z789");
    }

    #[test]
    fn key_hint_trims_whitespace() {
        assert_eq!(key_hint_from("  gsk_AbCdEf1234  "), "****1234");
    }

    #[test]
    fn storage_mode_strings_are_stable() {
        // The UI compares against these strings; changing them breaks
        // AICleanupSection.tsx's mode-driven copy branch.
        assert_eq!(StorageMode::Keyring.as_str(), "keyring");
        assert_eq!(StorageMode::Plaintext.as_str(), "plaintext");
    }
}
