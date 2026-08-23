// 验证提供商刷新、远端同步阶段与过期结果处理。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::message::settings::types::ProvidersLoaded;
use crate::app::state::{ModelCatalogEntry, ProviderSummary};
use std::collections::HashMap;

fn app() -> App {
    App::new().0
}

fn catalog_item(provider_id: &str, provider_name: &str) -> ModelCatalogEntry {
    ModelCatalogEntry {
        provider_id: provider_id.to_string(),
        provider_name: provider_name.to_string(),
        model_id: "model".to_string(),
        model_name: "Model".to_string(),
    }
}

fn loaded(
    patterns: Vec<&str>,
    configured: bool,
    catalog: Vec<ModelCatalogEntry>,
) -> ProvidersLoaded {
    Ok((HashMap::new(), patterns.into_iter().map(str::to_string).collect(), configured, catalog))
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn refresh_starts_loading_and_clears_transient_errors() {
    let mut app = app();
    app.provider_settings.connect_error = Some("connect".to_string());
    app.provider_settings.save_error = Some("save".to_string());
    app.provider_settings.disconnect_confirm_provider_id = Some("openai".to_string());

    let _ = update(&mut app, SettingsMessage::ProvidersRefresh);

    assert!(app.provider_settings.loading);
    assert!(app.provider_settings.connect_error.is_none());
    assert!(app.provider_settings.save_error.is_none());
    assert!(app.provider_settings.disconnect_confirm_provider_id.is_none());
}

#[test]
fn refresh_results_preserve_unconfigured_patterns_and_empty_catalog() {
    let mut app = app();
    app.provider_settings.loading = true;
    app.provider_settings.catalog_loading = true;
    app.provider_settings.popular_patterns = vec!["Existing".to_string()];
    app.provider_settings.catalog_items = vec![catalog_item("existing", "Existing")];

    let _ = update(
        &mut app,
        SettingsMessage::ProvidersRefreshed(loaded(Vec::new(), false, Vec::new())),
    );

    assert!(!app.provider_settings.loading);
    assert!(!app.provider_settings.catalog_loading);
    assert_eq!(app.provider_settings.popular_patterns, ["Existing"]);
    assert_eq!(app.provider_settings.catalog_items[0].provider_id, "existing");

    let _ =
        update(&mut app, SettingsMessage::ProvidersRefreshed(Err("refresh failed".to_string())));
    assert_eq!(app.provider_settings.save_error.as_deref(), Some("refresh failed"));
}

#[test]
fn remote_sync_rejects_busy_states_and_advances_through_labels() {
    let mut app = app();
    app.provider_settings.loading = true;
    let _ = update(&mut app, SettingsMessage::ProviderModelsSyncRemote);
    assert!(!app.provider_settings.models_syncing);

    app.provider_settings.loading = false;
    let _ = update(&mut app, SettingsMessage::ProviderModelsSyncRemote);
    assert!(app.provider_settings.models_syncing);
    assert_eq!(app.provider_settings.models_sync_progress, 0.08);
    assert!(app.provider_settings.models_sync_label.contains("models.dev"));

    app.provider_settings.models_sync_progress = 0.34;
    let _ = update(&mut app, SettingsMessage::ProviderModelsSyncTick);
    assert_eq!(app.provider_settings.models_sync_label, "正在写入本地缓存…");

    app.provider_settings.models_sync_progress = 0.69;
    let _ = update(&mut app, SettingsMessage::ProviderModelsSyncTick);
    assert_eq!(app.provider_settings.models_sync_label, "正在刷新提供商视图…");

    app.provider_settings.models_sync_progress = 0.89;
    let _ = update(&mut app, SettingsMessage::ProviderModelsSyncTick);
    assert_eq!(app.provider_settings.models_sync_progress, 0.9);

    app.provider_settings.models_syncing = false;
    app.provider_settings.models_sync_progress = 0.42;
    let _ = update(&mut app, SettingsMessage::ProviderModelsSyncTick);
    assert_eq!(app.provider_settings.models_sync_progress, 0.42);
}

#[test]
fn remote_sync_tick_keeps_fetch_label_during_initial_phase_and_ignores_busy_request() {
    let mut app = app();
    app.provider_settings.models_syncing = true;
    app.provider_settings.models_sync_progress = 0.08;
    app.provider_settings.models_sync_label = "starting".to_string();

    let _ = update(&mut app, SettingsMessage::ProviderModelsSyncRemote);
    assert_eq!(app.provider_settings.models_sync_progress, 0.08);
    assert_eq!(app.provider_settings.models_sync_label, "starting");

    let _ = update(&mut app, SettingsMessage::ProviderModelsSyncTick);
    assert_eq!(app.provider_settings.models_sync_progress, 0.15);
    assert_eq!(app.provider_settings.models_sync_label, "正在从 models.dev 拉取模型目录…");
}

#[test]
fn remote_sync_results_replace_configured_data_and_reset_progress() {
    let mut app = app();
    app.provider_settings.models_syncing = true;
    app.provider_settings.models_sync_progress = 0.8;
    app.provider_settings.models_sync_label = "working".to_string();
    app.provider_settings.catalog_loading = true;

    let _ = update(
        &mut app,
        SettingsMessage::ProviderModelsSyncDone(loaded(
            vec!["New"],
            true,
            vec![catalog_item("new", "New")],
        )),
    );

    assert_eq!(app.provider_settings.popular_patterns, ["New"]);
    assert_eq!(app.provider_settings.catalog_items[0].provider_id, "new");
    assert!(!app.provider_settings.catalog_loading);
    assert!(!app.provider_settings.models_syncing);
    assert_eq!(app.provider_settings.models_sync_progress, 0.0);
    assert!(app.provider_settings.models_sync_label.is_empty());

    app.provider_settings.models_syncing = true;
    let _ =
        update(&mut app, SettingsMessage::ProviderModelsSyncDone(Err("sync failed".to_string())));
    assert_eq!(app.provider_settings.save_error.as_deref(), Some("sync failed"));
    assert!(!app.provider_settings.models_syncing);
}

#[test]
fn connect_modal_prefers_catalog_name_and_resets_competing_modals() {
    let mut app = app();
    app.provider_settings.catalog_items = vec![catalog_item("openai", "Catalog OpenAI")];
    app.provider_settings.providers = vec![ProviderSummary {
        id: "openai".to_string(),
        name: "Provider OpenAI".to_string(),
        source_label: "API".to_string(),
        connected: true,
    }];
    app.provider_settings.catalog_open = true;
    app.provider_settings.custom_provider_modal_open = true;
    app.provider_settings.custom_editing_provider_id = Some("custom".to_string());
    app.provider_settings.disconnect_confirm_provider_id = Some("openai".to_string());

    let _ = update(&mut app, SettingsMessage::ProviderConnectOpen("openai".to_string()));

    let modal = app.provider_settings.connect_modal.as_ref().unwrap();
    assert_eq!(modal.provider_name, "Catalog OpenAI");
    assert!(!app.provider_settings.catalog_open);
    assert!(!app.provider_settings.custom_provider_modal_open);
    assert!(app.provider_settings.custom_editing_provider_id.is_none());
    assert!(app.provider_settings.disconnect_confirm_provider_id.is_none());
}

#[test]
fn connect_modal_falls_back_to_provider_summary_then_provider_id() {
    let mut app = app();
    app.provider_settings.providers = vec![ProviderSummary {
        id: "anthropic".to_string(),
        name: "Anthropic".to_string(),
        source_label: "API".to_string(),
        connected: false,
    }];

    let _ = update(&mut app, SettingsMessage::ProviderConnectOpen("anthropic".to_string()));
    assert_eq!(app.provider_settings.connect_modal.as_ref().unwrap().provider_name, "Anthropic");

    let _ = update(&mut app, SettingsMessage::ProviderConnectOpen("unknown".to_string()));
    assert_eq!(app.provider_settings.connect_modal.as_ref().unwrap().provider_name, "unknown");

    let _ = update(&mut app, SettingsMessage::ProviderConnectClose);
    let _ = update(&mut app, SettingsMessage::ProviderConnectApiKeyChanged("ignored".to_string()));
    assert!(app.provider_settings.connect_modal.is_none());
}

#[test]
fn connect_submit_validates_key_and_handles_both_results() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::ProviderConnectSubmit);
    assert!(!app.provider_settings.loading);

    app.provider_settings.connect_modal = Some(ProviderConnectState {
        provider_id: "openai".to_string(),
        provider_name: "OpenAI".to_string(),
        api_key: " ".to_string(),
    });
    let _ = update(&mut app, SettingsMessage::ProviderConnectSubmit);
    assert_eq!(app.provider_settings.connect_error.as_deref(), Some("请输入 API 密钥"));

    let _ = update(&mut app, SettingsMessage::ProviderConnectApiKeyChanged("secret".to_string()));
    let _ = update(&mut app, SettingsMessage::ProviderConnectSubmit);
    assert!(app.provider_settings.loading);
    assert!(app.provider_settings.connect_error.is_none());

    let _ = update(&mut app, SettingsMessage::ProviderConnectDone(Err("denied".to_string())));
    assert!(!app.provider_settings.loading);
    assert_eq!(app.provider_settings.connect_error.as_deref(), Some("denied"));

    let _ = update(
        &mut app,
        SettingsMessage::ProviderConnectDone(loaded(Vec::new(), false, Vec::new())),
    );
    assert!(app.provider_settings.connect_modal.is_none());
    assert!(app.provider_settings.connect_error.is_none());

    let _ = update(&mut app, SettingsMessage::ProviderConnectClose);
    assert!(app.provider_settings.connect_modal.is_none());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn disconnect_messages_manage_confirmation_and_result_errors() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::ProviderDisconnectRequested("openai".to_string()));
    assert_eq!(app.provider_settings.disconnect_confirm_provider_id.as_deref(), Some("openai"));

    let _ = update(&mut app, SettingsMessage::ProviderDisconnectCanceled);
    assert!(app.provider_settings.disconnect_confirm_provider_id.is_none());

    app.provider_settings.connect_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::ProviderDisconnectConfirmed("openai".to_string()));
    assert!(app.provider_settings.loading);
    assert!(app.provider_settings.connect_error.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::ProviderDisconnectDone(Err("disconnect failed".to_string())),
    );
    assert!(!app.provider_settings.loading);
    assert_eq!(app.provider_settings.save_error.as_deref(), Some("disconnect failed"));

    let _ = update(&mut app, SettingsMessage::ProviderDisconnect("other".to_string()));
    assert!(app.provider_settings.loading);
    let _ = update(
        &mut app,
        SettingsMessage::ProviderDisconnectDone(loaded(Vec::new(), false, Vec::new())),
    );
    assert!(!app.provider_settings.loading);
    assert!(app.provider_settings.disconnect_confirm_provider_id.is_none());
}

#[test]
fn custom_provider_open_close_and_edit_results_manage_modal_state() {
    let mut app = app();
    app.provider_settings.connect_error = Some("old".to_string());
    app.provider_settings.catalog_open = true;

    let _ = update(&mut app, SettingsMessage::CustomProviderOpen);
    assert!(app.provider_settings.custom_provider_modal_open);
    assert!(app.provider_settings.connect_error.is_none());
    assert!(!app.provider_settings.catalog_open);

    let _ = update(&mut app, SettingsMessage::CustomProviderEditOpen("custom".to_string()));
    assert!(app.provider_settings.loading);
    assert!(!app.provider_settings.custom_provider_modal_open);

    let draft = CustomProviderDraft {
        provider_id: "custom".to_string(),
        display_name: "Custom".to_string(),
        base_url: "https://example.test".to_string(),
        api_key: String::new(),
        headers: vec![ProviderHeaderDraft::default()],
        models: vec![CustomProviderModelDraft::default()],
    };
    let _ = update(&mut app, SettingsMessage::CustomProviderEditLoaded(Ok(draft)));
    assert!(!app.provider_settings.loading);
    assert!(app.provider_settings.custom_provider_modal_open);
    assert_eq!(app.provider_settings.custom_editing_provider_id.as_deref(), Some("custom"));

    let _ = update(&mut app, SettingsMessage::CustomProviderClose);
    assert!(!app.provider_settings.custom_provider_modal_open);
    assert!(app.provider_settings.custom_editing_provider_id.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderEditLoaded(Err("invalid config".to_string())),
    );
    assert_eq!(app.provider_settings.save_error.as_deref(), Some("invalid config"));
}

#[test]
fn custom_provider_and_model_tabs_select_and_reset_on_open() {
    let mut app = app();
    app.provider_settings.custom_provider_tab = CustomProviderTab::Models;

    let _ = update(&mut app, SettingsMessage::CustomProviderOpen);
    assert_eq!(app.provider_settings.custom_provider_tab, CustomProviderTab::Basic);

    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderTabSelected(CustomProviderTab::Authentication),
    );
    assert_eq!(app.provider_settings.custom_provider_tab, CustomProviderTab::Authentication);

    let _ = update(&mut app, SettingsMessage::CustomProviderModelOpen(None));
    assert_eq!(
        app.provider_settings.custom_model_modal.as_ref().unwrap().active_tab,
        CustomProviderModelTab::Info
    );

    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelTabSelected(CustomProviderModelTab::Capabilities),
    );
    assert_eq!(
        app.provider_settings.custom_model_modal.as_ref().unwrap().active_tab,
        CustomProviderModelTab::Capabilities
    );
}

#[test]
fn custom_provider_fields_and_headers_ignore_out_of_range_indices() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CustomProviderIdChanged("custom".to_string()));
    let _ = update(&mut app, SettingsMessage::CustomProviderNameChanged("Custom".to_string()));
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderBaseUrlChanged("https://example.test".to_string()),
    );
    let _ = update(&mut app, SettingsMessage::CustomProviderApiKeyChanged("key".to_string()));
    assert_eq!(app.provider_settings.custom.provider_id, "custom");
    assert_eq!(app.provider_settings.custom.display_name, "Custom");
    assert_eq!(app.provider_settings.custom.base_url, "https://example.test");
    assert_eq!(app.provider_settings.custom.api_key, "key");

    let _ = update(&mut app, SettingsMessage::CustomProviderHeaderRemove(0));
    assert_eq!(app.provider_settings.custom.headers.len(), 1);
    let _ = update(&mut app, SettingsMessage::CustomProviderHeaderAdd);
    let _ =
        update(&mut app, SettingsMessage::CustomProviderHeaderKeyChanged(1, "X-Test".to_string()));
    let _ =
        update(&mut app, SettingsMessage::CustomProviderHeaderValueChanged(1, "value".to_string()));
    assert_eq!(app.provider_settings.custom.headers[1].key, "X-Test");
    assert_eq!(app.provider_settings.custom.headers[1].value, "value");

    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderHeaderKeyChanged(99, "ignored".to_string()),
    );
    let _ = update(&mut app, SettingsMessage::CustomProviderHeaderRemove(99));
    assert_eq!(app.provider_settings.custom.headers.len(), 2);
}

#[test]
fn custom_model_modal_adds_edits_closes_and_removes_models() {
    let mut app = app();
    app.provider_settings.custom.models.clear();
    let _ = update(&mut app, SettingsMessage::CustomProviderModelOpen(None));
    let _ = update(&mut app, SettingsMessage::CustomProviderModelModalSave);
    assert_eq!(app.provider_settings.save_error.as_deref(), Some("模型 ID 不能为空"));

    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalIdChanged(" model-1 ".to_string()),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalNameChanged(" Model One ".to_string()),
    );
    let _ = update(&mut app, SettingsMessage::CustomProviderModelModalSave);
    assert_eq!(app.provider_settings.custom.models[0].model_id, "model-1");
    assert_eq!(app.provider_settings.custom.models[0].display_name, "Model One");
    assert!(app.provider_settings.custom_model_modal.is_none());

    let _ = update(&mut app, SettingsMessage::CustomProviderModelOpen(Some(0)));
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalNameChanged("Renamed".to_string()),
    );
    let _ = update(&mut app, SettingsMessage::CustomProviderModelModalSave);
    assert_eq!(app.provider_settings.custom.models[0].display_name, "Renamed");

    let _ = update(&mut app, SettingsMessage::CustomProviderModelOpen(Some(99)));
    let modal = app.provider_settings.custom_model_modal.as_ref().unwrap();
    assert!(modal.model_id.is_empty());
    let _ =
        update(&mut app, SettingsMessage::CustomProviderModelModalIdChanged("ghost".to_string()));
    let _ = update(&mut app, SettingsMessage::CustomProviderModelModalSave);
    assert_eq!(app.provider_settings.custom.models.len(), 1);
    assert!(app.provider_settings.custom_model_modal.is_none());

    let _ = update(&mut app, SettingsMessage::CustomProviderModelOpen(Some(0)));
    let _ = update(&mut app, SettingsMessage::CustomProviderModelClose);
    assert!(app.provider_settings.custom_model_modal.is_none());

    let _ = update(&mut app, SettingsMessage::CustomProviderModelRemove(99));
    assert_eq!(app.provider_settings.custom.models.len(), 1);
    let _ = update(&mut app, SettingsMessage::CustomProviderModelRemove(0));
    assert!(app.provider_settings.custom.models.is_empty());
}

#[test]
fn custom_provider_model_filter_and_enable_toggle_update_state() {
    use crate::app::state::CustomProviderModelFilter;

    let mut app = app();
    app.provider_settings.custom.models = vec![
        CustomProviderModelDraft {
            model_id: "enabled".to_string(),
            enabled: true,
            from_catalog: true,
            ..Default::default()
        },
        CustomProviderModelDraft {
            model_id: "disabled".to_string(),
            enabled: false,
            from_catalog: false,
            ..Default::default()
        },
    ];

    assert_eq!(app.provider_settings.custom_models_filter, CustomProviderModelFilter::Enabled);
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelFilterSelected(CustomProviderModelFilter::All),
    );
    assert_eq!(app.provider_settings.custom_models_filter, CustomProviderModelFilter::All);

    let _ = update(&mut app, SettingsMessage::CustomProviderModelEnabledToggled(0, false));
    assert!(!app.provider_settings.custom.models[0].enabled);

    let _ = update(&mut app, SettingsMessage::CustomProviderModelRemove(0));
    assert!(!app.provider_settings.custom.models[0].enabled);
    assert_eq!(app.provider_settings.custom.models.len(), 2);

    let _ = update(&mut app, SettingsMessage::CustomProviderModelRemove(1));
    assert_eq!(app.provider_settings.custom.models.len(), 1);
}

#[test]
fn custom_model_modal_restores_and_saves_full_metadata() {
    let mut app = app();
    app.provider_settings.custom.models = vec![CustomProviderModelDraft {
        model_id: "model".to_string(),
        display_name: "Model".to_string(),
        family: "family".to_string(),
        context_limit: "32768".to_string(),
        input_limit: "28672".to_string(),
        output_limit: "4096".to_string(),
        reasoning: Some(true),
        tool_call: Some(false),
        temperature: Some(true),
        attachment: Some(true),
        input_modalities: Some(vec!["text".to_string(), "image".to_string()]),
        output_modalities: Some(vec!["text".to_string()]),
        ..Default::default()
    }];

    let _ = update(&mut app, SettingsMessage::CustomProviderModelOpen(Some(0)));
    let modal = app.provider_settings.custom_model_modal.as_ref().unwrap();
    assert_eq!(modal.family, "family");
    assert_eq!(modal.context_limit, "32768");
    assert_eq!(modal.input_limit, "28672");
    assert_eq!(modal.output_limit, "4096");
    assert_eq!(modal.reasoning, Some(true));
    assert_eq!(modal.tool_call, Some(false));
    assert_eq!(modal.temperature, Some(true));
    assert_eq!(modal.attachment, Some(true));
    assert_eq!(
        modal.input_modalities.as_deref(),
        Some(["text".to_string(), "image".to_string()].as_slice())
    );
    assert_eq!(modal.output_modalities.as_deref(), Some(["text".to_string()].as_slice()));

    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalFamilyChanged("new-family".to_string()),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalContextLimitChanged("65536".to_string()),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalInputLimitChanged("60000".to_string()),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalOutputLimitChanged("4096".to_string()),
    );
    let _ = update(&mut app, SettingsMessage::CustomProviderModelModalToolCallChanged(true));
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalInputModalityToggled("image".to_string(), false),
    );
    let _ = update(&mut app, SettingsMessage::CustomProviderModelModalSave);

    let model = &app.provider_settings.custom.models[0];
    assert_eq!(model.family, "new-family");
    assert_eq!(model.context_limit, "65536");
    assert_eq!(model.input_limit, "60000");
    assert_eq!(model.output_limit, "4096");
    assert_eq!(model.tool_call, Some(true));
    assert_eq!(model.input_modalities.as_deref(), Some(["text".to_string()].as_slice()));
}

#[test]
fn custom_model_modal_rejects_duplicate_ids_and_invalid_limit_relationships() {
    let mut app = app();
    app.provider_settings.custom.models =
        vec![CustomProviderModelDraft { model_id: "existing".to_string(), ..Default::default() }];

    let _ = update(&mut app, SettingsMessage::CustomProviderModelOpen(None));
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalIdChanged(" existing ".to_string()),
    );
    let _ = update(&mut app, SettingsMessage::CustomProviderModelModalSave);
    assert_eq!(app.provider_settings.save_error.as_deref(), Some("模型 ID 已存在：existing"));
    assert!(app.provider_settings.custom_model_modal.is_some());

    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalIdChanged("new-model".to_string()),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalContextLimitChanged("100".to_string()),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalInputLimitChanged("101".to_string()),
    );
    let _ = update(&mut app, SettingsMessage::CustomProviderModelModalSave);
    assert!(app.provider_settings.save_error.is_none());
    assert!(app.provider_settings.custom_model_modal.is_none());
    assert_eq!(app.provider_settings.custom.models.last().unwrap().model_id, "new-model");
    assert_eq!(app.provider_settings.custom.models.last().unwrap().input_limit, "101");

    let _ = update(&mut app, SettingsMessage::CustomProviderModelOpen(None));
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalIdChanged("overflow-output".to_string()),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalContextLimitChanged("100".to_string()),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelModalOutputLimitChanged("101".to_string()),
    );
    let _ = update(&mut app, SettingsMessage::CustomProviderModelModalSave);
    assert_eq!(
        app.provider_settings.save_error.as_deref(),
        Some("模型 overflow-output 的输出限制不能超过上下文限制")
    );
    assert!(app.provider_settings.custom_model_modal.is_some());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn custom_provider_save_rejects_duplicate_model_ids_before_async_save() {
    let mut app = app();
    app.provider_settings.custom.provider_id = "custom".to_string();
    app.provider_settings.custom.base_url = "https://example.test".to_string();
    app.provider_settings.custom.models = vec![
        CustomProviderModelDraft { model_id: "model".to_string(), ..Default::default() },
        CustomProviderModelDraft { model_id: " model ".to_string(), ..Default::default() },
    ];

    let _ = update(&mut app, SettingsMessage::CustomProviderSave);
    assert_eq!(app.provider_settings.save_error.as_deref(), Some("模型 ID 重复：model"));
    assert!(!app.provider_settings.loading);
}

#[test]
fn custom_model_discovery_tracks_busy_state_and_merges_results() {
    let mut app = app();
    app.provider_settings.custom_provider_modal_open = true;
    app.provider_settings.custom_editing_provider_id = Some("custom".to_string());
    app.provider_settings.custom.provider_id = "custom".to_string();
    app.provider_settings.custom.models = vec![CustomProviderModelDraft {
        model_id: "model-b".to_string(),
        display_name: "保留名称".to_string(),
        ..Default::default()
    }];

    let _ = update(&mut app, SettingsMessage::CustomProviderModelsDiscover);
    assert!(app.provider_settings.custom_models_discovering);
    assert!(app.provider_settings.save_error.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelsDiscovered {
            provider_id: "custom".to_string(),
            result: Ok(vec!["model-a".to_string(), "model-b".to_string()]),
        },
    );
    assert!(!app.provider_settings.custom_models_discovering);
    assert_eq!(
        app.provider_settings
            .custom
            .models
            .iter()
            .map(|model| model.model_id.as_str())
            .collect::<Vec<_>>(),
        vec!["model-a", "model-b"]
    );
    assert_eq!(app.provider_settings.custom.models[1].display_name, "保留名称");

    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelsDiscovered {
            provider_id: "custom".to_string(),
            result: Err("发现失败".to_string()),
        },
    );
    assert!(!app.provider_settings.custom_models_discovering);
    assert_eq!(app.provider_settings.save_error.as_deref(), Some("发现失败"));
}

#[test]
fn custom_model_discovery_starts_for_new_provider_while_modal_is_open() {
    let mut app = app();
    app.provider_settings.custom_provider_modal_open = true;
    app.provider_settings.custom.provider_id = "new-provider".to_string();

    let _ = update(&mut app, SettingsMessage::CustomProviderModelsDiscover);

    assert!(app.provider_settings.custom_models_discovering);
}

#[test]
fn custom_model_discovery_ignores_stale_provider_results() {
    let mut app = app();
    app.provider_settings.custom_provider_modal_open = true;
    app.provider_settings.custom_editing_provider_id = Some("provider-b".to_string());
    app.provider_settings.custom_models_discovering = true;

    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelsDiscovered {
            provider_id: "provider-a".to_string(),
            result: Ok(vec!["stale-model".to_string()]),
        },
    );

    assert!(app.provider_settings.custom_models_discovering);
    assert!(app.provider_settings.custom.models[0].model_id.is_empty());
}

#[test]
fn custom_model_discovery_ignores_stale_results_for_new_provider() {
    let mut app = app();
    app.provider_settings.custom_provider_modal_open = true;
    app.provider_settings.custom.provider_id = "provider-b".to_string();
    app.provider_settings.custom_models_discovering = true;

    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderModelsDiscovered {
            provider_id: "provider-a".to_string(),
            result: Ok(vec!["stale-model".to_string()]),
        },
    );

    assert!(app.provider_settings.custom_models_discovering);
    assert!(app.provider_settings.custom.models[0].model_id.is_empty());
}

#[test]
fn custom_provider_save_validates_each_boundary_and_handles_results() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CustomProviderSave);
    assert!(app.provider_settings.save_error.as_deref().unwrap().contains("提供商 ID"));

    app.provider_settings.custom.provider_id = "custom".to_string();
    let _ = update(&mut app, SettingsMessage::CustomProviderSave);
    assert_eq!(app.provider_settings.save_error.as_deref(), Some("基础 URL 不能为空"));

    app.provider_settings.custom.base_url = "https://example.test".to_string();
    let _ = update(&mut app, SettingsMessage::CustomProviderSave);
    assert!(app.provider_settings.loading);
    assert!(app.provider_settings.save_error.is_none());

    let _ =
        update(&mut app, SettingsMessage::CustomProviderSaveDone(Err("save failed".to_string())));
    assert!(!app.provider_settings.loading);
    assert_eq!(app.provider_settings.save_error.as_deref(), Some("save failed"));

    app.provider_settings.custom_provider_modal_open = true;
    app.provider_settings.custom_editing_provider_id = Some("custom".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::CustomProviderSaveDone(loaded(
            vec!["Saved"],
            true,
            vec![catalog_item("saved", "Saved")],
        )),
    );
    assert!(!app.provider_settings.custom_provider_modal_open);
    assert!(app.provider_settings.custom_editing_provider_id.is_none());
    assert!(app.provider_settings.custom.provider_id.is_empty());
    assert_eq!(app.provider_settings.popular_patterns, ["Saved"]);
    assert_eq!(app.provider_settings.catalog_items[0].provider_id, "saved");
}

#[test]
fn catalog_open_reuses_cached_items_and_closes_competing_model_modal() {
    let mut app = app();
    app.provider_settings.catalog_items = vec![catalog_item("cached", "Cached")];
    app.provider_settings.catalog_query = "old".to_string();
    app.provider_settings.save_error = Some("old save".to_string());
    let _ = update(&mut app, SettingsMessage::CustomProviderModelOpen(None));

    let _ = update(&mut app, SettingsMessage::ProviderCatalogOpen);

    assert!(app.provider_settings.catalog_open);
    assert!(!app.provider_settings.catalog_loading);
    assert!(app.provider_settings.catalog_query.is_empty());
    assert!(app.provider_settings.save_error.is_none());
    assert!(app.provider_settings.custom_model_modal.is_none());
}

#[test]
fn catalog_and_popular_provider_messages_cover_success_error_and_bounds() {
    let mut app = app();
    app.provider_settings.popular_patterns = vec!["Existing".to_string()];
    app.provider_settings.connect_modal = Some(ProviderConnectState::default());
    app.provider_settings.custom_provider_modal_open = true;
    app.provider_settings.catalog_query = "old".to_string();

    let _ = update(&mut app, SettingsMessage::ProviderCatalogOpen);
    assert!(app.provider_settings.catalog_open);
    assert!(app.provider_settings.catalog_loading);
    assert!(app.provider_settings.catalog_query.is_empty());
    assert!(app.provider_settings.connect_modal.is_none());
    assert!(!app.provider_settings.custom_provider_modal_open);

    let _ = update(&mut app, SettingsMessage::ProviderCatalogQueryChanged("gpt".to_string()));
    assert_eq!(app.provider_settings.catalog_query, "gpt");

    let _ =
        update(&mut app, SettingsMessage::ProviderCatalogLoaded(Err("catalog failed".to_string())));
    assert!(!app.provider_settings.catalog_loading);
    assert_eq!(app.provider_settings.save_error.as_deref(), Some("catalog failed"));

    let _ = update(
        &mut app,
        SettingsMessage::ProviderCatalogLoaded(Ok(vec![catalog_item("openai", "OpenAI")])),
    );
    assert_eq!(app.provider_settings.catalog_items.len(), 1);

    let _ = update(&mut app, SettingsMessage::ProviderCatalogAddToPopular("openai".to_string()));
    let _ = update(&mut app, SettingsMessage::ProviderCatalogAddToPopular("openai".to_string()));
    assert_eq!(
        app.provider_settings
            .popular_patterns
            .iter()
            .filter(|provider| provider.as_str() == "openai")
            .count(),
        1
    );

    let previous = app.provider_settings.popular_patterns.clone();
    let _ = update(&mut app, SettingsMessage::PopularProviderRemove(99));
    assert_eq!(app.provider_settings.popular_patterns, previous);
    let _ = update(&mut app, SettingsMessage::PopularProviderRemove(0));
    assert_eq!(app.provider_settings.popular_patterns, ["openai"]);

    let _ = update(&mut app, SettingsMessage::PopularProvidersSaved(Ok(())));
    assert!(app.provider_settings.save_error.is_none());
    let _ =
        update(&mut app, SettingsMessage::PopularProvidersSaved(Err("save failed".to_string())));
    assert_eq!(app.provider_settings.save_error.as_deref(), Some("save failed"));

    let _ = update(&mut app, SettingsMessage::ProviderCatalogClose);
    assert!(!app.provider_settings.catalog_open);
}

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。
#[test]
fn shiju_official_provider_messages_are_idempotent() {
    let mut app = app();
    app.provider_settings.popular_patterns.clear();

    let _ = update(&mut app, SettingsMessage::ProviderCatalogAddToPopular("shiju".to_string()));
    assert!(app.provider_settings.popular_patterns.is_empty());

    app.provider_settings.popular_patterns = vec!["shiju".to_string(), "openai".to_string()];
    let _ = update(&mut app, SettingsMessage::PopularProviderRemove(0));
    assert_eq!(app.provider_settings.popular_patterns, ["shiju", "openai"]);

    let _ = update(
        &mut app,
        SettingsMessage::ProvidersRefreshed(loaded(vec!["shiju", "openai"], true, Vec::new())),
    );
    assert_eq!(app.provider_settings.popular_patterns, ["openai"]);

    let _ = update(&mut app, SettingsMessage::ProviderConnectOpen("shiju".to_string()));
    let modal = app.provider_settings.connect_modal.as_ref().unwrap();
    assert_eq!(modal.provider_id, "shiju");
    assert_eq!(modal.provider_name, "识矩");
}
