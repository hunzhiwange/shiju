//! 权限请求管理路由模块

use axum::Json;
use axum::Router;
use axum::extract::{Extension, Path, Query};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::app::agent::approval::{
    ApprovalResponse, PendingApprovalError, PendingNonCliApprovalRequest,
};
use crate::app::agent::config;
use crate::app::agent::config::schema::save_config;
use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::approval_state;
use crate::app::agent::gateway::instance::{InstanceQuery, resolve_directory, with_instance};
use crate::app::agent::permission::next as permission_next;

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/permission", get(permission_list))
        .route("/permission/{request_id}/reply", post(permission_reply))
}

async fn permission_list(
    Extension(tenant): Extension<vw_api_types::id::TenantScope>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<permission_next::Request>>, ApiError> {
    let dir = resolve_directory(&query, &headers);
    let rows = crate::session::ui_store::with_tenant_scope(
        tenant,
        with_instance(dir, move || {
            Box::pin(async move {
                let manager = approval_state::approval_manager_for_current_instance().await;
                Ok(manager
                    .list_non_cli_pending_requests(None, None, None)
                    .into_iter()
                    .map(permission_request_from_pending)
                    .collect::<Vec<_>>())
            })
        }),
    )
    .await?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
struct PermissionReplyRequest {
    reply: permission_next::Reply,
    message: Option<String>,
}

async fn permission_reply(
    Extension(tenant): Extension<vw_api_types::id::TenantScope>,
    Query(query): Query<InstanceQuery>,
    headers: HeaderMap,
    Path(request_id): Path<String>,
    Json(body): Json<PermissionReplyRequest>,
) -> Result<Json<bool>, ApiError> {
    let dir = resolve_directory(&query, &headers);
    let applied = crate::session::ui_store::with_tenant_scope(
        tenant,
        with_instance(dir, move || {
            Box::pin(async move {
                let manager = approval_state::approval_manager_for_current_instance().await;
                reply_pending_request(&manager, &request_id, body.reply, body.message).await
            })
        }),
    )
    .await?;

    if !applied {
        return Err(ApiError::not_found("request not found"));
    }
    Ok(Json(true))
}

fn permission_request_from_pending(
    request: PendingNonCliApprovalRequest,
) -> permission_next::Request {
    let mut metadata = Map::new();
    let acp = request.requested_channel == approval_state::ACP_APPROVAL_CHANNEL;
    if let Some(reason) = request.reason.as_ref().filter(|value| !value.trim().is_empty()) {
        metadata.insert("reason".to_string(), Value::String(reason.clone()));
        if acp {
            metadata.insert("title".to_string(), Value::String(reason.clone()));
        }
    }
    if !request.arguments.is_null() {
        metadata.insert("arguments".to_string(), request.arguments.clone());
    }
    metadata.insert("requested_by".to_string(), Value::String(request.requested_by.clone()));
    metadata
        .insert("requested_channel".to_string(), Value::String(request.requested_channel.clone()));
    metadata.insert(
        "requested_reply_target".to_string(),
        Value::String(request.requested_reply_target.clone()),
    );
    metadata.insert("created_at".to_string(), Value::String(request.created_at.clone()));
    if !acp {
        metadata.insert("expires_at".to_string(), Value::String(request.expires_at.clone()));
    }

    permission_next::Request {
        id: request.request_id,
        session_id: request.requested_reply_target,
        permission: request.tool_name.clone(),
        patterns: Vec::new(),
        metadata,
        always: if acp { Vec::new() } else { vec![request.tool_name] },
        tool: match (request.message_id, request.call_id) {
            (Some(message_id), Some(call_id))
                if !message_id.trim().is_empty() && !call_id.trim().is_empty() =>
            {
                Some(permission_next::ToolInfo { message_id, call_id })
            }
            _ => None,
        },
    }
}

async fn reply_pending_request(
    manager: &crate::app::agent::approval::ApprovalManager,
    request_id: &str,
    reply: permission_next::Reply,
    _message: Option<String>,
) -> Result<bool, ApiError> {
    let Some(request) = manager
        .list_non_cli_pending_requests(None, None, None)
        .into_iter()
        .find(|request| request.request_id == request_id)
    else {
        return Ok(false);
    };

    match reply {
        permission_next::Reply::Always
            if request.requested_channel == approval_state::ACP_APPROVAL_CHANNEL =>
        {
            return Err(ApiError::bad_request(
                "ACP permission requests allow one-time approval only",
            ));
        }
        permission_next::Reply::Once => {
            manager
                .reply_to_non_cli_pending_once(&request, true)
                .map_err(map_pending_approval_error)?;
        }
        permission_next::Reply::Always => {
            manager
                .confirm_non_cli_pending_request(
                    &request.request_id,
                    &request.requested_by,
                    &request.requested_channel,
                    &request.requested_reply_target,
                )
                .map_err(map_pending_approval_error)?;
            manager.grant_non_cli_session(&request.tool_name);
            manager.apply_persistent_runtime_grant(&request.tool_name);
            if let Err(error) = persist_non_cli_approval(&request.tool_name).await {
                tracing::warn!(
                    target: "vw_agent",
                    tool_name = %request.tool_name,
                    error = %error,
                    "failed to persist non-CLI approval from gateway permission reply"
                );
            }
            manager.record_non_cli_pending_resolution(&request.request_id, ApprovalResponse::Yes);
        }
        permission_next::Reply::Reject => {
            manager
                .reply_to_non_cli_pending_once(&request, false)
                .map_err(map_pending_approval_error)?;
        }
    }

    Ok(true)
}

async fn persist_non_cli_approval(tool_name: &str) -> Result<(), ApiError> {
    let mut current = config::get().await;
    let mut changed = false;

    if !current.autonomy.auto_approve.iter().any(|entry| entry == tool_name) {
        current.autonomy.auto_approve.push(tool_name.to_string());
        changed = true;
    }

    let before_always_ask = current.autonomy.always_ask.len();
    current.autonomy.always_ask.retain(|entry| entry != tool_name);
    if current.autonomy.always_ask.len() != before_always_ask {
        changed = true;
    }

    if !changed {
        return Ok(());
    }

    save_config(&current).await.map_err(|error| ApiError::internal(error.to_string()))
}

fn map_pending_approval_error(error: PendingApprovalError) -> ApiError {
    match error {
        PendingApprovalError::NotFound => ApiError::not_found("request not found"),
        PendingApprovalError::Expired => ApiError::bad_request("request expired"),
        PendingApprovalError::RequesterMismatch => ApiError::bad_request("request actor mismatch"),
    }
}

#[cfg(test)]
#[path = "permission_tests.rs"]
mod permission_tests;
