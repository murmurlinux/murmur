#[derive(Debug, Clone)]
pub struct SanityConfig {
    pub min_length_ratio: f32,
    pub max_length_ratio: f32,
}

impl Default for SanityConfig {
    fn default() -> Self {
        Self {
            min_length_ratio: 0.5,
            max_length_ratio: 2.0,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SanityReason {
    #[error("empty output")]
    Empty,
    #[error("output too short (ratio {ratio:.2} < min {min:.2})")]
    TooShort { ratio: f32, min: f32 },
    #[error("output too long (ratio {ratio:.2} > max {max:.2})")]
    TooLong { ratio: f32, max: f32 },
    #[error("output contains prompt echo markers")]
    PromptEcho,
}

pub fn check_output(input: &str, output: &str, cfg: &SanityConfig) -> Result<(), SanityReason> {
    if output.trim().is_empty() {
        return Err(SanityReason::Empty);
    }
    let lower = output.to_lowercase();
    const ECHO_MARKERS: &[&str] = &[
        "as an ai language model",
        "i cannot fulfill",
        "here is the cleaned",
        "here is the corrected",
        "sure! here",
    ];
    for marker in ECHO_MARKERS {
        if lower.contains(marker) {
            return Err(SanityReason::PromptEcho);
        }
    }
    let in_len = input.chars().count() as f32;
    let out_len = output.chars().count() as f32;
    if in_len > 0.0 {
        let ratio = out_len / in_len;
        if ratio < cfg.min_length_ratio {
            return Err(SanityReason::TooShort {
                ratio,
                min: cfg.min_length_ratio,
            });
        }
        if ratio > cfg.max_length_ratio {
            return Err(SanityReason::TooLong {
                ratio,
                max: cfg.max_length_ratio,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_good_output() {
        check_output(
            "hello world this is a test",
            "Hello world, this is a test.",
            &SanityConfig::default(),
        )
        .unwrap();
    }

    #[test]
    fn rejects_empty() {
        let err = check_output("hello world", "   ", &SanityConfig::default()).unwrap_err();
        assert!(matches!(err, SanityReason::Empty));
    }

    #[test]
    fn rejects_too_short() {
        let err = check_output(
            "hello world this is a longer sentence",
            "Hi.",
            &SanityConfig::default(),
        )
        .unwrap_err();
        assert!(matches!(err, SanityReason::TooShort { .. }));
    }

    #[test]
    fn rejects_too_long() {
        let err = check_output("hi", &"x".repeat(200), &SanityConfig::default()).unwrap_err();
        assert!(matches!(err, SanityReason::TooLong { .. }));
    }

    #[test]
    fn rejects_prompt_echo() {
        let err = check_output(
            "hello world this is a test",
            "Here is the cleaned text: Hello world, this is a test.",
            &SanityConfig::default(),
        )
        .unwrap_err();
        assert!(matches!(err, SanityReason::PromptEcho));
    }
}
