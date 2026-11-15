//! 会话消息与消息片段 API。
//!
//! 本模块提供桌面网关读取会话消息、读取单条消息、删除片段和更新片段的处理器。
//! 所有操作都会先解析实例目录，再在对应实例上下文内访问 session 存储，避免跨项目误读。

use axum::Json;
use axum::extract::Path;
use axum::extract::Query;
use axum::http::HeaderMap;
use vw_api_session::session::GatewaySessionMessageListQuery;

use super::shared::with_tenant_instance;

use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::instance::InstanceQuery;
use crate::app::agent::gateway::instance::resolve_directory;
use crate::app::agent::session as agent_session;

pub(super) struct Host;
impl gateway_session::message_host::MessageHost for Host {
    fn current_tenant_scope() -> vw_api_transport::TenantScope {
        crate::session::ui_store::current_tenant_scope()
    }
    fn resolve_directory(query: &gateway_instance::host::InstanceQuery, headers: &HeaderMap) -> String {
        resolve_directory(&InstanceQuery { directory: query.directory.clone() }, headers)
    }
    fn with_tenant_instance<T: Send + 'static>(
        tenant: vw_api_transport::TenantScope,
        directory: String,
        action: impl FnOnce() -> gateway_instance::host::HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> gateway_instance::host::HostFuture<Result<T, ApiError>> {
        Box::pin(with_tenant_instance(tenant, directory, action))
    }
}

pub(super) async fn session_message_list(
    Path(session_id): Path<String>,
    Query(query): Query<GatewaySessionMessageListQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<agent_session::message::WithParts>>, ApiError> {
    gateway_session::message_ops::session_message_list::<Host>(Path(session_id), Query(query), headers).await
}

pub(super) async fn session_message_get(
    Path((session_id, message_id)): Path<(String, String)>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<agent_session::message::WithParts>, ApiError> {
    gateway_session::message_ops::session_message_get::<Host>(Path((session_id, message_id)), Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers).await
}

pub(super) async fn session_message_part_delete(
    Path((session_id, message_id, part_id)): Path<(String, String, String)>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<bool>, ApiError> {
    gateway_session::message_ops::session_message_part_delete::<Host>(Path((session_id, message_id, part_id)), Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers).await
}

pub(super) async fn session_message_part_patch(
    Path((session_id, message_id, part_id)): Path<(String, String, String)>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Json(part): Json<agent_session::message::Part>,
) -> Result<Json<agent_session::message::Part>, ApiError> {
    gateway_session::message_ops::session_message_part_patch::<Host>(Path((session_id, message_id, part_id)), Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers, Json(part)).await
}

#[cfg(test)]
#[path = "message_ops_tests.rs"]
mod message_ops_tests;
