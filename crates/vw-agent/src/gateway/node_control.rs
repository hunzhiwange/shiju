//! 实验性节点控制协议端点，执行认证、节点准入和能力调用限制。
//!
//! 当前只实现节点枚举和描述；能力调用保持明确的未实现响应，不伪造执行结果。

use super::state::AppState;
use crate::app::agent::security::pairing::constant_time_eq;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};

#[derive(Debug, Clone, serde::Deserialize)]
/// 节点控制协议请求。
pub struct NodeControlRequest {
    /// 协议方法名。
    pub method: String,
    #[serde(default)]
    /// 可选目标节点标识。
    pub node_id: Option<String>,
    #[serde(default)]
    /// 可选目标能力名称。
    pub capability: Option<String>,
    #[serde(default)]
    /// 方法参数；未提供时为空 JSON 值。
    pub arguments: serde_json::Value,
}

/// 判断节点是否满足配置的允许列表；空列表和 `*` 均表示允许。
pub(crate) fn node_id_allowed(node_id: &str, allowed_node_ids: &[String]) -> bool {
    if allowed_node_ids.is_empty() {
        return true;
    }

    allowed_node_ids.iter().any(|candidate| candidate == "*" || candidate == node_id)
}

/// 处理实验性 `/api/node-control` 请求。
///
/// Supported methods:
/// - `node.list`
/// - `node.describe`
/// - `node.invoke` (stubbed as not implemented)
pub async fn handle_node_control(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<NodeControlRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Err(err) = super::api::auth::require_auth(&state, &headers) {
        return err.into_response();
    }

    let Json(request) = match body {
        Ok(body) => body,
        Err(e) => {
            tracing::warn!("Node-control JSON parse error: {e}");
            let err = serde_json::json!({
                "error": "Invalid JSON body for node-control request"
            });
            return (StatusCode::BAD_REQUEST, Json(err)).into_response();
        }
    };

    let node_control = { state.config.lock().gateway.node_control.clone() };
    if !node_control.enabled {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Node-control API is disabled"})),
        )
            .into_response();
    }

    // Optional second-factor shared token for node-control endpoints.
    if let Some(expected_token) =
        node_control.auth_token.as_deref().map(str::trim).filter(|value| !value.is_empty())
    {
        let provided_token = headers
            .get("X-Node-Control-Token")
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .unwrap_or("");
        if !constant_time_eq(expected_token, provided_token) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid X-Node-Control-Token"})),
            )
                .into_response();
        }
    }

    let method = request.method.trim();
    let response = match method {
        "node.list" => {
            let nodes = node_control
                .allowed_node_ids
                .iter()
                .map(|node_id| {
                    serde_json::json!({
                        "node_id": node_id,
                        "status": "unpaired",
                        "capabilities": []
                    })
                })
                .collect::<Vec<_>>();

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "method": "node.list",
                    "nodes": nodes
                })),
            )
        }
        "node.describe" => {
            let Some(node_id) =
                request.node_id.as_deref().map(str::trim).filter(|value| !value.is_empty())
            else {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "node_id is required for node.describe"})),
                )
                    .into_response();
            };
            if !node_id_allowed(node_id, &node_control.allowed_node_ids) {
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"error": "node_id is not allowed"})),
                )
                    .into_response();
            }

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "method": "node.describe",
                    "node_id": node_id,
                    "description": {
                        "status": "stub",
                        "capabilities": [],
                        "message": "Node descriptor scaffold is enabled; runtime backend is not wired yet."
                    }
                })),
            )
        }
        "node.invoke" => {
            let Some(node_id) =
                request.node_id.as_deref().map(str::trim).filter(|value| !value.is_empty())
            else {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "node_id is required for node.invoke"})),
                )
                    .into_response();
            };
            if !node_id_allowed(node_id, &node_control.allowed_node_ids) {
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"error": "node_id is not allowed"})),
                )
                    .into_response();
            }

            (
                StatusCode::NOT_IMPLEMENTED,
                Json(serde_json::json!({
                    "ok": false,
                    "method": "node.invoke",
                    "node_id": node_id,
                    "capability": request.capability,
                    "arguments": request.arguments,
                    "error": "node.invoke backend is not implemented yet in this scaffold"
                })),
            )
        }
        _ => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Unsupported method",
                "supported_methods": ["node.list", "node.describe", "node.invoke"]
            })),
        ),
    };
    response.into_response()
}

#[cfg(test)]
#[path = "node_control_tests.rs"]
mod node_control_tests;
