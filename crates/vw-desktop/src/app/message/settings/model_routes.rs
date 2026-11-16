//! 模型路由草稿校验与共享异步保存。
use super::messages::{ModelRoutesMessage, SettingsMessage};
use super::routing_persistence;
use crate::app::{App, Message, state::QueryClassificationRuleInput};
use iced::Task;

#[cfg(test)]
#[path = "model_routes_tests.rs"]
mod model_routes_tests;

fn queue_valid_routes(app: &mut App) -> Task<Message> {
    let Some(projection) =
        desktop_settings_providers::model_routes::prepare_routes(&mut app.model_routes_settings)
    else {
        return Task::none();
    };
    let rules = projection.rules;
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
    routing_persistence::enqueue(app, projection.patch)
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
                || !app
                    .model_routes_settings
                    .persistence
                    .load_is_current(&crate::app::config::gateway_client_endpoint())
            {
                return Task::none();
            }
            match result {
                Ok((routes, classification)) => {
                    let (routes, error) = desktop_settings_providers::model_routes::loaded_routes(
                        routes,
                        &classification,
                    );
                    app.model_routes_settings.routes = routes;
                    app.model_routes_settings.save_error = error;
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
        message => {
            use desktop_settings_providers::view_message::ModelRoutesMessage as Local;
            let message = match message {
                ModelRoutesMessage::AddRoute => Local::AddRoute,
                ModelRoutesMessage::RemoveRoute(index) => Local::RemoveRoute(index),
                ModelRoutesMessage::PatternChanged(index, value) => {
                    Local::PatternChanged(index, value)
                }
                ModelRoutesMessage::ProviderChanged(index, value) => {
                    Local::ProviderChanged(index, value)
                }
                ModelRoutesMessage::ModelChanged(index, value) => Local::ModelChanged(index, value),
                ModelRoutesMessage::PriorityChanged(index, value) => {
                    Local::PriorityChanged(index, value)
                }
                _ => unreachable!("persistence messages handled above"),
            };
            desktop_settings_providers::model_routes::edit_routes(
                &mut app.model_routes_settings.routes,
                message,
            );
        }
    }
    app.model_routes_settings.persistence.edited();
    queue_valid_routes(app)
}
