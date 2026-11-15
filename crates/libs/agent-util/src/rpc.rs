//! RPC 通信协议模块
//!
//! 本模块提供了轻量级的 RPC（远程过程调用）通信协议实现，用于在不同组件之间
//! 进行结构化的消息传递。该协议基于 JSON 序列化，支持请求-响应模式和事件推送模式。
//!
//! # 协议设计
//!
//! 消息格式采用标签联合（tagged union）模式，通过 `type` 字段区分不同的消息类型：
//! - **请求消息**：客户端发起的方法调用请求，包含方法名、输入参数和请求 ID
//! - **结果消息**：服务端返回的方法调用结果，包含结果数据和对应的请求 ID
//! - **事件消息**：单向的事件通知，包含事件名和事件数据
//!
//! # 使用示例
//!
//! ```ignore
//! use serde_json::json;
//!
//! // 创建一个 RPC 请求
//! let req = rpc::request("getUser", json!({"id": 123}), 1);
//!
//! // 编码为 JSON 字符串
//! let encoded = rpc::encode(&req)?;
//!
//! // 解码 JSON 字符串
//! let decoded = rpc::decode(&encoded)?;
//!
//! // 创建一个事件消息
//! let evt = rpc::event("userUpdated", json!({"name": "Alice"}));
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// RPC 线路协议消息枚举
///
/// 定义了 RPC 通信中的三种消息类型，采用标签联合模式进行序列化。
/// 每种消息类型在 JSON 中通过 `type` 字段进行区分。
///
/// # 消息类型
///
/// - `Request`: 客户端发起的方法调用请求
/// - `Result`: 服务端返回的方法调用结果
/// - `Event`: 单向事件通知消息
///
/// # 序列化示例
///
/// ```json
/// {"type": "rpc.request", "method": "getUser", "input": {"id": 123}, "id": 1}
/// {"type": "rpc.result", "result": {"name": "Alice"}, "id": 1}
/// {"type": "rpc.event", "event": "userUpdated", "data": {"name": "Bob"}}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Wire {
    /// RPC 请求消息
    ///
    /// 表示客户端发起的一次方法调用请求。
    ///
    /// # 字段说明
    ///
    /// - `method`: 要调用的方法名称，如 "getUser"、"createOrder" 等
    /// - `input`: 方法的输入参数，以 JSON Value 形式传递
    /// - `id`: 请求的唯一标识符，用于匹配对应的响应结果
    #[serde(rename = "rpc.request")]
    Request {
        /// 要调用的方法名称
        method: String,
        /// 方法的输入参数
        input: Value,
        /// 请求的唯一标识符
        id: u64,
    },

    /// RPC 结果消息
    ///
    /// 表示服务端对某个请求的响应结果。
    ///
    /// # 字段说明
    ///
    /// - `result`: 方法执行的返回结果，以 JSON Value 形式返回
    /// - `id`: 对应请求的唯一标识符，用于将结果与请求进行匹配
    #[serde(rename = "rpc.result")]
    Result {
        /// 方法执行的返回结果
        result: Value,
        /// 对应请求的唯一标识符
        id: u64,
    },

    /// RPC 事件消息
    ///
    /// 表示一个单向的事件通知，不需要响应。
    ///
    /// # 字段说明
    ///
    /// - `event`: 事件名称，如 "userUpdated"、"orderCreated" 等
    /// - `data`: 事件携带的数据，以 JSON Value 形式传递
    #[serde(rename = "rpc.event")]
    Event {
        /// 事件名称
        event: String,
        /// 事件数据
        data: Value,
    },
}

/// 将 RPC 消息编码为 JSON 字符串
///
/// 将 `Wire` 消息序列化为 JSON 字符串格式，便于在网络上传输。
///
/// # 参数
///
/// - `msg`: 要编码的 RPC 消息引用
///
/// # 返回值
///
/// - `Ok(String)`: 编码成功，返回 JSON 字符串
/// - `Err(serde_json::Error)`: 序列化失败，返回错误信息
///
/// # 示例
///
/// ```ignore
/// let req = rpc::request("getUser", json!({"id": 123}), 1);
/// let encoded = rpc::encode(&req)?;
/// // encoded: {"type":"rpc.request","method":"getUser","input":{"id":123},"id":1}
/// ```
pub fn encode(msg: &Wire) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

/// 将 JSON 字符串解码为 RPC 消息
///
/// 将 JSON 字符串反序列化为 `Wire` 消息对象。
///
/// # 参数
///
/// - `s`: 包含 RPC 消息的 JSON 字符串
///
/// # 返回值
///
/// - `Ok(Wire)`: 解码成功，返回 RPC 消息对象
/// - `Err(serde_json::Error)`: 反序列化失败，返回错误信息
///
/// # 示例
///
/// ```ignore
/// let json_str = r#"{"type":"rpc.request","method":"getUser","input":{"id":123},"id":1}"#;
/// let decoded = rpc::decode(json_str)?;
/// ```
pub fn decode(s: &str) -> Result<Wire, serde_json::Error> {
    serde_json::from_str(s)
}

/// 创建 RPC 请求消息
///
/// 辅助函数，用于快速创建一个 `Wire::Request` 变体。
///
/// # 参数
///
/// - `method`: 要调用的方法名称，可以是任何实现了 `Into<String>` trait 的类型
/// - `input`: 方法的输入参数，以 JSON Value 形式传递
/// - `id`: 请求的唯一标识符
///
/// # 返回值
///
/// 返回 `Wire::Request` 变体
///
/// # 示例
///
/// ```ignore
/// let req = rpc::request("getUser", json!({"id": 123}), 1);
/// ```
pub fn request(method: impl Into<String>, input: Value, id: u64) -> Wire {
    Wire::Request { method: method.into(), input, id }
}

/// 创建 RPC 结果消息
///
/// 辅助函数，用于快速创建一个 `Wire::Result` 变体。
///
/// # 参数
///
/// - `result`: 方法执行的返回结果，以 JSON Value 形式传递
/// - `id`: 对应请求的唯一标识符
///
/// # 返回值
///
/// 返回 `Wire::Result` 变体
///
/// # 示例
///
/// ```ignore
/// let res = rpc::result(json!({"name": "Alice", "age": 30}), 1);
/// ```
pub fn result(result: Value, id: u64) -> Wire {
    Wire::Result { result, id }
}

/// 创建 RPC 事件消息
///
/// 辅助函数，用于快速创建一个 `Wire::Event` 变体。
///
/// # 参数
///
/// - `event`: 事件名称，可以是任何实现了 `Into<String>` trait 的类型
/// - `data`: 事件携带的数据，以 JSON Value 形式传递
///
/// # 返回值
///
/// 返回 `Wire::Event` 变体
///
/// # 示例
///
/// ```ignore
/// let evt = rpc::event("userUpdated", json!({"userId": 123, "changes": {"name": "Bob"}}));
/// ```
pub fn event(event: impl Into<String>, data: Value) -> Wire {
    Wire::Event { event: event.into(), data }
}
