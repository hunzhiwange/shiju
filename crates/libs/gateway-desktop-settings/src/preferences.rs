use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use gateway_http::apply_request_context;
use crate::GatewayDesktopSettingsClient;

impl GatewayDesktopSettingsClient {
    /// 从全局配置中读取桌面系统设置并反序列化。
    pub async fn desktop_system_settings_get<T>(&self) -> Result<Option<T>, String>
    where
        T: DeserializeOwned,
    {
        let Some(value) = gateway_config::GatewayConfigClient::new(self.transport.clone()).global_config_get_path(&["app_ui", "system_settings"]).await? else {
            return Ok(None);
        };
        serde_json::from_value(value).map(Some).map_err(|err| err.to_string())
    }

    /// 以补丁方式写入桌面系统设置。
    pub async fn desktop_system_settings_patch<T>(&self, patch: &T) -> Result<(), String>
    where
        T: Serialize,
    {
        let value = serde_json::to_value(patch).map_err(|err| err.to_string())?;
        if value.get("gateway_client").is_some() {
            // A rejected connection snapshot may be echoed in the response.
            // Keep both the payload and the server's error text out of logs and UI messages.
            let request = self.transport.client()
                .patch(format!("{}/v1/global/config", self.transport.endpoint().base_url()))
                .json(&serde_json::json!({"app_ui": {"system_settings": value}}));
            let response = apply_request_context(request, self.transport.endpoint())
                .send()
                .await
                .map_err(|_| "gateway connection sync request failed".to_string())?;
            let status = response.status();
            if !status.is_success() {
                return Err(format!("gateway connection sync failed: {status}"));
            }
            response
                .json::<Value>()
                .await
                .map_err(|_| "invalid gateway connection sync response".to_string())?;
            return Ok(());
        }
        gateway_config::GatewayConfigClient::new(self.transport.clone()).global_config_patch_path(&["app_ui", "system_settings"], value).await
    }

    /// 读取桌面偏好设置。
    pub async fn desktop_preferences_get(&self) -> Result<Value, String> {
        self.transport.get_json("/v1/desktop/preferences", &[]).await
    }

    /// 更新桌面偏好设置并返回最新值。
    pub async fn desktop_preferences_patch(&self, patch: &Value) -> Result<Value, String> {
        self.transport.patch_json("/v1/desktop/preferences", &[], patch).await
    }

    /// 读取某类桌面工具的持久化内容。
    pub async fn desktop_tool_content_get(&self, tool_type: &str) -> Result<String, String> {
        let tool_type = desktop_tool_type_path(tool_type)?;
        let value: Value =
            self.transport.get_json(&format!("/v1/desktop/tool-content/{tool_type}"), &[]).await?;
        Ok(value.get("content").and_then(Value::as_str).unwrap_or_default().to_string())
    }

    /// 写入某类桌面工具的持久化内容。
    pub async fn desktop_tool_content_put(
        &self,
        tool_type: &str,
        content: &str,
    ) -> Result<(), String> {
        let tool_type = desktop_tool_type_path(tool_type)?;
        let _: Value = self.transport.put_json(
                &format!("/v1/desktop/tool-content/{tool_type}"),
                &[],
                &serde_json::json!({ "content": content }),
            )
            .await?;
        Ok(())
    }

    /// 读取项目级桌面偏好设置。
    pub async fn desktop_project_preferences_get(
        &self,
        project_path: &str,
    ) -> Result<Option<(String, bool, Option<String>)>, String> {
        let query = vec![("project_path".to_string(), project_path.to_string())];
        let Some(value) =
            self.transport.get_json_with_404::<Value>("/v1/desktop/project-preferences", &query).await?
        else {
            return Ok(None);
        };
        let model = value.get("model").and_then(Value::as_str).unwrap_or_default().to_string();
        let auto_model = value.get("auto_model").and_then(Value::as_bool).unwrap_or(false);
        let acp_agent = value.get("acp_agent").and_then(Value::as_str).map(ToString::to_string);
        if model.trim().is_empty() { Ok(None) } else { Ok(Some((model, auto_model, acp_agent))) }
    }

    /// 写入项目级桌面偏好设置。
    pub async fn desktop_project_preferences_put(
        &self,
        project_path: &str,
        model: &str,
        auto_model: bool,
        acp_agent: Option<&str>,
    ) -> Result<(), String> {
        let query = vec![("project_path".to_string(), project_path.to_string())];
        let _: Value = self.transport.put_json(
                "/v1/desktop/project-preferences",
                &query,
                &serde_json::json!({
                    "model": model,
                    "auto_model": auto_model,
                    "acp_agent": acp_agent,
                }),
            )
            .await?;
        Ok(())
    }
}

fn desktop_tool_type_path(tool_type: &str) -> Result<String, String> {
    if tool_type.trim().is_empty() {
        return Err("desktop tool type must not be empty".to_string());
    }
    let mut encoded = String::new();
    for byte in tool_type.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    Ok(encoded)
}
