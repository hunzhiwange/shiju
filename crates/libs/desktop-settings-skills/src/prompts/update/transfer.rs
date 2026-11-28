//! Host files are read by the gateway; client files are uploaded as content only.
use super::{Input, Event as Message};
use desktop_provider_apps::AgentAppsContext;
use super::{message, Event as PromptEvent};
use iced::Task;
use gateway_app_prompts::{AgentAppPromptCatalog, AgentAppPromptCopies, AgentAppPromptCopyOutcome, AgentAppPromptImportSource, AgentAppPromptsScope};
use {vw_api_app_identity::AppId};
#[derive(Clone, Debug)]
pub enum Event {
    Open(String), Close, Select(AppId, bool), Copy, Body(String), Path, Reveal,
    ImportHost, ImportClient,
    Finished(AgentAppsContext, u64, u64, AgentAppPromptsScope, Result<Outcome, String>),
}
#[derive(Clone, Debug)]
pub enum Outcome { Copies(AgentAppPromptCopies), Imported(AgentAppPromptCatalog), Cancelled }
fn send(event: Event) -> Message { message(PromptEvent::Transfer(event)) }
pub fn update(app: &mut Input<'_>, event: Event) -> Task<Message> {
    let client = match app.client.clone() { Ok(c) => c, Err(e) => return crate::prompts::update::Input::show_error_toast(e) };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let prompts_client = gateway_app_prompts::GatewayAppPromptsClient::new(client.transport().clone());
    let catalog_generation = app.generation;
    if app.context.as_ref() != Some(&context) { return Task::none(); }
    let state = &mut app.prompts;
    let Some(scope) = state.scope.clone() else { return Task::none(); };
    if scope.target_host_id != app.host || app.project_id.as_deref() != Some(scope.project_id.as_ref()) { return Task::none(); }
    if let Event::Finished(request, catalog, generation, reply_scope, result) = event {
        if request != context || catalog != catalog_generation || generation != state.generation || !state.mutating
            || reply_scope.app_id != scope.app_id || reply_scope.project_id != scope.project_id || reply_scope.target_host_id != scope.target_host_id { return Task::none(); }
        state.mutating = false;
        return match result {
            Err(error) => crate::prompts::update::Input::show_error_toast(error),
            Ok(Outcome::Cancelled) => Task::none(),
            Ok(Outcome::Imported(catalog)) => { state.catalog = Some(catalog); state.undo = None; crate::prompts::update::Input::show_success_toast("已导入为停用条目") },
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
                state.transfer = crate::prompts::transfer::State { entry: Some(id), ..Default::default() };
            }
            return Task::none();
        }
        Event::Close => { state.transfer = Default::default(); return Task::none(); }
        Event::Select(id, selected) => {
            if state.transfer.entry.is_some() && id != scope.app_id && crate::prompts::transfer::TARGETS.contains(&id) {
                state.transfer.targets.retain(|t| *t != id);
                if selected { state.transfer.targets.push(id); }
            }
            return Task::none();
        }
        Event::Body(id) => return state.catalog.as_ref().and_then(|c| c.entries.iter().find(|e| e.id == id)).map(|e| iced::clipboard::write(e.content.clone())).unwrap_or_else(Task::none),
        Event::Path => return state.preview.as_ref().and_then(|p| p.display_path.clone()).map(iced::clipboard::write).unwrap_or_else(Task::none),
        _ => {}
    }
    let path = state.preview.as_ref().and_then(|preview| preview.display_path.clone());
    if matches!(event, Event::Reveal) {
        let Some(path) = path else { return Task::none(); };
        if !client.endpoint().is_loopback() || scope.target_host_id != local_app_auth::HOST_ID {
            return iced::clipboard::write(path);
        }
        return Task::perform(open_prompt_folder(path), |result| match result {
            Ok(()) => PromptEvent::Toast(false, "已打开所在文件夹".into()),
            Err(error) => PromptEvent::Toast(true, error.into()),
        });
    }
    let Some(catalog) = state.catalog.as_ref() else { return Task::none(); };
    let revision = catalog.revision;
    let id = state.transfer.entry.clone();
    let targets = state.transfer.targets.clone();
    if matches!(event, Event::Copy) && (id.is_none() || targets.is_empty()) { return Task::none(); }
    state.mutating = true;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let token = app.token.clone();
    Task::perform(async move {
        let result = async {
            match event {
                Event::Copy => prompts_client.agent_app_prompts_copy(&scope, id.as_deref().unwrap_or_default(), &targets, &token).await.map(Outcome::Copies).map_err(|e| e.to_string()),
                Event::ImportHost | Event::ImportClient => {
                    #[cfg(target_arch = "wasm32")]
                    if matches!(event, Event::ImportClient) {
                        return Err("此平台不支持从本地文件导入提示词".into());
                    }
                    #[cfg(target_arch = "wasm32")]
                    let source = AgentAppPromptImportSource::CurrentFile;
                    #[cfg(not(target_arch = "wasm32"))]
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
                    prompts_client.agent_app_prompts_import(&scope, revision, &source, &token).await.map(Outcome::Imported).map_err(|e| e.to_string())
                }
                _ => Ok(Outcome::Cancelled),
            }
        }.await;
        Event::Finished(context, catalog_generation, generation, scope, result)
    }, send)
}

async fn open_prompt_folder(path: String) -> Result<(), &'static str> {
    tokio::task::spawn_blocking(move || {
        use std::path::{Component, Path};

        let home = std::env::var("HOME").map_err(|_| "无法确定本机用户目录")?;
        let path = if let Some(relative) = path.strip_prefix("~/") {
            Path::new(&home).join(relative)
        } else {
            Path::new(&path).to_path_buf()
        };
        let relative = path.strip_prefix(&home).map_err(|_| "目标路径不可用")?;
        if !relative.components().all(|part| matches!(part, Component::Normal(_))) {
            return Err("目标路径不可用");
        }
        let folder = path.parent().filter(|folder| folder.is_dir())
            .and_then(Path::to_str).ok_or("所在文件夹不存在")?;
        desktop_provider_apps::message::paths::open_local(&home, folder)
            .map_err(|_| "无法打开所在文件夹")
    }).await.map_err(|_| "打开文件夹任务失败")?
}
