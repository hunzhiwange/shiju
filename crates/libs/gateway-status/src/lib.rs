//! # 状态与健康检查 API 处理器
//!
//! 本模块提供网关的状态查询和健康检查相关 HTTP 端点。
//!
//! ## 主要功能
//!
//! - **状态查询**：返回代理的完整运行状态，包括：
//!   - 当前使用的 Provider 和模型配置
//!   - 系统运行时长（uptime）
//!   - skey 鉴权状态和通道配置
//!   - 内存后端信息
//!   - 健康检查快照
//!
//! - **健康检查**：返回轻量级的系统健康状态快照
//!
//! ## 安全性
//!
//! 所有端点均需要通过身份验证（`require_auth`）才能访问。

#![cfg(not(target_arch = "wasm32"))]

use agent_config::runtime_schema::ChannelsConfigExt;
use gateway_types::AppState;

/// 复用宿主现有认证边界，保留原错误响应。
pub type RequireAuth<S, C, H> = fn(
    &AppState<S, C, H>,
    &HeaderMap,
) -> Result<(), (axum::http::StatusCode, Json<serde_json::Value>)>;
use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Json},
};

/// 处理 API 状态查询请求
///
/// 返回代理的完整运行状态信息，包括配置参数、通道状态和健康指标。
///
/// # 请求
///
/// - 需要在请求头中提供有效的身份验证凭据
///
/// # 返回
///
/// 返回 JSON 格式的状态对象，包含以下字段：
/// - `provider`: 当前默认的 Provider 名称
/// - `model`: 当前使用的模型名称
/// - `temperature`: 模型温度参数
/// - `uptime_seconds`: 系统运行时长（秒）
/// - `gateway_port`: 网关监听端口
/// - `locale`: 当前语言区域设置
/// - `memory_backend`: 内存存储后端名称
/// - `auth_enabled`: 是否启用 skey 鉴权
/// - `active_skeys`: 当前有效 skey 数量
/// - `channels`: 各通道的启用状态映射
/// - `health`: 系统健康状态快照
///
/// # 身份验证
///
/// 如果身份验证失败，将返回 401 Unauthorized 响应。
///
/// # 示例
///
/// ```text
/// GET /api/status
/// Authorization: Bearer <skey>
/// ```
pub async fn handle_api_status<S, C, H>(
    State(state): State<AppState<S, C, H>>,
    headers: HeaderMap,
    require_auth: RequireAuth<S, C, H>,
) -> impl IntoResponse {
    // 验证请求身份，失败则立即返回错误响应
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    // 获取配置的克隆副本，避免长时间持有锁
    let config = state.config.lock().clone();

    // 获取系统健康状态快照
    let health = agent_health::snapshot();

    // 构建通道状态映射表
    let mut channels = serde_json::Map::new();

    // 遍历所有配置的通道，记录其是否已启用
    for (channel, present) in config.channels_config.channels() {
        channels.insert(channel.name().to_string(), serde_json::Value::Bool(present));
    }

    // 构建完整的响应体
    let body = serde_json::json!({
        "provider": config.default_provider,
        "model": state.model,
        "temperature": state.temperature,
        "uptime_seconds": health.uptime_seconds,
        "gateway_port": config.gateway.port,
        "locale": "en",
        "memory_backend": state.mem.name(),
        "auth_enabled": state.pairing.auth_enabled(),
        "active_skeys": state.pairing.active_skey_count(),
        "channels": channels,
        "health": health,
    });

    Json(body).into_response()
}

/// 处理 API 健康检查请求
///
/// 返回轻量级的系统健康状态快照，适合负载均衡器和监控系统定期轮询。
///
/// # 请求
///
/// - 需要在请求头中提供有效的身份验证凭据
///
/// # 返回
///
/// 返回 JSON 格式的健康状态对象：
/// - `health`: 系统健康状态快照
///
/// # 身份验证
///
/// 如果身份验证失败，将返回 401 Unauthorized 响应。
///
/// # 示例
///
/// ```text
/// GET /api/health
/// Authorization: Bearer <skey>
/// ```
pub async fn handle_api_health<S, C, H>(
    State(state): State<AppState<S, C, H>>,
    headers: HeaderMap,
    require_auth: RequireAuth<S, C, H>,
) -> impl IntoResponse {
    // 验证请求身份，失败则立即返回错误响应
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    // 获取并返回系统健康状态快照
    let snapshot = agent_health::snapshot();
    Json(serde_json::json!({"health": snapshot})).into_response()
}
