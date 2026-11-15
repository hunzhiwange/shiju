//! 全局路由模块
//!
//! 该模块提供了 VibeWindow 代理运行时的全局级别 HTTP 路由端点。
//! 这些端点独立于特定项目，用于处理系统级的操作，包括：
//!
//! - 健康检查：用于监控系统运行状态
//! - 服务器发送事件（SSE）：用于实时推送系统事件
//! - 全局配置管理：获取和更新全局配置
//! - 实例销毁：清理所有项目实例

#![cfg(not(target_arch = "wasm32"))]

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
use vw_api_transport::id::TenantScope;

use agent_bus as bus;
use agent_config::runtime as config;
use gateway_types::{ApiError, AppState};
use agent_health as health;
use agent_installation as installation;
use agent_project as project;

async fn merged_global_acp_config() -> HashMap<String, config::AcpAgentConfig> {
    let global_cfg = config::get_global().await;
    let overrides = global_cfg
        .acp_saved
        .iter()
        .chain(global_cfg.acp.iter())
        .map(|(name, spec)| {
            (
                name.clone(),
                acp_agent_registry::AgentCommandSpec {
                    display_name: name.trim().to_string(),
                    command: spec.command.trim().to_string(),
                    args: spec.args.clone(),
                    env: spec.env.clone(),
                },
            )
        })
        .collect::<HashMap<_, _>>();

    acp_agent_registry::merge_agent_specs(Some(&overrides))
        .into_iter()
        .map(|(name, spec)| {
            (
                name,
                config::AcpAgentConfig {
                    command: spec.command,
                    args: spec.args,
                    env: spec.env,
                },
            )
        })
        .collect()
}

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/global/health", get(global_health))
        .route("/global/event", get(global_event_sse))
        .route("/global/config", get(global_config_get).patch(global_config_patch))
        .route("/global/config/acp", get(global_config_acp_get))
        .route("/global/config/acp/defaults", get(global_config_acp_defaults_get))
        .route("/global/config/acp/installed", get(global_acp_installed_get))
        .route("/global/dispose", post(global_dispose))
}

pub fn stateful_router<E, C, H>() -> Router<AppState<E, C, H>>
where
    E: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    H: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/global/health", get(global_health))
        .route("/global/event", get(global_event_sse))
        .route("/global/config", get(global_config_get).patch(global_config_patch_stateful::<E, C, H>))
        .route("/global/config/acp", get(global_config_acp_get))
        .route("/global/config/acp/defaults", get(global_config_acp_defaults_get))
        .route("/global/config/acp/installed", get(global_acp_installed_get))
        .route("/global/dispose", post(global_dispose))
}

pub async fn global_health() -> Json<Value> {
    Json(serde_json::json!({
        "status": "ok",
        "healthy": true,
        "version": installation::version(),
        "health": health::snapshot_json()
    }))
}

pub async fn global_event_sse() -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
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

pub fn config_response(mut value: config::Config) -> Result<Json<Value>, ApiError> {
    config::encrypt_config_secrets(&mut value)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let value =
        serde_json::to_value(value).map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(Json(value))
}

pub async fn global_config_get(
    Extension(tenant): Extension<TenantScope>,
) -> Result<Json<Value>, ApiError> {
    let config = config::get_global_for_tenant(&tenant)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    config_response(config)
}

async fn global_config_acp_defaults_get() -> Json<Value> {
    Json(serde_json::to_value(acp_agent_registry::built_in_agent_specs()).expect("serializable ACP defaults"))
}

pub async fn global_config_acp_get() -> Json<Value> {
    Json(serde_json::to_value(merged_global_acp_config().await).unwrap_or(Value::Null))
}

pub async fn global_config_patch(
    Extension(tenant): Extension<TenantScope>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    config::update_global_for_tenant(&tenant, patch)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok(Json(Value::Null))
}

static REFRESH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub async fn global_config_patch_stateful<E, C, H>(
    Extension(tenant): Extension<TenantScope>,
    State(state): State<AppState<E, C, H>>,
    Json(patch): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let response = global_config_patch(Extension(tenant.clone()), Json(patch)).await?;
    // Keep a slower reload from publishing an older snapshot after a newer one.
    // Persistence remains coordinated by the database across gateway processes.
    let _refresh = REFRESH_LOCK.lock().await;
    let next = config::get_global_for_tenant(&tenant)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    state.pairing.update_from_skeys(next.gateway.auth_enabled, &next.gateway.skeys);
    *state.config.lock() = next;
    Ok(response)
}

pub async fn global_dispose() -> Result<Json<bool>, ApiError> {
    project::instance::dispose_all().await;
    let _ = bus::publish(
        bus::define("global.disposed"),
        serde_json::json!({}),
        Some("global".to_string()),
    );
    Ok(Json(true))
}

/// Check launch executables on the gateway; never starts or installs a CLI.
async fn global_acp_installed_get() -> Json<std::collections::BTreeMap<String, bool>> {
    Json(
        merged_global_acp_config()
            .await
            .into_iter()
            .map(|(key, config)| {
                let installed = acp_command_installed(&config.command);
                (key, installed)
            })
            .collect(),
    )
}

pub fn acp_command_installed(command: &str) -> bool {
    // npx is a downloader, not evidence that its ACP package is installed.
    let name = command.trim().rsplit(['/', '\\']).next().unwrap_or_default().to_ascii_lowercase();
    if matches!(name.as_str(), "npx" | "npx.cmd" | "npx.exe" | "npx.bat" | "uvx" | "uvx.exe") {
        return false;
    }
    shell_command::resolve_executable(command).is_some()
}
