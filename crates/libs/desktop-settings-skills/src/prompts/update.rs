//! Scope and request generation travel with every response, including failures.
use Event as Message;
use vw_api_app_operations::{PromptTargetPreview};
pub mod shared_import;
pub mod pi_files;
pub mod transfer;
pub mod templates;
pub mod sync;
use desktop_provider_apps::AgentAppsContext;
use iced::Task;
use crate::prompts::activation::{Action, Undo, error_text, effect};
use gateway_app_prompts::{AgentAppPromptCatalog, AgentAppPromptsScope};
use {vw_api_types::agent_apps::{AppId}};
#[derive(Clone, Debug)]
pub enum Event {
    Query(String),
    Menu(Option<super::list::Menu>),
    Toast(bool, String),
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
    Reply(AgentAppsContext, u64, u64, AgentAppPromptsScope,
        Result<AgentAppPromptCatalog, String>, Result<PromptTargetPreview, String>),
}
pub fn message(event: Event) -> Event { event }
pub fn update(app: &mut Input<'_>, event: Event) -> Task<Message> {
    // The editor owns its surface; only draft edits and matching save replies may pass.
    if app.prompts.editor.is_some() && !matches!(&event,
        Event::Name(_) | Event::Description(_) | Event::Body(_) | Event::CloseEditor
        | Event::Save | Event::Saved(..) | Event::Toast(..)) {
        return Task::none();
    }
    if let Event::SharedImport(event) = event { return shared_import::update(app, event); }
    if app.prompts.shared_import.is_some() { return Task::none(); }
    if !matches!(&event, Event::Query(_) | Event::Menu(_)) { app.prompts.menu = None; }
    if let Event::Sync(event) = event { return sync::update(app, event); }
    if let Event::Template(event) = event { return templates::update(app, event); }
    if let Event::Transfer(event) = event { return transfer::update(app, event); }
    if let Event::PiFile(event) = event {
        if app.prompts.mutating { return Task::none(); }
        return pi_files::update(app, event);
    }
    match &event {
        Event::Query(value) => { app.prompts.query = value.clone(); app.prompts.menu = None; return Task::none(); }
        Event::Menu(menu) => {
            if menu.is_none() || (!app.prompts.mutating && (!app.prompts.loading
                || matches!(menu, Some(super::list::Menu::Apps | super::list::Menu::Help)))) {
                app.prompts.menu = menu.clone();
            }
            return Task::none();
        }
        _ => { app.prompts.menu = None; }
    }
    let client = match app.client.clone() {
        Ok(client) => client,
        Err(error) => { return crate::prompts::update::Input::show_error_toast(error); }
    };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let client = gateway_app_prompts::GatewayAppPromptsClient::new(client.transport().clone());
    let catalog_generation = app.generation;
    match event {
        Event::Query(_) | Event::Menu(_) => Task::none(),
        Event::Toast(..) => Task::none(),
        Event::SharedImport(_) | Event::PiFile(_) | Event::Sync(_) | Event::Transfer(_) | Event::Template(_) => unreachable!(),
        Event::Edit(id) => {
            let state = &mut app.prompts;
            if state.loading || state.mutating || state.editor.is_some() { return Task::none(); }
            let (Some(catalog), Some(scope)) = (&state.catalog, &state.scope) else { return Task::none(); };
            let entry = match id {
                Some(id) => match catalog.entries.iter().find(|entry| entry.id == id) { Some(entry) => Some(entry.clone()), None => return Task::none() },
                None => None,
            };
            state.editor = Some(crate::prompts::editor::Draft::new(entry, catalog.revision, scope.app_id == AppId::Mcode));
            Task::none()
        }
        Event::CloseEditor => {
            if app.prompts.editor.as_ref().is_some_and(|draft| !draft.saving) { app.prompts.editor = None; }
            Task::none()
        }
        Event::Name(value) => {
            if let Some(draft) = app.prompts.editor.as_mut().filter(|d| !d.saving) { draft.entry.name = value; }
            Task::none()
        }
        Event::Description(value) => {
            if let Some(draft) = app.prompts.editor.as_mut().filter(|d| !d.saving) { draft.entry.description = Some(value); }
            Task::none()
        }
        Event::Body(action) => {
            if let Some(draft) = app.prompts.editor.as_mut().filter(|d| !d.saving) { draft.body.perform(action); }
            Task::none()
        }
        Event::Save if app.prompts.mutating => Task::none(),
        Event::Save => {
            if app.context.as_ref() != Some(&context) { return crate::prompts::update::Input::show_error_toast("请先连接应用管理"); }
            let state = &mut app.prompts;
            let Some(scope) = state.scope.clone() else { return Task::none(); };
            if scope.target_host_id != app.host || app.project_id.as_deref() != Some(scope.project_id.as_ref()) { return Task::none(); }
            let Some(draft) = state.editor.as_mut().filter(|d| !d.saving) else { return Task::none(); };
            draft.entry.content = draft.body.text();
            if draft.entry.name.trim().is_empty() {
                draft.error = Some("请填写提示词名称".into());
                return Task::none();
            }
            if draft.mcode && draft.entry.content.len() > 32768 {
                draft.error = Some("MCode 正文最多 32768 UTF-8 字节".into());
                return Task::none();
            }
            draft.saving = true;
            draft.error = None;
            let entry = draft.entry.clone();
            let revision = draft.revision;
            let updating = draft.updating;
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            let token = app.token.clone();
            Task::perform(async move {
                let result = client.agent_app_prompts_save(&scope, revision, &entry, updating, &token).await.map_err(|error| match error {
                    gateway_app_prompts::AgentAppPromptsError::Http { status: 413, .. } => "正文超过服务端限长；MCode 最多 32768 UTF-8 字节".to_owned(),
                    gateway_app_prompts::AgentAppPromptsError::Http { status: 409, .. } => "内容版本已变化，请保留草稿并刷新后重试".to_owned(),
                    error => error.to_string(),
                });
                Event::Saved(context, catalog_generation, generation, scope, result)
            }, message)
        }
        Event::Saved(request, catalog_generation_reply, generation, scope, result) => {
            let state = &mut app.prompts;
            if request != context || catalog_generation_reply != catalog_generation || generation != state.generation
                || !state.scope.as_ref().is_some_and(|s| s.app_id == scope.app_id && s.project_id == scope.project_id && s.target_host_id == scope.target_host_id)
                || scope.target_host_id != app.host || app.project_id.as_deref() != Some(scope.project_id.as_ref())
                || !state.editor.as_ref().is_some_and(|d| d.saving) { return Task::none(); }
            match result {
                Ok(catalog) => { state.catalog = Some(catalog); state.editor = None; state.undo = None;
                    // Reload the target snapshot as well as the catalog before enabling writes.
                    let refresh = update(app, Event::Open(scope.app_id));
                    Task::batch([refresh, Input::show_success_toast(format!("提示词已保存。{}", effect(scope.app_id)))])}
                Err(error) => { let draft = state.editor.as_mut().expect("saving editor"); draft.saving = false; draft.error = Some(error.clone()); crate::prompts::update::Input::show_error_toast(error)}
            }
        }
        Event::Mutate(_) | Event::Undo if app.prompts.loading || app.prompts.mutating || app.prompts.editor.is_some() => Task::none(),
        Event::Mutate(_) | Event::Undo if app.context.as_ref() != Some(&context) => crate::prompts::update::Input::show_error_toast("请先连接应用管理"),
        Event::Mutate(action) => mutate(app, client, context, catalog_generation, action, false),
        Event::Undo => {
            let Some(undo) = app.prompts.undo.clone() else { return Task::none(); };
            mutate(app, client, context, catalog_generation, undo.action, true)
        }
        Event::Mutated(request, catalog_generation_reply, generation, scope, result) => {
            let state = &mut app.prompts;
            if request != context || catalog_generation_reply != catalog_generation || state.generation != generation
                || !state.mutating || state.scope.as_ref().is_none_or(|s| s.app_id != scope.app_id || s.project_id != scope.project_id || s.target_host_id != scope.target_host_id)
                || scope.target_host_id != app.host || app.project_id.as_deref() != Some(scope.project_id.as_ref()) { return Task::none(); }
            state.mutating = false;
            match result {
                Err(error) => { state.error = Some(error.clone()); crate::prompts::update::Input::show_error_toast(error) }
                Ok((catalog, preview, safe_reverse)) => {
                    state.catalog = Some(catalog.clone());
                    match preview {
                        Ok(preview) => {
                            state.undo = safe_reverse.map(|action| Undo { revision: catalog.revision, content: preview.snapshot.content.clone(), action });
                            state.preview = Some(preview);
                            state.preview_error = None;
                            crate::prompts::update::Input::show_success_toast(effect(scope.app_id))
                        }
                        Err(error) => {
                            state.preview = None;
                            state.preview_error = Some(error);
                            crate::prompts::update::Input::show_error_toast("操作已保存，但目标快照刷新失败；请刷新后继续。")
                        }
                    }
                }
            }
        }
        Event::Open(_) if app.prompts.mutating => Task::none(),
        Event::Reply(request, catalog, generation, scope, library, preview) => {
            let state = &mut app.prompts;
            let same_scope = state.scope.as_ref().is_some_and(|current| current.app_id == scope.app_id && current.project_id == scope.project_id && current.target_host_id == scope.target_host_id);
            if request != context || catalog != catalog_generation || generation != state.generation || !same_scope || !state.loading
                || scope.target_host_id != app.host || app.project_id.as_deref() != Some(scope.project_id.as_ref()) { return Task::none(); }
            state.loading = false;
            match library {
                Ok(catalog) => state.catalog = Some(catalog),
                Err(error) => state.error = Some(error),
            }
            match preview {
                Ok(preview) => state.preview = Some(preview),
                Err(error) => state.preview_error = Some(error),
            }
            Task::none()
        }
        Event::Open(id) => {
            if app.prompts.editor.is_some() { return Task::none(); }
            app.prompts.invalidate();
            if matches!(id, AppId::ClaudeDesktop | AppId::OpenClaw) { return crate::prompts::update::Input::show_error_toast("请使用该应用的专属入口"); }
            if app.context.as_ref() != Some(&context) { return crate::prompts::update::Input::show_error_toast("请先连接应用管理"); }
            let project = vw_api_transport::local_settings_project(app.project_id.as_deref());
            let scope = AgentAppPromptsScope { project_id: project.into(), target_host_id: app.host.clone(), app_id: id };
            let state = &mut app.prompts;
            state.scope = Some(scope.clone());
            state.loading = true;
            let generation = state.generation;
            let token = app.token.clone();
            Task::perform(async move {
                let library = client.agent_app_prompts(&scope, "", &token).await
                    .map_err(|error| read_error("提示词库", error));
                let preview = client.agent_app_prompts_current(&scope, &token).await
                    .map_err(|error| read_error("目标文件", error));
                Event::Reply(context, catalog_generation, generation, scope, library, preview)
            }, message)
        }
    }
}

fn read_error(subject: &str, error: gateway_app_prompts::AgentAppPromptsError) -> String {
    use gateway_app_prompts::AgentAppPromptsError as Error;
    let reason = match error {
        Error::Http { status: 401 | 403, .. } => "网关未授权".into(),
        Error::Http { status: 422, body } => match body["error"]["code"].as_str() {
            Some("invalid_utf8") => "文件编码不是 UTF-8".into(),
            Some("invalid_configuration") => "文件路径不可访问或配置无效".into(),
            Some("unsupported_target") => "当前主机不支持此应用".into(),
            _ => "网关拒绝读取".into(),
        },
        Error::Http { status: 409, .. } => "读取期间发生了变化".into(),
        Error::Http { status: 503, body } => match body["error"]["code"].as_str() {
            Some("prompt_storage_unavailable") => "网关的提示词存储暂不可用（503）".into(),
            Some("prompt_initialization_failed") => "网关未能初始化提示词库（503）".into(),
            _ => "网关暂时无法完成读取（503）".into(),
        },
        Error::Http { status, .. } => format!("网关返回 HTTP {status}"),
        Error::Transport => "无法连接网关".into(),
        Error::Decode => "网关返回的数据格式无效".into(),
        Error::InvalidInput => "请求参数无效".into(),
    };
    format!("{subject}读取失败 · {reason}")
}

fn mutate(app: &mut Input<'_>, client: gateway_app_prompts::GatewayAppPromptsClient, context: AgentAppsContext,
    catalog_generation: u64, action: Action, undoing: bool) -> Task<Message> {
    let state = &mut app.prompts;
    let (Some(scope), Some(catalog), Some(preview)) = (state.scope.clone(), state.catalog.as_ref(), state.preview.as_ref()) else { return Task::none(); };
    if scope.target_host_id != app.host || app.project_id.as_deref() != Some(scope.project_id.as_ref()) { return Task::none(); }
    let revision = catalog.revision;
    let content = preview.snapshot.content.clone();
    if undoing && state.undo.as_ref().is_none_or(|u| u.revision != revision || u.content != content) {
        state.undo = None;
        return crate::prompts::update::Input::show_error_toast("撤销已过期：提示词或目标文件已变化。");
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
    let token = app.token.clone();
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
            let preview = client.agent_app_prompts_current(&scope, &token).await
                .map_err(|error| read_error("目标文件", error));
            // A file changed after the write must never become the baseline for undo.
            let reverse = reverse.filter(|_| preview.as_ref().is_ok_and(|p| p.snapshot.content == expected_after));
            Ok((catalog, preview, reverse))
        }.await;
        Event::Mutated(context, catalog_generation, generation, scope, result)
    }, message)
}

/// Inputs used by prompt updates; the desktop owns connection selection and toast rendering.
pub struct Input<'a> {
    pub prompts: &'a mut crate::prompts::State<AgentAppsContext>,
    pub context: Option<AgentAppsContext>,
    pub generation: u64,
    pub host: String,
    pub token: String,
    pub project_id: Option<String>,
    pub project_path: Option<String>,
    pub client: Result<vw_gateway_client::GatewayClient, String>,
}
impl Input<'_> {
    fn show_error_toast( text: impl Into<String>) -> Task<Event> { Task::done(Event::Toast(true, text.into())) }
    fn show_success_toast( text: impl Into<String>) -> Task<Event> { Task::done(Event::Toast(false, text.into())) }
}
