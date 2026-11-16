use crate::state::{ModelRoute, ModelRoutesSettingsState};
use crate::view_message::ModelRoutesMessage;
fn update(state: &mut ModelRoutesSettingsState, message: ModelRoutesMessage) {
    let _ = crate::model_routes::edit_draft(state, message);
}
#[test]
fn invalid_route_edits_update_fields_and_expose_validation_errors() {
    let mut app = crate::state::ModelRoutesSettingsState::default();
    app.routes = vec![ModelRoute::default()];

    let _ = update(&mut app, ModelRoutesMessage::ProviderChanged(0, "openai".to_string()));
    assert_eq!(app.routes[0].provider, "openai");
    assert_eq!(app.save_error.as_deref(), Some("第 1 条路由缺少 pattern"));

    let _ = update(&mut app, ModelRoutesMessage::PatternChanged(0, "chat".to_string()));
    assert_eq!(app.routes[0].pattern, "chat");
    assert_eq!(app.save_error.as_deref(), Some("第 1 条路由缺少 model"));

    app.routes[0].provider.clear();
    let _ = update(&mut app, ModelRoutesMessage::ModelChanged(0, "gpt-5".to_string()));
    assert_eq!(app.routes[0].model, "gpt-5");
    assert_eq!(app.save_error.as_deref(), Some("第 1 条路由缺少 provider"));

    app.routes[0].provider = "openai".to_string();
    let _ = update(&mut app, ModelRoutesMessage::PriorityChanged(0, "high".to_string()));
    assert_eq!(app.routes[0].priority_input, "high");
    assert_eq!(app.save_error.as_deref(), Some("第 1 条路由的 priority 不是有效数字"));
}

fn route(pattern: &str, provider: &str, model: &str, priority: &str) -> ModelRoute {
    ModelRoute {
        pattern: pattern.into(),
        provider: provider.into(),
        model: model.into(),
        priority_input: priority.into(),
    }
}
#[test]
fn remove_route_updates_existing_rows_and_ignores_out_of_range_indexes() {
    let mut app = ModelRoutesSettingsState::default();
    app.routes = vec![
        route("chat", "openai", "gpt-5", "3"),
        route("code", "anthropic", "claude-sonnet", "9"),
    ];
    app.save_error = Some(("stale".to_string()).into());

    let _ = update(&mut app, ModelRoutesMessage::RemoveRoute(9));

    assert_eq!(app.routes.len(), 2);

    let _ = update(&mut app, ModelRoutesMessage::RemoveRoute(0));

    assert_eq!(app.routes.len(), 1);
    assert_eq!(app.routes[0].pattern, "code");

    let _ = update(&mut app, ModelRoutesMessage::RemoveRoute(0));

    assert!(app.routes.is_empty());
}
