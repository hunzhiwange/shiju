//! 领域更新效果的宿主持久化适配。
use crate::app::{App, Message};
use iced::Task;
use super::messages::*;

pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Runtime(message) = message else { return Task::none(); };
    let message = desktop_settings_runtime::messages::SettingsMessage::Runtime(message);
    let Some(config) = desktop_settings_runtime::runtime::update(&mut app.runtime_settings, message) else { return Task::none(); };
    crate::app::config::update_runtime_config_async(move |current| *current = config)
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;
