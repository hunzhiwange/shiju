use super::openclaw_model::*;
use vw_gateway_client::vw_api_types::openclaw_settings::OpenClawDefaultModel;

#[test]
fn openclaw_model_editor_preserves_selection_missing_from_catalog() {
    let options = choices(&[], "old/model", &[]);
    assert_eq!(options.len(), 2);
    assert!(options[1].to_string().contains("old/model"));
}

#[test]
fn openclaw_model_editor_preserves_selection_and_extensions_deduplicating_fallbacks() {
    let mut model = OpenClawDefaultModel {
        primary: "provider/main".into(),
        fallbacks: vec!["provider/main".into(), "old/model".into(), "old/model".into(), String::new()],
        extra: [("future".into(), serde_json::json!({"keep": true}))].into(),
    };
    normalize(&mut model);
    assert_eq!(model.primary, "provider/main");
    assert_eq!(model.fallbacks, vec!["old/model"]);
    assert_eq!(model.extra["future"], serde_json::json!({"keep": true}));
}
