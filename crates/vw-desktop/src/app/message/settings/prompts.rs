//! Scope and request generation travel with every response, including failures.
pub(crate) mod shared_import;
pub(crate) mod pi_files;
pub(crate) mod transfer;
pub(crate) mod templates;
pub(crate) mod sync;
use crate::app::{App, Message, state::AgentAppsContext};
use iced::Task;
use crate::app::components::system_settings_prompts::activation::{Action, Undo, error_text, effect};
use vw_gateway_client::{AgentAppPromptCatalog, AgentAppPromptsScope, vw_api_types::agent_apps::{AppId, PromptTargetPreview}};
#[derive(Clone, Debug)]
pub enum Event {
    Open(AppId),
    SharedImport(shared_import::Event),
    Transfer(transfer::Event),
    Template(templates::Event),
    Sync(sync::Event),
    PiFile(pi_files::Event),
    Edit(Option<String>),
    CloseEditor,
    Name(String),
    Description(String),
    Body(iced::widget::text_editor::Action),
    Save,
    Saved(AgentAppsContext, u64, u64, AgentAppPromptsScope, Result<AgentAppPromptCatalog, String>),
    Mutate(Action),
    Undo,
    Mutated(AgentAppsContext, u64, u64, AgentAppPromptsScope,
        Result<(AgentAppPromptCatalog, Result<PromptTargetPreview, String>, Option<Action>), String>),
    Reply(AgentAppsContext, u64, u64, AgentAppPromptsScope, Result<(AgentAppPromptCatalog, PromptTargetPreview), String>),
}
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(super::SettingsMessage::AppsCatalog(super::AppsCatalogMessage::Prompts(event)))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::SharedImport(event) = event { return shared_import::update(app, event); }
    if app.apps_catalog.prompts.shared_import.is_some() { return Task::none(); }
    if let Event::Sync(event) = event { return sync::update(app, event); }
    if let Event::Template(event) = event { return templates::update(app, event); }
    if let Event::Transfer(event) = event { return transfer::update(app, event); }
    if let Event::PiFile(event) = event {
        if app.apps_catalog.prompts.mutating { return Task::none(); }
        return pi_files::update(app, event);
    }
    let client = match crate::app::config::gateway_client() {
        Ok(client) => client,
        Err(error) => { return app.show_error_toast(error); }
    };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let catalog_generation = app.apps_catalog.generation;
    match event {
        Event::SharedImport(_) | Event::PiFile(_) | Event::Sync(_) | Event::Transfer(_) | Event::Template(_) => unreachable!(),
        Event::Edit(id) => {
            let state = &mut app.apps_catalog.prompts;
            if state.loading || state.mutating || state.editor.is_some() { return Task::none(); }
            let (Some(catalog), Some(scope)) = (&state.catalog, &state.scope) else { return Task::none(); };
            let entry = match id {
                Some(id) => match catalog.entries.iter().find(|entry| entry.id == id) { Some(entry) => Some(entry.clone()), None => return Task::none() },
                None => None,
            };
            state.editor = Some(crate::app::components::system_settings_prompts::editor::Draft::new(entry, catalog.revision, scope.app_id == AppId::Mcode));
            Task::none()
        }
        Event::CloseEditor => {
            if app.apps_catalog.prompts.editor.as_ref().is_some_and(|draft| !draft.saving) { app.apps_catalog.prompts.editor = None; }
            Task::none()
        }
        Event::Name(value) => {
            if let Some(draft) = app.apps_catalog.prompts.editor.as_mut().filter(|d| !d.saving) { draft.entry.name = value; }
            Task::none()
        }
        Event::Description(value) => {
            if let Some(draft) = app.apps_catalog.prompts.editor.as_mut().filter(|d| !d.saving) { draft.entry.description = Some(value); }
            Task::none()
        }
        Event::Body(action) => {
            if let Some(draft) = app.apps_catalog.prompts.editor.as_mut().filter(|d| !d.saving) { draft.body.perform(action); }
            Task::none()
        }
        Event::Save if app.apps_catalog.prompts.mutating => Task::none(),
        Event::Save => {
            if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() { return app.show_error_toast("请先连接应用管理"); }
            let state = &mut app.apps_catalog.prompts;
            let Some(scope) = state.scope.clone() else { return Task::none(); };
            if scope.target_host_id != app.apps_catalog.host || app.project_id.as_deref() != Some(scope.project_id.as_ref()) { return Task::none(); }
            let Some(draft) = state.editor.as_mut().filter(|d| !d.saving) else { return Task::none(); };
            draft.entry.content = draft.body.text();
            draft.saving = true;
            draft.error = None;
            let entry = draft.entry.clone();
            let revision = draft.revision;
            let updating = draft.updating;
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            let token = app.apps_catalog.token.clone();
            Task::perform(async move {
                let result = client.agent_app_prompts_save(&scope, revision, &entry, updating, &token).await.map_err(|error| match error {
                    vw_gateway_client::AgentAppPromptsError::Http { status: 413, .. } => "正文超过服务端限长；MCode 最多 32768 UTF-8 字节".to_owned(),
                    vw_gateway_client::AgentAppPromptsError::Http { status: 409, .. } => "内容版本已变化，请保留草稿并刷新后重试".to_owned(),
                    error => error.to_string(),
                });
                Event::Saved(context, catalog_generation, generation, scope, result)
            }, message)
        }
        Event::Saved(request, catalog_generation_reply, generation, scope, result) => {
            let state = &mut app.apps_catalog.prompts;
            if request != context || catalog_generation_reply != catalog_generation || generation != state.generation
                || !state.scope.as_ref().is_some_and(|s| s.app_id == scope.app_id && s.project_id == scope.project_id && s.target_host_id == scope.target_host_id)
                || scope.target_host_id != app.apps_catalog.host || app.project_id.as_deref() != Some(scope.project_id.as_ref())
                || !state.editor.as_ref().is_some_and(|d| d.saving) { return Task::none(); }
            match result {
                Ok(catalog) => { state.catalog = Some(catalog); state.editor = None; state.undo = None; return app.show_success_toast("提示词已保存"); }
                Err(error) => { let draft = state.editor.as_mut().expect("saving editor"); draft.saving = false; draft.error = Some(error.clone()); return app.show_error_toast(error); }
            }
            Task::none()
        }
        Event::Mutate(_) | Event::Undo if app.apps_catalog.prompts.loading || app.apps_catalog.prompts.mutating || app.apps_catalog.prompts.editor.is_some() => Task::none(),
        Event::Mutate(_) | Event::Undo if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() => app.show_error_toast("请先连接应用管理"),
        Event::Mutate(action) => mutate(app, client, context, catalog_generation, action, false),
        Event::Undo => {
            let Some(undo) = app.apps_catalog.prompts.undo.clone() else { return Task::none(); };
            mutate(app, client, context, catalog_generation, undo.action, true)
        }
        Event::Mutated(request, catalog_generation_reply, generation, scope, result) => {
            let state = &mut app.apps_catalog.prompts;
            if request != context || catalog_generation_reply != catalog_generation || state.generation != generation
                || !state.mutating || state.scope.as_ref().is_none_or(|s| s.app_id != scope.app_id || s.project_id != scope.project_id || s.target_host_id != scope.target_host_id)
                || scope.target_host_id != app.apps_catalog.host || app.project_id.as_deref() != Some(scope.project_id.as_ref()) { return Task::none(); }
            state.mutating = false;
            match result {
                Err(error) => { state.error = Some(error.clone()); app.show_error_toast(error) }
                Ok((catalog, preview, safe_reverse)) => {
                    state.catalog = Some(catalog.clone());
                    match preview {
                        Ok(preview) => {
                            state.undo = safe_reverse.map(|action| Undo { revision: catalog.revision, content: preview.snapshot.content.clone(), action });
                            state.preview = Some(preview);
                            app.show_success_toast(effect(scope.app_id))
                        }
                        Err(error) => {
                            state.preview = None;
                            state.error = Some(error);
                            app.show_error_toast("操作已保存，但目标快照刷新失败；请刷新后继续。")
                        }
                    }
                }
            }
        }
        Event::Open(_) if app.apps_catalog.prompts.mutating => Task::none(),
        Event::Reply(request, catalog, generation, scope, result) => {
            let state = &mut app.apps_catalog.prompts;
            let same_scope = state.scope.as_ref().is_some_and(|current| current.app_id == scope.app_id && current.project_id == scope.project_id && current.target_host_id == scope.target_host_id);
            if request != context || catalog != catalog_generation || generation != state.generation || !same_scope || !state.loading
                || scope.target_host_id != app.apps_catalog.host || app.project_id.as_deref() != Some(scope.project_id.as_ref()) { return Task::none(); }
            state.loading = false;
            match result {
                Ok((catalog, preview)) => { state.catalog = Some(catalog); state.preview = Some(preview); }
                Err(error) => { state.error = Some(error.clone()); return app.show_error_toast(error); }
            }
            Task::none()
        }
        Event::Open(id) => {
            if app.apps_catalog.prompts.editor.is_some() { return Task::none(); }
            app.apps_catalog.prompts.invalidate();
            if matches!(id, AppId::ClaudeDesktop | AppId::OpenClaw) { return app.show_error_toast("请使用该应用的专属入口"); }
            if app.apps_catalog.token.is_empty() || app.apps_catalog.context.as_ref() != Some(&context) { return app.show_error_toast("请先连接应用管理"); }
            let Some(project) = app.project_id.as_deref().filter(|p| !p.is_empty()) else { return app.show_error_toast("请先选择项目"); };
            let scope = AgentAppPromptsScope { project_id: project.into(), target_host_id: app.apps_catalog.host.clone(), app_id: id };
            let state = &mut app.apps_catalog.prompts;
            state.scope = Some(scope.clone());
            state.loading = true;
            let generation = state.generation;
            let token = app.apps_catalog.token.clone();
            Task::perform(async move {
                let result = async {
                    let catalog = client.agent_app_prompts(&scope, "", &token).await.map_err(|e| e.to_string())?;
                    let preview = client.agent_app_prompts_current(&scope, &token).await.map_err(|e| e.to_string())?;
                    Ok((catalog, preview))
                }.await;
                Event::Reply(context, catalog_generation, generation, scope, result)
            }, message)
        }
    }
}

fn mutate(app: &mut App, client: vw_gateway_client::GatewayClient, context: AgentAppsContext,
    catalog_generation: u64, action: Action, undoing: bool) -> Task<Message> {
    let state = &mut app.apps_catalog.prompts;
    let (Some(scope), Some(catalog), Some(preview)) = (state.scope.clone(), state.catalog.as_ref(), state.preview.as_ref()) else { return Task::none(); };
    if scope.target_host_id != app.apps_catalog.host || app.project_id.as_deref() != Some(scope.project_id.as_ref()) { return Task::none(); }
    let revision = catalog.revision;
    let content = preview.snapshot.content.clone();
    if undoing && state.undo.as_ref().is_none_or(|u| u.revision != revision || u.content != content) {
        state.undo = None;
        return app.show_error_toast("撤销已过期：提示词或目标文件已变化。");
    }
    let reverse = if undoing { None } else { match &action {
        Action::Activate(id) => Some(catalog.active_id.clone().map(Action::Activate).unwrap_or_else(|| Action::Deactivate(id.clone()))),
        Action::Deactivate(id) => Some(Action::Activate(id.clone())),
        Action::Delete(id) => catalog.entries.iter().find(|e| &e.id == id).cloned().map(Action::Restore),
        Action::Restore(_) => None,
    }};
    state.undo = None;
    state.error = None;
    state.mutating = true;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let token = app.apps_catalog.token.clone();
    Task::perform(async move {
        let result = async {
            let expected_after = match &action {
                Action::Activate(_) => None,
                Action::Deactivate(_) => content.as_ref().map(|_| String::new()),
                Action::Delete(_) | Action::Restore(_) => content.clone(),
            };
            let activation_id = match &action { Action::Activate(id) => Some(id.clone()), _ => None };
            let catalog = match action {
                Action::Activate(id) => client.agent_app_prompts_activate(&scope, revision, &id, content.as_deref(), &token).await,
                Action::Deactivate(id) => client.agent_app_prompts_deactivate(&scope, revision, &id, content.as_deref(), &token).await,
                Action::Delete(id) => client.agent_app_prompts_delete(&scope, revision, &id, &token).await,
                // The existing create contract allocates a new ID. CAS prevents restoring over newer edits.
                Action::Restore(entry) => client.agent_app_prompts_save(&scope, revision, &entry, false, &token).await,
            }.map_err(error_text)?;
            let expected_after = if let Some(id) = activation_id {
                catalog.entries.iter().find(|e| e.id == id).map(|e| e.content.clone())
            } else { expected_after };
            let preview = client.agent_app_prompts_current(&scope, &token).await.map_err(error_text);
            // A file changed after the write must never become the baseline for undo.
            let reverse = reverse.filter(|_| preview.as_ref().is_ok_and(|p| p.snapshot.content == expected_after));
            Ok((catalog, preview, reverse))
        }.await;
        Event::Mutated(context, catalog_generation, generation, scope, result)
    }, message)
}
