//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::config::server_config_unreachable_error;
use crate::app::config::update_system_settings_config;
use crate::app::i18n::settings_models::messages::{Feedback, Message as OwnedMessage};
use crate::app::{
    App, Message, load_model_routes_config, load_query_classification_config,
    state::{ModelRoute, QueryClassificationRuleInput},
    update_model_routes_config_result, update_query_classification_config_result,
};
use iced::Task;
use vw_config_types::routing::{ClassificationRule, ModelRouteConfig};

use super::messages::{ModelRoutesMessage, SettingsMessage};

fn sync_to_config(app: &mut App) {
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

    update_system_settings_config(|system| {
        system.model_routes = ui_routes;
    });
    if let Err(err) = update_model_routes_config_result(|routes| {
        *routes = model_routes;
    }) {
        app.model_routes_settings.save_error = Some((server_config_unreachable_error(err)).into());
    }
}

#[cfg(test)]
#[path = "model_routes_tests.rs"]
mod model_routes_tests;

fn sync_query_classification(app: &mut App) {
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

    if let Err(err) = update_query_classification_config_result(|cfg| {
        cfg.rules = rules;
        cfg.enabled = !cfg.rules.is_empty();
    }) {
        app.query_classification_settings.save_error = Some(server_config_unreachable_error(err));
    }
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
            return Err(Feedback::owned(
                OwnedMessage::RouteMissingPattern,
                vec![(idx + 1).to_string()],
            ));
        }
        if provider.is_empty() {
            return Err(Feedback::owned(
                OwnedMessage::RouteMissingProvider,
                vec![(idx + 1).to_string()],
            ));
        }
        if model.is_empty() {
            return Err(Feedback::owned(
                OwnedMessage::RouteMissingModel,
                vec![(idx + 1).to_string()],
            ));
        }
        if !priority.is_empty() && priority.parse::<u32>().is_err() {
            return Err(Feedback::owned(
                OwnedMessage::RoutePriorityInvalid,
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

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::ModelRoutes(message) = message else {
        return Task::none();
    };

    match message {
        ModelRoutesMessage::Refresh => {
            let routes = load_model_routes_config();
            let query_classification = load_query_classification_config();
            app.model_routes_settings.routes = routes
                .into_iter()
                .map(|route| {
                    let hint = route.hint;
                    ModelRoute {
                        pattern: hint.clone(),
                        provider: route.provider,
                        model: route.model,
                        priority_input: query_classification
                            .rules
                            .iter()
                            .find(|rule| rule.hint == hint)
                            .map(|rule| rule.priority.to_string())
                            .unwrap_or_else(|| "0".to_string()),
                    }
                })
                .collect();
            app.query_classification_settings.enabled = query_classification.enabled;
            app.query_classification_settings.rules = query_classification
                .rules
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
            app.model_routes_settings.save_error = if app.model_routes_settings.routes.is_empty()
                && !query_classification.rules.is_empty()
            {
                Some(Feedback::owned(OwnedMessage::ClassificationWithoutRoutes, vec![]))
            } else {
                None
            };
            Task::none()
        }
        ModelRoutesMessage::AddRoute => {
            app.model_routes_settings.routes.push(ModelRoute::default());
            app.model_routes_settings.save_error = None;
            sync_to_config(app);
            sync_query_classification(app);
            Task::none()
        }
        ModelRoutesMessage::RemoveRoute(idx) => {
            if idx < app.model_routes_settings.routes.len() {
                app.model_routes_settings.routes.remove(idx);
            }
            app.model_routes_settings.save_error = None;
            sync_to_config(app);
            sync_query_classification(app);
            Task::none()
        }
        ModelRoutesMessage::PatternChanged(idx, value) => {
            if let Some(route) = app.model_routes_settings.routes.get_mut(idx) {
                route.pattern = value;
            }
            match validate_routes(app) {
                Ok(_) => {
                    app.model_routes_settings.save_error = None;
                    sync_to_config(app);
                    sync_query_classification(app);
                }
                Err(err) => app.model_routes_settings.save_error = Some((err).into()),
            }
            Task::none()
        }
        ModelRoutesMessage::ProviderChanged(idx, value) => {
            if let Some(route) = app.model_routes_settings.routes.get_mut(idx) {
                route.provider = value;
            }
            match validate_routes(app) {
                Ok(_) => {
                    app.model_routes_settings.save_error = None;
                    sync_to_config(app);
                    sync_query_classification(app);
                }
                Err(err) => app.model_routes_settings.save_error = Some((err).into()),
            }
            Task::none()
        }
        ModelRoutesMessage::ModelChanged(idx, value) => {
            if let Some(route) = app.model_routes_settings.routes.get_mut(idx) {
                route.model = value;
            }
            match validate_routes(app) {
                Ok(_) => {
                    app.model_routes_settings.save_error = None;
                    sync_to_config(app);
                    sync_query_classification(app);
                }
                Err(err) => app.model_routes_settings.save_error = Some((err).into()),
            }
            Task::none()
        }
        ModelRoutesMessage::PriorityChanged(idx, value) => {
            if let Some(route) = app.model_routes_settings.routes.get_mut(idx) {
                route.priority_input = value;
            }
            match validate_routes(app) {
                Ok(_) => {
                    app.model_routes_settings.save_error = None;
                    sync_to_config(app);
                    sync_query_classification(app);
                }
                Err(err) => app.model_routes_settings.save_error = Some((err).into()),
            }
            Task::none()
        }
    }
}
