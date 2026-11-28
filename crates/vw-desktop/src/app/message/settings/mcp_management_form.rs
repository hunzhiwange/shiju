//! Global MCP form and explicit synchronization through the gateway.
use super::{McpMessage, SettingsMessage};
use crate::app::{App, Message};
use desktop_settings_mcp::management_context::Context;
use gateway_mcp::{GatewayMcpClient, global_sync::SyncRequest};
use iced::Task;

pub use desktop_settings_mcp::management_form::{Draft, FormMessage};

pub(crate) fn context() -> Option<Context> {
    let client = crate::app::config::gateway_client().ok()?;
    Some(Context {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        host: local_app_auth::HOST_ID.into(),
    })
}

pub(crate) fn message(event: FormMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::ConnectionForm(event)))
}

fn open_draft(app: &mut App, mut draft: Draft) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else {
        return Task::none();
    };
    let generation = draft.generation;
    let context = draft.context.clone();
    draft.loading = true;
    app.mcp_settings.page.global_sync_menu = None;
    app.mcp_settings.connection_form = Some(draft);
    Task::perform(
        async move {
            let result = GatewayMcpClient::new(client.transport().clone())
                .global_options(&context.host)
                .await
                .map(std::sync::Arc::new)
                .map_err(|_| "读取 MCP 模板和同步应用失败".to_owned());
            FormMessage::OptionsLoaded(context, generation, result)
        },
        message,
    )
}

pub(crate) fn update(app: &mut App, event: FormMessage) -> Task<Message> {
    use FormMessage as M;
    let current = context();
    let open = matches!(&event, M::OpenGlobal(..) | M::OpenLink(_));
    if open {
        if !crate::app::components::system_settings_mcp::is_visible(app)
            || app.mcp_settings.modal_open()
            || app.mcp_settings.mutating()
        {
            return Task::none();
        }
        let Some(context) = current else {
            return app.show_error_toast("网关不可用");
        };
        let draft = match event {
            M::OpenGlobal(index, sync_only) => {
                if app.mcp_settings.page.loading || app.mcp_settings.page.transfer_pending {
                    return Task::none();
                }
                match Draft::open(context, &app.mcp_settings.page, index, sync_only) {
                    Ok(draft) => draft,
                    Err(error) => return app.show_error_toast(error),
                }
            }
            M::OpenLink(candidates) => {
                let mut draft = Draft::new(context);
                draft.candidates = candidates;
                if draft.candidates.len() == 1 {
                    draft.edit(M::ChoosePaste(0));
                }
                draft
            }
            _ => return Task::none(),
        };
        return open_draft(app, draft);
    }

    let Some(draft) = app.mcp_settings.connection_form.as_mut() else {
        return Task::none();
    };
    if current.as_ref() != Some(&draft.context) {
        if matches!(event, M::Completed { .. }) {
            app.mcp_settings.connection_form = None;
        }
        return Task::none();
    }
    match event {
        M::OptionsLoaded(context, generation, result) => {
            if draft.context == context && draft.generation == generation {
                draft.loading = false;
                match result {
                    Ok(options) => draft.set_options(&options),
                    Err(error) => draft.error = Some(error),
                }
            }
        }
        M::Completed { context, generation, written, requested, result } => {
            if draft.context != context || draft.generation != generation || !draft.saving {
                return Task::none();
            }
            draft.saving = false;
            let saved = written.is_some();
            if let Some(server) = written {
                draft.mark_saved(server);
            }
            match result {
                Ok(items) => {
                    draft.retry = items
                        .iter()
                        .filter(|item| item.failure.is_some())
                        .map(|item| item.app_id)
                        .collect();
                    for item in items {
                        draft.deliveries.retain(|old| old.app_id != item.app_id);
                        draft.deliveries.push(item);
                    }
                    if draft.retry.is_empty() && !draft.sync_only {
                        app.mcp_settings.connection_form =
                            app.mcp_settings.connection_form.take().and_then(Draft::next_candidate);
                    }
                }
                Err(error) => {
                    draft.error = Some(error);
                    if saved || draft.sync_only || !draft.retry.is_empty() {
                        draft.retry = requested;
                    }
                }
            }
            if saved {
                return super::mcp::refresh_configuration(app);
            }
        }
        M::Close if !draft.saving => app.mcp_settings.connection_form = None,
        M::Save | M::Retry if !draft.saving => {
            let retry = matches!(event, M::Retry);
            let (original, definition, applications) = if retry {
                if draft.retry.is_empty() {
                    return Task::none();
                }
                (draft.original().cloned(), None, draft.retry.clone())
            } else {
                if !draft.retry.is_empty() || !draft.deliveries.is_empty() {
                    return Task::none();
                }
                if draft
                    .targets
                    .iter()
                    .any(|(target, selected)| *selected && !draft.target_supported(target))
                {
                    draft.error = Some("目标不再可写或不支持当前传输，请取消勾选后保存".into());
                    return Task::none();
                }
                let applications = draft
                    .targets
                    .iter()
                    .filter(|(_, selected)| *selected)
                    .map(|(target, _)| target.app_id)
                    .collect::<Vec<_>>();
                if draft.sync_only && applications.is_empty() {
                    return Task::none();
                }
                let definition = if draft.sync_only {
                    None
                } else {
                    match draft.save_value() {
                        Ok(server) => Some(server),
                        Err(error) => {
                            draft.error = Some(error);
                            return Task::none();
                        }
                    }
                };
                (draft.original().cloned(), definition, applications)
            };
            let Ok(client) = crate::app::config::gateway_client() else {
                return app.show_error_toast("网关不可用");
            };
            let generation = draft.generation;
            let context = draft.context.clone();
            let name = draft.id.trim().to_owned();
            draft.saving = true;
            draft.error = None;
            return Task::perform(
                async move {
                    let mut written = None;
                    let result = async {
                        desktop_settings_mcp::management_form::persist(
                            &client,
                            original.as_ref(),
                            definition.as_ref(),
                        )
                        .await?;
                        written = definition;
                        if applications.is_empty() {
                            return Ok(Vec::new());
                        }
                        let items = GatewayMcpClient::new(client.transport().clone())
                            .sync_global(&SyncRequest {
                                target_host_id: context.host.clone(),
                                name,
                                applications: applications.clone(),
                            })
                            .await
                            .map_err(|_| "同步失败".to_owned())?;
                        if items.len() != applications.len()
                            || applications.iter().any(|id| {
                                items.iter().filter(|item| item.app_id == *id).count() != 1
                            })
                        {
                            return Err("响应不完整或目标不匹配；请刷新核实结果".into());
                        }
                        Ok(items)
                    }
                    .await;
                    M::Completed { context, generation, written, requested: applications, result }
                },
                message,
            );
        }
        event if !draft.saving && draft.retry.is_empty() && draft.deliveries.is_empty() => {
            if !draft.sync_only || matches!(event, M::Target(..)) {
                draft.edit(event);
            }
        }
        _ => {}
    }
    Task::none()
}
