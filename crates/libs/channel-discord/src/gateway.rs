//! Discord Gateway WebSocket 连接管理模块
//!
//! 本模块负责与 Discord Gateway 建立 WebSocket 连接，处理握手协议，
//! 并提供基础的消息收发能力。主要功能包括：
//!
//! - 获取 Discord Gateway 连接 URL
//! - 建立 WebSocket 连接
//! - 处理 Gateway Hello 消息（获取心跳间隔）
//! - 发送 Identify 消息进行身份认证
//!
//! # 架构说明
//!
//! Discord Gateway 使用 WebSocket 协议进行实时通信。连接建立后，
//! 客户端需要按照 Discord Gateway 协议进行握手和心跳维护。
//! 本模块提供了这些基础能力，供上层 Channel 实现使用。

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

/// Discord Gateway WebSocket 流类型别名
///
/// 这是一个完整的 WebSocket 流类型，封装了可能的 TLS 加密层和底层 TCP 流。
/// 用于与 Discord Gateway 进行实时双向通信。
type GatewayWs =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Gateway WebSocket 连接封装
///
/// 持有一个已建立的 Discord Gateway WebSocket 连接，将其拆分为独立的
/// 写入端和读取端，允许并行的消息收发操作。
///
/// # 字段说明
///
/// - `write`: WebSocket 写入端，用于发送消息到 Discord Gateway
/// - `read`: WebSocket 读取端，用于接收来自 Discord Gateway 的消息
pub(super) struct GatewayConnection {
    /// WebSocket 写入端（Sink），用于向 Gateway 发送消息
    pub(super) write: GatewayWs,
    /// WebSocket 读取端（Stream），用于从 Gateway 接收消息
    pub(super) read: SplitStream<GatewayWs>,
}

/// 从 Discord API 获取 Gateway WebSocket 连接 URL
///
/// 调用 Discord REST API 获取推荐 Gateway 连接地址。
/// 该 API 会返回当前最优的 Gateway 端点，客户端应使用此地址建立连接。
///
/// # 参数
///
/// - `client`: HTTP 客户端，用于发送 REST API 请求
/// - `bot_token`: Discord Bot 令牌，用于 API 认证
///
/// # 返回值
///
/// 成功时返回 Gateway WebSocket URL 字符串。
/// 如果 API 调用失败或响应中缺少 URL 字段，将返回错误或使用默认值。
///
/// # 错误
///
/// 可能因网络问题、认证失败或响应解析错误而失败。
///
/// # 示例
///
/// ```ignore
/// let client = reqwest::Client::new();
/// let url = fetch_gateway_url(&client, "your_bot_token").await?;
/// println!("Gateway URL: {}", url);
/// ```
pub(super) async fn fetch_gateway_url(
    client: &reqwest::Client,
    bot_token: &str,
    api_base_url: &str,
) -> anyhow::Result<String> {
    // 向 Discord API 请求 Gateway 信息
    let gw_resp: serde_json::Value = client
        .get(format!("{}/gateway/bot", api_base_url.trim_end_matches('/')))
        .header("Authorization", format!("Bot {bot_token}"))
        .send()
        .await?
        .json()
        .await?;

    // 从响应中提取 URL，如果不存在则使用 Discord 官方默认 Gateway 地址
    // 默认值 "wss://gateway.discord.gg" 是 Discord 的主 Gateway 端点
    Ok(gateway_url_from_response(&gw_resp))
}

fn gateway_url_from_response(gw_resp: &serde_json::Value) -> String {
    gw_resp.get("url").and_then(|u| u.as_str()).unwrap_or("wss://gateway.discord.gg").to_string()
}

/// 建立 Discord Gateway WebSocket 连接
///
/// 使用提供的 URL 建立 WebSocket 连接，并将连接流拆分为独立的
/// 写入端（Sink）和读取端（Stream），以便进行并行的消息收发。
///
/// # 参数
///
/// - `url`: Gateway WebSocket 连接 URL（通常从 `fetch_gateway_url` 获取）
///
/// # 返回值
///
/// 成功时返回一个元组，包含：
/// - 写入端 `SplitSink`：用于发送消息
/// - 读取端 `SplitStream`：用于接收消息
///
/// # 错误
///
/// 可能因网络问题、TLS 握手失败或 WebSocket 协议错误而失败。
///
/// # 示例
///
/// ```ignore
/// let url = fetch_gateway_url(&client, &token).await?;
/// let (write, read) = connect_gateway(&url).await?;
/// // 现在可以使用 write 发送消息，使用 read 接收消息
/// ```
pub(super) async fn connect_gateway(
    url: &str,
) -> anyhow::Result<(SplitSink<GatewayWs, Message>, SplitStream<GatewayWs>)> {
    // 建立 WebSocket 连接并忽略 HTTP 响应头
    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;
    // 将 WebSocket 流拆分为独立的读写端
    Ok(ws_stream.split())
}

/// 读取 Gateway Hello 消息并提取心跳间隔
///
/// 连接建立后，Discord Gateway 会首先发送一条 Hello 消息，
/// 其中包含心跳间隔时间（毫秒）。客户端必须按照此间隔发送心跳包
/// 以保持连接活跃。
///
/// # 参数
///
/// - `read`: WebSocket 读取端，用于接收 Hello 消息
///
/// # 返回值
///
/// 成功时返回心跳间隔时间（毫秒）。
/// 如果 Hello 消息格式不正确，将使用默认值 41250 毫秒（约 41.25 秒）。
///
/// # 错误
///
/// - 如果连接关闭且未收到消息，返回 "No hello" 错误
/// - 如果消息解析失败，返回 JSON 解析错误
///
/// # 消息格式
///
/// Hello 消息的 JSON 格式如下：
/// ```json
/// {
///     "op": 10,
///     "d": {
///         "heartbeat_interval": 41250
///     }
/// }
/// ```
///
/// # 示例
///
/// ```ignore
/// let (write, mut read) = connect_gateway(&url).await?;
/// let heartbeat_interval = read_gateway_hello(&mut read).await?;
/// println!("心跳间隔: {}ms", heartbeat_interval);
/// ```
pub(super) async fn read_gateway_hello(read: &mut SplitStream<GatewayWs>) -> anyhow::Result<u64> {
    // 等待接收第一条消息（应该是 Hello 消息）
    // 如果连接立即关闭，返回错误
    let hello = read.next().await.ok_or(anyhow::anyhow!("No hello"))??;
    if hello.is_close() {
        anyhow::bail!("No hello");
    }

    // 解析 Hello 消息的 JSON 数据
    let hello_data: serde_json::Value = serde_json::from_str(hello.to_text()?)?;

    // 从消息的 "d.heartbeat_interval" 字段提取心跳间隔
    // 如果字段缺失，使用 Discord 推荐的默认值 41250 毫秒
    Ok(hello_data
        .get("d")
        .and_then(|d| d.get("heartbeat_interval"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(41250))
}

/// 发送 Identify 消息进行身份认证
///
/// 在建立连接并收到 Hello 消息后，客户端必须发送 Identify 消息
/// 以完成身份认证。Identify 消息包含 Bot 令牌、所需的 Intent 权限
/// 以及客户端连接属性。
///
/// # 参数
///
/// - `write`: WebSocket 写入端，用于发送 Identify 消息
/// - `bot_token`: Discord Bot 令牌，用于身份认证
///
/// # 返回值
///
/// 成功时返回 `Ok(())`。
///
/// # 错误
///
/// 可能因 WebSocket 发送失败而返回错误。
///
/// # Intent 说明
///
/// 当前使用的 Intent 值为 37377，包含以下权限：
/// - GUILDS (1): 服务器相关事件
/// - GUILD_MESSAGES (512): 服务器消息事件
/// - DIRECT_MESSAGES (4096): 私信事件
/// - MESSAGE_CONTENT (32768): 消息内容（Privileged Intent）
///
/// # 消息格式
///
/// Identify 消息的 JSON 格式如下：
/// ```json
/// {
///     "op": 2,
///     "d": {
///         "token": "Bot令牌",
///         "intents": 37377,
///         "properties": {
///             "os": "linux",
///             "browser": "vibewindow",
///             "device": "vibewindow"
///         }
///     }
/// }
/// ```
///
/// # 示例
///
/// ```ignore
/// let (mut write, mut read) = connect_gateway(&url).await?;
/// let _ = read_gateway_hello(&mut read).await?;
/// send_identify(&mut write, "your_bot_token").await?;
/// // 认证成功后即可开始接收事件
/// ```
pub(super) async fn send_identify(
    write: &mut SplitSink<GatewayWs, Message>,
    bot_token: &str,
) -> anyhow::Result<()> {
    // 构建 Identify 消息负载
    // op=2 表示 Identify 操作码
    // intents 定义了客户端希望接收的事件类型
    let identify = json!({
        "op": 2,
        "d": {
            "token": bot_token,
            "intents": 37377,
            "properties": {
                "os": "linux",
                "browser": "vibewindow",
                "device": "vibewindow"
            }
        }
    });

    // 通过 WebSocket 发送 Identify 消息
    write.send(Message::Text(identify.to_string().into())).await?;
    Ok(())
}

#[cfg(test)]
#[path = "gateway_tests.rs"]
mod gateway_tests;
