//! Default draft for a new custom Codex provider.

use std::collections::BTreeMap;

pub struct CodexTemplate {
    /// The empty key is a placeholder for the caller to fill.
    pub auth: BTreeMap<String, String>,
    pub config: String,
}

/// Return an independently editable draft.
pub fn codex_custom_template() -> CodexTemplate {
    let config = r#"model_provider = "custom"
model = "gpt-5.6-sol"
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.custom]
name = "custom"
wire_api = "responses"
requires_openai_auth = true"#;

    CodexTemplate {
        auth: BTreeMap::from([("OPENAI_API_KEY".into(), String::new())]),
        config: config.into(),
    }
}
