//! # 配对端点处理模块
//!
//! 本模块提供网关的设备配对功能，允许新客户端通过一次性配对码获取
//! 长期有效的 Bearer 认证令牌。
//!
//! ## 核心功能
//!
//! - **配对验证**：验证客户端提交的一次性配对码
//! - **令牌颁发**：成功配对后颁发持久化 Bearer 令牌
//! - **速率限制**：防止配对端点被滥用
//! - **令牌持久化**：将配对令牌保存到配置文件
//!
//! ## 安全考虑
//!
//! - 配对码具有时效性和一次性特性
//! - 实施严格的速率限制防止暴力破解
//! - 失败次数过多会触发锁定机制
//! - 令牌持久化失败不影响当前会话使用

use super::RATE_LIMIT_WINDOW_SECS;
use super::state::AppState;
use crate::app::agent::config::Config;
use crate::app::agent::config::schema::save_config;
use crate::app::agent::security::pairing::PairingGuard;
use anyhow::Context;
use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use parking_lot::Mutex;
use std::net::SocketAddr;
use std::sync::Arc;

/// 处理配对请求的 HTTP 端点处理器
///
/// 该函数处理 `POST /pair` 端点的请求，用于将一次性配对码交换为
/// 持久化的 Bearer 认证令牌。这是新客户端首次接入系统的标准流程。
///
/// # 请求格式
///
/// 客户端需要在请求头中包含：
/// - `X-Pairing-Code`: 系统生成的一次性配对码
///
/// # 响应格式
///
/// ## 成功响应 (200 OK)
/// ```json
/// {
///     "paired": true,
///     "persisted": true,
///     "token": "bearer_token_here",
///     "message": "保存此令牌 — 使用方式：Authorization: Bearer <token>"
/// }
/// ```
///
/// ## 配对成功但持久化失败 (200 OK)
/// ```json
/// {
///     "paired": true,
///     "persisted": false,
///     "token": "bearer_token_here",
///     "message": "配对成功但令牌持久化失败..."
/// }
/// ```
///
/// ## 无效配对码 (403 Forbidden)
/// ```json
/// {
///     "error": "Invalid pairing code"
/// }
/// ```
///
/// ## 速率限制 (429 Too Many Requests)
/// ```json
/// {
///     "error": "Too many pairing requests. Please retry later.",
///     "retry_after": 60
/// }
/// ```
///
/// ## 锁定状态 (429 Too Many Requests)
/// ```json
/// {
///     "error": "Too many failed attempts. Try again in 120s.",
///     "retry_after": 120
/// }
/// }
/// ```
///
/// # 参数
///
/// - `state`: 应用共享状态，包含配对守卫、速率限制器和配置
/// - `peer_addr`: 客户端的套接字地址，用于速率限制识别
/// - `headers`: HTTP 请求头，用于提取配对码和客户端标识
///
/// # 返回
///
/// 返回实现 `IntoResponse` trait 的响应对象，包含状态码和 JSON 响应体
///
/// # 示例
///
/// ```bash
/// curl -X POST http://localhost:8080/pair \
///   -H "X-Pairing-Code: ABC123XYZ"
/// ```
///
/// # 安全机制
///
/// 1. **速率限制**：同一客户端在时间窗口内只能发起有限次数的配对请求
/// 2. **锁定机制**：连续失败多次后，客户端会被临时锁定
/// 3. **一次性配对码**：配对码使用后即失效
#[axum::debug_handler]
pub async fn handle_pair(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // 步骤 1: 生成客户端标识键
    // 根据客户端 IP 和可能的代理头信息生成唯一标识符
    // 用于后续的速率限制和锁定检查
    let rate_key = super::util::client_key_from_request(
        Some(peer_addr),
        &headers,
        state.trust_forwarded_headers,
    );

    // 步骤 2: 检查速率限制
    // 防止单个客户端过于频繁地尝试配对
    if !state.rate_limiter.allow_pair(&rate_key) {
        tracing::warn!("/pair rate limit exceeded");
        let err = serde_json::json!({
            "error": "Too many pairing requests. Please retry later.",
            "retry_after": RATE_LIMIT_WINDOW_SECS,
        });
        return (StatusCode::TOO_MANY_REQUESTS, Json(err));
    }

    // 步骤 3: 提取配对码
    // 从请求头中获取 X-Pairing-Code，如果不存在则使用空字符串
    // 空字符串必然无法匹配有效的配对码
    let code = headers.get("X-Pairing-Code").and_then(|v| v.to_str().ok()).unwrap_or("");

    // 步骤 4: 尝试配对
    // try_pair 会验证配对码的有效性，并检查是否处于锁定状态
    match state.pairing.try_pair(code, &rate_key).await {
        // 情况 A: 配对成功，获得了令牌
        Ok(Some(token)) => {
            tracing::info!("🔐 New client paired successfully");

            // 步骤 4a: 尝试持久化令牌到配置文件
            // 即使持久化失败，当前会话仍可使用该令牌
            if let Err(err) = persist_pairing_tokens(state.config.clone(), &state.pairing).await {
                tracing::error!("🔐 Pairing succeeded but token persistence failed: {err:#}");
                let body = serde_json::json!({
                    "paired": true,
                    "persisted": false,
                    "token": token,
                    "message": "Paired for this process, but failed to persist token to the system config database. Check database access permissions.",
                });
                return (StatusCode::OK, Json(body));
            }

            // 步骤 4b: 配对成功且令牌已持久化
            let body = serde_json::json!({
                "paired": true,
                "persisted": true,
                "token": token,
                "message": "Save this token — use it as Authorization: Bearer <token>"
            });
            (StatusCode::OK, Json(body))
        }
        // 情况 B: 配对码无效（不存在或已使用）
        Ok(None) => {
            tracing::warn!("🔐 Pairing attempt with invalid code");
            let err = serde_json::json!({"error": "Invalid pairing code"});
            (StatusCode::FORBIDDEN, Json(err))
        }
        // 情况 C: 客户端因多次失败被锁定
        // 返回的 u64 值表示锁定剩余秒数
        Err(lockout_secs) => {
            tracing::warn!(
                "🔐 Pairing locked out — too many failed attempts ({lockout_secs}s remaining)"
            );
            let err = serde_json::json!({
                "error": format!("Too many failed attempts. Try again in {lockout_secs}s."),
                "retry_after": lockout_secs
            });
            (StatusCode::TOO_MANY_REQUESTS, Json(err))
        }
    }
}

/// 返回当前 loopback 客户端可用的配对引导信息。
///
/// 仅允许本地回环地址访问，用于 Desktop 在无 bearer token 时完成首次自动配对。
pub async fn handle_pair_code(
    State(state): State<AppState>,
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    if !peer_addr.ip().is_loopback() {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "pairing bootstrap is only available from loopback clients"
            })),
        )
            .into_response();
    }

    Json(serde_json::json!({
        "require_pairing": state.pairing.require_pairing(),
        "paired": state.pairing.is_paired(),
        "pairing_code": if state.pairing.require_pairing() {
            state.pairing.ensure_pairing_code()
        } else {
            None
        },
    }))
    .into_response()
}

/// 将配对令牌持久化到配置文件
///
/// 该函数负责将当前内存中的配对令牌列表保存到磁盘配置文件中，
/// 确保在应用重启后客户端仍然可以使用已配对的令牌进行认证。
///
/// # 参数
///
/// - `config`: 共享的应用配置对象（使用 `Arc<Mutex>` 包装以保证线程安全）
/// - `pairing`: 配对守卫引用，从中获取当前的配对令牌列表
///
/// # 返回
///
/// - `Ok(())`: 令牌成功持久化
/// - `Err(anyhow::Error)`: 持久化失败，可能原因包括：
///   - 配置文件路径不可写
///   - 磁盘空间不足
///   - 文件系统权限问题
///
/// # 实现细节
///
/// 该函数使用"读取-修改-写入"模式：
/// 1. 从配对守卫获取当前令牌列表
/// 2. 克隆配置对象（避免长时间持有锁）
/// 3. 更新克隆配置中的令牌字段
/// 4. 将更新后的配置保存到磁盘
/// 5. 用新配置替换内存中的共享配置
///
/// # 线程安全
///
/// 由于 `parking_lot::Mutex` 的 guard 不是 `Send`，我们需要克隆配置对象
/// 而不是直接持有锁。这是临时的兼容方案，未来应迁移到异步互斥锁。
///
/// # 错误处理
///
/// - 使用 `context()` 添加详细的错误上下文信息
/// - 持久化失败不会阻止当前会话使用令牌
/// - 失败信息会通过日志记录，便于故障排查
///
/// # 示例
///
/// ```ignore
/// let config = Arc::new(Mutex::new(Config::load().await?));
/// let pairing = PairingGuard::new();
///
/// // 在配对成功后调用
/// persist_pairing_tokens(config, &pairing).await?;
/// ```
pub async fn persist_pairing_tokens(
    config: Arc<Mutex<Config>>,
    pairing: &PairingGuard,
) -> anyhow::Result<()> {
    // 从配对守卫获取当前的配对令牌列表
    let paired_tokens = pairing.tokens();

    // 克隆配置对象以避免长时间持有锁
    // 注意：parking_lot 的 MutexGuard 不是 Send，因此需要在同步块内完成克隆
    // 这是一个临时的兼容方案，理想情况下应该使用异步互斥锁
    let mut updated_cfg = { config.lock().clone() };

    // 更新配置中的配对令牌字段
    updated_cfg.gateway.paired_tokens = paired_tokens;

    // 将更新后的配置保存到磁盘
    // 使用 context() 添加错误上下文，便于故障排查
    save_config(&updated_cfg)
        .await
        .context("Failed to persist paired tokens to the system config database")?;

    // 更新内存中的共享配置，保持运行时状态与持久化状态一致
    // 这确保后续的配置读取能够获取到最新的配对令牌列表
    *config.lock() = updated_cfg;

    Ok(())
}

#[cfg(test)]
#[path = "pairing_tests.rs"]
mod pairing_tests;
