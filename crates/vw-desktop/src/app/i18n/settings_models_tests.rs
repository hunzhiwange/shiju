use config_ui::DesktopLocale;

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
        app.general_settings.desktop_locale = locale;
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
        app.general_settings.desktop_locale = locale;
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
