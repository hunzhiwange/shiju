// 验证模型路由行的规范化、必填校验与错误定位。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::state::ModelRoute;

fn app() -> App {
    App::new().0
}

fn route(pattern: &str, provider: &str, model: &str, priority: &str) -> ModelRoute {
    ModelRoute {
        pattern: pattern.to_string(),
        provider: provider.to_string(),
        model: model.to_string(),
        priority_input: priority.to_string(),
    }
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn validate_routes_skips_blank_rows_and_trims_complete_routes() {
    let mut app = app();
    app.model_routes_settings.routes =
        vec![route("  ", " ", "\t", ""), route("  coding  ", "  openai  ", "  gpt-5  ", "  7  ")];

    let routes = validate_routes(&app).expect("blank and complete routes should be valid");

    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].hint, "coding");
    assert_eq!(routes[0].provider, "openai");
    assert_eq!(routes[0].model, "gpt-5");
    assert_eq!(routes[0].max_tokens, None);
    assert_eq!(routes[0].api_key, None);
}

#[test]
fn validate_routes_accepts_complete_route_without_priority() {
    let mut app = app();
    app.model_routes_settings.routes =
        vec![route("  analysis  ", "  anthropic  ", "  claude-sonnet  ", "  ")];

    let routes = validate_routes(&app).expect("priority is optional");

    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].hint, "analysis");
    assert_eq!(routes[0].provider, "anthropic");
    assert_eq!(routes[0].model, "claude-sonnet");
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn validate_routes_reports_provider_model_and_priority_errors_with_row_number() {
    let mut app = app();
    app.model_routes_settings.routes =
        vec![ModelRoute::default(), route("chat", " ", "gpt-5", "1")];
    assert_eq!(validate_routes(&app).unwrap_err(), "第 2 条路由缺少 provider");

    app.model_routes_settings.routes[1].provider = "openai".to_string();
    app.model_routes_settings.routes[1].model.clear();
    assert_eq!(validate_routes(&app).unwrap_err(), "第 2 条路由缺少 model");

    app.model_routes_settings.routes[1].model = "gpt-5".to_string();
    app.model_routes_settings.routes[1].priority_input = "-1".to_string();
    assert_eq!(validate_routes(&app).unwrap_err(), "第 2 条路由的 priority 不是有效数字");
}

#[test]
fn model_routes_validate_required_fields() {
    let mut app = app();
    app.model_routes_settings.routes = vec![ModelRoute {
        pattern: "  ".to_string(),
        provider: "openai".to_string(),
        model: "gpt-5".to_string(),
        priority_input: "1".to_string(),
    }];
    assert!(validate_routes(&app).unwrap_err().contains("pattern"));

    app.model_routes_settings.routes[0].pattern = "chat".to_string();
    app.model_routes_settings.routes[0].priority_input = "oops".to_string();
    assert!(validate_routes(&app).unwrap_err().contains("priority"));
}

#[test]
fn model_routes_update_syncs_query_classification_state() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::ModelRoutes(ModelRoutesMessage::AddRoute));
    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::PatternChanged(0, "chat".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::ProviderChanged(0, "openai".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::ModelChanged(0, "gpt-5".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::PriorityChanged(0, "7".to_string())),
    );

    assert!(app.query_classification_settings.enabled);
    assert_eq!(app.query_classification_settings.rules.len(), 1);
    assert_eq!(app.query_classification_settings.rules[0].category, "chat");
    assert_eq!(app.model_routes_settings.routes[0].provider, "openai");
    assert_eq!(app.model_routes_settings.routes[0].model, "gpt-5");

    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::PriorityChanged(0, "bad".to_string())),
    );
    assert!(app.model_routes_settings.save_error.as_deref().unwrap_or("").contains("priority"));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn invalid_route_edits_update_fields_and_expose_validation_errors() {
    let mut app = app();
    app.model_routes_settings.routes = vec![ModelRoute::default()];

    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::ProviderChanged(0, "openai".to_string())),
    );
    assert_eq!(app.model_routes_settings.routes[0].provider, "openai");
    assert_eq!(app.model_routes_settings.save_error.as_deref(), Some("第 1 条路由缺少 pattern"));

    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::PatternChanged(0, "chat".to_string())),
    );
    assert_eq!(app.model_routes_settings.routes[0].pattern, "chat");
    assert_eq!(app.model_routes_settings.save_error.as_deref(), Some("第 1 条路由缺少 model"));

    app.model_routes_settings.routes[0].provider.clear();
    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::ModelChanged(0, "gpt-5".to_string())),
    );
    assert_eq!(app.model_routes_settings.routes[0].model, "gpt-5");
    assert_eq!(app.model_routes_settings.save_error.as_deref(), Some("第 1 条路由缺少 provider"));

    app.model_routes_settings.routes[0].provider = "openai".to_string();
    let _ = update(
        &mut app,
        SettingsMessage::ModelRoutes(ModelRoutesMessage::PriorityChanged(0, "high".to_string())),
    );
    assert_eq!(app.model_routes_settings.routes[0].priority_input, "high");
    assert_eq!(
        app.model_routes_settings.save_error.as_deref(),
        Some("第 1 条路由的 priority 不是有效数字")
    );
}

#[test]
fn remove_route_updates_existing_rows_and_ignores_out_of_range_indexes() {
    let mut app = app();
    app.model_routes_settings.routes = vec![
        route("chat", "openai", "gpt-5", "3"),
        route("code", "anthropic", "claude-sonnet", "9"),
    ];
    app.model_routes_settings.save_error = Some("stale".to_string());

    let _ = update(&mut app, SettingsMessage::ModelRoutes(ModelRoutesMessage::RemoveRoute(9)));

    assert_eq!(app.model_routes_settings.routes.len(), 2);
    assert!(app.query_classification_settings.enabled);

    let _ = update(&mut app, SettingsMessage::ModelRoutes(ModelRoutesMessage::RemoveRoute(0)));

    assert_eq!(app.model_routes_settings.routes.len(), 1);
    assert_eq!(app.model_routes_settings.routes[0].pattern, "code");
    assert_eq!(app.query_classification_settings.rules.len(), 1);
    assert_eq!(app.query_classification_settings.rules[0].category, "code");

    let _ = update(&mut app, SettingsMessage::ModelRoutes(ModelRoutesMessage::RemoveRoute(0)));

    assert!(app.model_routes_settings.routes.is_empty());
    assert!(!app.query_classification_settings.enabled);
    assert!(app.query_classification_settings.rules.is_empty());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn update_ignores_messages_for_other_settings_sections() {
    let mut app = app();
    app.model_routes_settings.routes = vec![route("chat", "openai", "gpt-5", "3")];
    app.model_routes_settings.save_error = Some("keep".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert_eq!(app.model_routes_settings.routes.len(), 1);
    assert_eq!(app.model_routes_settings.routes[0].pattern, "chat");
    assert_eq!(app.model_routes_settings.routes[0].provider, "openai");
    assert_eq!(app.model_routes_settings.routes[0].model, "gpt-5");
    assert_eq!(app.model_routes_settings.routes[0].priority_input, "3");
    assert_eq!(app.model_routes_settings.save_error.as_deref(), Some("keep"));
}
