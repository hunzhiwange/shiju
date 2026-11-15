//! 领域更新效果的宿主持久化适配。
use crate::app::{App, Message};
use iced::Task;
use super::messages::*;
pub(super) use desktop_settings_runtime::query_classification::rule_input_from_config;
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
let SettingsMessage::QueryClassification(message) = message else { return Task::none(); };
let message = desktop_settings_runtime::messages::SettingsMessage::QueryClassification(message);
use desktop_settings_runtime::query_classification::{self, Effect};
// Refresh never marks a draft as edited.
let refresh = matches!(&message, desktop_settings_runtime::messages::SettingsMessage::QueryClassification(QueryClassificationMessage::Refresh));
if !refresh { app.model_routes_settings.persistence.edited(); }
match query_classification::update(&mut app.query_classification_settings, message) {
 Effect::None => Task::none(), Effect::Refresh => super::routing_persistence::refresh(app),
 Effect::Save(patch) => { app.model_routes_settings.persistence.classification_invalid = false; super::routing_persistence::enqueue(app, patch) },
 Effect::Invalid => { app.model_routes_settings.persistence.classification_invalid = true; Task::none() },
}
}
#[cfg(test)]
#[path = "query_classification_tests.rs"]
mod query_classification_tests;
