//! Portable Pi prompt-template names, matching the upstream Rust validator.

const MAX_TEMPLATE_SLUG_BYTES: usize = 128;

/// Validate one slash-command token without trimming or normalizing it.
///
/// The limit counts UTF-8 bytes. Internal dots and Unicode are accepted, but
/// whitespace, control characters, path separators and portable filename
/// hazards are rejected on every host, including macOS.
/// This validates a name only; it does not authorize filesystem access.
pub fn validate_template_slug(slug: &str) -> Result<(), String> {
    let basename = slug.split('.').next().unwrap_or_default();
    let windows_reserved = matches!(
        basename.to_ascii_lowercase().as_str(),
        "con"
            | "prn"
            | "aux"
            | "nul"
            | "com1"
            | "com2"
            | "com3"
            | "com4"
            | "com5"
            | "com6"
            | "com7"
            | "com8"
            | "com9"
            | "lpt1"
            | "lpt2"
            | "lpt3"
            | "lpt4"
            | "lpt5"
            | "lpt6"
            | "lpt7"
            | "lpt8"
            | "lpt9"
    );
    let valid = !slug.is_empty()
        && slug.len() <= MAX_TEMPLATE_SLUG_BYTES
        && !slug.starts_with('.')
        && !slug.ends_with('.')
        && !windows_reserved
        && slug.chars().all(|character| {
            !character.is_control()
                && !character.is_whitespace()
                && !matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
        });
    if valid {
        Ok(())
    } else {
        Err("Pi prompt-template name must be one portable slash-command token".to_string())
    }
}
