use crate::app::{App, Message};
use desktop_settings_mcp::extensions::{ActiveInstall, Message as Event};
use gateway_desktop_settings::{AcpInstallReport, GatewayDesktopSettingsClient, McpExtension};
use gateway_endpoint::GatewayEndpoint;
use iced::Task;
use serde_json::Value;
use std::time::Duration;

pub(crate) fn message(event: Event) -> Message {
    Message::Settings(super::SettingsMessage::Mcp(super::McpMessage::Extension(event)))
}

fn local_endpoint() -> Result<GatewayEndpoint, String> {
    let endpoint = crate::app::config::gateway_client_endpoint();
    if cfg!(target_arch = "wasm32") || !endpoint.is_loopback() {
        return Err("一键安装仅支持连接本机网关的桌面客户端。".into());
    }
    Ok(endpoint)
}

fn client() -> Result<GatewayDesktopSettingsClient, String> {
    gateway_http::GatewayTransport::new(local_endpoint()?).map(GatewayDesktopSettingsClient::new)
}

fn instructions_task(extension: McpExtension) -> Task<Message> {
    Task::perform(
        async move { client()?.mcp_install_instructions(extension).await },
        move |result| message(Event::Instructions(extension, result)),
    )
}

pub(crate) fn refresh_statuses(app: &mut App) -> Task<Message> {
    if local_endpoint().is_err() {
        return Task::none();
    }
    Task::batch(McpExtension::ALL.into_iter().map(|extension| refresh_status(app, extension)))
}

fn refresh_status(app: &mut App, extension: McpExtension) -> Task<Message> {
    let state = &mut app.mcp_settings.extensions;
    if state.active.as_ref().is_some_and(|active| active.extension == extension) {
        return Task::none();
    }
    let card = state.cards.entry(extension).or_default();
    if card.status_loading {
        return Task::none();
    }
    card.status_loading = true;
    Task::perform(async move { client()?.mcp_install_status(extension).await }, move |result| {
        message(Event::Status(extension, result))
    })
}

fn poll_progress(active: &ActiveInstall) -> Task<Message> {
    let id = active.id.clone();
    let extension = active.extension;
    Task::perform(
        async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let progress = match client() {
                Ok(client) => client.mcp_install_progress(extension, &id).await.ok().flatten(),
                Err(_) => None,
            };
            (id, progress)
        },
        |(id, progress)| message(Event::Progress(id, progress)),
    )
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let state = &mut app.mcp_settings;
    match event {
        Event::Toggle(extension) => {
            let card = state.extensions.cards.entry(extension).or_default();
            card.expanded = !card.expanded;
            if card.expanded
                && (extension != McpExtension::ChromeDevTools
                    || !matches!(&card.instructions, Some(Ok(_))))
            {
                card.instructions = None;
                return instructions_task(extension);
            }
            Task::none()
        }
        Event::Instructions(extension, result) => {
            let card = state.extensions.cards.entry(extension).or_default();
            card.instructions = Some(result);
            card.copy_feedback.clear();
            Task::none()
        }
        Event::RefreshStatus(extension) => refresh_status(app, extension),
        Event::Status(extension, result) => {
            let card = state.extensions.cards.entry(extension).or_default();
            if std::mem::take(&mut card.status_loading) {
                card.status = Some(result);
            }
            Task::none()
        }
        Event::Install(extension) => {
            if state.loading
                || state.saving
                || state.transfer_pending
                || state.new_server_expanded
                || state.extensions.active.is_some()
            {
                return Task::none();
            }
            let Some(target) = state
                .extensions
                .cards
                .get(&extension)
                .filter(|card| !card.status_loading)
                .and_then(|card| card.status.as_ref())
                .and_then(|status| status.as_ref().ok())
                .and_then(|status| status.action.as_ref())
                .map(|action| action.target_version().to_owned())
            else {
                return Task::none();
            };
            let id = format!("{:032x}", rand::random::<u128>());
            let active = ActiveInstall { extension, id: id.clone(), progress: None };
            let progress = poll_progress(&active);
            state.extensions.active = Some(active);
            state.page.saving = true;
            let editor = state.editor.text();
            let card = state.extensions.cards.entry(extension).or_default();
            card.expanded = true;
            card.report = None;
            let instructions =
                if extension != McpExtension::ChromeDevTools || card.instructions.is_none() {
                    card.instructions = None;
                    instructions_task(extension)
                } else {
                    Task::none()
                };
            let install = Task::perform(
                async move {
                    let result = async {
                        let endpoint = local_endpoint()?;
                        let client = GatewayDesktopSettingsClient::new(
                            gateway_http::GatewayTransport::new(endpoint.clone())?,
                        );
                        let draft = serde_json::from_str::<Vec<Value>>(&editor)
                            .map_err(|_| "请先修正并保存 MCP 配置草稿")?;
                        let current = desktop_config::ConfigAccess::new(endpoint, None)
                            .load_global_config_value_async()
                            .await?;
                        let configured =
                            current["mcpServers"].as_array().map(Vec::as_slice).unwrap_or_default();
                        if draft.as_slice() != configured {
                            return Err("请先保存 MCP 配置草稿或刷新配置".into());
                        }
                        client.mcp_install(extension, &id, &target).await
                    }
                    .await;
                    (id, result)
                },
                |(id, result)| message(Event::Finished(id, result)),
            );
            Task::batch([instructions, install, progress])
        }
        Event::Progress(id, progress) => {
            let Some(active) = state.extensions.active.as_mut().filter(|active| active.id == id)
            else {
                return Task::none();
            };
            if progress.is_some() {
                active.progress = progress;
            }
            poll_progress(active)
        }
        Event::Finished(id, result) => {
            if !state.extensions.active.as_ref().is_some_and(|active| active.id == id) {
                return Task::none();
            }
            let active = state.extensions.active.take().expect("matching installation");
            state.page.saving = false;
            let report = result.unwrap_or_else(|message| AcpInstallReport {
                success: false,
                message,
                steps: Vec::new(),
            });
            let success = report.success;
            state.extensions.cards.entry(active.extension).or_default().report = Some(report);
            if success {
                state.page.loading = true;
                state.page.inspect_loading = true;
                return Task::batch([
                    super::refresh_all_task(app.navigation_tabs.project_path.clone()),
                    refresh_status(app, active.extension),
                ]);
            }
            refresh_status(app, active.extension)
        }
        Event::Copy(value) => iced::clipboard::write(value),
        #[cfg(not(target_arch = "wasm32"))]
        Event::CopyInstruction(extension, item) => {
            use desktop_settings_mcp::extensions::InstructionCopy;

            let Some(card) = state.extensions.cards.get_mut(&extension) else {
                return Task::none();
            };
            let Some(Ok(instructions)) = &card.instructions else {
                return Task::none();
            };
            let value = match item {
                InstructionCopy::Command(index) => instructions.commands.get(index),
                InstructionCopy::Configuration => Some(&instructions.configuration),
            };
            let Some(value) = value else { return Task::none() };

            state.extensions.copy_sequence = state.extensions.copy_sequence.wrapping_add(1);
            let request = state.extensions.copy_sequence;
            card.copy_feedback.entry(item).or_default().request = request;
            crate::app::message::write_native_clipboard(value.clone()).map(move |result| {
                message(Event::InstructionCopied(extension, item, request, result))
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        Event::InstructionCopied(extension, item, request, result) => {
            let Some(card) = state.extensions.cards.get_mut(&extension) else {
                return Task::none();
            };
            let Some(feedback) =
                card.copy_feedback.get_mut(&item).filter(|copy| copy.request == request)
            else {
                return Task::none();
            };
            if result.is_err() {
                card.copy_feedback.remove(&item);
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "复制失败：系统剪贴板不可用或访问被拒绝",
                    vec![],
                );
            }
            feedback.copied = true;
            crate::app::message::after(
                Duration::from_secs(2),
                message(Event::ClearCopyFeedback(extension, item, request)),
            )
        }
        #[cfg(not(target_arch = "wasm32"))]
        Event::ClearCopyFeedback(extension, item, request) => {
            if let Some(card) = state.extensions.cards.get_mut(&extension)
                && card.copy_feedback.get(&item).is_some_and(|copy| copy.request == request)
            {
                card.copy_feedback.remove(&item);
            }
            Task::none()
        }
    }
}
