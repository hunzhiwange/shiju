//! Host files are read by the gateway; client files are uploaded as content only.
use crate::app::{App, Message, state::AgentAppsContext};
use super::{message, Event as PromptEvent};
use iced::Task;
use vw_gateway_client::{AgentAppPromptCatalog, AgentAppPromptCopies, AgentAppPromptCopyOutcome, AgentAppPromptImportSource, AgentAppPromptsScope, vw_api_types::agent_apps::AppId};
#[derive(Clone, Debug)]
pub enum Event {
    Open(String), Close, Select(AppId, bool), Copy, Body(String), Path, Reveal,
    ImportHost, ImportClient,
    Finished(AgentAppsContext, u64, u64, AgentAppPromptsScope, Result<Outcome, String>),
}
#[derive(Clone, Debug)]
pub enum Outcome { Copies(AgentAppPromptCopies), Imported(AgentAppPromptCatalog), Cancelled, Revealed }
fn send(event: Event) -> Message { message(PromptEvent::Transfer(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let client = match crate::app::config::gateway_client() { Ok(c) => c, Err(e) => return app.show_error_toast(e) };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let catalog_generation = app.apps_catalog.generation;
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() { return Task::none(); }
    let state = &mut app.apps_catalog.prompts;
    let Some(scope) = state.scope.clone() else { return Task::none(); };
    if scope.target_host_id != app.apps_catalog.host || app.project_id.as_deref() != Some(scope.project_id.as_ref()) { return Task::none(); }
    if let Event::Finished(request, catalog, generation, reply_scope, result) = event {
        if request != context || catalog != catalog_generation || generation != state.generation || !state.mutating
            || reply_scope.app_id != scope.app_id || reply_scope.project_id != scope.project_id || reply_scope.target_host_id != scope.target_host_id { return Task::none(); }
        state.mutating = false;
        return match result {
            Err(error) => app.show_error_toast(error),
            Ok(Outcome::Cancelled | Outcome::Revealed) => Task::none(),
            Ok(Outcome::Imported(catalog)) => { state.catalog = Some(catalog); state.undo = None; app.show_success_toast("已导入为停用条目") },
            Ok(Outcome::Copies(copies)) => {
                state.transfer.results = copies.results.into_iter().map(|copy| match copy.outcome {
                    AgentAppPromptCopyOutcome::Copied { .. } => format!("{:?}：已复制（停用）", copy.app_id),
                    AgentAppPromptCopyOutcome::Failed { error } => format!("{:?}：失败 {}（{}）", copy.app_id, error.code, error.status),
                }).collect();
                // Clear submitted targets: retry requires an explicit new selection.
                state.transfer.targets.clear();
                Task::none()
            }
        };
    }
    if state.loading || state.mutating || state.editor.is_some() { return Task::none(); }
    match event {
        Event::Open(id) => {
            if state.catalog.as_ref().is_some_and(|c| c.entries.iter().any(|e| e.id == id)) {
                state.transfer = crate::app::components::system_settings_prompts::transfer::State { entry: Some(id), ..Default::default() };
            }
            return Task::none();
        }
        Event::Close => { state.transfer = Default::default(); return Task::none(); }
        Event::Select(id, selected) => {
            if state.transfer.entry.is_some() && id != scope.app_id && crate::app::components::system_settings_prompts::transfer::TARGETS.contains(&id) {
                state.transfer.targets.retain(|t| *t != id);
                if selected { state.transfer.targets.push(id); }
            }
            return Task::none();
        }
        Event::Body(id) => return state.catalog.as_ref().and_then(|c| c.entries.iter().find(|e| e.id == id)).map(|e| iced::clipboard::write(e.content.clone())).unwrap_or_else(Task::none),
        Event::Path => return state.preview.as_ref().and_then(|p| p.display_path.clone()).map(iced::clipboard::write).unwrap_or_else(Task::none),
        _ => {}
    }
    let Some(catalog) = state.catalog.as_ref() else { return Task::none(); };
    let revision = catalog.revision;
    let id = state.transfer.entry.clone();
    let targets = state.transfer.targets.clone();
    if matches!(event, Event::Copy) && (id.is_none() || targets.is_empty()) { return Task::none(); }
    let path = state.preview.as_ref().and_then(|p| p.display_path.clone());
    if matches!(event, Event::Reveal) && (!client.endpoint().is_loopback() || scope.target_host_id != vw_shared::local_apps::HOST_ID) {
        return path.map(iced::clipboard::write).unwrap_or_else(Task::none);
    }
    state.mutating = true;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let token = app.apps_catalog.token.clone();
    Task::perform(async move {
        let result = async {
            match event {
                Event::Copy => client.agent_app_prompts_copy(&scope, id.as_deref().unwrap_or_default(), &targets, &token).await.map(Outcome::Copies).map_err(|e| e.to_string()),
                Event::ImportHost | Event::ImportClient => {
                    let source = if matches!(event, Event::ImportHost) { AgentAppPromptImportSource::CurrentFile } else {
                        let Some(file) = rfd::AsyncFileDialog::new().add_filter("提示词", &["md", "txt"]).pick_file().await else { return Ok(Outcome::Cancelled); };
                        // Read only the explicitly selected client file, with a bounded UTF-8 payload.
                        use std::io::Read;
                        let file_path = file.path().to_owned();
                        let content = tokio::task::spawn_blocking(move || {
                            let mut bytes = Vec::new();
                            std::fs::File::open(file_path).map_err(|_| "无法读取所选文件")?.take(1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|_| "无法读取所选文件")?;
                            if bytes.len() > 1024 * 1024 { return Err("所选文件超过 1 MiB"); }
                            String::from_utf8(bytes).map_err(|_| "所选文件不是 UTF-8 文本")
                        }).await.map_err(|_| "读取文件任务失败".to_owned())?.map_err(str::to_owned)?;
                        AgentAppPromptImportSource::UploadedContent { name: file.file_name(), content, description: None }
                    };
                    client.agent_app_prompts_import(&scope, revision, &source, &token).await.map(Outcome::Imported).map_err(|e| e.to_string())
                }
                Event::Reveal => {
                    let path = path.filter(|p| std::path::Path::new(p).is_absolute()).ok_or("目标路径不可用")?;
                    #[cfg(target_os = "macos")]
                    {
                        let status = tokio::task::spawn_blocking(move || std::process::Command::new("/usr/bin/open").arg("-R").arg(path).status()).await.map_err(|_| "Finder 定位任务失败")?.map_err(|_| "无法在 Finder 定位")?;
                        if !status.success() { return Err("无法在 Finder 定位".to_owned()); }
                        Ok(Outcome::Revealed)
                    }
                    #[cfg(not(target_os = "macos"))]
                    { let _ = path; Err("仅支持 macOS Finder 定位".to_owned()) }
                }
                _ => Ok(Outcome::Cancelled),
            }
        }.await;
        Event::Finished(context, catalog_generation, generation, scope, result)
    }, send)
}
