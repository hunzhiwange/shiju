//! 全局路由模块
//!
//! 该模块提供了 VibeWindow 代理运行时的全局级别 HTTP 路由端点。
//! 这些端点独立于特定项目，用于处理系统级的操作，包括：
//!
//! - 健康检查：用于监控系统运行状态
//! - 服务器发送事件（SSE）：用于实时推送系统事件
//! - 全局配置管理：获取和更新全局配置
//! - 实例销毁：清理所有项目实例

use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::extract::{Extension, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use vw_api_types::id::TenantScope;

use crate::app::agent::bus;
use crate::app::agent::config;
use crate::app::agent::gateway::{ApiError, AppState};
use crate::app::agent::health;
use crate::app::agent::installation;
use crate::app::agent::project;

async fn merged_global_acp_config() -> HashMap<String, config::schema::AcpAgentConfig> {
    let global_cfg = config::get_global().await;
    let overrides = global_cfg
        .acp
        .iter()
        .map(|(name, spec)| {
            (
                name.clone(),
                vw_acp::AgentCommandSpec {
                    display_name: name.trim().to_string(),
                    command: spec.command.trim().to_string(),
                    args: spec.args.clone(),
                    env: spec.env.clone(),
                },
            )
        })
        .collect::<HashMap<_, _>>();

    vw_acp::merge_agent_specs(Some(&overrides))
        .into_iter()
        .map(|(name, spec)| {
            (
                name,
                config::schema::AcpAgentConfig {
                    command: spec.command,
                    args: spec.args,
                    env: spec.env,
                },
            )
        })
        .collect()
}

pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/global/health", get(global_health))
        .route("/global/event", get(global_event_sse))
        .route("/global/config", get(global_config_get).patch(global_config_patch))
        .route("/global/config/acp", get(global_config_acp_get))
        .route("/global/dispose", post(global_dispose))
}

pub(crate) fn stateful_router() -> Router<AppState> {
    Router::new()
        .route("/global/health", get(global_health))
        .route("/global/event", get(global_event_sse))
        .route("/global/config", get(global_config_get).patch(global_config_patch_stateful))
        .route("/global/config/acp", get(global_config_acp_get))
        .route("/global/dispose", post(global_dispose))
}

async fn global_health() -> Json<Value> {
    Json(serde_json::json!({
        "status": "ok",
        "healthy": true,
        "version": installation::version(),
        "health": health::snapshot_json()
    }))
}

async fn global_event_sse() -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::channel::<Event>(256);
    let tx_sub = tx.clone();
    let unsub = bus::global_subscribe(move |evt| {
        let directory = evt.directory.unwrap_or_else(|| "global".to_string());
        let payload = serde_json::json!({ "directory": directory, "payload": evt.payload });
        let data = payload.to_string();
        let _ = tx_sub.try_send(Event::default().data(data));
    });

    let tx_task = tx.clone();
    tokio::spawn(async move {
        let connected = serde_json::json!({
            "directory": "global",
            "payload": { "type": "server.connected", "properties": {} }
        });
        let _ = tx_task.try_send(Event::default().data(serde_json::to_string(&connected).unwrap()));

        let mut interval = tokio::time::interval(Duration::from_secs(30));
        let mut stream_open = true;
        while stream_open {
            interval.tick().await;
            let hb = serde_json::json!({
                "directory": "global",
                "payload": { "type": "server.heartbeat", "properties": {} }
            });
            stream_open = tx_task
                .send(Event::default().data(serde_json::to_string(&hb).unwrap()))
                .await
                .is_ok();
        }
        unsub();
    });

    Sse::new(ReceiverStream::new(rx).map(Ok)).keep_alive(KeepAlive::new())
}

fn config_response(mut value: config::Config) -> Result<Json<Value>, ApiError> {
    config::encrypt_config_secrets(&mut value)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let value =
        serde_json::to_value(value).map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(Json(value))
}

async fn global_config_get(
    Extension(tenant): Extension<TenantScope>,
) -> Result<Json<Value>, ApiError> {
    let config = config::get_global_for_tenant(&tenant)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    config_response(config)
}

async fn global_config_acp_get() -> Json<Value> {
    Json(serde_json::to_value(merged_global_acp_config().await).unwrap_or(Value::Null))
}

async fn global_config_patch(
    Extension(tenant): Extension<TenantScope>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    config::update_global_for_tenant(&tenant, patch)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok(Json(Value::Null))
}

async fn global_config_patch_stateful(
    Extension(tenant): Extension<TenantScope>,
    State(state): State<AppState>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let response = global_config_patch(Extension(tenant.clone()), Json(patch)).await?;
    let next = config::get_global_for_tenant(&tenant)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    state.pairing.update_from_skeys(next.gateway.auth_enabled, &next.gateway.skeys);
    *state.config.lock() = next;
    Ok(response)
}

async fn global_dispose() -> Result<Json<bool>, ApiError> {
    project::instance::dispose_all().await;
    let _ = bus::publish(
        bus::define("global.disposed"),
        serde_json::json!({}),
        Some("global".to_string()),
    );
    Ok(Json(true))
}

#[cfg(test)]
#[path = "global_tests.rs"]
mod global_tests;
