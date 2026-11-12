//! plan-run 确定性会话预留与首次成功回复提交。

use crate::installation;
use crate::project;
use crate::session::session::{self, Info, TimeInfo};
use crate::session::store::FirstResponseCommit;
use crate::session::ui_store;
use crate::session::ui_types::{ChatMessage, ChatRole, ChatSession};
use std::path::Path;
use vw_api_types::id::TenantScope;

/// 按固定 ID 预留 plan-run 会话，重试复用原会话及其提交状态。
pub(crate) async fn ensure_plan_run_session_info(
    session_id: &str,
    parent_id: Option<&str>,
    root: &str,
) -> Result<(), String> {
    let session_id = session_id.trim();
    let root = root.trim();
    if session_id.is_empty() {
        return Err("plan-run session id 不能为空".to_string());
    }
    if root.is_empty() {
        return Err("plan-run session root 不能为空".to_string());
    }

    let tenant = ui_store::current_tenant_scope();
    let project_id = resolve_project_id(&tenant, root).await?;
    let now = session::now_ms();
    let parent_id = parent_id.map(str::trim).filter(|value| !value.is_empty()).map(str::to_string);
    let session_id = session_id.to_string();
    let root = root.to_string();

    crate::global_database::run_isolated(move || {
        let info = Info {
            id: session_id,
            slug: Info::plan_run_slug(&session::create_slug()),
            project_id,
            directory: root,
            parent_id,
            summary: None,
            share: None,
            title: String::new(),
            version: installation::version(),
            time: TimeInfo { created: now, updated: now, compacting: None, archived: None },
            permission: None,
            fork_execution_started: None,
            revert: None,
        };
        let reserved = ui_store::reserve_pending_session(&tenant, &info)
            .map_err(|error| format!("预留 plan-run Info 失败: {error}"))?;
        if !reserved.is_plan_run() || reserved.parent_id != info.parent_id {
            return Err(format!("plan-run 会话来源或父会话不匹配: {}", info.id));
        }
        Ok(())
    })
    .await
    .map_err(|error| format!("plan-run Info 存储线程异常: {error}"))?
}

/// 原子提交本次成功的 user/assistant 历史和阶段标题。
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

    if assistant_content.trim().is_empty() {
        return Err("plan-run session 缺少有效回复".to_string());
    }

    let tenant = ui_store::current_tenant_scope();
    let project_id = resolve_project_id(&tenant, root).await?;
    let now = session::now_ms();
    let session_id = session_id.to_string();
    let title = title.to_string();
    let user_content = user_content.to_string();
    let assistant_content = assistant_content.to_string();

    crate::global_database::run_isolated(move || {
        let scope = Some(project_id.as_str());
        let info = ui_store::load_agent_session_scoped_for_tenant(&tenant, &session_id, scope)
            .filter(Info::is_plan_run)
            .ok_or_else(|| format!("缺少 plan-run 会话预留: {session_id}"))?;
        let mut chat =
            ui_store::load_session_scoped_for_tenant_fallible(&tenant, &session_id, scope)
                .map_err(|error| error.to_string())?
                .unwrap_or_else(|| ChatSession {
                    id: info.id.clone(),
                    title: info.title.clone(),
                    messages: Vec::new(),
                    message_ids: Vec::new(),
                    calls: Vec::new(),
                    steps: Vec::new(),
                    created_ms: info.time.created,
                    updated_ms: info.time.updated,
                });
        let pending = info.is_first_response_pending();
        if !pending {
            let replace_legacy =
                chat.messages.is_empty() || is_legacy_failure_history(&chat.messages);
            if !replace_legacy
                || ui_store::has_committed_first_response(&tenant, &project_id, &session_id)
                    .map_err(|error| error.to_string())?
            {
                return Ok(());
            }
        }

        chat.messages = vec![
            ChatMessage {
                role: ChatRole::User,
                content: user_content,
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: assistant_content,
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        ];
        chat.message_ids = vec![None, None];
        chat.updated_ms = chat.updated_ms.max(now);
        chat.title = info.title;
        if pending {
            let published = ui_store::commit_first_response(
                &tenant,
                &project_id,
                FirstResponseCommit {
                    preview: Some(&chat.messages[0].content),
                    snapshot: &chat,
                    raw_artifacts: &[],
                    expected_finished_steps: &[],
                    task_title: Some(&title),
                },
            )
            .map_err(|error| error.to_string())?;
            if !published
                && !ui_store::has_committed_first_response(&tenant, &project_id, &session_id)
                    .map_err(|error| error.to_string())?
            {
                return Err(format!("plan-run 会话预留已失效: {session_id}"));
            }
        } else {
            ui_store::save_session_ui_scoped_for_tenant(&tenant, &chat, scope, true)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|error| format!("plan-run ChatSession 存储线程异常: {error}"))?
}

// 旧版会把失败摘要当作两条聊天消息写入，重跑成功后需要替换占位。
fn is_legacy_failure_history(messages: &[ChatMessage]) -> bool {
    matches!(messages, [user, assistant]
        if user.role == ChatRole::User && assistant.role == ChatRole::Assistant
            && ["规划会话失败: ", "子任务失败: ", "目标续轮失败: "]
                .iter()
                .any(|prefix| assistant.content.starts_with(prefix)))
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
