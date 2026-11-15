//! 记忆路由的宿主认证与共享状态组装。
use crate::app::agent::gateway::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json as JsonResponse},
};
pub use gateway_memory::{MemoryQuery, MemoryStoreBody};

struct MemoryAuth;
impl gateway_memory::MemoryAuth for MemoryAuth {
    fn require_auth<S, C, H>(state: &gateway_types::AppState<S, C, H>, headers: &HeaderMap)
        -> Result<(), (StatusCode, JsonResponse<serde_json::Value>)> {
        super::super::auth::require_auth(state, headers)
    }
}

pub async fn handle_api_memory_list(
    state: State<AppState>,
    headers: HeaderMap,
    params: Query<MemoryQuery>,
) -> impl IntoResponse {
    gateway_memory::handle_api_memory_list::<MemoryAuth, _, _, _>(state, headers, params).await
}
pub async fn handle_api_memory_store(
    state: State<AppState>,
    headers: HeaderMap,
    body: JsonResponse<MemoryStoreBody>,
) -> impl IntoResponse {
    gateway_memory::handle_api_memory_store::<MemoryAuth, _, _, _>(state, headers, body).await
}
pub async fn handle_api_memory_delete(
    state: State<AppState>,
    headers: HeaderMap,
    key: Path<String>,
) -> impl IntoResponse {
    gateway_memory::handle_api_memory_delete::<MemoryAuth, _, _, _>(state, headers, key).await
}
