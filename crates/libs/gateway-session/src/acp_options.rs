//! Session-scoped ACP controls. Reads never start an agent or send a prompt.
use gateway_types::ApiError;
use gateway_instance::host::InstanceQuery;
use axum::{extract::Query, http::HeaderMap};
use crate::session_host::SessionHost;
use session_engine::agent_binding::AgentBinding;
use crate::acp_host::AcpHost;
use axum::{Json, extract::Path};
use vw_api_acp_model::acp_session::{AcpSessionOptions, AcpSessionSelection};
pub async fn get<H: AcpHost + SessionHost>(
    path: Path<String>,
    query: Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<AcpSessionOptions>, ApiError> {
    let session = crate::session_ops::ui_session_get::<H>(path, query, headers).await?.0;
    let mut options = H::read_options(&session.id, &session.directory)
        .await
        .map_err(ApiError::bad_request)?;
    if options.model_source_capabilities.is_empty()
        && let Some(binding) =
            H::load_binding(&session.project_id, &session.id)
                .await
                .map_err(|error| ApiError::bad_request(error.to_string()))?
        && let Some(agent) = binding.identity().acp_agent
    {
        H::annotate_model_source(&mut options, &agent);
    }
    Ok(Json(options))
}

pub async fn set<H: AcpHost + SessionHost>(
    path: Path<String>,
    query: Query<InstanceQuery>,
    headers: HeaderMap,
    Json(selection): Json<AcpSessionSelection>,
) -> Result<Json<AcpSessionOptions>, ApiError> {
    let session = crate::session_ops::ui_session_get::<H>(path, query, headers).await?.0;
    H::select_option(&session.id, &session.directory, selection)
        .await
        .map(Json)
        .map_err(ApiError::bad_request)
}

pub async fn set_source<H: AcpHost + SessionHost>(
    path: Path<String>,
    query: Query<InstanceQuery>,
    headers: HeaderMap,
    Json(update): Json<vw_api_acp_model::acp_model_source::AcpModelSourceUpdate>,
) -> Result<Json<AcpSessionOptions>, ApiError> {
    let session = crate::session_ops::ui_session_get::<H>(path, query, headers).await?.0;
    let _guard = session_engine::message::lock_stream_prompt(
        &H::current_tenant_scope(),
        &session.project_id,
        &session.id,
    )
    .await;
    let fork_state = H::fork_started(&session.project_id, &session.id)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    let fork_selectable = fork_state.as_ref().is_some_and(|state| !*state);
    if fork_state.is_some_and(|state| state)
        || (!fork_selectable && !session.is_first_response_pending())
    {
        return Err(ApiError::bad_request("changing a session model requires a new session"));
    }
    let previous_binding =
        H::load_binding(&session.project_id, &session.id)
            .await
            .map_err(|e| ApiError::bad_request(e.to_string()))?;
    if !fork_selectable
        && let Some(binding) = previous_binding.as_ref()
        && binding.identity().acp_agent.as_deref() != Some(update.agent.as_str())
    {
        return Err(ApiError::bad_request("ACP agent does not match the session binding"));
    }
    H::with_tenant_instance(
        H::current_tenant_scope(),
        session.directory.clone(),
        move || {
            Box::pin(async move {
                let preference = if fork_selectable {
                    let inherited = previous_binding.filter(|binding| {
                        binding.identity().acp_agent.as_deref() == Some(update.agent.as_str())
                    });
                    let agent = inherited
                        .as_ref()
                        .map(|binding| binding.agent.clone())
                        .unwrap_or_else(|| format!("acp-{}", update.agent));
                    let definition = H::agent(&agent).await.ok_or_else(|| {
                        ApiError::bad_request(format!("unknown or disabled agent: {agent}"))
                    })?;
                    Some(inherited.unwrap_or_else(|| {
                        AgentBinding::from_definition(
                            agent,
                            &definition,
                        )
                    }))
                } else {
                    None
                };
                let options = H::select_model_source(
                    &session.id,
                    &session.directory,
                    update,
                )
                .await
                .map_err(ApiError::bad_request)?;
                if let Some(preference) = preference {
                    H::save_binding(
                        &session.project_id,
                        &session.id,
                        &preference,
                    )
                    .await
                    .map_err(|e| ApiError::bad_request(e.to_string()))?;
                }
                Ok(Json(options))
            })
        },
    )
    .await
}

pub async fn source_capabilities<H: AcpHost + SessionHost>(Path(agent): Path<String>) -> Json<AcpSessionOptions> {
    let mut options = AcpSessionOptions::default();
    H::annotate_model_source(&mut options, &agent);
    Json(options)
}
