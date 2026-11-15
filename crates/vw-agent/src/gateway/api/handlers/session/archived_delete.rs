//! Typed archived deletion route assembly.
use axum::{Json, extract::{Path, Query, State}, http::HeaderMap};
use crate::gateway::{ApiError, AppState, instance::InstanceQuery};
use vw_api_session::session::ArchivedChatDeleteResponse;
pub(super) async fn delete(state: State<AppState>, path: Path<String>, Query(query): Query<InstanceQuery>, headers: HeaderMap) -> Result<Json<ArchivedChatDeleteResponse>, ApiError> {
    gateway_session::archived_routes::delete::<super::message_ops::Host>(state, path, Query(gateway_instance::host::InstanceQuery { directory: query.directory }), headers).await
}
