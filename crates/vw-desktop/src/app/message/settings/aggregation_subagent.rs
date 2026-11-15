//! Codex subagent intent; configuration and publication remain gateway-owned.
use crate::app::{
    App, Message,
    message::app_providers::list::{self, ListMessage},
};
use iced::Task;
use gateway_app_settings::AgentAppsSettingsError;
use gateway_endpoint::GatewayAuth;
use vw_gateway_client::GatewayClient;
use vw_api_app_identity::AppId;
use vw_api_app_aggregation::*;
pub use desktop_apps_ui::aggregation::subagent::{Context, Event, Form};
pub(crate) fn message(e: Event) -> Message {
    list::message(ListMessage::CodexSubagent(e))
}
fn current(app: &App) -> Result<(GatewayClient, Context), String> {
    let (client, scope) = list::current(app, AppId::Codex)?;
    let project = app.project_id.as_ref().filter(|p| !p.is_empty()).ok_or("请先打开已注册项目")?;
    if app.app_providers.scope.as_ref() != Some(&scope)
        || scope.host.is_empty()
        || app.apps_catalog.token.is_empty()
    {
        return Err("请先连接已授权应用执行主机".into());
    }
    let endpoint = client
        .endpoint()
        .clone()
        .with_auth(GatewayAuth { skey: Some(app.apps_catalog.token.clone()) });
    Ok((
        GatewayClient::new(endpoint)?,
        Context { scope, project: project.as_str().into(), access: app.apps_catalog.generation },
    ))
}
pub(crate) async fn execute(
    client: GatewayClient,
    context: Context,
    write: Option<CodexSettingsRequest>,
) -> Result<CodexAggregationSettings, AgentAppsSettingsError> {
    let client = gateway_aggregation_codex::GatewayAggregationCodexClient::new(client.transport().clone());
    match write {
        Some(body) => client.aggregation_codex_settings_save(&body).await,
        None => {
            client
                .aggregation_codex_settings(&AggregationQuery {
                    target_host_id: context.scope.host,
                    project_id: context.project,
                })
                .await
        }
    }
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let (client, context) = match current(app) {
        Ok(v) => v,
        Err(e) => {
            app.app_providers.codex_subagent = None;
            return if matches!(event, Event::Finished { .. }) {
                Task::none()
            } else {
                app.show_error_toast(e)
            };
        }
    };
    if matches!(event, Event::Open) {
        let s = &mut app.app_providers;
        if s.loading || s.form.is_some() || s.editor.is_some() || s.codex_subagent.is_some() {
            return Task::none();
        }
        s.codex_subagent = Some(Form { context: Some(context.clone()), ..Default::default() });
        return update(app, Event::Refresh);
    }
    let state = &mut app.app_providers;
    let Some(form) = &mut state.codex_subagent else { return Task::none() };
    if form.context.as_ref() != Some(&context) {
        state.codex_subagent = None;
        return Task::none();
    }
    if let Event::Finished { context: old, generation, saved, result } = event {
        if old != context || !form.finish(generation, saved, result) {
            return Task::none();
        }
        return if let Some(e) = form.error {
            app.show_error_toast(e)
        } else if saved {
            app.show_info_toast("Codex 设置已保存，请新建 Codex 会话后使用。")
        } else {
            Task::none()
        };
    }
    if form.pending.is_some() {
        return Task::none();
    }
    let write = match event {
        Event::Close => {
            state.codex_subagent = None;
            return Task::none();
        }
        Event::Model(id) if form.snapshot.is_some() => {
            form.select_model(id);
            return Task::none();
        }
        Event::Effort(e) if form.snapshot.is_some() => {
            form.select_effort(e);
            return Task::none();
        }
        Event::Review(id) => {
            form.select_review(id);
            return Task::none();
        }
        Event::Classic(enabled) => {
            form.select_classic(enabled);
            return Task::none();
        }
        Event::Refresh => {
            form.snapshot = None;
            None
        }
        Event::Save => match form.write(&context) {
            Some(w) => Some(w),
            None => return app.show_error_toast("请刷新配置后再保存"),
        },
        _ => return Task::none(),
    };
    let saved = write.is_some();
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    form.pending = Some(generation);
    form.error = None;
    form.saved = false;
    Task::perform(execute(client, context.clone(), write), move |result| {
        message(Event::Finished { context: context.clone(), generation, saved, result })
    })
}

#[cfg(test)]
#[path = "aggregation_review_tests.rs"]
mod aggregation_review_tests;
