// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::state::*;
use crate::provider_state::*;
#[test]
fn model_detail_and_model_settings_defaults_are_stable() {
    let custom_modal = CustomProviderModelModalState {
        active_tab: CustomProviderModelTab::Capabilities,
        edit_index: Some(2),
        model_id: "claude-sonnet".to_string(),
        display_name: "Claude Sonnet".to_string(),
        family: "claude".to_string(),
        context_limit: "200000".to_string(),
        input_limit: "190000".to_string(),
        output_limit: "10000".to_string(),
        reasoning: Some(true),
        tool_call: Some(true),
        temperature: Some(false),
        attachment: Some(true),
        interleaved: Some(true),
        input_modalities: Some(vec!["text".to_string(), "image".to_string()]),
        output_modalities: Some(vec!["text".to_string()]),
        inherited_capabilities: None,
        original_model_id: Some("claude-sonnet".to_string()),
    };
    assert_eq!(custom_modal.active_tab, CustomProviderModelTab::Capabilities);
    assert_eq!(custom_modal.edit_index, Some(2));
    assert_eq!(custom_modal.model_id, "claude-sonnet");
    assert_eq!(custom_modal.display_name, "Claude Sonnet");
    assert_eq!(custom_modal.family, "claude");
    assert_eq!(custom_modal.context_limit, "200000");
    assert_eq!(custom_modal.input_limit, "190000");
    assert_eq!(custom_modal.output_limit, "10000");
    assert_eq!(custom_modal.reasoning, Some(true));
    assert_eq!(custom_modal.tool_call, Some(true));
    assert_eq!(custom_modal.temperature, Some(false));
    assert_eq!(custom_modal.attachment, Some(true));
    assert_eq!(custom_modal.interleaved, Some(true));
    assert_eq!(
        custom_modal.input_modalities.as_deref(),
        Some(["text".to_string(), "image".to_string()].as_slice())
    );
    assert_eq!(custom_modal.output_modalities.as_deref(), Some(["text".to_string()].as_slice()));

    let row =
        ModelDetailRow { label: "Context".to_string().into(), value: "128k".to_string().into() };
    let modal = ModelDetailModalState {
        provider_id: "openai".to_string(),
        provider_name: "OpenAI".to_string(),
        model_id: "gpt".to_string(),
        model_name: "GPT".to_string(),
        rows: vec![row],
        raw_json: "{}".to_string(),
        show_raw: true,
    };
    assert_eq!(modal.provider_id, "openai");
    assert_eq!(modal.provider_name, "OpenAI");
    assert_eq!(modal.model_id, "gpt");
    assert_eq!(modal.model_name, "GPT");
    assert_eq!(modal.rows[0].label, "Context");
    assert_eq!(modal.rows[0].value, "128k");
    assert_eq!(modal.raw_json, "{}");
    assert!(modal.show_raw);

    let state = ModelSettingsState::default();
    assert!(!state.loading);
    assert!(state.query.is_empty());
    assert!(state.providers.is_empty());
    assert!(state.save_error.is_none());
    assert!(state.detail_modal.is_none());
}
}

mod behavior_1 {
use crate::app_init::state::*;
#[test]
fn route_classification_and_heartbeat_defaults_are_ready_to_edit_0() {
    let embedding = EmbeddingRouteDraft::default();
    assert!(embedding.pattern.is_empty());
    assert!(embedding.provider.is_empty());
    assert!(embedding.model.is_empty());
    assert!(embedding.dimensions.is_empty());
    assert!(embedding.api_key_input.is_empty());
    let embedding_routes = EmbeddingRoutesSettingsState::default();
    assert!(embedding_routes.routes.is_empty());
    assert!(embedding_routes.save_error.is_none());
    assert!(!embedding_routes.save_success);


}
}
