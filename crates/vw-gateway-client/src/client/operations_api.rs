//! 集成、记忆与诊断运维接口。

use serde_json::{Value, json};

use super::GatewayClient;

impl GatewayClient {
    /// 列出 Gateway 注册的集成及当前状态。
    pub async fn integrations_get(&self) -> Result<Value, String> {
        self.get_json("/v1/integrations", &[]).await
    }

    /// 读取集成设置与配置修订版本。
    pub async fn integration_settings_get(&self) -> Result<Value, String> {
        self.get_json("/v1/integrations/settings", &[]).await
    }

    /// 从结构化安全输入更新指定集成的凭据。
    pub async fn integration_credentials_put(
        &self,
        integration_id: &str,
        body: &Value,
    ) -> Result<Value, String> {
        let integration_id = required_path_value(integration_id, "integration id")?;
        self.put_json(
            &format!(
                "/v1/integrations/{}/credentials",
                encode_path_segment(integration_id)
            ),
            &[],
            body,
        )
        .await
    }

    /// 按语义查询或按类别列出记忆。
    pub async fn memory_query_get(
        &self,
        query: Option<&str>,
        category: Option<&str>,
    ) -> Result<Value, String> {
        let mut params = Vec::new();
        if let Some(query) = non_empty_optional(query, "memory query")? {
            params.push(("query".to_string(), query.to_string()));
        }
        if let Some(category) = non_empty_optional(category, "memory category")? {
            params.push(("category".to_string(), category.to_string()));
        }
        self.get_json("/v1/memory", &params).await
    }

    /// 保存一条记忆。
    pub async fn memory_store(&self, body: &Value) -> Result<Value, String> {
        self.post_json("/v1/memory", &[], body).await
    }

    /// 删除指定键的记忆。
    pub async fn memory_delete(&self, key: &str) -> Result<Value, String> {
        let key = required_path_value(key, "memory key")?;
        self.delete_json(
            &format!("/v1/memory/{}", encode_path_segment(key)),
            &[],
            &Value::Null,
        )
        .await
    }

    /// 以 GET 执行 Gateway 诊断。
    pub async fn doctor_get(&self) -> Result<Value, String> {
        self.get_json("/v1/doctor", &[]).await
    }

    /// 以兼容 POST 方法执行 Gateway 诊断。
    pub async fn doctor_post(&self) -> Result<Value, String> {
        self.post_json("/v1/doctor", &[], &json!({})).await
    }
}

fn non_empty_optional<'a>(value: Option<&'a str>, label: &str) -> Result<Option<&'a str>, String> {
    value
        .map(|value| required_path_value(value, label))
        .transpose()
}

fn required_path_value<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(value)
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
