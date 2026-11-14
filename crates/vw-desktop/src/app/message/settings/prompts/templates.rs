//! Requests and replies remain bound to the connected project, host and generation.
use crate::app::{App, Message, state::AgentAppsContext};
use crate::app::components::system_settings_prompts::templates::{State, Draft};
use iced::{Task, widget::text_editor};
use vw_gateway_client::{PiPromptResourcesScope, PiPromptTemplate, AgentAppPromptsError};

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Open, Close, Load, Edit(Option<String>), Cancel, Slug(String), Body(text_editor::Action),
    Save, Delete(String), Undo, Copy(String),
    Reply(AgentAppsContext, u64, u64, Result<Reply, String>),
}
#[derive(Debug, Clone)]
pub(crate) enum Reply { List(Vec<PiPromptTemplate>), Saved(PiPromptTemplate), Deleted(PiPromptTemplate) }
pub(crate) fn message(event: Event) -> Message { super::message(super::Event::Template(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let client = match crate::app::config::gateway_client() { Ok(c) => c, Err(e) => return app.show_error_toast(e) };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let catalog = app.apps_catalog.generation;
    let state = &mut app.apps_catalog.prompts.templates;
    if let Event::Reply(request, request_catalog, generation, result) = event {
        if request != context || catalog != request_catalog || generation != state.generation || !state.open || !state.busy
            || state.scope.as_ref().is_none_or(|s| s.target_host_id != app.apps_catalog.host || Some(s.project_id.as_ref()) != app.project_id.as_deref()) { return Task::none(); }
        state.busy = false;
        match result {
            Ok(Reply::List(entries)) => { state.entries = entries; state.loaded = true; }
            Ok(Reply::Saved(entry)) => { state.entries.retain(|e| e.slug != entry.slug && state.draft.as_ref().and_then(|d| d.original.as_ref()).is_none_or(|o| o.slug != e.slug)); state.entries.push(entry); state.draft = None; state.undo = None; }
            Ok(Reply::Deleted(entry)) => { state.entries.retain(|e| e.slug != entry.slug); state.undo = Some(entry); }
            Err(error) => { state.error = Some(error.clone()); return app.show_error_toast(error); }
        }
        state.entries.sort_by(|a, b| a.slug.cmp(&b.slug));
        return Task::none();
    }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() { return app.show_error_toast("请先连接应用管理"); }
    if matches!(event, Event::Open) {
        let Some(project) = app.project_id.as_deref().filter(|p| !p.is_empty()) else { return app.show_error_toast("请先选择项目"); };
        let scope = PiPromptResourcesScope { project_id: project.into(), target_host_id: app.apps_catalog.host.clone() };
        if state.scope.as_ref().is_none_or(|s| s.project_id != scope.project_id || s.target_host_id != scope.target_host_id) { *state = State { generation: state.generation.wrapping_add(1), ..State::default() }; }
        state.scope = Some(scope); state.open = true;
    }
    if !state.open || state.busy || state.scope.as_ref().is_none_or(|s| s.target_host_id != app.apps_catalog.host || Some(s.project_id.as_ref()) != app.project_id.as_deref()) { return Task::none(); }
    match &event {
        Event::Close => { state.open = false; return Task::none(); }
        Event::Cancel => { state.draft = None; state.error = None; return Task::none(); }
        Event::Edit(slug) => {
            let original = match slug { Some(slug) => match state.entries.iter().find(|e| &e.slug == slug) { Some(e) => Some(e.clone()), None => return Task::none() }, None => None };
            state.draft = Some(Draft { edited: false, slug: original.as_ref().map(|e| e.slug.clone()).unwrap_or_default(), body: text_editor::Content::with_text(original.as_ref().map(|e| e.content.as_str()).unwrap_or("")), original });
            state.error = None; state.delete = None; return Task::none();
        }
        Event::Slug(value) => { if let Some(d) = &mut state.draft { d.slug = value.clone(); } return Task::none(); }
        Event::Body(action) => { if let Some(d) = &mut state.draft { d.edited |= action.is_edit(); d.body.perform(action.clone()); } return Task::none(); }
        Event::Copy(slug) => { return if state.entries.iter().any(|e| &e.slug == slug) { iced::clipboard::write(format!("/{slug}")) } else { Task::none() }; }
        Event::Delete(slug) if state.delete.as_ref() != Some(slug) => { state.delete = Some(slug.clone()); return Task::none(); }
        _ => {}
    }
    let Some(scope) = state.scope.clone() else { return Task::none(); };
    let draft = state.draft.clone();
    let entry = match &event {
        Event::Delete(slug) => state.entries.iter().find(|e| &e.slug == slug).cloned(),
        Event::Undo => state.undo.clone(),
        _ => None,
    };
    if matches!(event, Event::Save) && draft.is_none() || matches!(event, Event::Delete(_) | Event::Undo) && entry.is_none() { return Task::none(); }
    if !matches!(event, Event::Open | Event::Load | Event::Save | Event::Delete(_) | Event::Undo) { return Task::none(); }
    // Reopening a retained draft must not replace it with a fresh list response.
    state.busy = true; state.error = None; state.delete = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let token = app.apps_catalog.token.clone();
    let draft = draft.map(|d| {
        let content = if d.edited { d.body.text() } else { d.original.as_ref().map(|e| e.content.clone()).unwrap_or_default() };
        (d.slug, d.original, content)
    });
    Task::perform(async move {
        let result = match event {
            Event::Save => {
                let (slug, original, content) = draft.expect("checked draft");
                client.pi_prompt_template_upsert(&scope, &slug, original.as_ref().map(|e| e.slug.as_str()), original.as_ref().map(|e| e.revision.as_str()).unwrap_or("missing"), &content, &token).await.map(Reply::Saved)
            }
            Event::Delete(_) => {
                let e = entry.expect("checked entry");
                client.pi_prompt_template_delete(&scope, &e.slug, &e.revision, &token).await.and_then(|r| if r.deleted { Ok(Reply::Deleted(e)) } else { Err(AgentAppPromptsError::InvalidInput) })
            }
            Event::Undo => {
                let e = entry.expect("checked undo");
                client.pi_prompt_template_upsert(&scope, &e.slug, None, "missing", &e.content, &token).await.map(Reply::Saved)
            }
            _ => client.pi_prompt_templates(&scope, &token).await.map(Reply::List),
        }.map_err(|e| match e { AgentAppPromptsError::Http { status: 409, .. } => "名称或 revision 冲突；草稿已保留，请返回列表刷新后重新编辑，勿覆盖当前模板。".into(), other => other.to_string() });
        Event::Reply(context, catalog, generation, result)
    }, message)
}
