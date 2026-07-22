use super::*;
use crate::app::App;
use crate::app::state::EmbeddingRouteDraft;

fn app() -> App {
    App::new().0
}

fn route(pattern: &str, provider: &str, model: &str, dimensions: &str) -> EmbeddingRouteDraft {
    EmbeddingRouteDraft {
        pattern: pattern.to_string(),
        provider: provider.to_string(),
        model: model.to_string(),
        dimensions: dimensions.to_string(),
        api_key_input: String::new(),
    }
}

#[test]
fn trim_to_option_trims_present_values_and_rejects_blank_values() {
    assert_eq!(trim_to_option("  test-api-key\n"), Some("test-api-key".to_string()));
    assert_eq!(trim_to_option(" \n\t "), None);
}

#[test]
fn template_routes_contain_complete_provider_defaults() {
    let alibaba = alibaba_template_route();
    assert_eq!(alibaba.pattern, "semantic");
    assert_eq!(alibaba.provider, "alibaba-cn");
    assert_eq!(alibaba.model, "text-embedding-v4");
    assert_eq!(alibaba.dimensions, "1024");
    assert!(alibaba.api_key_input.is_empty());

    let doubao = doubao_template_route();
    assert_eq!(doubao.pattern, "multimodal");
    assert_eq!(doubao.provider, "custom:https://ark.cn-beijing.volces.com/api/v3");
    assert_eq!(doubao.model, "doubao-embedding-vision-250615");
    assert!(doubao.dimensions.is_empty());
    assert!(doubao.api_key_input.is_empty());
}

#[test]
fn add_and_template_messages_append_routes_and_clear_save_status() {
    let mut app = app();
    app.embedding_routes_settings.save_error = Some("old error".to_string());
    app.embedding_routes_settings.save_success = true;

    let _ = update(&mut app, SettingsMessage::EmbeddingRoutes(EmbeddingRoutesMessage::AddRoute));
    assert_eq!(app.embedding_routes_settings.routes.len(), 1);
    assert!(app.embedding_routes_settings.routes[0].pattern.is_empty());
    assert!(app.embedding_routes_settings.save_error.is_none());
    assert!(!app.embedding_routes_settings.save_success);

    app.embedding_routes_settings.save_error = Some("another error".to_string());
    app.embedding_routes_settings.save_success = true;
    let _ = update(
        &mut app,
        SettingsMessage::EmbeddingRoutes(EmbeddingRoutesMessage::InsertAlibabaTemplate),
    );
    assert_eq!(app.embedding_routes_settings.routes[1].provider, "alibaba-cn");
    assert!(app.embedding_routes_settings.save_error.is_none());
    assert!(!app.embedding_routes_settings.save_success);

    app.embedding_routes_settings.save_error = Some("last error".to_string());
    app.embedding_routes_settings.save_success = true;
    let _ = update(
        &mut app,
        SettingsMessage::EmbeddingRoutes(EmbeddingRoutesMessage::InsertDoubaoTemplate),
    );
    assert_eq!(app.embedding_routes_settings.routes[2].pattern, "multimodal");
    assert!(app.embedding_routes_settings.save_error.is_none());
    assert!(!app.embedding_routes_settings.save_success);
}

#[test]
fn field_change_messages_update_every_editable_value() {
    let mut app = app();
    app.embedding_routes_settings.routes = vec![EmbeddingRouteDraft::default()];
    app.embedding_routes_settings.save_error = Some("old error".to_string());
    app.embedding_routes_settings.save_success = true;

    for message in [
        EmbeddingRoutesMessage::PatternChanged(0, "docs".to_string()),
        EmbeddingRoutesMessage::ProviderChanged(0, "openai".to_string()),
        EmbeddingRoutesMessage::ModelChanged(0, "text-embedding-3-large".to_string()),
        EmbeddingRoutesMessage::DimensionsChanged(0, "3072".to_string()),
        EmbeddingRoutesMessage::ApiKeyChanged(0, "test-api-key".to_string()),
    ] {
        let _ = update(&mut app, SettingsMessage::EmbeddingRoutes(message));
    }

    let draft = &app.embedding_routes_settings.routes[0];
    assert_eq!(draft.pattern, "docs");
    assert_eq!(draft.provider, "openai");
    assert_eq!(draft.model, "text-embedding-3-large");
    assert_eq!(draft.dimensions, "3072");
    assert_eq!(draft.api_key_input, "test-api-key");
    assert!(app.embedding_routes_settings.save_error.is_none());
    assert!(!app.embedding_routes_settings.save_success);
}

#[test]
fn remove_route_deletes_only_an_existing_index() {
    let mut app = app();
    app.embedding_routes_settings.routes = vec![
        route("first", "provider-a", "model-a", ""),
        route("second", "provider-b", "model-b", ""),
    ];
    app.embedding_routes_settings.save_error = Some("old error".to_string());
    app.embedding_routes_settings.save_success = true;

    let _ =
        update(&mut app, SettingsMessage::EmbeddingRoutes(EmbeddingRoutesMessage::RemoveRoute(0)));
    assert_eq!(app.embedding_routes_settings.routes.len(), 1);
    assert_eq!(app.embedding_routes_settings.routes[0].pattern, "second");
    assert!(app.embedding_routes_settings.save_error.is_none());
    assert!(!app.embedding_routes_settings.save_success);

    let _ =
        update(&mut app, SettingsMessage::EmbeddingRoutes(EmbeddingRoutesMessage::RemoveRoute(9)));
    assert_eq!(app.embedding_routes_settings.routes.len(), 1);
    assert_eq!(app.embedding_routes_settings.routes[0].pattern, "second");
}

#[test]
fn out_of_bounds_field_changes_leave_the_existing_route_unchanged() {
    let mut app = app();
    app.embedding_routes_settings.routes = vec![route("docs", "openai", "model", "1536")];
    app.embedding_routes_settings.routes[0].api_key_input = "original-key".to_string();
    app.embedding_routes_settings.save_error = Some("old error".to_string());
    app.embedding_routes_settings.save_success = true;

    for message in [
        EmbeddingRoutesMessage::PatternChanged(4, "changed".to_string()),
        EmbeddingRoutesMessage::ProviderChanged(4, "changed".to_string()),
        EmbeddingRoutesMessage::ModelChanged(4, "changed".to_string()),
        EmbeddingRoutesMessage::DimensionsChanged(4, "42".to_string()),
        EmbeddingRoutesMessage::ApiKeyChanged(4, "changed".to_string()),
    ] {
        let _ = update(&mut app, SettingsMessage::EmbeddingRoutes(message));
    }

    let draft = &app.embedding_routes_settings.routes[0];
    assert_eq!(draft.pattern, "docs");
    assert_eq!(draft.provider, "openai");
    assert_eq!(draft.model, "model");
    assert_eq!(draft.dimensions, "1536");
    assert_eq!(draft.api_key_input, "original-key");
    assert!(app.embedding_routes_settings.save_error.is_none());
    assert!(!app.embedding_routes_settings.save_success);
}

#[test]
fn persist_reports_each_required_field_with_its_one_based_row_number() {
    let mut app = app();
    app.embedding_routes_settings.routes =
        vec![route("valid", "provider-a", "model-a", ""), route(" ", "provider-b", "model-b", "")];

    assert!(persist_embedding_routes_settings(&mut app).is_none());
    assert_eq!(
        app.embedding_routes_settings.save_error.as_deref(),
        Some("第 2 条路由的 pattern 不能为空")
    );
    assert!(!app.embedding_routes_settings.save_success);

    app.embedding_routes_settings.routes[1].pattern = "second".to_string();
    app.embedding_routes_settings.routes[1].provider = "\t".to_string();
    assert!(persist_embedding_routes_settings(&mut app).is_none());
    assert_eq!(
        app.embedding_routes_settings.save_error.as_deref(),
        Some("第 2 条路由的 provider 不能为空")
    );

    app.embedding_routes_settings.routes[1].provider = "provider-b".to_string();
    app.embedding_routes_settings.routes[1].model = "\n".to_string();
    assert!(persist_embedding_routes_settings(&mut app).is_none());
    assert_eq!(
        app.embedding_routes_settings.save_error.as_deref(),
        Some("第 2 条路由的 model 不能为空")
    );
}

#[test]
fn persist_rejects_zero_nonnumeric_and_overflowing_dimensions() {
    let mut app = app();
    app.embedding_routes_settings.routes = vec![route("docs", "openai", "model", "")];

    for dimensions in ["0", "1.5", "4294967296"] {
        app.embedding_routes_settings.routes[0].dimensions = dimensions.to_string();
        app.embedding_routes_settings.save_error = None;
        app.embedding_routes_settings.save_success = true;

        assert!(persist_embedding_routes_settings(&mut app).is_none());
        assert_eq!(
            app.embedding_routes_settings.save_error.as_deref(),
            Some("第 1 条路由的 dimensions 必须是大于 0 的整数")
        );
        assert!(!app.embedding_routes_settings.save_success);
    }
}

#[test]
fn persist_accepts_empty_routes_blank_dimensions_and_u32_maximum() {
    let mut app = app();
    app.embedding_routes_settings.save_error = Some("old error".to_string());

    assert!(persist_embedding_routes_settings(&mut app).is_some());
    assert!(app.embedding_routes_settings.save_error.is_none());
    assert!(app.embedding_routes_settings.save_success);

    app.embedding_routes_settings.routes = vec![
        route(" docs ", " openai ", " model ", " \n"),
        route("code", "provider-b", "model-b", &u32::MAX.to_string()),
    ];
    app.embedding_routes_settings.routes[0].api_key_input = " \t".to_string();
    app.embedding_routes_settings.routes[1].api_key_input = " test-api-key ".to_string();

    assert!(persist_embedding_routes_settings(&mut app).is_some());
    assert!(app.embedding_routes_settings.save_error.is_none());
    assert!(app.embedding_routes_settings.save_success);
}

#[test]
fn save_message_exposes_validation_result_in_settings_state() {
    let mut app = app();
    app.embedding_routes_settings.routes = vec![route("docs", "openai", "model", "bad")];

    let _ = update(&mut app, SettingsMessage::EmbeddingRoutes(EmbeddingRoutesMessage::Save));
    assert_eq!(
        app.embedding_routes_settings.save_error.as_deref(),
        Some("第 1 条路由的 dimensions 必须是大于 0 的整数")
    );
    assert!(!app.embedding_routes_settings.save_success);

    app.embedding_routes_settings.routes[0].dimensions = "1536".to_string();
    let _ = update(&mut app, SettingsMessage::EmbeddingRoutes(EmbeddingRoutesMessage::Save));
    assert!(app.embedding_routes_settings.save_error.is_none());
    assert!(app.embedding_routes_settings.save_success);
}

#[test]
fn non_embedding_message_leaves_embedding_routes_state_unchanged() {
    let mut app = app();
    app.embedding_routes_settings.routes = vec![route("docs", "openai", "model", "1536")];
    app.embedding_routes_settings.save_error = Some("keep error".to_string());
    app.embedding_routes_settings.save_success = true;

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert_eq!(app.embedding_routes_settings.routes.len(), 1);
    assert_eq!(app.embedding_routes_settings.routes[0].pattern, "docs");
    assert_eq!(app.embedding_routes_settings.save_error.as_deref(), Some("keep error"));
    assert!(app.embedding_routes_settings.save_success);
}
