use super::ComputerUseClient;
use super::response::ComputerUseResponse;
use anyhow::Context;
use serde_json::{Value, json};
use std::time::Duration;

impl ComputerUseClient {
    /// 执行浏览器自动化动作
    pub(crate) async fn execute_action(
        &self,
        action: &str,
        args: &Value,
    ) -> anyhow::Result<tool_types::ToolResult> {
        let endpoint = self.endpoint_url()?;

        let mut params = args
            .as_object()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("browser args must be a JSON object"))?;
        params.remove("action");

        self.validate_action(action, &params)?;

        if action == "screen_capture" {
            if let Some(path) = params.get("path").and_then(Value::as_str) {
                let resolved = self.resolve_output_path_for_write("path", path).await?;
                params.insert(
                    "path".to_string(),
                    Value::String(resolved.to_string_lossy().into_owned()),
                );
            }
        }

        let payload = json!({
            "action": action,
            "params": params,
            "policy": {
                "allowed_domains": self.allowed_domains,
                "window_allowlist": self.config.window_allowlist,
                "max_coordinate_x": self.config.max_coordinate_x,
                "max_coordinate_y": self.config.max_coordinate_y,
            },
            "metadata": {
                "session_name": self.session_name,
                "source": "vibewindow.browser",
                "version": env!("CARGO_PKG_VERSION"),
            }
        });

        let client = agent_config::runtime::build_runtime_proxy_client("tool.browser");
        let mut request = client
            .post(endpoint)
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .json(&payload);

        if let Some(api_key) = self.config.api_key.as_deref() {
            let token = api_key.trim();
            if !token.is_empty() {
                request = request.bearer_auth(token);
            }
        }

        let response = request.send().await.with_context(|| {
            format!("Failed to call computer-use sidecar at {}", self.config.endpoint)
        })?;

        let status = response.status();
        let body =
            response.text().await.context("Failed to read computer-use sidecar response body")?;

        if let Ok(parsed) = serde_json::from_str::<ComputerUseResponse>(&body) {
            if status.is_success() && parsed.success.unwrap_or(true) {
                let output = parsed
                    .data
                    .map(|data| serde_json::to_string_pretty(&data).unwrap_or_default())
                    .unwrap_or_else(|| {
                        serde_json::to_string_pretty(&json!({
                            "backend": "computer_use",
                            "action": action,
                            "ok": true,
                        }))
                        .unwrap_or_default()
                    });

                return Ok(tool_types::ToolResult {
                    success: true,
                    output,
                    error: None,
                });
            }

            let error = parsed.error.or_else(|| {
                if status.is_success() && parsed.success == Some(false) {
                    Some("computer-use sidecar returned success=false".to_string())
                } else {
                    Some(format!("computer-use sidecar request failed with status {status}"))
                }
            });

            return Ok(tool_types::ToolResult {
                success: false,
                output: String::new(),
                error,
            });
        }

        if status.is_success() {
            return Ok(tool_types::ToolResult {
                success: true,
                output: body,
                error: None,
            });
        }

        Ok(tool_types::ToolResult {
            success: false,
            output: String::new(),
            error: Some(format!(
                "computer-use sidecar request failed with status {status}: {}",
                body.trim()
            )),
        })
    }
}
#[cfg(test)]
#[path = "execution_tests.rs"]
mod execution_tests;
