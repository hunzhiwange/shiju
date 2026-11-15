//! Domain dispatch supplies a decoded payload after opening the explicit target scope.
use vw_api_app_operations::{ImportPromptShareRequest, ImportPromptShareResult, PromptActivationChoice, PromptSharePayload};
use super::{Input, Event as Message};
use desktop_provider_apps::AgentAppsContext;
use super::{message, Event as PromptEvent};
use iced::Task;
use gateway_app_prompts::{AgentAppPromptsScope};
#[derive(Clone, Debug)]
pub enum Event {
    Open(PromptSharePayload), Cancel, Enable(bool), Confirm,
    Finished(AgentAppsContext, u64, u64, AgentAppPromptsScope, Result<ImportPromptShareResult, String>),
}
fn send(event: Event) -> Message { message(PromptEvent::SharedImport(event)) }
pub fn update(app: &mut Input<'_>, event: Event) -> Task<Message> {
    // Cancellation is purely local, including when the gateway has disconnected.
    if matches!(&event, Event::Cancel) {
        if !app.prompts.mutating { app.prompts.shared_import = None; }
        return Task::none();
    }
    let client = match app.client.clone() { Ok(c) => c, Err(e) => return crate::prompts::update::Input::show_error_toast(e) };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let client = gateway_app_prompts::GatewayAppPromptsClient::new(client.transport().clone());
    let catalog_generation = app.generation;
    if app.context.as_ref() != Some(&context) || app.token.is_empty() { return Task::none(); }
    let state = &mut app.prompts;
    let Some(scope) = state.scope.clone() else { return Task::none(); };
    if scope.target_host_id != app.host || app.project_id.as_deref() != Some(scope.project_id.as_ref()) { return Task::none(); }
    match event {
        Event::Finished(request, catalog, generation, reply_scope, result) => {
            if request != context || catalog != catalog_generation || generation != state.generation || !state.mutating
                || state.shared_import.is_none() || reply_scope.app_id != scope.app_id || reply_scope.project_id != scope.project_id || reply_scope.target_host_id != scope.target_host_id { return Task::none(); }
            state.mutating = false;
            match result {
                Err(error) => {
                    // Transport errors can follow persistence. Never offer a blind replay.
                    state.shared_import = None;
                    state.catalog = None;
                    state.preview = None;
                    crate::prompts::update::Input::show_error_toast(format!("导入结果未确认，请刷新列表核对，勿重复导入：{error}"))
                }
                Ok(result) => {
                    state.shared_import = None;
                    state.catalog = None;
                    state.preview = None;
                    state.undo = None;
                    match result {
                        ImportPromptShareResult::ImportedDisabled { .. } => crate::prompts::update::Input::show_success_toast("已导入停用副本，请刷新列表"),
                        ImportPromptShareResult::Activated { .. } => crate::prompts::update::Input::show_success_toast("已导入并激活，请刷新列表"),
                        ImportPromptShareResult::ImportedActivationFailed { .. } => crate::prompts::update::Input::show_error_toast("条目已导入但激活失败；请刷新核对停用副本，勿重复导入"),
                    }
                }
            }
        }
        Event::Open(payload) => {
            if state.loading || state.mutating || state.editor.is_some() || state.shared_import.is_some() || state.catalog.is_none() || state.preview.is_none()
                || !crate::prompts::transfer::TARGETS.contains(&scope.app_id) { return Task::none(); }
            state.shared_import = Some(crate::prompts::shared_import::Draft { payload, enable_after_import: false, error: None });
            Task::none()
        }
        Event::Enable(value) => { if !state.mutating { if let Some(draft) = &mut state.shared_import { draft.enable_after_import = value; } } Task::none() }
        Event::Confirm => {
            if state.loading || state.mutating { return Task::none(); }
            let (Some(draft), Some(catalog), Some(preview)) = (&state.shared_import, &state.catalog, &state.preview) else { return Task::none(); };
            let import = ImportPromptShareRequest { payload: draft.payload.clone(), target_app: scope.app_id, confirmed: true,
                activation: if draft.enable_after_import { PromptActivationChoice::Activate { expected_target_revision: preview.snapshot.revision.clone() } } else { PromptActivationChoice::KeepDisabled } };
            if import.validate().is_err() { return crate::prompts::update::Input::show_error_toast("不支持该提示词目标"); }
            let revision = catalog.revision;
            state.mutating = true;
            state.generation = state.generation.wrapping_add(1);
            let generation = state.generation;
            let token = app.token.clone();
            Task::perform(async move {
                let result = client.agent_app_prompts_import_share(&scope, revision, &import, &token).await.map_err(|e| e.to_string());
                Event::Finished(context, catalog_generation, generation, scope, result)
            }, send)
        }
        Event::Cancel => Task::none(),
    }
}
