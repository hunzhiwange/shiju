use super::settings_models::{enabled_count, model_metadata, text};
use vw_config_types::ui::DesktopLocale;
#[test]
fn desktop_i18n_settings_models_switches_visible_text() {
    for (locale, title, count) in [
        (DesktopLocale::SimplifiedChinese, "模型", "2 / 5 已启用"),
        (DesktopLocale::TraditionalChinese, "模型", "2 / 5 已啟用"),
        (DesktopLocale::English, "Models", "2 / 5 enabled"),
    ] {
        assert_eq!(text(locale, "模型"), title);
        assert_eq!(enabled_count(locale, 2, 5), count);
        let raw = "custom:原始/🚀";
        assert!(model_metadata(locale, true, false, raw).ends_with(raw));
    }
}

#[test]
fn desktop_i18n_settings_models_pending_feedback_switches_without_replacing_raw_content() {
    use super::settings_models::messages::{Feedback, render};
    let raw = "服务返回 {literal} {} 🔐 / model/保留";
    let feedback = Feedback::owned("service %s 失败: %s", vec!["restart".into(), raw.into()]);
    let cases = [
        (DesktopLocale::SimplifiedChinese, "service restart 失败: "),
        (DesktopLocale::TraditionalChinese, "service restart 失敗: "),
        (DesktopLocale::English, "service restart failed: "),
    ];
    for (locale, prefix) in cases {
        assert_eq!(feedback.display(locale), format!("{prefix}{raw}"));
        assert_eq!(Feedback::from(raw).display(locale), raw);
        let output = Feedback::from(raw)
            .join(Feedback::owned("本机 gateway 已恢复并刷新连接状态。", vec![]));
        assert_eq!(
            output.display(locale),
            format!("{raw}\n{}", render(locale, "本机 gateway 已恢复并刷新连接状态。", &[]))
        );
    }
}

#[test]
fn desktop_i18n_settings_models_validation_and_sync_keep_semantics_after_switch() {
    use crate::app::state::EmbeddingRouteDraft;
    use crate::app::{
        App,
        message::settings::{self, EmbeddingRoutesMessage, SettingsMessage},
    };
    let mut app = App::new().0;
    app.embedding_routes_settings.routes = vec![EmbeddingRouteDraft {
        pattern: String::new(),
        provider: "custom:原始/🚀".into(),
        model: "model/原始".into(),
        dimensions: "1024".into(),
        api_key_input: "原始-secret".into(),
    }];
    let _task =
        settings::update(&mut app, SettingsMessage::EmbeddingRoutes(EmbeddingRoutesMessage::Save));
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "第 1 条路由的 pattern 不能为空"),
        (DesktopLocale::TraditionalChinese, "第 1 條路由的 pattern 不能為空"),
        (DesktopLocale::English, "Route 1 pattern must not be empty"),
    ] {
        app.desktop_locale = locale;
        assert_eq!(
            app.embedding_routes_settings.save_error.as_ref().unwrap().display(locale),
            expected
        );
        assert_eq!(app.embedding_routes_settings.routes[0].provider, "custom:原始/🚀");
        assert_eq!(app.embedding_routes_settings.routes[0].api_key_input, "原始-secret");
    }
    app.provider_settings.loading = false;
    let _task = settings::update(&mut app, SettingsMessage::ProviderModelsSyncRemote);
    assert!(
        app.provider_settings
            .models_sync_label
            .display(DesktopLocale::English)
            .contains("Fetching")
    );
    app.provider_settings.models_sync_progress = 0.34;
    let _task = settings::update(&mut app, SettingsMessage::ProviderModelsSyncTick);
    assert_eq!(
        app.provider_settings.models_sync_label.display(DesktopLocale::English),
        "Writing the local cache…"
    );
}

#[test]
fn desktop_i18n_settings_models_search_matches_locale_stable_ids_and_chinese_navigation() {
    use crate::app::state::ModelConfigTab;
    use crate::app::{
        App,
        components::system_settings_search::{SystemSettingsSearchTarget, catalog},
    };
    let mut app = App::new().0;
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        app.desktop_locale = locale;
        let entries = catalog(&app);
        let route = entries
            .iter()
            .find(|entry| {
                entry.target == SystemSettingsSearchTarget::Models(ModelConfigTab::ModelRoutes)
            })
            .unwrap();
        assert!(route.matches("model routes"));
        assert!(route.matches("模型路由"));
        assert!(route.matches(route.label));
    }
}

#[test]
fn desktop_i18n_settings_models_builds_current_pages_in_all_locales() {
    use crate::app::{
        App,
        components::{
            system_settings_embedding_routes, system_settings_gateway,
            system_settings_gateway_client, system_settings_model_routes, system_settings_models,
            system_settings_network, system_settings_providers,
        },
    };
    use iced::advanced::widget::Tree;
    let mut app = App::new().0;
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        app.desktop_locale = locale;
        for view in [
            system_settings_models::main_view(&app),
            system_settings_model_routes::view(&app),
            system_settings_embedding_routes::embedded_view(&app),
            system_settings_gateway::view(&app),
            system_settings_gateway_client::view(&app),
            system_settings_network::view(&app),
            system_settings_providers::connected::view(&app),
        ] {
            let _tree = Tree::new(view.as_widget());
        }
    }
}

#[test]
fn desktop_i18n_settings_models_save_waits_for_gateway_acknowledgment() {
    use crate::app::state::EmbeddingRouteDraft;
    use crate::app::{
        App,
        message::settings::{self, EmbeddingRoutesMessage, SettingsMessage},
    };
    let mut app = App::new().0;
    app.embedding_routes_settings.routes = vec![EmbeddingRouteDraft {
        pattern: "semantic".into(),
        provider: "custom:保留".into(),
        model: "model".into(),
        dimensions: "1024".into(),
        api_key_input: String::new(),
    }];
    let _task =
        settings::update(&mut app, SettingsMessage::EmbeddingRoutes(EmbeddingRoutesMessage::Save));
    assert!(
        !app.embedding_routes_settings.save_success,
        "dispatching a save is not an acknowledgement"
    );
    let raw = "服务返回 {} raw-error";
    let _task = settings::update(
        &mut app,
        SettingsMessage::AgentConfigSaved { tag: "embedding_routes", result: Err(raw.into()) },
    );
    assert!(!app.embedding_routes_settings.save_success);
    assert!(
        app.embedding_routes_settings
            .save_error
            .as_ref()
            .unwrap()
            .display(DesktopLocale::English)
            .ends_with(raw)
    );
    let _task = settings::update(
        &mut app,
        SettingsMessage::AgentConfigSaved { tag: "embedding_routes", result: Ok(()) },
    );
    assert!(app.embedding_routes_settings.save_success);
    assert!(app.embedding_routes_settings.save_error.is_none());
    let _task = settings::update(
        &mut app,
        SettingsMessage::AgentConfigSaved { tag: "gateway", result: Err(raw.into()) },
    );
    assert!(
        app.gateway_settings
            .save_error
            .as_ref()
            .unwrap()
            .display(DesktopLocale::English)
            .ends_with(raw)
    );
}

#[test]
fn desktop_i18n_settings_models_cached_details_translate_without_mutating_service_json() {
    use super::settings_models::connection_text::text as connection_text;
    use crate::app::{
        App,
        message::settings::{self, SettingsMessage},
        state::{ModelSummary, ProviderModelsSummary},
    };
    let mut app = App::new().0;
    let raw = serde_json::json!({"name": "原始模型 {} 🚀", "capabilities": {"toolcall": true}, "custom": "保留"});
    app.model_settings.providers = vec![ProviderModelsSummary {
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
    let _task = settings::update(
        &mut app,
        SettingsMessage::ModelDetailOpen("provider/保留".into(), "model/保留".into()),
    );
    let modal = app.model_settings.detail_modal.as_ref().unwrap();
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
