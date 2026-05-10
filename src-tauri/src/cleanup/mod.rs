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
    #[error("authentication failed (invalid API key)")]
    Auth,
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
