//! Connection-owned wire response slots and trusted interaction identity.
use anyhow::{Result, bail};
use serde_json::Value;
use rmcp::model::RequestId;
use std::sync::Mutex;

#[derive(Clone)]
pub struct FormOwner {
    pub session: String,
    pub tenant: vw_api_transport::TenantScope,
}

/// A single response slot is sufficient: McpSessionConnections serializes each peer operation.
/// The response id must match; unsolicited/overlapping responses fail closed, never cross-route.
#[derive(Default)]
pub struct RawResponse(
    Mutex<Option<(Value, Value)>>,
    Mutex<std::collections::HashMap<String, Value>>,
    Mutex<std::collections::HashMap<String, Value>>,
);
impl RawResponse {
    pub fn tool_metadata(&self, name: &str) -> Option<Value> {
        self.2.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(name).cloned()
    }
    pub fn take_form(&self, id: &RequestId) -> Option<Value> {
        let key = serde_json::to_string(id).ok()?;
        self.1.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&key)
    }
    pub fn capture(&self, message: &Value) {
        if message["method"] == "elicitation/create" {
            if let (Some(id), Some(schema)) = (
                message.get("id"),
                if message.pointer("/params/mode").and_then(Value::as_str) == Some("url") {
                    message.get("params")
                } else {
                    message.pointer("/params/requestedSchema")
                },
            ) {
                let mut forms = self.1.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if forms.len() < 64 {
                    forms.insert(id.to_string(), schema.clone());
                }
            }
        }
        if message.get("method").is_some() {
            return;
        }
        if let Some(tools) = message.pointer("/result/tools").and_then(Value::as_array) {
            let mut metadata = self.2.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            for tool in tools {
                if let Some(name) = tool["name"].as_str() {
                    if metadata.len() < 2048 || metadata.contains_key(name) {
                        metadata.insert(
                            name.to_owned(),
                            tool.get("_meta").cloned().unwrap_or(Value::Null),
                        );
                    }
                }
            }
        }
        if let (Some(id), Some(result)) = (message.get("id"), message.get("result")) {
            if ["resources", "resourceTemplates", "contents", "prompts", "messages"]
                .iter()
                .any(|key| result.get(key).is_some())
            {
                *self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
                    Some((id.clone(), result.clone()));
            }
        }
    }
    pub fn take(&self, id: &RequestId) -> Result<Value> {
        let expected = serde_json::to_value(id)?;
        match self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take() {
            Some((actual, result)) if actual == expected => Ok(result),
            _ => bail!("MCP resource response is missing or does not match its request"),
        }
    }
}
