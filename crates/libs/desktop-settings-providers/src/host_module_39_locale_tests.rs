use config_ui::DesktopLocale;

#[test]
fn desktop_i18n_settings_models_cached_details_translate_without_mutating_service_json() {
    use desktop_i18n::settings_models::connection_text::text as connection_text;
    use crate::state::{ModelSummary, ProviderModelsSummary};
    let mut state = crate::state::ModelSettingsState::default();
    let raw = serde_json::json!({"name": "原始模型 {} 🚀", "capabilities": {"toolcall": true}, "custom": "保留"});
    state.providers = vec![ProviderModelsSummary {
        id: "provider/保留".into(),
        name: "原始提供商".into(),
        models: vec![ModelSummary {
            id: "model/保留".into(),
            name: "原始模型 {} 🚀".into(),
            enabled: true,
            toolcall: true,
            attachment: false,
            context_limit: 1024,
            detail: raw.clone(),
        }],
    }];
    let _ = crate::model_update::update(
        &mut state,
        crate::model_update::ModelMessage::ModelDetailOpen(
            "provider/保留".into(),
            "model/保留".into(),
        ),
    );
    let modal = state.detail_modal.as_ref().unwrap();
    for (locale, title, tools_label, supported) in [
        (
            DesktopLocale::SimplifiedChinese,
            "服务端网关配置",
            "支持工具调用 (capabilities.toolcall)",
            "支持",
        ),
        (
            DesktopLocale::TraditionalChinese,
            "服務端網關配置",
            "支持工具調用 (capabilities.toolcall)",
            "支持",
        ),
        (
            DesktopLocale::English,
            "Server Gateway settings",
            "Tool calling (capabilities.toolcall)",
            "Supported",
        ),
    ] {
        assert_eq!(connection_text(locale, "服务端网关配置"), title);
        let tool_row = modal
            .rows
            .iter()
            .find(|row| row.label == "支持工具调用 (capabilities.toolcall)")
            .unwrap();
        assert_eq!(tool_row.label.display(locale), tools_label);
        assert_eq!(tool_row.value.display(locale), supported);
        assert_eq!(serde_json::from_str::<serde_json::Value>(&modal.raw_json).unwrap(), raw);
        assert_eq!(modal.model_name, "原始模型 {} 🚀");
    }
}

#[test]
fn desktop_i18n_settings_models_validation_and_sync_keep_semantics_after_switch() {
    use crate::app_init::state::EmbeddingRouteDraft;
    let mut state = crate::app_init::state::EmbeddingRoutesSettingsState::default();
    state.routes = vec![EmbeddingRouteDraft {
        pattern: String::new(),
        provider: "custom:原始/🚀".into(),
        model: "model/原始".into(),
        dimensions: "1024".into(),
        api_key_input: "原始-secret".into(),
    }];
    let _ = crate::embedding_routes_update::update(
        &mut state,
        crate::embedding_view_message::EmbeddingRoutesMessage::Save,
    );
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "第 1 条路由的 pattern 不能为空"),
        (DesktopLocale::TraditionalChinese, "第 1 條路由的 pattern 不能為空"),
        (DesktopLocale::English, "Route 1 pattern must not be empty"),
    ] {
        assert_eq!(state.save_error.as_ref().unwrap().display(locale), expected);
        assert_eq!(state.routes[0].provider, "custom:原始/🚀");
        assert_eq!(state.routes[0].api_key_input, "原始-secret");
    }
    let mut provider = crate::provider_state::ProviderSettingsState::default();
    let mut models = crate::state::ModelSettingsState::default();
    provider.loading = false;
    let endpoint = gateway_endpoint::GatewayEndpoint::default();
    let config = desktop_config::ConfigAccess::new(endpoint.clone(), None);
    let _ = crate::provider_update::update(
        &mut provider,
        &mut models,
        endpoint.clone(),
        config.clone(),
        false,
        crate::provider_message::SettingsMessage::ProviderModelsSyncRemote,
    );
    assert!(provider.models_sync_label.display(DesktopLocale::English).contains("Fetching"));
    provider.models_sync_progress = 0.34;
    let _ = crate::provider_update::update(
        &mut provider,
        &mut models,
        endpoint,
        config,
        false,
        crate::provider_message::SettingsMessage::ProviderModelsSyncTick,
    );
    assert_eq!(
        provider.models_sync_label.display(DesktopLocale::English),
        "Writing the local cache…"
    );
}
