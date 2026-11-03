//! 处理 ACP 系统设置页的加载、新增与启用状态更新。

use std::time::Duration;

use crate::app::config::{
    AcpSettingsSnapshot, add_global_acp_agent_async, save_global_acp_agent_async,
    set_global_acp_agent_enabled_async,
};
use crate::app::{App, Message};
use iced::Task;
use vw_config_types::config::AcpAgentConfig;

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

fn new_agent_request(app: &App) -> Result<(String, AcpAgentConfig), String> {
    let name = app.acp_settings.new_agent_name_input.trim();
    if name.is_empty() {
        return Err("ACP 名称不能为空".to_string());
    }
    if app.acp_settings.editing_agent.is_none() && app.acp_settings.catalog.contains_key(name) {
        return Err(format!("ACP 名称已存在: {name}"));
    }

    let command = app.acp_settings.new_agent_command_input.trim();
    if command.is_empty() {
        return Err("ACP 启动命令不能为空".to_string());
    }

    if app.acp_settings.editing_agent.as_deref().is_some_and(|original| original != name) {
        return Err("已有 ACP 不支持重命名".into());
    }
    let mut env = std::collections::HashMap::new();
    for (key, value) in &app.acp_settings.editor_env {
        if key.trim().is_empty() || key.contains(['=', '\0']) {
            return Err("环境变量名称不能为空或包含等号、空字符".into());
        }
        if env.insert(key.clone(), value.clone()).is_some() {
            return Err("环境变量名称不能重复".into());
        }
    }
    Ok((
        name.to_string(),
        AcpAgentConfig {
            command: command.to_string(),
            args: app.acp_settings.editor_args.clone(),
            env,
        },
    ))
}

/// 处理 `AcpMessage` 对应的刷新、新增、启用和禁用流程。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Acp(message) = message else {
        return Task::none();
    };

    if app.acp_settings.adding_agent && !matches!(message, AcpMessage::AddAgentCompleted { .. }) {
        return Task::none();
    }
    match message {
        AcpMessage::EditAgent(agent) => {
            if let Some(config) = app.acp_settings.catalog.get(&agent) {
                app.acp_settings.editing_agent = Some(agent.clone());
                app.acp_settings.new_agent_name_input = agent;
                app.acp_settings.new_agent_command_input = config.command.clone();
                app.acp_settings.editor_args = config.args.clone();
                app.acp_settings.editor_env =
                    config.env.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                app.acp_settings.editor_env.sort_by(|a, b| a.0.cmp(&b.0));
                app.acp_settings.new_agent_expanded = true;
                app.acp_settings.save_error = None;
                app.acp_settings.error_translation = None;
            }
            Task::none()
        }
        AcpMessage::CancelEditor => {
            app.acp_settings.new_agent_expanded = false;
            app.acp_settings.editing_agent = None;
            Task::none()
        }
        AcpMessage::AddArgument => {
            app.acp_settings.editor_args.push(String::new());
            Task::none()
        }
        AcpMessage::RemoveArgument(i) => {
            if i < app.acp_settings.editor_args.len() {
                app.acp_settings.editor_args.remove(i);
            }
            Task::none()
        }
        AcpMessage::ArgumentChanged(i, value) => {
            if let Some(arg) = app.acp_settings.editor_args.get_mut(i) {
                *arg = value;
            }
            Task::none()
        }
        AcpMessage::AddEnvironment => {
            app.acp_settings.editor_env.push(Default::default());
            Task::none()
        }
        AcpMessage::RemoveEnvironment(i) => {
            if i < app.acp_settings.editor_env.len() {
                app.acp_settings.editor_env.remove(i);
            }
            Task::none()
        }
        AcpMessage::EnvironmentChanged(i, key, value) => {
            if let Some(pair) = app.acp_settings.editor_env.get_mut(i) {
                if key {
                    pair.0 = value;
                } else {
                    pair.1 = value;
                }
            }
            Task::none()
        }

        AcpMessage::Refresh => {
            app.acp_settings.loading = true;
            app.acp_settings.save_error = None;
            app.acp_settings.error_translation = None;
            app.acp_settings.status_message = None;
            app.acp_settings.status_translation = None;
            refresh_task()
        }
        AcpMessage::Loaded(result) => {
            app.acp_settings.loading = false;
            match result {
                Ok(snapshot) => {
                    apply_snapshot(app, snapshot);
                    app.acp_settings.save_error = None;
                    app.acp_settings.error_translation = None;
                }
                Err(err) => {
                    app.acp_settings.save_error = Some(err);
                }
            }
            Task::none()
        }
        AcpMessage::NewAgentPanelToggled => {
            app.acp_settings.new_agent_expanded = !app.acp_settings.new_agent_expanded;
            app.acp_settings.editing_agent = None;
            app.acp_settings.new_agent_name_input.clear();
            app.acp_settings.new_agent_command_input.clear();
            app.acp_settings.editor_args.clear();
            app.acp_settings.editor_env.clear();
            app.acp_settings.save_error = None;
            app.acp_settings.error_translation = None;
            Task::none()
        }
        AcpMessage::NewAgentNameChanged(value) => {
            if app.acp_settings.editing_agent.is_none() {
                app.acp_settings.new_agent_name_input = value;
            }
            app.acp_settings.save_error = None;
            app.acp_settings.error_translation = None;
            Task::none()
        }
        AcpMessage::NewAgentCommandChanged(value) => {
            app.acp_settings.new_agent_command_input = value;
            app.acp_settings.save_error = None;
            app.acp_settings.error_translation = None;
            Task::none()
        }
        AcpMessage::NewAgentArgsChanged(value) => {
            app.acp_settings.editor_args = value.lines().map(str::to_owned).collect();
            app.acp_settings.new_agent_args_input = value;
            app.acp_settings.save_error = None;
            app.acp_settings.error_translation = None;
            Task::none()
        }
        AcpMessage::AddAgent => {
            let (agent, spec) = match new_agent_request(app) {
                Ok(request) => request,
                Err(error) => {
                    app.acp_settings.error_translation = None;
                    app.acp_settings.save_error = Some(error);
                    return Task::none();
                }
            };
            app.acp_settings.adding_agent = true;
            app.acp_settings.save_error = None;
            app.acp_settings.error_translation = None;
            app.acp_settings.status_message = None;
            app.acp_settings.status_translation = None;
            let editing = app.acp_settings.editing_agent.is_some();
            let name = agent.clone();
            Task::perform(
                async move {
                    if editing {
                        save_global_acp_agent_async(name, spec).await
                    } else {
                        add_global_acp_agent_async(name, spec).await
                    }
                },
                move |result| {
                    Message::Settings(SettingsMessage::Acp(AcpMessage::AddAgentCompleted {
                        agent,
                        result,
                    }))
                },
            )
        }
        AcpMessage::AddAgentCompleted { agent, result } => {
            app.acp_settings.adding_agent = false;
            match result {
                Ok(snapshot) => {
                    apply_snapshot(app, snapshot);
                    app.acp_settings.editing_agent = None;
                    app.acp_settings.editor_args.clear();
                    app.acp_settings.editor_env.clear();
                    app.acp_settings.new_agent_expanded = false;
                    app.acp_settings.new_agent_name_input.clear();
                    app.acp_settings.new_agent_command_input.clear();
                    app.acp_settings.new_agent_args_input.clear();
                    app.acp_settings.save_error = None;
                    app.acp_settings.error_translation = None;
                    app.acp_settings.status_translation =
                        Some(crate::app::i18n::format::Message::new(
                            "%s 已保存",
                            vec![agent.clone().into()],
                        ));
                    app.acp_settings.status_message = Some(format!("{agent} 已保存"));
                }
                Err(error) => {
                    app.acp_settings.save_error = Some(error);
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
            app.acp_settings.error_translation = None;
            app.acp_settings.status_message = None;
            app.acp_settings.status_translation = None;
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
                    app.acp_settings.error_translation = None;
                    app.acp_settings.status_translation =
                        Some(crate::app::i18n::format::Message::new(
                            if enabled { "%s 已启用" } else { "%s 已禁用" },
                            vec![agent.clone().into()],
                        ));
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
