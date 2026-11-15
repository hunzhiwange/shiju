use super::*;
use crate::state::{ModelSummary, ProviderModelsSummary};
#[test]
fn provider_connect_header_and_custom_drafts_have_empty_defaults() {
    let connect = ProviderConnectState::default();
    assert!(connect.provider_id.is_empty());
    assert!(connect.provider_name.is_empty());
    assert!(connect.api_key.is_empty());

    let header = ProviderHeaderDraft::default();
    assert!(header.key.is_empty());
    assert!(header.value.is_empty());

    let model = CustomProviderModelDraft::default();
    assert!(model.model_id.is_empty());
    assert!(model.display_name.is_empty());
    assert!(model.family.is_empty());
    assert!(model.context_limit.is_empty());
    assert!(model.input_limit.is_empty());
    assert!(model.output_limit.is_empty());
    assert!(model.reasoning.is_none());
    assert!(model.tool_call.is_none());
    assert!(model.temperature.is_none());
    assert!(model.attachment.is_none());
    assert!(model.interleaved.is_none());
    assert!(model.input_modalities.is_none());
    assert!(model.output_modalities.is_none());
    assert!(model.enabled);
    assert!(!model.from_catalog);

    let provider = CustomProviderDraft::default();
    assert!(provider.provider_id.is_empty());
    assert!(provider.display_name.is_empty());
    assert!(provider.base_url.is_empty());
    assert!(provider.api_key.is_empty());
    assert_eq!(provider.headers.len(), 1);
    assert_eq!(provider.models.len(), 1);
}

#[test]
fn provider_settings_default_populates_popular_patterns_and_modal_state() {
    let state = ProviderSettingsState::default();

    assert!(!state.loading);
    assert!(!state.models_syncing);
    assert_eq!(state.models_sync_progress, 0.0);
    assert!(state.models_sync_label.is_empty());
    assert!(state.providers.is_empty());
    assert_eq!(
        state.popular_patterns,
        vec![
            "OpenCode Zen".to_string(),
            "Anthropic".to_string(),
            "GitHub Copilot".to_string(),
            "OpenAI".to_string(),
            "Google".to_string(),
            "OpenRouter".to_string(),
            "Vercel AI Gateway".to_string(),
        ]
    );
    assert!(!state.catalog_loading);
    assert!(!state.catalog_open);
    assert!(state.catalog_query.is_empty());
    assert!(state.catalog_items.is_empty());
    assert!(state.connect_modal.is_none());
    assert!(state.connect_error.is_none());
    assert!(state.disconnect_confirm_provider_id.is_none());
    assert!(!state.custom_provider_modal_open);
    assert_eq!(state.custom_provider_tab, CustomProviderTab::Basic);
    assert!(state.custom_editing_provider_id.is_none());
    assert!(!state.custom_models_discovering);
    assert_eq!(state.custom_models_filter, CustomProviderModelFilter::Enabled);
    assert!(state.custom_model_modal.is_none());
    assert!(state.save_error.is_none());
    assert!(state.popular_patterns.iter().all(|pattern| !is_shiju_official_provider_id(pattern)));
    assert_eq!(SHIJU_OFFICIAL_PROVIDER_ID, "shiju");
    assert_eq!(SHIJU_OFFICIAL_PROVIDER_NAME, "识矩");
}

#[test]
fn provider_and_model_summary_structs_preserve_public_fields() {
    let provider = ProviderSummary {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        source_label: "config".to_string().into(),
        connected: true,
    };
    assert_eq!(provider.id, "openai");
    assert_eq!(provider.name, "OpenAI");
    assert_eq!(provider.source_label, "config");
    assert!(provider.connected);

    let catalog = ModelCatalogEntry {
        provider_id: "anthropic".to_string(),
        provider_name: "Anthropic".to_string(),
        model_id: "claude".to_string(),
        model_name: "Claude".to_string(),
    };
    assert_eq!(catalog.provider_id, "anthropic");
    assert_eq!(catalog.provider_name, "Anthropic");
    assert_eq!(catalog.model_id, "claude");
    assert_eq!(catalog.model_name, "Claude");

    let model = ModelSummary {
        id: "gpt".to_string(),
        name: "GPT".to_string(),
        enabled: true,
        toolcall: true,
        attachment: false,
        context_limit: 128_000,
        detail: serde_json::json!({"family": "test"}),
    };
    assert_eq!(model.id, "gpt");
    assert_eq!(model.name, "GPT");
    assert!(model.enabled);
    assert!(model.toolcall);
    assert!(!model.attachment);
    assert_eq!(model.context_limit, 128_000);
    assert_eq!(model.detail["family"], "test");

    let provider_models = ProviderModelsSummary {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        models: vec![model],
    };
    assert_eq!(provider_models.models.len(), 1);
}
