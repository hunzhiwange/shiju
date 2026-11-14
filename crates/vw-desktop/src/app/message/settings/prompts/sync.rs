//! Only explicit user actions submit writes; progress queries never replay a submission.
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::{AgentAppPromptSync, PiPromptResourcesScope};
use vw_gateway_client::vw_api_types::agent_apps::AppId;
#[derive(Clone, Debug)]
pub(crate) enum Event {
    Open, Close, Start, Refresh, Cancel, Retry,
    Reply(AgentAppsContext, PiPromptResourcesScope, u64, Result<AgentAppPromptSync, String>),
}
pub(crate) fn message(event: Event) -> Message { super::message(super::Event::Sync(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) {
        app.apps_catalog.prompts.sync.open = false;
        return Task::none();
    }
    let client = match crate::app::config::gateway_client() { Ok(client) => client, Err(error) => return app.show_error_toast(error) };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let state = &mut app.apps_catalog.prompts.sync;
    if let Event::Reply(request, scope, generation, result) = event {
        // Keep the result even if the dialog closed or the selected application changed.
        if state.context.as_ref() != Some(&request) || state.generation != generation
            || state.scope.as_ref().is_none_or(|s| s.project_id != scope.project_id || s.target_host_id != scope.target_host_id) { return Task::none(); }
        state.busy = false;
        match result {
            Ok(report) if report.project_id == scope.project_id && report.target_host_id == scope.target_host_id
                && state.report.as_ref().is_none_or(|previous| previous.operation.operation_id == report.operation.operation_id) => {
                state.report = Some(report); state.error = None;
            }
            Ok(_) => state.error = Some("后端返回的操作范围不匹配；结果未应用。".into()),
            Err(error) => state.error = Some(error),
        }
        return Task::none();
    }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() { return app.show_error_toast("请先连接应用管理"); }
    let Some(project) = app.project_id.as_deref().filter(|p| !p.is_empty()) else { return app.show_error_toast("请先选择项目"); };
    let scope = PiPromptResourcesScope { project_id: project.into(), target_host_id: app.apps_catalog.host.clone() };
    let same = state.context.as_ref() == Some(&context) && state.scope.as_ref().is_some_and(|s| s.project_id == scope.project_id && s.target_host_id == scope.target_host_id);
    if matches!(event, Event::Open) {
        if !same {
            let generation = state.generation.wrapping_add(1);
            *state = Default::default();
            state.generation = generation;
            state.context = Some(context.clone()); state.scope = Some(scope.clone());
        }
        state.open = true;
        return Task::none();
    }
    if !same || !state.open || state.busy { return Task::none(); }
    let apps = match event {
        Event::Start if !state.submitted => vec![AppId::Claude, AppId::ClaudeDesktop, AppId::Codex, AppId::Gemini, AppId::GrokBuild, AppId::OpenCode, AppId::Hermes, AppId::Pi, AppId::Mcode, AppId::Kimi, AppId::Dsh, AppId::Agy, AppId::Omp],
        Event::Retry => state.retry_targets(),
        Event::Refresh | Event::Cancel if state.report.is_some() => Vec::new(),
        _ => return Task::none(),
    };
    if matches!(event, Event::Retry) && apps.is_empty() { return Task::none(); }
    let id = state.report.as_ref().map(|r| r.operation.operation_id.clone());
    let submit = !apps.is_empty();
    if submit { state.submitted = true; state.report = None; }
    state.busy = true; state.error = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let token = app.apps_catalog.token.clone();
    Task::perform(async move {
        let result = if submit { client.agent_app_prompts_sync(&scope, &apps, &token).await }
            else if let Some(id) = id {
                if matches!(event, Event::Cancel) { client.agent_app_prompts_sync_cancel(&scope.target_host_id, &id, &token).await }
                else { client.agent_app_prompts_sync_operation(&scope.target_host_id, &id, &token).await }
            } else { unreachable!("progress requires an operation") };
        Event::Reply(context, scope, generation, result.map_err(|_| "请求未确认。已有结果保留；可刷新已知操作，提交响应丢失时请先核对后端，勿重复提交。".into()))
    }, message)
}
