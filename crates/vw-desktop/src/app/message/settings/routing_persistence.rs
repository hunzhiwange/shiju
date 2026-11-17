//! Desktop mapping for the provider-owned routing persistence queue.
use super::messages::{ModelRoutesMessage, SettingsMessage};
use crate::app::config::gateway_client_endpoint;
use crate::app::{App, Message};
use iced::Task;
use serde_json::Value;
use desktop_settings_providers::routing_persistence as domain;

fn message(event: domain::Message) -> Message {
    Message::Settings(SettingsMessage::ModelRoutes(match event {
        domain::Message::Saved(revision, result) => ModelRoutesMessage::Saved(revision, result),
        domain::Message::Loaded(revision, result) => ModelRoutesMessage::Loaded(revision, result),
    }))
}
pub(super) fn enqueue(app: &mut App, patch: Value) -> Task<Message> {
    let revision = domain::enqueue(&mut app.model_routes_settings.persistence, gateway_client_endpoint(), patch);
    crate::app::message::after(std::time::Duration::from_millis(300), Message::Settings(SettingsMessage::ModelRoutes(ModelRoutesMessage::SaveDue(revision))))
}
pub(super) fn save_due(app: &mut App, revision: u64) -> Task<Message> {
    domain::save_due(&mut app.model_routes_settings.persistence, revision).map(message)
}
pub(super) fn saved(app: &mut App, revision: u64, result: Result<(), String>) -> Task<Message> {
    let outcome = domain::saved(&mut app.model_routes_settings.persistence, &gateway_client_endpoint(), revision, result);
    if outcome.clear_errors {
        app.model_routes_settings.save_error = None;
        app.query_classification_settings.save_error = None;
        app.query_classification_settings.save_error_translation = None;
    }
    if let Some(error) = outcome.error {
        let error = crate::app::config::server_config_unreachable_error(error);
        if !app.model_routes_settings.persistence.route_invalid {
            app.model_routes_settings.save_error = Some(error.clone().into());
        }
        if !app.model_routes_settings.persistence.classification_invalid {
            app.query_classification_settings.save_error = Some(error);
            app.query_classification_settings.save_error_translation = None;
        }
    }
    let next = outcome.task.map(message);
    if outcome.show_error {
        return Task::batch([next, app.show_error_toast("路由分类配置保存失败，请重试")]);
    }
    next
}
pub(super) fn refresh(app: &mut App) -> Task<Message> {
    domain::refresh(&mut app.model_routes_settings.persistence, gateway_client_endpoint()).map(message)
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn flush(app: &mut App) -> (Task<Message>, Option<tokio::sync::watch::Receiver<bool>>) {
    let (task, idle) = app.model_routes_settings.persistence.flush();
    (task.map(message), idle)
}
#[cfg(test)]
#[path = "routing_persistence_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) use domain::RoutingPersistence;
