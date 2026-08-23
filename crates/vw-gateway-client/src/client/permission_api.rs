//! 网关权限 API 模块，定义待处理权限请求的数据传输结构和客户端回复接口。

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::GatewayClient;
use crate::http::directory_query;

/// PendingPermissionToolDto 数据结构承载该模块对外传递的 PendingPermissionToolDto 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingPermissionToolDto {
    pub message_id: String,
    pub call_id: String,
}

/// PendingPermissionRequestDto 数据结构承载该模块对外传递的 PendingPermissionRequestDto 状态。
///
/// 字段保持可序列化或可渲染形态，便于调用方直接组合 UI 或持久化数据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingPermissionRequestDto {
    pub id: String,
    pub session_id: String,
    pub permission: String,
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub metadata: Map<String, Value>,
    #[serde(default)]
    pub always: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<PendingPermissionToolDto>,
}

/// PendingPermissionReplyDto 枚举描述该模块支持的 PendingPermissionReplyDto 取值集合。
///
/// 每个变体代表一个明确分支，调用方应通过显式匹配处理新增状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PendingPermissionReplyDto {
    Once,
    Always,
    Reject,
}

impl GatewayClient {
    /// 提供 permission list 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub async fn permission_list(
        &self,
        directory: Option<&str>,
    ) -> Result<Vec<PendingPermissionRequestDto>, String> {
        self.get_json("/v1/permission", &directory_query(directory)).await
    }

    /// 提供 permission reply 功能。
    ///
    /// 参数和返回值遵循调用方所在模块的工作流约定，错误会显式向上传递或由 UI 状态承载。
    pub async fn permission_reply(
        &self,
        request_id: &str,
        reply: PendingPermissionReplyDto,
        directory: Option<&str>,
        message: Option<String>,
    ) -> Result<bool, String> {
        let request_id = required_request_id(request_id)?;
        self.post_json(
            &format!("/v1/permission/{}/reply", encode_path_segment(request_id)),
            &directory_query(directory),
            &serde_json::json!({
                "reply": reply,
                "message": message,
            }),
        )
        .await
    }
}

fn required_request_id(request_id: &str) -> Result<&str, String> {
    let request_id = request_id.trim();
    if request_id.is_empty() {
        return Err("permission request id must not be empty".to_string());
    }
    Ok(request_id)
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
#[path = "permission_api_tests.rs"]
mod permission_api_tests;
