//! Session-scoped ACP controls. Reads never start an agent or send a prompt.
use crate::gateway::{ApiError, instance::InstanceQuery};
use crate::session::llm::{read_acp_session_options, select_acp_session_option};
use axum::{
    Json,
    extract::{Path, Query},
    http::HeaderMap,
};
use vw_api_types::acp_session::{AcpSessionOptions, AcpSessionSelection};

pub(super) async fn get(
    path: Path<String>,
    query: Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<AcpSessionOptions>, ApiError> {
    let session = super::session_ops::ui_session_get(path, query, headers).await?.0;
    let mut options = read_acp_session_options(&session.id, &session.directory)
        .await
        .map_err(ApiError::bad_request)?;
    if options.model_source_capabilities.is_empty()
        && let Some(binding) =
            crate::session::agent_binding::load_scoped(&session.project_id, &session.id)
                .await
                .map_err(|error| ApiError::bad_request(error.to_string()))?
        && let Some(agent) = binding.identity().acp_agent
    {
        crate::session::llm::annotate_acp_model_source(&mut options, &agent);
    }
    Ok(Json(options))
}

pub(super) async fn set(
    path: Path<String>,
    query: Query<InstanceQuery>,
    headers: HeaderMap,
    Json(selection): Json<AcpSessionSelection>,
) -> Result<Json<AcpSessionOptions>, ApiError> {
    let session = super::session_ops::ui_session_get(path, query, headers).await?.0;
    select_acp_session_option(&session.id, &session.directory, selection)
        .await
        .map(Json)
        .map_err(ApiError::bad_request)
}
