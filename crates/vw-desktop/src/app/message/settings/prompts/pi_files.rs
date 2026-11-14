//! All replies are bound to gateway, catalog generation, editor generation and target.
use crate::app::{App, Message, state::AgentAppsContext};
use crate::app::components::system_settings_prompts::pi_files::{State, kind};
use iced::{Task, widget::text_editor};
use vw_gateway_client::{AgentAppPromptsError, PiPromptResourcesScope, PiPromptFileSnapshot};
#[derive(Clone, Debug)]
pub(crate) enum Event {
    Open, Close, Select(usize), Edit(text_editor::Action), Read, Save, Delete, AcceptRevision,
    Reply(AgentAppsContext, u64, u64, usize, bool, Result<PiPromptFileSnapshot, String>),
}
fn message(event: Event) -> Message { super::message(super::Event::PiFile(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let client = match crate::app::config::gateway_client() { Ok(client) => client, Err(error) => return app.show_error_toast(error) };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let catalog = app.apps_catalog.generation;
    let state = &mut app.apps_catalog.prompts.pi_files;
    if let Event::Reply(request, request_catalog, generation, index, write, result) = event {
        if context != request || catalog != request_catalog || generation != state.generation || !state.open || index > 1
            || state.scope.as_ref().is_none_or(|scope| scope.target_host_id != app.apps_catalog.host || Some(scope.project_id.as_ref()) != app.project_id.as_deref()) || !state.files[index].busy { return Task::none(); }
        let file = &mut state.files[index];
        file.busy = false;
        match result {
            Ok(snapshot) => {
                if write {
                    // Deletion retains the unsaved local text for an explicit later creation.
                    if snapshot.exists { file.draft = text_editor::Content::with_text(&snapshot.content); }
                    file.base = Some(snapshot); file.latest = None; file.blocked = false;
                } else if file.base.is_none() {
                    file.draft = text_editor::Content::with_text(&snapshot.content); file.base = Some(snapshot);
                } else { file.latest = Some(snapshot); file.blocked = true; }
                file.error = None;
            }
            Err(error) => {
                if write { file.blocked = true; }
                file.error = Some(error.clone());
                return app.show_error_toast(error);
            }
        }
        return Task::none();
    }
    if matches!(event, Event::Open) {
        if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() { return app.show_error_toast("请先连接应用管理"); }
        let Some(project) = app.project_id.as_deref().filter(|id| !id.is_empty()) else { return app.show_error_toast("请先选择项目"); };
        let scope = PiPromptResourcesScope { project_id: project.into(), target_host_id: app.apps_catalog.host.clone() };
        let same = state.scope.as_ref().is_some_and(|s| s.project_id == scope.project_id && s.target_host_id == scope.target_host_id);
        if !same { *state = State { generation: state.generation.wrapping_add(1), ..State::default() }; }
        state.scope = Some(scope); state.open = true;
    } else if !state.open { return Task::none(); }
    if state.scope.as_ref().is_none_or(|scope| scope.target_host_id != app.apps_catalog.host || Some(scope.project_id.as_ref()) != app.project_id.as_deref()) { state.open = false; return Task::none(); }
    if state.files.iter().any(|file| file.busy) { return Task::none(); }
    let mut write = false;
    let mut delete = false;
    match event {
        Event::Close => { state.open = false; return Task::none(); }
        Event::Select(index) if index < 2 => { state.selected = index; if state.files[index].base.is_some() { return Task::none(); } }
        Event::Edit(action) => { let file = &mut state.files[state.selected]; file.draft.perform(action); file.confirm_delete = false; file.confirm_override = false; return Task::none(); }
        Event::AcceptRevision => {
            let file = &mut state.files[state.selected];
            if let Some(latest) = file.latest.take() { file.base = Some(latest); file.blocked = false; file.confirm_delete = false; file.confirm_override = false; }
            return Task::none();
        }
        Event::Save | Event::Delete => {
            let file = &mut state.files[state.selected];
            if file.base.is_none() || file.blocked { return Task::none(); }
            delete = matches!(event, Event::Delete);
            if delete && !file.confirm_delete { file.confirm_delete = true; return Task::none(); }
            if delete && !file.base.as_ref().is_some_and(|s| s.exists) { return Task::none(); }
            if !delete && (file.draft.text().trim().is_empty() || file.draft.text().len() > 1024 * 1024) { return app.show_error_toast("内容不能为空且不能超过 1 MiB"); }
            if !delete && state.selected == 1 && file.base.as_ref().is_some_and(|s| !s.exists) && !file.confirm_override { file.confirm_override = true; return Task::none(); }
            write = true;
        }
        Event::Open | Event::Read => {},
        _ => return Task::none(),
    }
    let Some(scope) = state.scope.clone() else { return Task::none(); };
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let index = state.selected;
    let file = &mut state.files[index];
    file.busy = true; file.latest = None; file.confirm_delete = false; file.confirm_override = false;
    let revision = file.base.as_ref().map(|s| s.revision.clone()).unwrap_or_default();
    let content = file.draft.text();
    let token = app.apps_catalog.token.clone();
    Task::perform(async move {
        let result = if delete {
            client.pi_prompt_file_delete(&scope, kind(index), &revision, &token).await.map(|_| PiPromptFileSnapshot { exists: false, revision: "missing".into(), content: String::new() })
        } else if write { client.pi_prompt_file_replace(&scope, kind(index), &revision, &content, &token).await }
        else { client.pi_prompt_file_read(&scope, kind(index), &token).await };
        let result = result.map_err(|error| match error { AgentAppPromptsError::Http { status: 409, .. } => "文件 revision 冲突，请重新读取并比较；草稿已保留".into(), other => other.to_string() });
        Event::Reply(context, catalog, generation, index, write, result)
    }, message)
}
