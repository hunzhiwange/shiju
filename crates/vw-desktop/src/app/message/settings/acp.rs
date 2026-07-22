//! 处理 ACP 系统设置页的加载与启用状态更新。

use std::time::Duration;

use crate::app::config::{AcpSettingsSnapshot, set_global_acp_agent_enabled_async};
use crate::app::{App, Message};
use iced::Task;

use super::messages::{AcpMessage, SettingsMessage};

fn sort_acp_agent_names(mut names: Vec<String>) -> Vec<String> {
    names.sort_by(|left, right| {
        let left_key = (left != "codex", left.as_str());
        let right_key = (right != "codex", right.as_str());
        left_key.cmp(&right_key)
    });
    names
}

fn refresh_task() -> Task<Message> {
    Task::perform(crate::app::config::load_acp_settings_snapshot_async(), |result| {
        Message::Settings(SettingsMessage::Acp(AcpMessage::Loaded(result)))
    })
}

fn apply_snapshot(app: &mut App, snapshot: AcpSettingsSnapshot) {
    app.acp_settings.catalog = snapshot.catalog;
    app.acp_settings.enabled = snapshot.enabled.keys().cloned().collect();
    app.acp_agents = sort_acp_agent_names(app.acp_settings.enabled.iter().cloned().collect());
    if app.acp_agent.as_ref().is_some_and(|agent| !app.acp_settings.enabled.contains(agent)) {
        app.acp_agent = None;
    }
}

/// 处理 `AcpMessage` 对应的刷新、启用和禁用流程。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Acp(message) = message else {
        return Task::none();
    };

    match message {
        AcpMessage::Refresh => {
            app.acp_settings.loading = true;
            app.acp_settings.save_error = None;
            app.acp_settings.status_message = None;
            refresh_task()
        }
        AcpMessage::Loaded(result) => {
            app.acp_settings.loading = false;
            match result {
                Ok(snapshot) => {
                    apply_snapshot(app, snapshot);
                    app.acp_settings.save_error = None;
                }
                Err(err) => {
                    app.acp_settings.save_error = Some(err);
                }
            }
            Task::none()
        }
        AcpMessage::CopyCommand { agent, command } => {
            app.acp_settings.copied_agent = Some(agent.clone());
            Task::batch(vec![
                iced::clipboard::write(command),
                crate::app::message::after(
                    Duration::from_secs(2),
                    Message::Settings(SettingsMessage::Acp(AcpMessage::ClearCopiedCommand(agent))),
                ),
            ])
        }
        AcpMessage::ClearCopiedCommand(agent) => {
            if app.acp_settings.copied_agent.as_deref() == Some(agent.as_str()) {
                app.acp_settings.copied_agent = None;
            }
            Task::none()
        }
        AcpMessage::SetEnabled { agent, enabled } => {
            let spec = app.acp_settings.catalog.get(&agent).cloned();
            app.acp_settings.saving_agent = Some(agent.clone());
            app.acp_settings.save_error = None;
            app.acp_settings.status_message = None;
            Task::perform(set_global_acp_agent_enabled_async(agent.clone(), enabled, spec), {
                move |result| {
                    Message::Settings(SettingsMessage::Acp(AcpMessage::SetEnabledCompleted {
                        agent,
                        enabled,
                        result,
                    }))
                }
            })
        }
        AcpMessage::SetEnabledCompleted { agent, enabled, result } => {
            app.acp_settings.saving_agent = None;
            match result {
                Ok(snapshot) => {
                    apply_snapshot(app, snapshot);
                    app.acp_settings.save_error = None;
                    app.acp_settings.status_message =
                        Some(format!("{} 已{}", agent, if enabled { "启用" } else { "禁用" }));
                }
                Err(err) => {
                    app.acp_settings.save_error = Some(err);
                }
            }
            Task::none()
        }
    }
}

#[cfg(test)]
#[path = "acp_tests.rs"]
mod acp_tests;
