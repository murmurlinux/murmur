use std::time::Duration;

use crate::cleanup::{anthropic::AnthropicCleanup, groq::GroqCleanup, CleanupService};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

pub fn build_cleanup_service(
    provider: &str,
    api_key: &str,
    timeout: Duration,
) -> Result<Box<dyn CleanupService>, String> {
    let base = base_url_override(provider, |k| std::env::var(k).ok());
    build_cleanup_service_with_base(provider, api_key, timeout, base.as_deref())
}

/// Returns the env-var name that overrides the base URL for `provider`,
/// or None if the provider is not recognised.
fn override_env_var(provider: &str) -> Option<&'static str> {
    match provider {
        "groq" => Some("MURMUR_GROQ_LLM_BASE_URL"),
        "anthropic" => Some("MURMUR_ANTHROPIC_LLM_BASE_URL"),
        _ => None,
    }
}

/// Looks up the base-URL override for `provider` via the supplied env reader.
/// The reader is injected so unit tests can avoid touching process state.
fn base_url_override<F>(provider: &str, env: F) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    let var = override_env_var(provider)?;
    env(var).filter(|s| !s.is_empty())
}

fn build_cleanup_service_with_base(
    provider: &str,
    api_key: &str,
    timeout: Duration,
    base_url: Option<&str>,
) -> Result<Box<dyn CleanupService>, String> {
    match provider {
        "groq" => Ok(Box::new(match base_url {
            Some(b) => GroqCleanup::new_with_base(api_key, b, timeout),
            None => GroqCleanup::new(api_key, timeout),
        })),
        "anthropic" => Ok(Box::new(match base_url {
            Some(b) => AnthropicCleanup::new_with_base(api_key, b, timeout),
            None => AnthropicCleanup::new(api_key, timeout),
        })),
        other => Err(format!("unknown cleanup provider: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_groq() {
        assert!(build_cleanup_service("groq", "k", DEFAULT_TIMEOUT).is_ok());
    }

    #[test]
    fn builds_anthropic() {
        assert!(build_cleanup_service("anthropic", "k", DEFAULT_TIMEOUT).is_ok());
    }

    #[test]
    fn rejects_unknown_provider() {
        match build_cleanup_service("openai", "k", DEFAULT_TIMEOUT) {
            Err(msg) => assert!(msg.contains("unknown")),
            Ok(_) => panic!("expected error for unknown provider"),
        }
    }

    #[test]
    fn override_env_var_maps_known_providers() {
        assert_eq!(override_env_var("groq"), Some("MURMUR_GROQ_LLM_BASE_URL"));
        assert_eq!(
            override_env_var("anthropic"),
            Some("MURMUR_ANTHROPIC_LLM_BASE_URL")
        );
        assert_eq!(override_env_var("openai"), None);
    }

    #[test]
    fn base_url_override_returns_value_when_env_set() {
        let env = |k: &str| -> Option<String> {
            if k == "MURMUR_GROQ_LLM_BASE_URL" {
                Some("http://localhost:1234".to_string())
            } else {
                None
            }
        };
        assert_eq!(
            base_url_override("groq", env),
            Some("http://localhost:1234".to_string())
        );
    }

    #[test]
    fn base_url_override_returns_none_when_env_unset() {
        let env = |_k: &str| -> Option<String> { None };
        assert_eq!(base_url_override("groq", env), None);
    }

    #[test]
    fn base_url_override_treats_empty_as_unset() {
        let env = |_k: &str| -> Option<String> { Some(String::new()) };
        assert_eq!(base_url_override("groq", env), None);
    }

    #[test]
    fn base_url_override_returns_none_for_unknown_provider() {
        let env = |_k: &str| -> Option<String> { Some("http://x".to_string()) };
        assert_eq!(base_url_override("openai", env), None);
    }
}
