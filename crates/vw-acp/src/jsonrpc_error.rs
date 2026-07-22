//! JSON-RPC 错误响应对象的构建辅助。

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::types::{OutputErrorAcpPayload, OutputErrorCode, OutputErrorOrigin};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcErrorObject {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse {
    pub jsonrpc: String,
    pub id: Value,
    pub error: JsonRpcErrorObject,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuildJsonRpcErrorParams {
    pub id: Option<Value>,
    pub output_code: OutputErrorCode,
    pub detail_code: Option<String>,
    pub origin: Option<OutputErrorOrigin>,
    pub message: String,
    pub retryable: Option<bool>,
    pub timestamp: Option<String>,
    pub session_id: Option<String>,
    pub acp: Option<OutputErrorAcpPayload>,
}

pub const fn output_error_jsonrpc_code(code: OutputErrorCode) -> i64 {
    match code {
        OutputErrorCode::NoSession => -32002,
        OutputErrorCode::Timeout => -32070,
        OutputErrorCode::PermissionDenied => -32071,
        OutputErrorCode::PermissionPromptUnavailable => -32072,
        OutputErrorCode::Runtime => -32603,
        OutputErrorCode::Usage => -32602,
    }
}

fn has_valid_acp_error(acp: Option<&OutputErrorAcpPayload>) -> bool {
    acp.is_some_and(|acp| !acp.message.trim().is_empty())
}

fn build_fallback_data(params: &BuildJsonRpcErrorParams) -> Value {
    let mut data = Map::new();
    data.insert(
        "vwacpCode".to_string(),
        serde_json::to_value(params.output_code).unwrap_or(Value::Null),
    );
    if let Some(detail_code) = params.detail_code.as_ref() {
        data.insert("detailCode".to_string(), Value::String(detail_code.clone()));
    }
    if let Some(origin) = params.origin {
        data.insert("origin".to_string(), serde_json::to_value(origin).unwrap_or(Value::Null));
    }
    if let Some(retryable) = params.retryable {
        data.insert("retryable".to_string(), Value::Bool(retryable));
    }
    if let Some(timestamp) = params.timestamp.as_ref() {
        data.insert("timestamp".to_string(), Value::String(timestamp.clone()));
    }
    if let Some(session_id) = params.session_id.as_ref() {
        data.insert("sessionId".to_string(), Value::String(session_id.clone()));
    }
    Value::Object(data)
}

fn build_error_object(params: &BuildJsonRpcErrorParams) -> JsonRpcErrorObject {
    if has_valid_acp_error(params.acp.as_ref()) {
        let acp = params.acp.as_ref().expect("validated above");
        return JsonRpcErrorObject {
            code: acp.code,
            message: acp.message.clone(),
            data: acp.data.clone(),
        };
    }

    JsonRpcErrorObject {
        code: output_error_jsonrpc_code(params.output_code),
        message: params.message.clone(),
        data: Some(build_fallback_data(params)),
    }
}

pub fn build_json_rpc_error_response(params: BuildJsonRpcErrorParams) -> JsonRpcErrorResponse {
    let id = params.id.clone().unwrap_or(Value::Null);
    let error = build_error_object(&params);
    JsonRpcErrorResponse { jsonrpc: "2.0".to_string(), id, error }
}
