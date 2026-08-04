//! plan-run 确定性会话 Info 绑定与可渲染历史落盘。
//!
//! 使用固定 session id 写入 `Info`（可选 `parent_id`），并保证 ChatSession
//! 至少有可被桌面聊天面板渲染的 user/assistant messages。

use crate::installation;
use crate::project;
use crate::session::session::{self, Info, TimeInfo};
use crate::session::ui_store;
use crate::session::ui_types::{ChatMessage, ChatRole, ChatSession};
use std::path::Path;
use vw_api_types::id::TenantScope;

/// 确保 plan-run 子会话写入指定 id 的 `Info`（含可选 `parent_id`）。
///
/// 不调用 `create_next`（会生成新 id）；通过 `ui_store` 直接 upsert 指定 id。
/// 无 `parent_id` 时仍创建独立 Info（`parent_id=None`）。
pub(crate) async fn ensure_plan_run_session_info(
    session_id: &str,
    parent_id: Option<&str>,
    root: &str,
    title: &str,
) -> Result<(), String> {
    let session_id = session_id.trim();
    let root = root.trim();
    let title = title.trim();
    if session_id.is_empty() {
        return Err("plan-run session id 不能为空".to_string());
    }
    if root.is_empty() {
        return Err("plan-run session root 不能为空".to_string());
    }
    if title.is_empty() {
        return Err("plan-run session title 不能为空".to_string());
    }

    let tenant = ui_store::current_tenant_scope();
    let project_id = resolve_project_id(&tenant, root).await?;
    let now = session::now_ms();
    let parent_id = parent_id.map(str::trim).filter(|value| !value.is_empty()).map(str::to_string);
    let session_id = session_id.to_string();
    let root = root.to_string();
    let title = title.to_string();

    crate::global_database::run_isolated(move || {
        let existing =
            ui_store::load_agent_session_scoped_for_tenant(&tenant, &session_id, Some(&project_id));
        let (created, slug) = match existing {
            Some(info) => (info.time.created, info.slug),
            None => (now, session::create_slug()),
        };
        let slug = Info::plan_run_slug(&slug);

        let info = Info {
            id: session_id.clone(),
            slug,
            project_id: project_id.clone(),
            directory: root,
            parent_id,
            summary: None,
            share: None,
            title,
            version: installation::version(),
            time: TimeInfo { created, updated: now, compacting: None, archived: None },
            permission: None,
            revert: None,
        };

        ui_store::save_agent_session_scoped_for_tenant(&tenant, &info, Some(&project_id))
            .ok_or_else(|| {
                format!("写入 plan-run Info 失败: session={session_id} project={project_id}")
            })?;
        Ok(())
    })
    .await
    .map_err(|error| format!("plan-run Info 存储线程异常: {error}"))?
}

/// 将最小可渲染 user/assistant 历史写入 ChatSession（messages 为空时补齐）。
pub(crate) async fn persist_plan_run_session_history(
    session_id: &str,
    root: &str,
    title: &str,
    user_content: &str,
    assistant_content: &str,
) -> Result<(), String> {
    let session_id = session_id.trim();
    let root = root.trim();
    let title = title.trim();
    if session_id.is_empty() {
        return Err("plan-run session id 不能为空".to_string());
    }
    if root.is_empty() {
        return Err("plan-run session root 不能为空".to_string());
    }

    let tenant = ui_store::current_tenant_scope();
    let project_id = resolve_project_id(&tenant, root).await?;
    let now = session::now_ms();
    let session_id = session_id.to_string();
    let title = title.to_string();
    let user_content = user_content.to_string();
    let assistant_content = assistant_content.to_string();

    crate::global_database::run_isolated(move || {
        let mut chat =
            ui_store::load_session_scoped_for_tenant(&tenant, &session_id, Some(&project_id))
                .unwrap_or(ChatSession {
                    id: session_id.clone(),
                    title: if title.is_empty() { "plan-run".to_string() } else { title.clone() },
                    messages: Vec::new(),
                    message_ids: Vec::new(),
                    calls: Vec::new(),
                    steps: Vec::new(),
                    created_ms: now,
                    updated_ms: 0,
                });

        if !title.is_empty() {
            chat.title = title;
        }

        if chat.messages.is_empty() {
            let user = if user_content.trim().is_empty() {
                "(空提示)".to_string()
            } else {
                user_content
            };
            let assistant = if assistant_content.trim().is_empty() {
                "(空输出)".to_string()
            } else {
                assistant_content
            };
            chat.messages = vec![
                ChatMessage { role: ChatRole::User, content: user, model: None, think_timing: Vec::new(), processing_timing: None },
                ChatMessage {
                    role: ChatRole::Assistant,
                    content: assistant,
                    model: None,
                    think_timing: Vec::new(),
                    processing_timing: None,
                },
            ];
            chat.message_ids = vec![None, None];
        }

        chat.updated_ms = now;
        ui_store::save_session_scoped_for_tenant(&tenant, &chat, Some(&project_id)).ok_or_else(
            || format!("写入 plan-run ChatSession 失败: session={session_id} project={project_id}"),
        )?;
        Ok(())
    })
    .await
    .map_err(|error| format!("plan-run ChatSession 存储线程异常: {error}"))?
}

async fn resolve_project_id(tenant: &TenantScope, root: &str) -> Result<String, String> {
    let root_path = Path::new(root);
    if !root_path.is_dir() {
        return Err(format!("plan-run root 不是目录: {root}"));
    }
    // 与 with_tenant_scope / save_*_for_tenant 使用同一租户，避免默认租户与请求租户不一致导致绑定失败。
    project::from_directory(tenant, root_path)
        .await
        .map(|(info, _)| info.id)
        .map_err(|error| format!("无法从 root 解析项目: {error}"))
}
