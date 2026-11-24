// 处理 ACP 系统设置页的加载、新增与启用状态更新。

use std::time::Duration;

use desktop_config::AcpSettingsSnapshot;
use crate::acp_state::{AcpDiagnosticStatus, AcpSettingsState};
use super::acp_update_message::{Message, AcpMessage};
use iced::Task;
use config_aggregate::AcpAgentConfig;



pub fn sort_acp_agent_names(mut names: Vec<String>) -> Vec<String> {
    names.sort_by(|left, right| {
        let left_key = (left != "codex", left.as_str());
        let right_key = (right != "codex", right.as_str());
        left_key.cmp(&right_key)
    });
    names
}

fn refresh_task(access: desktop_config::ConfigAccess) -> Task<Message> {
    Task::perform(async move { access.load_acp_settings_snapshot_async().await }, |result| {
        Message::Acp(AcpMessage::Loaded(result))
    })
}

fn apply_snapshot(app: &mut Update, snapshot: AcpSettingsSnapshot) {
    #[cfg(not(target_arch = "wasm32"))]
    { app.snapshot_applied = true; }
    app.state.diagnostic_results.retain(|agent, _| {
        app.state.catalog.get(agent).zip(snapshot.catalog.get(agent)).is_some_and(
            |(old, new)| old.command == new.command && old.args == new.args && old.env == new.env,
        )
    });
    app.state.defaults = snapshot.defaults;
    app.state.catalog = snapshot.catalog;
    app.state.installed = snapshot.installed;
    app.state.enabled = snapshot.enabled.keys().cloned().collect();
    *app.acp_agents = sort_acp_agent_names(app.state.enabled.iter()
        .filter(|agent| agent.as_str() != acp_agent_registry::EMPLOYEE_ACP_BACKEND)
        .cloned()
        .collect());
    if app.acp_agent.as_ref().is_some_and(|agent| !app.state.enabled.contains(agent)) {
        *app.acp_agent = None;
    }
}

fn request(state: &AcpSettingsState) -> Result<(String, AcpAgentConfig), desktop_i18n::format::Message> {
    let name = state.new_agent_name_input.trim();
    if name.is_empty() {
        return Err(desktop_i18n::format::Message::new("ACP 名称不能为空", vec![]));
    }
    if state.editing_agent.is_none() && state.catalog.contains_key(name) {
        return Err(desktop_i18n::format::Message::new("ACP 名称已存在: %s", vec![name.into()]));
    }

    let command = state.new_agent_command_input.trim();
    if command.is_empty() {
        return Err(desktop_i18n::format::Message::new("ACP 启动命令不能为空", vec![]));
    }

    if state.editing_agent.as_deref().is_some_and(|original| original != name) {
        return Err(desktop_i18n::format::Message::new("已有 ACP 不支持重命名", vec![]));
    }
    let mut env = std::collections::HashMap::new();
    for (key, value) in &state.editor_env {
        if key.trim().is_empty() || key.contains(['=', '\0']) {
            return Err(desktop_i18n::format::Message::new("环境变量名称不能为空或包含等号、空字符", vec![]));
        }
        if env.insert(key.clone(), value.clone()).is_some() {
            return Err(desktop_i18n::format::Message::new("环境变量名称不能重复", vec![]));
        }
    }
    Ok((
        name.to_string(),
        AcpAgentConfig {
            command: command.to_string(),
            args: state.editor_args.clone(),
            env,
        },
    ))
}

/// 处理 `AcpMessage` 对应的刷新、新增、启用和禁用流程。
pub fn update(app: &mut Update, message: AcpMessage) -> Task<Message> {

    if app.state.adding_agent
        && !matches!(
            message,
            AcpMessage::AddAgentCompleted { .. }
                | AcpMessage::InstallProgressLoaded { .. }
                | AcpMessage::CancelInstall
                | AcpMessage::InstallCancelFailed { .. }
                | AcpMessage::InstallCompleted { .. }
                | AcpMessage::InstallInstructionsLoaded { .. }
        )
    {
        return Task::none();
    }
    let invalidates = matches!(
        message,
        AcpMessage::AddAgent
            | AcpMessage::EditAgent(_)
            | AcpMessage::CancelEditor
            | AcpMessage::NewAgentPanelToggled
            | AcpMessage::NewAgentNameChanged(_)
            | AcpMessage::NewAgentCommandChanged(_)
            | AcpMessage::NewAgentArgsChanged(_)
            | AcpMessage::RestoreDefaultArguments
            | AcpMessage::RestoreDefaultEnvironment
            | AcpMessage::ArgumentChanged(..)
            | AcpMessage::AddArgument
            | AcpMessage::RemoveArgument(_)
            | AcpMessage::EnvironmentChanged(..)
            | AcpMessage::AddEnvironment
            | AcpMessage::RemoveEnvironment(_)
    );
    let cancellation = if invalidates {
        app.state.diagnostic_status = None;
        cancel_diagnostic(app)
    } else {
        Task::none()
    };
    let task = match message {
        AcpMessage::InstallFilterChanged(filter) => {
            app.state.install_filter = filter;
            Task::none()
        }
        AcpMessage::CatalogQueryChanged(query) => {
            app.state.catalog_query = query;
            Task::none()
        }
        AcpMessage::ToggleInstallDetails(agent) => {
            if !app.state.install_expanded.insert(agent.clone()) {
                app.state.install_expanded.remove(&agent);
                return Task::none();
            }
            load_install_instructions(app, agent)
        }
        AcpMessage::InstallInstructionsLoaded { agent, result } => {
            app.state.install_instructions.insert(agent, Some(result));
            Task::none()
        }
        AcpMessage::InstallAgent(agent) => {
            if app.state.installing_agent.is_some() {
                return Task::none();
            }
            if !app.state.install_supported(&agent) {
                return app.show_error_toast("该 ACP 暂无一键安装命令，请使用官方安装器");
            }
            let id = operation_id();
            app.state.install_id = Some(id.clone());
            app.state.install_progress = None;
            let progress = poll_install_progress(app, agent.clone(), id.clone());
            app.state.installing_agent = Some(agent.clone());
            app.state.install_expanded.insert(agent.clone());
            app.state.install_reports.remove(&agent);
            app.state.copied_install_reports.remove(&agent);
            let instructions = load_install_instructions(app, agent.clone());
            let client = app.client.clone();
            let install = Task::perform(
                async move {
                    let result = async {
                        let client = client?;
                        gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone())
                            .acp_install_report_with_id(&agent, Some(&id)).await
                    }.await;
                    (id, agent, result)
                },
                |(id, agent, result)| Message::Acp(AcpMessage::InstallCompleted { id, agent, result }),
            );
            Task::batch([instructions, install, progress])
        }
        AcpMessage::CancelInstall => {
            let (Some(agent), Some(id)) = (app.state.installing_agent.clone(), app.state.install_id.clone()) else {
                return Task::none();
            };
            if app.state.cancelling_install {
                return Task::none();
            }
            app.state.cancelling_install = true;
            let client = app.client.clone();
            Task::perform(
                async move {
                    let result = tokio::time::timeout(Duration::from_secs(12), async {
                        let client = gateway_desktop_settings::GatewayDesktopSettingsClient::new(client?.transport().clone());
                        // The cancellation request can reach the gateway before the install request.
                        for _ in 0..40 {
                            if client.acp_install_cancel(&agent, &id).await? {
                                return Ok(());
                            }
                            tokio::time::sleep(Duration::from_millis(250)).await;
                        }
                        Err("网关未确认取消 ACP 安装".into())
                    }).await.unwrap_or_else(|_| Err("等待网关确认取消超时".into()));
                    (id, agent, result)
                },
                |(id, agent, result)| match result {
                    Ok(()) => Message::Acp(AcpMessage::InstallCompleted {
                        id,
                        agent,
                        result: Err(gateway_desktop_settings::ACP_INSTALL_CANCELLED.into()),
                    }),
                    Err(message) => Message::Acp(AcpMessage::InstallCancelFailed { id, message }),
                },
            )
        }
        AcpMessage::InstallProgressLoaded { id, mut progress } => {
            let Some(agent) = app.state.installing_agent.clone() else {
                return Task::none();
            };
            if app.state.install_id.as_deref() != Some(&id) {
                return Task::none();
            }
            if let Some(download) = progress.as_mut().and_then(|progress| progress.download.as_mut())
                && app.state.install_progress.as_ref()
                    .and_then(|previous| previous.download.as_ref())
                    .is_some_and(|previous| previous.downloaded == download.downloaded)
                {
                    download.bytes_per_second = 0;
                }
            app.state.install_progress = progress;
            poll_install_progress(app, agent, id)
        },
        AcpMessage::InstallCancelFailed { id, message } => {
            if app.state.install_id.as_deref() != Some(&id) {
                return Task::none();
            }
            app.state.cancelling_install = false;
            app.show_error_toast(message)
        }
        AcpMessage::InstallCompleted { id, agent, result } => {
            if app.state.install_id.as_deref() != Some(&id) {
                return Task::none();
            }
            app.state.installing_agent = None;
            app.state.install_id = None;
            app.state.install_progress = None;
            app.state.cancelling_install = false;
            let report = result.unwrap_or_else(|message| gateway_desktop_settings::AcpInstallReport {
                success: false,
                message,
                steps: Vec::new(),
            });
            let task = if report.message == gateway_desktop_settings::ACP_INSTALL_CANCELLED {
                app.show_info_toast(report.message.clone())
            } else if report.success {
                let refresh = refresh_task(app.access.clone());
                let toast = app.show_info_toast(report.message.clone());
                Task::batch([refresh, toast])
            } else {
                app.state.install_expanded.insert(agent.clone());
                app.show_error_toast("ACP 安装失败，请查看安装详情")
            };
            app.state.install_reports.insert(agent, report);
            task
        }
        AcpMessage::DiagnosticCancelled => Task::none(),
        AcpMessage::StartDiagnostic | AcpMessage::TestAgent(_) => {
            let from_card = matches!(message, AcpMessage::TestAgent(_));
            let (agent, config) = match message {
                AcpMessage::TestAgent(agent) => {
                    let Some(config) = app.state.catalog.get(&agent).cloned() else {
                        return app.show_error_toast("ACP 不存在，请刷新列表");
                    };
                    (agent, config)
                }
                _ => match request(app.state) {
                    Ok(request) => request,
                    Err(error) => return app.show_error_toast(
                        error.render(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese))
                            .expect("valid ACP validation message"),
                    ),
                },
            };
            let previous = cancel_diagnostic(app);
            let employee = agent == acp_agent_registry::EMPLOYEE_ACP_BACKEND;
            let id = operation_id();
            app.state.diagnostic_id = Some(id.clone());
            app.state.diagnostic_status = Some(AcpDiagnosticStatus::Running);
            if from_card {
                app.state
                    .diagnostic_results
                    .insert(agent.clone(), AcpDiagnosticStatus::Running);
                app.state.diagnostic_agent = Some(agent);
            }
            Task::batch([
                previous,
                { let client = app.client.clone(); Task::perform(
                    async move {
                        let result = async {
                            let client = client.clone()?;
                            let value = if employee {
                                serde_json::json!({"builtin": acp_agent_registry::EMPLOYEE_ACP_BACKEND})
                            } else {
                                serde_json::to_value(config).map_err(|e| e.to_string())?
                            };
                            gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).acp_diagnostic(&id, &value).await
                        }
                        .await;
                        (id, result)
                    },
                    |(id, result)| {
                        Message::Acp(AcpMessage::DiagnosticCompleted {
                            id,
                            result,
                        })
                    },
                ) },
            ])
        }
        AcpMessage::CancelDiagnostic => {
            let task = cancel_diagnostic(app);
            app.state.diagnostic_status = Some(AcpDiagnosticStatus::Cancelled);
            task
        }
        AcpMessage::DiagnosticCompleted { id, result } => {
            if app.state.diagnostic_id.as_deref() != Some(&id) {
                return Task::none();
            }
            app.state.diagnostic_id = None;
            let status = match result.as_deref() {
                Ok("connected") => AcpDiagnosticStatus::Connected,
                Ok("authentication_required") => AcpDiagnosticStatus::AuthenticationRequired,
                Ok("startup_failed") => AcpDiagnosticStatus::StartupFailed,
                Ok("handshake_failed") => AcpDiagnosticStatus::HandshakeFailed,
                Ok("timed_out") => AcpDiagnosticStatus::TimedOut,
                Ok("cancelled") => AcpDiagnosticStatus::Cancelled,
                _ => AcpDiagnosticStatus::Unavailable,
            };
            app.state.diagnostic_status = Some(status);
            if let Some(agent) = app.state.diagnostic_agent.take() {
                app.state.diagnostic_results.insert(agent, status);
            }
            match status {
                AcpDiagnosticStatus::Connected => app.show_success_toast(status.description()),
                AcpDiagnosticStatus::StartupFailed
                | AcpDiagnosticStatus::HandshakeFailed
                | AcpDiagnosticStatus::TimedOut
                | AcpDiagnosticStatus::Unavailable => app.show_error_toast(status.description()),
                _ => app.show_info_toast(status.description()),
            }
        }
        AcpMessage::EditAgent(agent) => {
            if let Some(config) = app.state.catalog.get(&agent) {
                app.state.editing_agent = Some(agent.clone());
                app.state.new_agent_name_input = agent;
                app.state.new_agent_command_input = config.command.clone();
                app.state.editor_args = config.args.clone();
                app.state.editor_env =
                    config.env.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                app.state.editor_env.sort_by(|a, b| a.0.cmp(&b.0));
                app.state.new_agent_expanded = true;
                app.state.save_error = None;
                app.state.error_translation = None;
            }
            Task::none()
        }
        AcpMessage::CancelEditor => {
            app.state.new_agent_expanded = false;
            app.state.editing_agent = None;
            Task::none()
        }
        AcpMessage::RestoreDefaultArguments | AcpMessage::RestoreDefaultEnvironment => {
            let Some(defaults) = app
                .state
                .editing_agent
                .as_ref()
                .and_then(|agent| app.state.defaults.get(agent))
            else {
                return app.show_error_toast("该 ACP 没有内置默认配置");
            };
            if matches!(message, AcpMessage::RestoreDefaultArguments) {
                app.state.editor_args = defaults.args.clone();
            } else {
                app.state.editor_env =
                    defaults.env.iter().map(|(key, value)| (key.clone(), value.clone())).collect();
                app.state.editor_env.sort_by(|a, b| a.0.cmp(&b.0));
            }
            app.state.save_error = None;
            app.state.error_translation = None;
            app.show_info_toast("已恢复默认值，保存后生效")
        }
        AcpMessage::AddArgument => {
            app.state.editor_args.push(String::new());
            Task::none()
        }
        AcpMessage::RemoveArgument(i) => {
            if i < app.state.editor_args.len() {
                app.state.editor_args.remove(i);
            }
            Task::none()
        }
        AcpMessage::ArgumentChanged(i, value) => {
            if let Some(arg) = app.state.editor_args.get_mut(i) {
                *arg = value;
            }
            Task::none()
        }
        AcpMessage::AddEnvironment => {
            app.state.editor_env.push(Default::default());
            Task::none()
        }
        AcpMessage::RemoveEnvironment(i) => {
            if i < app.state.editor_env.len() {
                app.state.editor_env.remove(i);
            }
            Task::none()
        }
        AcpMessage::EnvironmentChanged(i, key, value) => {
            if let Some(pair) = app.state.editor_env.get_mut(i) {
                if key {
                    pair.0 = value;
                } else {
                    pair.1 = value;
                }
            }
            Task::none()
        }

        AcpMessage::Refresh => {
            app.state.loading = true;
            app.state.save_error = None;
            app.state.error_translation = None;
            app.state.status_message = None;
            app.state.status_translation = None;
            refresh_task(app.access.clone())
        }
        AcpMessage::Loaded(result) => {
            app.state.loading = false;
            match result {
                Ok(snapshot) => {
                    apply_snapshot(app, snapshot);
                    app.state.save_error = None;
                    app.state.error_translation = None;
                }
                Err(err) => {
                    app.state.save_error = Some(err);
                }
            }
            Task::none()
        }
        AcpMessage::NewAgentPanelToggled => {
            app.state.new_agent_expanded = !app.state.new_agent_expanded;
            app.state.editing_agent = None;
            app.state.new_agent_name_input.clear();
            app.state.new_agent_command_input.clear();
            app.state.editor_args.clear();
            app.state.editor_env.clear();
            app.state.save_error = None;
            app.state.error_translation = None;
            Task::none()
        }
        AcpMessage::NewAgentNameChanged(value) => {
            if app.state.editing_agent.is_none() {
                app.state.new_agent_name_input = value;
            }
            app.state.save_error = None;
            app.state.error_translation = None;
            Task::none()
        }
        AcpMessage::NewAgentCommandChanged(value) => {
            app.state.new_agent_command_input = value;
            app.state.save_error = None;
            app.state.error_translation = None;
            Task::none()
        }
        AcpMessage::NewAgentArgsChanged(value) => {
            app.state.editor_args = value.lines().map(str::to_owned).collect();
            app.state.new_agent_args_input = value;
            app.state.save_error = None;
            app.state.error_translation = None;
            Task::none()
        }
        AcpMessage::AddAgent => {
            let (agent, spec) = match request(app.state) {
                Ok(request) => request,
                Err(error) => {
                    app.state.save_error = Some(
                        error.render(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese))
                            .expect("valid ACP validation message"),
                    );
                    app.state.error_translation = Some(error);
                    return Task::none();
                }
            };
            app.state.adding_agent = true;
            app.state.save_error = None;
            app.state.error_translation = None;
            app.state.status_message = None;
            app.state.status_translation = None;
            let editing = app.state.editing_agent.is_some();
            let name = agent.clone();
            { let access = app.access.clone(); Task::perform(
                async move {
                    if editing {
                        access.save_global_acp_agent_async(name, spec).await
                    } else {
                        access.add_global_acp_agent_async(name, spec).await
                    }
                },
                move |result| {
                    Message::Acp(AcpMessage::AddAgentCompleted {
                        agent,
                        result,
                    })
                },
            ) }
        }
        AcpMessage::AddAgentCompleted { agent, result } => {
            app.state.adding_agent = false;
            match result {
                Ok(snapshot) => {
                    apply_snapshot(app, snapshot);
                    app.state.editing_agent = None;
                    app.state.editor_args.clear();
                    app.state.editor_env.clear();
                    app.state.new_agent_expanded = false;
                    app.state.new_agent_name_input.clear();
                    app.state.new_agent_command_input.clear();
                    app.state.new_agent_args_input.clear();
                    app.state.save_error = None;
                    app.state.error_translation = None;
                    app.state.status_translation =
                        Some(desktop_i18n::format::Message::new(
                            "%s 已保存",
                            vec![agent.clone().into()],
                        ));
                    app.state.status_message = Some(format!("{agent} 已保存"));
                }
                Err(error) => {
                    app.state.save_error = Some(error);
                }
            }
            Task::none()
        }
        AcpMessage::CopyInstallReport { agent, content } => {
            let copied_at = std::time::Instant::now();
            app.state.copied_install_reports.insert(agent.clone(), copied_at);
            Task::batch([
                iced::clipboard::write(content),
                Task::done(Message::After(
                    Duration::from_secs(2),
                    AcpMessage::ClearCopiedInstallReport { agent, copied_at },
                )),
            ])
        }
        AcpMessage::ClearCopiedInstallReport { agent, copied_at } => {
            if app.state.copied_install_reports.get(&agent) == Some(&copied_at) {
                app.state.copied_install_reports.remove(&agent);
            }
            Task::none()
        }
        AcpMessage::CopyCommand { agent, command } => {
            app.state.copied_agent = Some(agent.clone());
            Task::batch(vec![
                iced::clipboard::write(command),
                Task::done(Message::After(Duration::from_secs(2), AcpMessage::ClearCopiedCommand(agent))),
            ])
        }
        AcpMessage::ClearCopiedCommand(agent) => {
            if app.state.copied_agent.as_deref() == Some(agent.as_str()) {
                app.state.copied_agent = None;
            }
            Task::none()
        }
        AcpMessage::SetEnabled { agent, enabled } => {
            let spec = app.state.catalog.get(&agent).cloned();
            app.state.saving_agent = Some(agent.clone());
            app.state.save_error = None;
            app.state.error_translation = None;
            app.state.status_message = None;
            app.state.status_translation = None;
            { let access = app.access.clone(); let name = agent.clone(); Task::perform(async move { access.set_global_acp_agent_enabled_async(name, enabled, spec).await }, {
                move |result| {
                    Message::Acp(AcpMessage::SetEnabledCompleted {
                        agent,
                        enabled,
                        result,
                    })
                }
            }) }
        }
        AcpMessage::SetEnabledCompleted { agent, enabled, result } => {
            app.state.saving_agent = None;
            match result {
                Ok(snapshot) => {
                    apply_snapshot(app, snapshot);
                    app.state.save_error = None;
                    app.state.error_translation = None;
                    app.state.status_translation =
                        Some(desktop_i18n::format::Message::new(
                            if enabled { "%s 已启用" } else { "%s 已禁用" },
                            vec![agent.clone().into()],
                        ));
                    app.state.status_message =
                        Some(format!("{} 已{}", agent, if enabled { "启用" } else { "禁用" }));
                }
                Err(err) => {
                    app.state.save_error = Some(err);
                }
            }
            Task::none()
        }
    };
    Task::batch([cancellation, task])
}

fn load_install_instructions(app: &mut Update, agent: String) -> Task<Message> {
    // An empty entry denotes an in-flight request. Reopening after an error retries it.
    if matches!(app.state.install_instructions.get(&agent), Some(None | Some(Ok(_)))) {
        return Task::none();
    }
    app.state.install_instructions.insert(agent.clone(), None);
    let client = app.client.clone();
    Task::perform(
        async move {
            let result = async {
                let client = client?;
                gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone())
                    .acp_install_instructions(&agent).await
            }.await;
            (agent, result)
        },
        |(agent, result)| Message::Acp(AcpMessage::InstallInstructionsLoaded { agent, result }),
    )
}

fn cancel_diagnostic(app: &mut Update) -> Task<Message> {
    let Some(id) = app.state.diagnostic_id.take() else {
        return Task::none();
    };
    if let Some(agent) = app.state.diagnostic_agent.take() {
        app.state.diagnostic_results.insert(agent, AcpDiagnosticStatus::Cancelled);
    }
    let client = app.client.clone();
    Task::perform(
        async move {
            if let Ok(client) = client.clone() {
                // Cancellation can overtake the start on a separate connection.
                for _ in 0..40 {
                    match gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone()).acp_diagnostic_cancel(&id).await {
                        Ok(true) | Err(_) => break,
                        Ok(false) => tokio::time::sleep(Duration::from_millis(250)).await,
                    }
                }
            }
        },
        |_| Message::Acp(AcpMessage::DiagnosticCancelled),
    )
}


pub struct Update<'a> {
 pub state: &'a mut AcpSettingsState,
 pub acp_agents: &'a mut Vec<String>,
 pub acp_agent: &'a mut Option<String>,
 pub access: desktop_config::ConfigAccess,
 pub client: Result<vw_gateway_client::GatewayClient, String>,
 pub snapshot_applied: bool,
 pub toasts: Vec<Toast>,
}
impl Update<'_> {
 fn show_error_toast(&mut self, text: impl Into<String>) -> Task<Message> { self.toasts.push(Toast::Error(text.into())); Task::none() }
 fn show_info_toast(&mut self, text: impl Into<String>) -> Task<Message> { self.toasts.push(Toast::Info(text.into())); Task::none() }
 fn show_success_toast(&mut self, text: impl Into<String>) -> Task<Message> { self.toasts.push(Toast::Success(text.into())); Task::none() }
}
use super::acp_update_message::Toast;

#[cfg(test)]
#[path="acp_update_tests.rs"]
mod acp_tests;

fn operation_id() -> String {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

fn poll_install_progress(app: &Update<'_>, agent: String, id: String) -> Task<Message> {
    let Ok(client) = app.client.clone() else {
        return Task::none();
    };
    let client = gateway_desktop_settings::GatewayDesktopSettingsClient::new(client.transport().clone());
    Task::perform(
        async move {
            tokio::time::sleep(Duration::from_millis(750)).await;
            let progress = client.acp_install_progress(&agent, &id).await.ok().flatten();
            (id, progress)
        },
        |(id, progress)| Message::Acp(AcpMessage::InstallProgressLoaded { id, progress }),
    )
}
