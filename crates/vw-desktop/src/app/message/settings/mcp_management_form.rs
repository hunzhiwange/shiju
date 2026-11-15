//! Connection drafts use the public DTO; only the gateway validates and persists definitions.
use super::{McpMessage, SettingsMessage, mcp_management_list as catalog};
use crate::app::{App, Message};
use iced::Task;
#[cfg(test)]
use std::collections::BTreeMap;
use vw_api_mcp_management::*;
#[cfg(test)]
use vw_gateway_client::GatewayClient;

pub use desktop_settings_mcp::management_form::{Draft, FormMessage, save};
pub(crate) fn message(message: FormMessage) -> Message {
    Message::Settings(SettingsMessage::Mcp(McpMessage::ConnectionForm(message)))
}
fn authorized(app: &App, context: &catalog::Context) -> bool {
    catalog::context(app).as_ref() == Some(context)
        && !context.host.is_empty()
        && !app.apps_catalog.token.is_empty()
        && app.apps_catalog.context.as_ref() == Some(&context.endpoint)
}
pub(crate) fn update(app: &mut App, event: FormMessage) -> Task<Message> {
    match event {
        FormMessage::Open(record) => {
            if app.mcp_settings.connection_form.is_some() {
                return Task::none();
            }
            let Some(context) = catalog::context(app).filter(|context| authorized(app, context))
            else {
                return app.show_error_toast("请先在应用管理中连接目标主机");
            };
            if let Some(record) = &record {
                let current = catalog::visible(app).and_then(|r| r.as_ref().ok());
                if !current.is_some_and(|c| c.records.contains(record)) {
                    return app.show_error_toast("目录已变化，请刷新");
                }
            }
            let new = record.is_none();
            app.mcp_settings.connection_form = Some(Draft::new(context.clone(), record));
            if new && let Ok(client) = crate::app::config::gateway_client() {
                let token = app.apps_catalog.token.clone();
                let generation = app.mcp_settings.connection_form.as_ref().unwrap().generation;
                return Task::perform(
                    async move {
                        let result =
                            gateway_mcp_read::GatewayMcpReadClient::new(client.transport().clone())
                                .mcp_management_presets(&context.host, &token)
                                .await
                                .map_err(|_| "预设读取失败，可手动编辑".into());
                        FormMessage::Presets(context, generation, result)
                    },
                    message,
                );
            }
        }
        FormMessage::Saved(context, result) => {
            let current = authorized(app, &context);
            let Some(draft) = app
                .mcp_settings
                .connection_form
                .as_mut()
                .filter(|d| d.context == context && d.saving)
            else {
                return Task::none();
            };
            if !current {
                draft.finish(&Err("连接上下文已变化，请关闭后重新编辑".into()));
                return catalog::update(app, catalog::CatalogMessage::Refresh);
            }
            draft.finish(&result);
            let toast = match result {
                Ok(_) => {
                    app.mcp_settings.connection_form = None;
                    app.show_success_toast("定义已保存；尚未分发或连接")
                }
                Err(error) => app.show_error_toast(error),
            };
            return Task::batch([toast, catalog::update(app, catalog::CatalogMessage::Refresh)]);
        }
        FormMessage::Save => {
            let Some(draft) = app.mcp_settings.connection_form.as_ref() else {
                return Task::none();
            };
            if draft.saving {
                return Task::none();
            }
            if !authorized(app, &draft.context) {
                return app.show_error_toast("连接上下文已变化，请关闭后重新编辑");
            }
            if let Err(error) = draft.connection() {
                app.mcp_settings.connection_form.as_mut().unwrap().error = Some(error);
                return Task::none();
            }
            let Ok(client) = crate::app::config::gateway_client() else {
                return app.show_error_toast("网关不可用");
            };
            let draft = app.mcp_settings.connection_form.as_mut().unwrap();
            draft.saving = true;
            draft.error = None;
            let snapshot = draft.clone();
            let token = app.apps_catalog.token.clone();
            return Task::perform(
                async move {
                    let context = snapshot.context.clone();
                    let result = save(&client, &token, snapshot).await;
                    FormMessage::Saved(context, result)
                },
                message,
            );
        }
        event => {
            let Some(draft) = app.mcp_settings.connection_form.as_mut() else {
                return Task::none();
            };
            if draft.saving {
                return Task::none();
            }
            match event {
                FormMessage::Close => app.mcp_settings.connection_form = None,
                event @ (FormMessage::Field(..)
                | FormMessage::ValueAdd
                | FormMessage::ValueRemove(_)
                | FormMessage::ValueName(..)
                | FormMessage::ValueKind(..)
                | FormMessage::ValueText(..)
                | FormMessage::ValueService(..)
                | FormMessage::Editor(_)
                | FormMessage::Json(_)
                | FormMessage::Preset(_)) => draft.edit(event),
                FormMessage::Presets(context, generation, result)
                    if draft.context == context && draft.generation == generation =>
                {
                    match result {
                        Ok(response) => draft.presets = response.presets,
                        Err(error) => draft.error = Some(error),
                    }
                }
                _ => {}
            }
        }
    }
    Task::none()
}

#[cfg(test)]
#[path = "mcp_management_form_tests.rs"]
mod tests;
