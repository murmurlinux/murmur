use std::time::Duration;

use crate::cleanup::{parse_error_detail, prompt, CleanupError, CleanupService};

const GROQ_DEFAULT_BASE: &str = "https://api.groq.com";
const GROQ_MODEL: &str = "llama-3.3-70b-versatile";

pub struct GroqCleanup {
    api_key: String,
    base_url: String,
    timeout: Duration,
    model: String,
}

impl GroqCleanup {
    pub fn new(api_key: &str, timeout: Duration) -> Self {
        Self::new_with_base(api_key, GROQ_DEFAULT_BASE, timeout)
    }

    pub fn new_with_base(api_key: &str, base_url: &str, timeout: Duration) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            timeout,
            model: GROQ_MODEL.to_string(),
        }
    }
}

impl CleanupService for GroqCleanup {
    fn cleanup(&self, text: &str, _language: &str) -> Result<String, CleanupError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| CleanupError::Network(e.to_string()))?;

        let body = serde_json::json!({
            "model": self.model,
            "temperature": 0.0,
            "messages": [
                { "role": "system", "content": prompt::build_system_prompt() },
                { "role": "user", "content": prompt::build_user_message(text) },
            ],
        });

        let url = format!("{}/openai/v1/chat/completions", self.base_url);
        let response: reqwest::blocking::Response = match client
            .post(&url)
            .bearer_auth(&self.api_key)
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
        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CleanupError::ProviderError {
                status: 200,
                body: body_text.clone(),
            })?
            .trim()
            .to_string();

        Ok(content)
    }
}
