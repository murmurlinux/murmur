//! Baked-in prompt templates for the v1 cleanup pipeline.

pub fn build_system_prompt() -> &'static str {
    "You clean up voice-dictated text. Fix punctuation, capitalisation, and obvious typos. \
     Fix broken grammar and run-on sentences. Remove filler words like \"um\", \"uh\", \
     \"you know\", \"like\" when they are clearly disfluencies. \
     Preserve the user's wording and meaning exactly. Do not rephrase for style. \
     Do not add content. Do not summarise. Do not remove substantive words. \
     Return only the cleaned text with no commentary or quoting."
}

pub fn build_user_message(raw: &str) -> String {
    raw.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_is_non_empty_and_stable() {
        let p = build_system_prompt();
        assert!(p.len() > 200);
        assert!(p.contains("Preserve the user's wording"));
    }

    #[test]
    fn user_message_is_passthrough() {
        assert_eq!(build_user_message("hello world"), "hello world");
    }
}
