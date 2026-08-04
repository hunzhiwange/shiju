//! 健康检查与指标暴露模块
//!
//! 本模块提供网关的健康检查和 Prometheus 指标暴露端点，用于系统监控和运维管理。
//!
//! # 主要功能
//!
//! - **健康检查端点** (`/v1/health`): 返回服务运行状态、skey 鉴权状态和运行时健康快照
//! - **指标端点** (`/metrics`): 暴露 Prometheus 格式的系统指标数据
//!
//! # 安全设计
//!
//! - 健康检查端点始终公开，不泄露任何敏感信息
//! - 指标端点根据 skey 鉴权策略和客户端地址进行访问控制：
//!   - skey 鉴权启用时：需要有效的 `Authorization: Bearer <skey>`
//!   - skey 鉴权禁用时：仅允许本地回环地址访问
//!
//! # 使用示例
//!
//! ```bash
//! # 健康检查
//! curl http://localhost:8080/v1/health
//!
//! # 获取指标（skey 鉴权）
//! curl -H "Authorization: Bearer <skey>" http://localhost:8080/metrics
//!
//! # 获取指标（未启用 skey 鉴权，仅本地）
//! curl http://127.0.0.1:8080/metrics
//! ```

use super::state::AppState;
use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json, Response},
};
use std::net::SocketAddr;

/// 处理健康检查请求
///
/// 返回服务的健康状态信息，包括 skey 鉴权状态和运行时快照。
/// 此端点始终公开访问，不会泄露任何敏感信息。
///
/// # 端点
///
/// `GET /v1/health`
///
/// # 参数
///
/// - `state`: 应用状态共享引用，包含 skey 鉴权状态等全局信息
///
/// # 返回值
///
/// 返回 JSON 格式的健康状态对象，包含以下字段：
/// - `status`: 服务状态，固定为 `"ok"`
/// - `build_id`: 进程启动时记录的可执行文件 SHA-256
/// - `auth_enabled`: 是否启用 skey 鉴权
/// - `active_skeys`: 当前有效 skey 数量
/// - `runtime`: 运行时健康快照（内存、线程数等）
///
/// # 示例响应
///
/// ```json
/// {
///     "status": "ok",
///     "build_id": "9f86d081884c...",
///     "auth_enabled": true,
///     "active_skeys": 1,
///     "runtime": {
///         "memory_mb": 128,
///         "threads": 4
///     }
/// }
/// ```
pub async fn handle_health(State(state): State<AppState>) -> impl IntoResponse {
    let body = serde_json::json!({
        "status": "ok",
        "build_id": crate::app::agent::health::runtime_build_id(),
        "auth_enabled": state.pairing.auth_enabled(),
        "active_skeys": state.pairing.active_skey_count(),
        "runtime": crate::app::agent::health::snapshot_json(),
    });
    Json(body)
}

/// Prometheus 文本格式的 MIME 内容类型
///
/// 用于 Prometheus 指标响应的 Content-Type 头部值。
/// 遵循 Prometheus 文本格式规范版本 0.0.4。
pub(crate) const PROMETHEUS_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// 处理 Prometheus 指标请求
///
/// 返回 Prometheus 文本格式的时间序列指标数据，用于监控系统集成。
/// 根据 skey 鉴权策略和客户端地址实施严格的访问控制。
///
/// # 端点
///
/// `GET /metrics`
///
/// # 参数
///
/// - `state`: 应用状态共享引用，包含 skey 鉴权状态和观测器
/// - `peer_addr`: 客户端套接字地址，用于访问控制检查
/// - `headers`: HTTP 请求头，用于提取认证令牌
///
/// # 返回值
///
/// 返回 Prometheus 文本格式的指标数据，或者以下错误响应之一：
/// - `401 Unauthorized`: skey 鉴权失败，需要提供有效的 `Authorization: Bearer <skey>`
/// - `403 Forbidden`: 未启用 skey 鉴权时非本地客户端尝试访问
///
/// # 访问控制策略
///
/// 1. **skey 鉴权启用** (`auth_enabled = true`)
///    - 要求请求头包含 `Authorization: Bearer <skey>`
///    - skey 必须通过服务端哈希验证
///
/// 2. **skey 鉴权禁用** (`auth_enabled = false`)
///    - 仅允许来自本地回环地址（127.0.0.1 / ::1）的请求
///    - 拒绝所有远程客户端访问，防止指标泄露
///
/// # 示例
///
/// ```bash
/// # skey 鉴权下访问
/// curl -H "Authorization: Bearer my-skey" http://localhost:8080/metrics
///
/// # 未启用 skey 鉴权时本地访问
/// curl http://127.0.0.1:8080/metrics
/// ```
pub async fn handle_metrics(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    // skey 鉴权开启时检查 Authorization Bearer。
    if state.pairing.auth_enabled() {
        let skey = super::api::auth::extract_auth_skey(&headers).unwrap_or("");
        if !state.pairing.is_authenticated(skey) {
            return (
                StatusCode::UNAUTHORIZED,
                [(header::CONTENT_TYPE, PROMETHEUS_CONTENT_TYPE)],
                String::from("# unauthorized: provide Authorization: Bearer <skey> for /metrics\n"),
            )
                .into_response();
        }
    } else if !peer_addr.ip().is_loopback() {
        // 未启用 skey 鉴权时拒绝远程客户端访问。
        // 仅允许本地回环地址访问，防止指标数据泄露到外部
        return (
            StatusCode::FORBIDDEN,
            [(header::CONTENT_TYPE, PROMETHEUS_CONTENT_TYPE)],
            String::from(
                "# metrics disabled for non-loopback clients when skey auth is disabled\n",
            ),
        )
            .into_response();
    }

    // 尝试从观测器中获取 Prometheus 后端并编码指标
    let body = if let Some(prom) = state
        .observer
        .as_ref()
        .as_any()
        .downcast_ref::<crate::app::agent::observability::PrometheusObserver>(
    ) {
        // Prometheus 后端已启用，编码当前指标数据
        prom.encode()
    } else {
        // Prometheus 后端未启用，返回配置提示
        String::from(
            "# Prometheus backend not enabled. Set [observability] backend = \"prometheus\" in config.\n",
        )
    };

    (StatusCode::OK, [(header::CONTENT_TYPE, PROMETHEUS_CONTENT_TYPE)], body).into_response()
}

#[cfg(test)]
#[path = "health_tests.rs"]
mod health_tests;
