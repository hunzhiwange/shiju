//! Session 路由模块
//!
//! 模块入口保留统一路由注册，具体实现按职责拆分到子文件：
//! - `stream`: SSE 流式聊天与流消息持久化
//! - `session_ops`: 会话 CRUD、状态、fork、diff、summarize 等
//! - `message_ops`: 会话消息与消息 part 操作
//! - `ui_ops`: UI 快照、归档、scope 等 UI store 相关处理器
//! - `shared`: 多个处理器共享的查询体与辅助函数

mod message_ops;
mod raw_commit;
mod session_ops;
mod shared;
pub(crate) mod stream;
mod ui_ops;

use axum::Router;
use axum::extract::{Extension, Request};
use axum::middleware::{Next, from_fn};
use axum::response::Response;
use axum::routing::{get, patch, post};

use crate::app::agent::gateway::AppState;
use message_ops::{
    session_message_get, session_message_list, session_message_part_delete,
    session_message_part_patch,
};
use session_ops::{
    session_children, session_diff, session_fork, session_reset, session_summarize,
    session_title_generate, session_todo_get, session_todo_put, ui_session_create,
    ui_session_delete, ui_session_get, ui_session_list, ui_session_patch, ui_session_status,
};
use stream::chat_stream;
use ui_ops::{
    session_archived_get, session_archived_put, session_path_get, session_scope_get,
    session_scope_put, session_step_llm_raw_get, session_ui_get, session_ui_get_any,
    session_ui_preview_meta, session_ui_previews, session_ui_save,
};
use vw_api_types::id::TenantScope;

/// 构建会话相关路由器
///
/// 返回配置好所有会话端点的 Axum Router 实例
///
/// # 示例
///
/// ```ignore
/// let router = router();
/// // router 现在包含 /session、/chat/stream 等路由
/// ```
pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/chat/stream", post(chat_stream))
        .route("/session", get(ui_session_list).post(ui_session_create))
        .route("/session/status", get(ui_session_status))
        .route(
            "/session/{session_id}",
            get(ui_session_get).patch(ui_session_patch).delete(ui_session_delete),
        )
        .route("/session/{session_id}/children", get(session_children))
        .route("/session/{session_id}/todo", get(session_todo_get).put(session_todo_put))
        .route("/session/{session_id}/fork", post(session_fork))
        .route("/session/{session_id}/reset", post(session_reset))
        .route("/session/{session_id}/diff", get(session_diff))
        .route("/session/{session_id}/summarize", post(session_summarize))
        .route("/session/{session_id}/message", get(session_message_list))
        .route("/session/{session_id}/message/{message_id}", get(session_message_get))
        .route(
            "/session/{session_id}/message/{message_id}/part/{part_id}",
            patch(session_message_part_patch).delete(session_message_part_delete),
        )
        .route("/session/ui-previews", get(session_ui_previews))
        .route("/session/archived", get(session_archived_get).put(session_archived_put))
        .route("/session/scope", get(session_scope_get).put(session_scope_put))
        .route("/session/{session_id}/ui", get(session_ui_get).put(session_ui_save))
        .route(
            "/session/{session_id}/turn/{turn_id}/step/{step_index}/llm-raw",
            get(session_step_llm_raw_get),
        )
        .route("/session/{session_id}/preview", get(session_ui_preview_meta))
        .route("/session/{session_id}/path", get(session_path_get))
        .route("/session/{session_id}/any", get(session_ui_get_any))
        .route("/session/{session_id}/title", post(session_title_generate))
        .layer(from_fn(session_tenant_scope))
}

async fn session_tenant_scope(
    Extension(tenant): Extension<TenantScope>,
    request: Request,
    next: Next,
) -> Response {
    crate::session::ui_store::with_tenant_scope(tenant, next.run(request)).await
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
