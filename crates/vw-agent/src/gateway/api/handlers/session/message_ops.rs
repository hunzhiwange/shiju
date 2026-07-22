//! 会话消息与消息片段 API。
//!
//! 本模块提供桌面网关读取会话消息、读取单条消息、删除片段和更新片段的处理器。
//! 所有操作都会先解析实例目录，再在对应实例上下文内访问 session 存储，避免跨项目误读。

use axum::Json;
use axum::extract::Path;
use axum::extract::Query;
use axum::http::HeaderMap;
use vw_api_types::session::GatewaySessionMessageListQuery;

use super::shared::with_tenant_instance;

use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::instance::InstanceQuery;
use crate::app::agent::gateway::instance::resolve_directory;
use crate::app::agent::session as agent_session;

/// 列出会话消息。
///
/// # 参数
///
/// * `session_id` - 路径中的会话 id。
/// * `query` - 可选目录和数量限制。
/// * `headers` - 可携带实例目录 header。
///
/// # 返回值
///
/// 返回包含消息片段的消息列表。
///
/// # 错误处理
///
/// 实例解析、消息读取或存储访问失败时返回 [`ApiError`]。
pub(super) async fn session_message_list(
    Path(session_id): Path<String>,
    Query(query): Query<GatewaySessionMessageListQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<agent_session::message::WithParts>>, ApiError> {
    let dir = resolve_directory(&InstanceQuery { directory: query.directory }, &headers);
    let result =
        with_tenant_instance(crate::session::ui_store::current_tenant_scope(), dir, move || {
            Box::pin(async move {
                agent_session::message::messages(&session_id, query.limit)
                    .await
                    .map_err(|e| ApiError::bad_request(e.to_string()))
            })
        })
        .await?;
    Ok(Json(result))
}

/// 获取单条会话消息。
///
/// # 参数
///
/// * `session_id` - 路径中的会话 id。
/// * `message_id` - 路径中的消息 id。
/// * `query` - 实例目录查询。
/// * `headers` - 可携带实例目录 header。
///
/// # 返回值
///
/// 返回指定消息及其片段。
pub(super) async fn session_message_get(
    Path((session_id, message_id)): Path<(String, String)>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<agent_session::message::WithParts>, ApiError> {
    let dir = resolve_directory(&query, &headers);
    let result =
        with_tenant_instance(crate::session::ui_store::current_tenant_scope(), dir, move || {
            Box::pin(async move {
                agent_session::message::get(&session_id, &message_id)
                    .await
                    .map_err(|e| ApiError::bad_request(e.to_string()))
            })
        })
        .await?;
    Ok(Json(result))
}

/// 删除一个消息片段。
///
/// # 参数
///
/// * `session_id` - 路径中的会话 id。
/// * `message_id` - 路径中的消息 id。
/// * `part_id` - 路径中的片段 id。
/// * `query` - 实例目录查询。
/// * `headers` - 可携带实例目录 header。
///
/// # 返回值
///
/// 删除成功时返回 `true`。
pub(super) async fn session_message_part_delete(
    Path((session_id, message_id, part_id)): Path<(String, String, String)>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<bool>, ApiError> {
    let dir = resolve_directory(&query, &headers);
    with_tenant_instance(crate::session::ui_store::current_tenant_scope(), dir, move || {
        Box::pin(async move {
            agent_session::message::remove_part(&session_id, &message_id, &part_id)
                .await
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
            Ok(())
        })
    })
    .await?;
    Ok(Json(true))
}

/// 更新一个消息片段。
///
/// # 参数
///
/// * `session_id` - 路径中的会话 id。
/// * `message_id` - 路径中的消息 id。
/// * `part_id` - 路径中的片段 id。
/// * `query` - 实例目录查询。
/// * `headers` - 可携带实例目录 header。
/// * `part` - 新片段内容。
///
/// # 返回值
///
/// 返回已保存的片段。
///
/// # 错误处理
///
/// 请求体中的 session/message/part id 与路径不一致时返回 bad request，避免客户端
/// 通过错误路径覆盖其他消息片段。
pub(super) async fn session_message_part_patch(
    Path((session_id, message_id, part_id)): Path<(String, String, String)>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(part): Json<agent_session::message::Part>,
) -> Result<Json<agent_session::message::Part>, ApiError> {
    // 路径 id 是访问边界，请求体 id 是待写实体；两者不一致时必须拒绝，防止越权改写。
    if part.id() != part_id || part.message_id() != message_id || part.session_id() != session_id {
        return Err(ApiError::bad_request(format!(
            "part mismatch: path session_id='{}' message_id='{}' part_id='{}'",
            session_id, message_id, part_id
        )));
    }

    let dir = resolve_directory(&query, &headers);
    let saved =
        with_tenant_instance(crate::session::ui_store::current_tenant_scope(), dir, move || {
            Box::pin(async move {
                agent_session::message::update_part(&part)
                    .await
                    .map_err(|e| ApiError::bad_request(e.to_string()))?;
                Ok(part)
            })
        })
        .await?;
    Ok(Json(saved))
}

#[cfg(test)]
#[path = "message_ops_tests.rs"]
mod message_ops_tests;
