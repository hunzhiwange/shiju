use crate::app::state::{ModelSummary, ProviderModelsSummary};
use serde_json::json;
pub(in crate::app::components) fn model(
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

pub(in crate::app::components) fn provider(
    id: &str,
    models: Vec<ModelSummary>,
) -> ProviderModelsSummary {
    ProviderModelsSummary { id: id.to_string(), name: format!("提供者 {id}"), models }
}

#[test]
fn multimodal_search_keywords_include_ocr_and_image_model() {
    use crate::app::components::system_settings::{SystemTab, system_tab_matches_query};

    assert!(system_tab_matches_query(SystemTab::Models, "OCR"));
    assert!(system_tab_matches_query(SystemTab::Models, "图片模型"));
}
