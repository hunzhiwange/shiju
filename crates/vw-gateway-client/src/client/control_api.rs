//! Provider 凭据与 Gateway 实例控制接口。

use serde_json::{Value, json};

use super::GatewayClient;
use crate::http::directory_query;

impl GatewayClient {
    /// 保存指定 provider 的认证材料。
    pub async fn auth_set(&self, provider_id: &str, info: &Value) -> Result<bool, String> {
        let provider_id = required_provider_id(provider_id)?;
        self.put_json(&format!("/v1/auth/{}", encode_path_segment(provider_id)), &[], info).await
    }

    /// 删除指定 provider 的认证材料。
    pub async fn auth_remove(&self, provider_id: &str) -> Result<bool, String> {
        let provider_id = required_provider_id(provider_id)?;
        self.delete_json(
            &format!("/v1/auth/{}", encode_path_segment(provider_id)),
            &[],
            &Value::Null,
        )
        .await
    }

    /// 释放当前租户的全部 Gateway 实例。
    pub async fn global_dispose(&self) -> Result<bool, String> {
        self.post_json("/v1/global/dispose", &[], &json!({})).await
    }

    /// 释放指定目录对应的 Gateway 实例。
    pub async fn instance_dispose(&self, directory: Option<&str>) -> Result<bool, String> {
        self.post_json("/v1/instance/dispose", &directory_query(directory), &json!({})).await
    }
}

fn required_provider_id(provider_id: &str) -> Result<&str, String> {
    let provider_id = provider_id.trim();
    if provider_id.is_empty() {
        return Err("provider id must not be empty".to_string());
    }
    Ok(provider_id)
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
