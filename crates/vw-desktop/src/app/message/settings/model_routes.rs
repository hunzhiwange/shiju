//! 模型路由草稿校验与共享异步保存。
use super::messages::{ModelRoutesMessage, SettingsMessage};
use super::routing_persistence;
use crate::app::i18n::settings_models::messages::Feedback;
use crate::app::{
    App, Message,
    state::{ModelRoute, QueryClassificationRuleInput},
};
use iced::Task;
use vw_config_types::routing::{ClassificationRule, ModelRouteConfig};

#[cfg(test)]
#[path = "model_routes_tests.rs"]
mod model_routes_tests;

fn queue_valid_routes(app: &mut App) -> Task<Message> {
    if let Err(error) = validate_routes(app) {
        app.model_routes_settings.persistence.route_invalid = true;
        app.model_routes_settings.save_error = Some(error);
        return Task::none();
    }
    app.model_routes_settings.persistence.route_invalid = false;
    app.model_routes_settings.persistence.classification_invalid = false;
    app.model_routes_settings.save_error = None;
    let ui_routes = app
        .model_routes_settings
        .routes
        .iter()
        .filter(|route| {
            !route.pattern.trim().is_empty()
                || !route.provider.trim().is_empty()
                || !route.model.trim().is_empty()
                || !route.priority_input.trim().is_empty()
        })
        .map(|route| vw_config_types::ui::ModelRoute {
            pattern: route.pattern.trim().to_string(),
            provider: route.provider.trim().to_string(),
            model: route.model.trim().to_string(),
            priority: route.priority_input.trim().parse::<u32>().unwrap_or(0),
        })
        .collect::<Vec<_>>();

    let model_routes = ui_routes
        .iter()
        .map(|route| ModelRouteConfig {
            hint: route.pattern.clone(),
            provider: route.provider.clone(),
            model: route.model.clone(),
            max_tokens: None,
            api_key: None,
        })
        .collect::<Vec<_>>();

    let rules = app
        .model_routes_settings
        .routes
        .iter()
        .filter_map(|route| {
            let hint = route.pattern.trim();
            if hint.is_empty() {
                return None;
            }
            Some(ClassificationRule {
                hint: hint.to_string(),
                keywords: vec![hint.to_string()],
                patterns: Vec::new(),
                min_length: None,
                max_length: None,
                priority: route.priority_input.trim().parse::<i32>().unwrap_or(0),
            })
        })
        .collect::<Vec<_>>();

    app.query_classification_settings.enabled = !rules.is_empty();
    app.query_classification_settings.rules = rules
        .iter()
        .map(|rule| QueryClassificationRuleInput {
            pattern: rule
                .patterns
                .first()
                .cloned()
                .or_else(|| rule.keywords.first().cloned())
                .unwrap_or_else(|| rule.hint.clone()),
            category: rule.hint.clone(),
            priority_input: rule.priority.to_string(),
        })
        .collect();
    app.query_classification_settings.save_error = None;

    app.query_classification_settings.save_error_translation = None;
    routing_persistence::enqueue(
        app,
        serde_json::json!({
            "app_ui": {"system_settings": {"model_routes": ui_routes}},
            "model_routes": model_routes,
            "query_classification": {"enabled": !rules.is_empty(), "rules": rules}
        }),
    )
}

fn validate_routes(app: &App) -> Result<Vec<ModelRouteConfig>, Feedback> {
    let mut routes = Vec::with_capacity(app.model_routes_settings.routes.len());
    for (idx, route) in app.model_routes_settings.routes.iter().enumerate() {
        let pattern = route.pattern.trim();
        let provider = route.provider.trim();
        let model = route.model.trim();
        let priority = route.priority_input.trim();

        if pattern.is_empty() && provider.is_empty() && model.is_empty() && priority.is_empty() {
            continue;
        }
        if pattern.is_empty() {
            return Err(Feedback::owned("第 %s 条路由缺少 pattern", vec![(idx + 1).to_string()]));
        }
        if provider.is_empty() {
            return Err(Feedback::owned("第 %s 条路由缺少 provider", vec![(idx + 1).to_string()]));
        }
        if model.is_empty() {
            return Err(Feedback::owned("第 %s 条路由缺少 model", vec![(idx + 1).to_string()]));
        }
        if !priority.is_empty() && priority.parse::<u32>().is_err() {
            return Err(Feedback::owned(
                "第 %s 条路由的 priority 不是有效数字",
                vec![(idx + 1).to_string()],
            ));
        }

        routes.push(ModelRouteConfig {
            hint: pattern.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            max_tokens: None,
            api_key: None,
        });
    }
    Ok(routes)
}

pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::ModelRoutes(message) = message else {
        return Task::none();
    };
    match message {
        ModelRoutesMessage::Refresh => return routing_persistence::refresh(app),
        ModelRoutesMessage::SaveDue(revision) => {
            return routing_persistence::save_due(app, revision);
        }
        ModelRoutesMessage::Saved(revision, result) => {
            return routing_persistence::saved(app, revision, result);
        }
        ModelRoutesMessage::Loaded(revision, result) => {
            if app.model_routes_settings.persistence.revision != revision
                || app.model_routes_settings.persistence.dirty
                || !app.model_routes_settings.persistence.load_is_current()
            {
                return Task::none();
            }
            match result {
                Ok((routes, classification)) => {
                    app.model_routes_settings.routes = routes
                        .into_iter()
                        .map(|route| ModelRoute {
                            priority_input: classification
                                .rules
                                .iter()
                                .find(|rule| rule.hint == route.hint)
                                .map(|rule| rule.priority.to_string())
                                .unwrap_or_else(|| "0".into()),
                            pattern: route.hint,
                            provider: route.provider,
                            model: route.model,
                        })
                        .collect();
                    app.model_routes_settings.save_error =
                        if app.model_routes_settings.routes.is_empty()
                            && !classification.rules.is_empty()
                        {
                            Some(Feedback::owned(
                                "已检测到 query_classification 规则，但桌面模型路由列表为空",
                                vec![],
                            ))
                        } else {
                            None
                        };
                    app.query_classification_settings.enabled = classification.enabled;
                    app.query_classification_settings.rules = classification
                        .rules
                        .into_iter()
                        .map(super::query_classification::rule_input_from_config)
                        .collect();
                    app.query_classification_settings.save_error = None;
                    app.query_classification_settings.save_error_translation = None;
                }
                Err(error) => {
                    let error = crate::app::config::server_config_unreachable_error(error);
                    app.model_routes_settings.save_error = Some(error.clone().into());
                    app.query_classification_settings.save_error = Some(error);
                    app.query_classification_settings.save_error_translation = None;
                }
            }
            return Task::none();
        }
        ModelRoutesMessage::AddRoute => {
            app.model_routes_settings.routes.push(ModelRoute::default())
        }
        ModelRoutesMessage::RemoveRoute(index) => {
            if index < app.model_routes_settings.routes.len() {
                app.model_routes_settings.routes.remove(index);
            }
        }
        ModelRoutesMessage::PatternChanged(index, value) => {
            if let Some(route) = app.model_routes_settings.routes.get_mut(index) {
                route.pattern = value;
            }
        }
        ModelRoutesMessage::ProviderChanged(index, value) => {
            if let Some(route) = app.model_routes_settings.routes.get_mut(index) {
                route.provider = value;
            }
        }
        ModelRoutesMessage::ModelChanged(index, value) => {
            if let Some(route) = app.model_routes_settings.routes.get_mut(index) {
                route.model = value;
            }
        }
        ModelRoutesMessage::PriorityChanged(index, value) => {
            if let Some(route) = app.model_routes_settings.routes.get_mut(index) {
                route.priority_input = value;
            }
        }
    }
    app.model_routes_settings.persistence.edited();
    queue_valid_routes(app)
}
