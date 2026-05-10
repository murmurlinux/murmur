//! LLM cleanup post-processing (Pro feature #14).

pub mod anthropic;
pub mod factory;
pub mod groq;
pub mod prompt;
pub mod sanity;

use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CleanupError {
    #[error("cleanup request timed out after {0:?}")]
    Timeout(Duration),
    #[error("network error: {0}")]
    Network(String),
    #[error("authentication failed: {detail}")]
    Auth { detail: String },
    #[error("provider rate-limited the request")]
    RateLimit,
    #[error("provider error {status}: {body}")]
    ProviderError { status: u16, body: String },
    #[error("sanity check failed: {0}")]
    SanityFailed(#[from] sanity::SanityReason),
}

pub trait CleanupService: Send + Sync {
    fn cleanup(&self, text: &str, language: &str) -> Result<String, CleanupError>;
}

/// Extract a human-readable detail from a provider's error response body.
/// Both Groq and Anthropic put the user-visible reason at `error.message`
/// when something fails; if parsing fails, fall back to the first 120
/// characters of the raw body so VPN/origin-block style messages still
/// reach the UI instead of being collapsed to a generic "invalid key".
pub fn parse_error_detail(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "no response body from provider".to_string();
    }
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(msg) = json
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            return msg.to_string();
        }
    }
    let max = 120;
    if trimmed.chars().count() > max {
        let truncated: String = trimmed.chars().take(max).collect();
        format!("{}...", truncated)
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod parse_error_detail_tests {
    use super::parse_error_detail;

    #[test]
    fn extracts_groq_style_message() {
        let body = r#"{"error":{"message":"Invalid API Key","type":"invalid_request_error"}}"#;
        assert_eq!(parse_error_detail(body), "Invalid API Key");
    }

    #[test]
    fn extracts_anthropic_style_message() {
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        assert_eq!(parse_error_detail(body), "invalid x-api-key");
    }

    #[test]
    fn falls_back_to_raw_body_when_not_json() {
        let body = "Forbidden: origin not allowed";
        assert_eq!(parse_error_detail(body), "Forbidden: origin not allowed");
    }

    #[test]
    fn truncates_long_raw_body() {
        let body = "x".repeat(500);
        let out = parse_error_detail(&body);
        assert!(out.ends_with("..."));
        assert!(out.chars().count() <= 124);
    }

    #[test]
    fn handles_empty_body() {
        assert_eq!(parse_error_detail(""), "no response body from provider");
    }
}

pub fn run_cleanup(
    service: &dyn CleanupService,
    raw: &str,
    language: &str,
    sanity_cfg: &sanity::SanityConfig,
) -> Result<String, CleanupError> {
    let cleaned = service.cleanup(raw, language)?;
    sanity::check_output(raw, &cleaned, sanity_cfg)?;
    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubOk(&'static str);
    impl CleanupService for StubOk {
        fn cleanup(&self, _t: &str, _l: &str) -> Result<String, CleanupError> {
            Ok(self.0.to_string())
        }
    }

    #[test]
    fn run_cleanup_passes_sanity_on_good_output() {
        let svc = StubOk("Hello world, this is a test.");
        let out = run_cleanup(
            &svc,
            "hello world this is a test",
            "en",
            &sanity::SanityConfig::default(),
        )
        .unwrap();
        assert_eq!(out, "Hello world, this is a test.");
    }

    #[test]
    fn run_cleanup_rejects_empty_output() {
        let svc = StubOk("");
        let err = run_cleanup(
            &svc,
            "hello world this is a test",
            "en",
            &sanity::SanityConfig::default(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            CleanupError::SanityFailed(sanity::SanityReason::Empty)
        ));
    }
}
