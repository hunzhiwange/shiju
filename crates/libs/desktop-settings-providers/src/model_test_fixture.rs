#![allow(dead_code)] // Also imported by host cross-domain routing tests.
use desktop_settings_providers::state::{ModelSummary, ProviderModelsSummary};
use serde_json::json;
pub fn model(
    id: &str,
    enabled: bool,
    image_input: serde_json::Value,
) -> ModelSummary {
    ModelSummary {
        id: id.to_string(),
        name: format!("动态模型 {id}"),
        enabled,
        toolcall: false,
        attachment: false,
        context_limit: 8_192,
        detail: json!({
            "capabilities": {
                "input": {
                    "image": image_input,
                },
            },
        }),
    }
}

pub fn provider(
    id: &str,
    models: Vec<ModelSummary>,
) -> ProviderModelsSummary {
    ProviderModelsSummary { id: id.to_string(), name: format!("提供者 {id}"), models }
}
