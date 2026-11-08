//! Explicit User-Agent overrides for the model catalog.

use reqwest::header::HeaderValue;

use crate::providers::validation::header_value;

/// Preset values in display order. The selected string uses the same path as custom input.
pub const USER_AGENT_PRESETS: &[&str] = &[
    "claude-cli/2.1.161 (external, cli)",
    "claude-cli/2.1.161",
    "claude-code/1.0.0",
    "claude-code/0.1.0",
    "Kilo-Code/1.0",
];

/// Resolve only an explicitly supplied value. An absent or blank value leaves the request alone.
pub fn custom_user_agent(value: Option<&str>) -> Result<Option<HeaderValue>, &'static str> {
    // Keep illegal control characters at the edges so the header parser can reject them.
    let value = value.map(|value| {
        value.trim_matches(|c: char| c.is_whitespace() && (!c.is_ascii_control() || c == '\t'))
    });
    value
        .filter(|value| !value.is_empty())
        .map(|value| header_value(value).map_err(|_| "invalid custom User-Agent"))
        .transpose()
}
