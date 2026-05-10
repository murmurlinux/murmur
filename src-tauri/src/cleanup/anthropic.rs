use std::time::Duration;

use crate::cleanup::{parse_error_detail, prompt, CleanupError, CleanupService};

const ANTHROPIC_DEFAULT_BASE: &str = "https://api.anthropic.com";
const ANTHROPIC_MODEL: &str = "claude-haiku-4-5";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 4096;

pub struct AnthropicCleanup {
    api_key: String,
    base_url: String,
    timeout: Duration,
    model: String,
}

impl AnthropicCleanup {
    pub fn new(api_key: &str, timeout: Duration) -> Self {
        Self::new_with_base(api_key, ANTHROPIC_DEFAULT_BASE, timeout)
    }

    pub fn new_with_base(api_key: &str, base_url: &str, timeout: Duration) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            timeout,
            model: ANTHROPIC_MODEL.to_string(),
        }
    }
}

impl CleanupService for AnthropicCleanup {
    fn cleanup(&self, text: &str, _language: &str) -> Result<String, CleanupError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| CleanupError::Network(e.to_string()))?;

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "system": prompt::build_system_prompt(),
            "messages": [
                { "role": "user", "content": prompt::build_user_message(text) }
            ],
        });

        let url = format!("{}/v1/messages", self.base_url);
        let response: reqwest::blocking::Response = match client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body)
            .send()
        {
            Ok(r) => r,
            Err(e) if e.is_timeout() => return Err(CleanupError::Timeout(self.timeout)),
            Err(e) => return Err(CleanupError::Network(e.to_string())),
        };

        let status = response.status();
        let body_text = response
            .text()
            .map_err(|e: reqwest::Error| CleanupError::Network(e.to_string()))?;

        match status.as_u16() {
            200..=299 => {}
            401 | 403 => {
                return Err(CleanupError::Auth {
                    detail: parse_error_detail(&body_text),
                })
            }
            429 => return Err(CleanupError::RateLimit),
            s => {
                return Err(CleanupError::ProviderError {
                    status: s,
                    body: body_text,
                })
            }
        }

        let json: serde_json::Value = serde_json::from_str(&body_text)
            .map_err(|e| CleanupError::Network(format!("parse: {e}")))?;
        let text_out = json
            .pointer("/content/0/text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CleanupError::ProviderError {
                status: 200,
                body: body_text.clone(),
            })?
            .trim()
            .to_string();

        Ok(text_out)
    }
}
