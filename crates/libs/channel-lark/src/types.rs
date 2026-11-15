//! 飞书 WebSocket 长连接类型定义模块
//!
//! 本模块定义了飞书 WebSocket 长连接通信所需的数据结构和协议类型。
//! 主要基于 pbbp2.proto 协议规范实现帧编解码功能。
//!
//! # 核心类型
//!
//! - [`PbHeader`]: 协议头键值对
//! - [`PbFrame`]: WebSocket 通信帧结构
//! - [`WsClientConfig`]: 客户端配置信息
//! - [`LarkEvent`]: 飞书事件信封结构

/// 协议头键值对
///
/// 用于在 WebSocket 帧中传递元数据信息，采用键值对形式。
#[derive(Clone, PartialEq, prost::Message)]
pub(crate) struct PbHeader {
    /// 头部字段的键名
    #[prost(string, tag = "1")]
    pub key: String,
    /// 头部字段的值
    #[prost(string, tag = "2")]
    pub value: String,
}

/// 飞书 WebSocket 通信帧（基于 pbbp2.proto 协议）
///
/// 这是飞书 WebSocket 长连接的基础通信单元。
///
/// # 帧类型
///
/// - `method=0`: 控制帧（CONTROL），用于心跳检测（ping/pong）
/// - `method=1`: 数据帧（DATA），用于传递事件消息
///
/// # 字段说明
///
/// - `seq_id`: 序列号，用于消息排序和确认
/// - `log_id`: 日志标识，用于追踪和调试
/// - `service`: 服务标识
/// - `method`: 方法类型（0=控制, 1=数据）
/// - `headers`: 元数据头部列表
/// - `payload`: 可选的消息负载
#[derive(Clone, PartialEq, prost::Message)]
pub(crate) struct PbFrame {
    /// 序列号，用于消息排序和确认
    #[prost(uint64, tag = "1")]
    pub seq_id: u64,
    /// 日志标识，用于追踪和调试
    #[prost(uint64, tag = "2")]
    pub log_id: u64,
    /// 服务标识
    #[prost(int32, tag = "3")]
    pub service: i32,
    /// 方法类型：0=控制帧（ping/pong），1=数据帧（事件）
    #[prost(int32, tag = "4")]
    pub method: i32,
    /// 元数据头部列表
    #[prost(message, repeated, tag = "5")]
    pub headers: Vec<PbHeader>,
    /// 可选的消息负载数据
    #[prost(bytes = "vec", optional, tag = "8")]
    pub payload: Option<Vec<u8>>,
}

impl PbFrame {
    /// 获取指定键的头部值
    ///
    /// # 参数
    ///
    /// - `key`: 要查找的头部键名
    ///
    /// # 返回值
    ///
    /// 返回对应键的值字符串，如果未找到则返回空字符串
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let frame = PbFrame { /* ... */ };
    /// let content_type = frame.header_value("Content-Type");
    /// ```
    pub(crate) fn header_value<'a>(&'a self, key: &str) -> &'a str {
        self.headers.iter().find(|h| h.key == key).map(|h| h.value.as_str()).unwrap_or("")
    }
}

/// 服务器下发的客户端配置（从 pong 响应负载中解析）
///
/// 包含 WebSocket 连接的运行时配置参数。
#[derive(Debug, serde::Deserialize, Default, Clone)]
pub(crate) struct WsClientConfig {
    /// 心跳间隔时间（毫秒）
    #[serde(rename = "PingInterval")]
    pub ping_interval: Option<u64>,
}

/// POST /callback/ws/endpoint 接口的响应结构
///
/// 用于获取 WebSocket 连接端点信息的 HTTP 响应。
#[derive(Debug, serde::Deserialize)]
pub(crate) struct WsEndpointResp {
    /// 响应状态码（0 表示成功）
    pub code: i32,
    /// 响应消息（可选）
    #[serde(default)]
    pub msg: Option<String>,
    /// 端点数据（可选）
    #[serde(default)]
    pub data: Option<WsEndpoint>,
}

/// WebSocket 端点信息
///
/// 包含连接到飞书 WebSocket 服务所需的 URL 和配置。
#[derive(Debug, serde::Deserialize)]
pub(crate) struct WsEndpoint {
    /// WebSocket 连接 URL
    #[serde(rename = "URL")]
    pub url: String,
    /// 客户端配置（可选）
    #[serde(rename = "ClientConfig")]
    pub client_config: Option<WsClientConfig>,
}

/// 飞书事件信封结构（method=1 / type=event 负载）
///
/// 这是飞书推送事件的外层包装结构，包含事件头和事件内容。
#[derive(Debug, serde::Deserialize)]
pub(crate) struct LarkEvent {
    /// 事件头部信息
    pub header: LarkEventHeader,
    /// 事件内容（具体结构根据事件类型而定）
    pub event: serde_json::Value,
}

/// 飞书事件头部信息
#[derive(Debug, serde::Deserialize)]
pub(crate) struct LarkEventHeader {
    /// 事件类型标识
    pub event_type: String,
    /// 事件唯一标识
    #[allow(dead_code)]
    pub event_id: String,
}

/// 消息接收事件负载
///
/// 当收到消息时，事件内容会被解析为此结构。
#[derive(Debug, serde::Deserialize)]
pub(crate) struct MsgReceivePayload {
    /// 消息发送者信息
    pub sender: LarkSender,
    /// 消息内容
    pub message: LarkMessage,
}

/// 飞书消息发送者信息
#[derive(Debug, serde::Deserialize)]
pub(crate) struct LarkSender {
    /// 发送者标识信息
    pub sender_id: LarkSenderId,
    /// 发送者类型（如 "app"、"user" 等）
    #[serde(default)]
    pub sender_type: String,
}

/// 飞书发送者标识
///
/// 包含发送者的开放平台标识（open_id）。
#[derive(Debug, serde::Deserialize, Default)]
pub(crate) struct LarkSenderId {
    /// 用户的开放平台标识
    pub open_id: Option<String>,
}

/// 飞书消息结构
///
/// 包含消息的完整信息，包括消息 ID、会话 ID、消息类型和内容等。
#[derive(Debug, serde::Deserialize)]
pub(crate) struct LarkMessage {
    /// 消息唯一标识
    pub message_id: String,
    /// 会话（聊天）标识
    pub chat_id: String,
    /// 会话类型（如 "p2p"、"group" 等）
    pub chat_type: String,
    /// 消息类型（如 "text"、"image" 等）
    pub message_type: String,
    /// 消息内容（JSON 字符串格式）
    #[serde(default)]
    pub content: String,
    /// 消息中提及的用户列表
    #[serde(default)]
    pub mentions: Vec<serde_json::Value>,
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
