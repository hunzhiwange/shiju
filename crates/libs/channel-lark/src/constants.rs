//! Lark/飞书通道常量定义模块
//!
//! 本模块集中定义了 Lark（飞书国际版）和飞书（国内版）集成所需的各类常量，
//! 包括 API 端点、WebSocket 连接参数、消息表情反应以及令牌管理相关配置。
//!
//! # 模块职责
//!
//! - **API 端点**：定义飞书和 Lark 的 REST API 及 WebSocket 基础 URL
//! - **表情反应**：按语言区域提供消息确认用的表情符号列表
//! - **连接超时**：WebSocket 心跳超时等时间参数
//! - **令牌管理**：访问令牌的刷新偏移、默认 TTL 及错误码
//!
//! # 使用场景
//!
//! 这些常量主要被 [`super::client`] 和 [`super::websocket`] 模块使用，
//! 用于配置 HTTP 客户端、建立长连接以及处理认证令牌的生命周期。

use std::time::Duration;

// ============================================================================
// API 端点常量
// ============================================================================

/// 飞书（国内版）REST API 基础 URL
///
/// 用于构建所有飞书 Open API 的请求路径。
/// 所有 API 请求都应以此 URL 为前缀。
///
/// # 示例
///
/// ```ignore
/// let url = format!("{}/auth/v3/tenant_access_token/internal", FEISHU_BASE_URL);
/// // 结果: "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal"
/// ```
pub(crate) const FEISHU_BASE_URL: &str = "https://open.feishu.cn/open-apis";

/// 飞书（国内版）WebSocket 基础 URL
///
/// 用于建立与飞书服务器的 WebSocket 长连接，
/// 接收实时消息推送和事件通知。
pub(crate) const FEISHU_WS_BASE_URL: &str = "https://open.feishu.cn";

/// Lark（飞书国际版）REST API 基础 URL
///
/// 用于构建所有 Lark Open API 的请求路径。
/// 海外用户和租户应使用此端点而非飞书国内版。
///
/// # 示例
///
/// ```ignore
/// let url = format!("{}/auth/v3/tenant_access_token/internal", LARK_BASE_URL);
/// // 结果: "https://open.larksuite.com/open-apis/auth/v3/tenant_access_token/internal"
/// ```
pub(crate) const LARK_BASE_URL: &str = "https://open.larksuite.com/open-apis";

/// Lark（飞书国际版）WebSocket 基础 URL
///
/// 用于建立与 Lark 服务器的 WebSocket 长连接，
/// 接收实时消息推送和事件通知。
pub(crate) const LARK_WS_BASE_URL: &str = "https://open.larksuite.com";

// ============================================================================
// 消息表情反应常量
// ============================================================================

/// 简体中文环境下的消息确认表情列表
///
/// 当收到用户消息后，机器人可使用这些表情之一进行"已收到"确认。
/// 这些表情是飞书/Lark 原生支持的表情类型标识符。
///
/// # 表情含义
///
/// - `OK`：确认/收到
/// - `JIAYI`：加油
/// - `APPLAUSE`：鼓掌
/// - `THUMBSUP`：点赞
/// - `MUSCLE`：强壮/给力
/// - `SMILE`：微笑
/// - `DONE`：完成
pub(crate) const LARK_ACK_REACTIONS_ZH_CN: &[&str] =
    &["OK", "JIAYI", "APPLAUSE", "THUMBSUP", "MUSCLE", "SMILE", "DONE"];

/// 繁体中文（台湾）环境下的消息确认表情列表
///
/// 与简体中文版本相比，使用 `FINGERHEART`（比心）替代 `MUSCLE`，
/// 以适应台湾地区的表情使用习惯。
pub(crate) const LARK_ACK_REACTIONS_ZH_TW: &[&str] =
    &["OK", "JIAYI", "APPLAUSE", "THUMBSUP", "FINGERHEART", "SMILE", "DONE"];

/// 英文环境下的消息确认表情列表
///
/// 为英语用户提供的表情集合，包含 `THANKS`（感谢）表情。
/// 与中文版本相比，表情选择更符合英语文化习惯。
pub(crate) const LARK_ACK_REACTIONS_EN: &[&str] =
    &["OK", "THUMBSUP", "THANKS", "MUSCLE", "FINGERHEART", "APPLAUSE", "SMILE", "DONE"];

/// 日文环境下的消息确认表情列表
///
/// 为日本用户提供的表情集合，与英文版本保持一致，
/// 便于多语言环境的统一管理。
pub(crate) const LARK_ACK_REACTIONS_JA: &[&str] =
    &["OK", "THUMBSUP", "THANKS", "MUSCLE", "FINGERHEART", "APPLAUSE", "SMILE", "DONE"];

// ============================================================================
// WebSocket 连接超时常量
// ============================================================================

/// WebSocket 心跳超时时间
///
/// 如果在此时间窗口内未收到任何二进制帧（pong 响应或事件推送），
/// 则认为连接已断开，需要触发重连。
///
/// # 设计说明
///
/// - 该值必须大于飞书/Lark 的 ping 间隔（默认 120 秒）
/// - 设置为 300 秒（5 分钟）以容忍合理的网络延迟和服务器负载波动
/// - 过短会导致频繁误判断线，过长会延迟故障检测
///
/// # 相关配置
///
/// 飞书/Lark 官方建议的 ping 间隔为 120 秒，本常量提供了 180 秒的冗余缓冲。
pub(crate) const WS_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(300);

// ============================================================================
// 令牌管理常量
// ============================================================================

/// 租户访问令牌的提前刷新偏移时间
///
/// 在令牌声明的过期时间之前此偏移量时，主动刷新令牌，
/// 避免在 API 调用过程中令牌过期导致请求失败。
///
/// # 设计说明
///
/// - 设置为 120 秒（2 分钟），为网络延迟和刷新请求预留足够时间
/// - 配合令牌的 `expires_in` 字段计算实际刷新时机
///
/// # 示例
///
/// ```ignore
/// let refresh_at = token_expire_time - LARK_TOKEN_REFRESH_SKEW;
/// if Instant::now() >= refresh_at {
///     // 执行令牌刷新
/// }
/// ```
pub(crate) const LARK_TOKEN_REFRESH_SKEW: Duration = Duration::from_secs(120);

/// 租户访问令牌的默认有效期
///
/// 当 API 响应中未包含 `expire` 或 `expires_in` 字段时，
/// 使用此值作为令牌的默认有效期。
///
/// # 设计说明
///
/// - 7200 秒 = 2 小时，这是飞书/Lark 令牌的典型有效期
/// - 该值应小于实际有效期以确保安全边界
pub(crate) const LARK_DEFAULT_TOKEN_TTL: Duration = Duration::from_secs(7200);

/// 无效/过期访问令牌的 API 业务错误码
///
/// 当租户访问令牌无效或已过期时，飞书/Lark API 会返回此错误码。
/// 收到此错误码时应立即刷新令牌并重试请求。
///
/// # 错误处理流程
///
/// 1. 检测到响应包含此错误码
/// 2. 使用 `app_id` 和 `app_secret` 重新获取令牌
/// 3. 更新本地缓存的令牌
/// 4. 使用新令牌重试原始请求
///
/// # 参考文档
///
/// 飞书开放平台错误码文档中对此错误码有详细说明。
pub(crate) const LARK_INVALID_ACCESS_TOKEN_CODE: i64 = 99_991_663;

// ============================================================================
// 消息处理常量
// ============================================================================

/// 图片消息下载失败时的回退文本
///
/// 当收到图片消息但无法成功下载图片内容时，
/// 使用此文本作为占位符告知用户。
///
/// # 使用场景
///
/// - 图片下载超时
/// - 图片 URL 无效或已过期
/// - 网络连接异常
/// - 存储空间不足
pub(crate) const LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT: &str =
    "[Image message received but could not be downloaded]";

#[cfg(test)]
#[path = "constants_tests.rs"]
mod constants_tests;
