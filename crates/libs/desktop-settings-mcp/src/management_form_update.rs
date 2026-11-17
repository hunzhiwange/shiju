use crate::management_form::*;
use crate::management_update::{self as scope, Update};
use iced::Task;
fn authorized(app: &Update<'_>, context: &crate::management_context::Context) -> bool {
    app.context.as_ref() == Some(context)
        && app.authorized
        && !context.host.is_empty()
        && !app.token.is_empty()
}
pub fn update(app: &mut Update<'_>, event: FormMessage) -> Task<FormMessage> {
    match event {
        FormMessage::Open(record) => {
            if app.state.connection_form.is_some() {
                return Task::none();
            }
            let Some(context) = scope::context(app).filter(|context| authorized(app, context))
            else {
                return app.error("请先在应用管理中连接目标主机");
            };
            if let Some(record) = &record {
                let current = scope::visible(app).and_then(|r| r.as_ref().ok());
                if !current.is_some_and(|c| c.records.contains(record)) {
                    return app.error("目录已变化，请刷新");
                }
            }
            let new = record.is_none();
            app.state.connection_form = Some(Draft::new(context.clone(), record));
            if new && let Ok(client) = app.client.clone() {
                let token = app.token.clone();
                let generation = app.state.connection_form.as_ref().unwrap().generation;
                return Task::perform(
                    async move {
                        let result =
                            gateway_mcp_read::GatewayMcpReadClient::new(client.transport().clone())
                                .mcp_management_presets(&context.host, &token)
                                .await
                                .map_err(|_| "预设读取失败，可手动编辑".into());
                        FormMessage::Presets(context, generation, result)
                    },
                    std::convert::identity,
                );
            }
        }
        FormMessage::Saved(context, result) => {
            let current = authorized(app, &context);
            let Some(draft) =
                app.state.connection_form.as_mut().filter(|d| d.context == context && d.saving)
            else {
                return Task::none();
            };
            if !current {
                draft.finish(&Err("连接上下文已变化，请关闭后重新编辑".into()));
                return app.refresh();
            }
            draft.finish(&result);
            let toast = match result {
                Ok(_) => {
                    app.state.connection_form = None;
                    app.success("定义已保存；尚未分发或连接")
                }
                Err(error) => app.error(error),
            };
            return Task::batch([toast, app.refresh()]);
        }
        FormMessage::Save => {
            let Some(draft) = app.state.connection_form.as_ref() else {
                return Task::none();
            };
            if draft.saving {
                return Task::none();
            }
            if !authorized(app, &draft.context) {
                return app.error("连接上下文已变化，请关闭后重新编辑");
            }
            if let Err(error) = draft.connection() {
                app.state.connection_form.as_mut().unwrap().error = Some(error);
                return Task::none();
            }
            let Ok(client) = app.client.clone() else {
                return app.error("网关不可用");
            };
            let draft = app.state.connection_form.as_mut().unwrap();
            draft.saving = true;
            draft.error = None;
            let snapshot = draft.clone();
            let token = app.token.clone();
            return Task::perform(
                async move {
                    let context = snapshot.context.clone();
                    let result = save(&client, &token, snapshot).await;
                    FormMessage::Saved(context, result)
                },
                std::convert::identity,
            );
        }
        event => {
            let Some(draft) = app.state.connection_form.as_mut() else {
                return Task::none();
            };
            if draft.saving {
                return Task::none();
            }
            match event {
                FormMessage::Close => app.state.connection_form = None,
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
