use crate::{DesktopServiceCommand, DesktopServiceCommandResponseDto};
use serde_json::Value;
use crate::GatewayDesktopSettingsClient;

impl GatewayDesktopSettingsClient {
    /// 在网关主机安装浏览器依赖，沿用认证上下文并允许下载时间。
    pub async fn browser_install(&self, backend: &str) -> Result<String, String> {
        if !matches!(backend, "agent_browser" | "auto") {
            return Err("该后端需要手工准备服务，请查看安装帮助".into());
        }
        let path = format!("/v1/desktop/browser-install/{backend}");
        let request = self.transport.client()
            .post(format!("{}{}", self.transport.endpoint().base_url(), path))
            .json(&serde_json::json!({}));
        #[cfg(not(target_arch = "wasm32"))]
        let request = request.timeout(std::time::Duration::from_secs(630));
        let response = gateway_http::apply_request_context(request, self.transport.endpoint())
            .send()
            .await
            .map_err(|err| gateway_http::transport_error("POST", self.transport.endpoint(), &path, err))?;
        gateway_http::parse_json_response("POST", self.transport.endpoint(), &path, response).await
    }

    /// 安装可能需要下载较大的包，使用独立请求超时。
    pub async fn acp_install(&self, agent: &str) -> Result<String, String> {
        if agent.is_empty() || !agent.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
            return Err("invalid ACP install name".into());
        }
        let path = format!("/v1/desktop/acp-install/{agent}");
        let request = self.transport.client()
            .post(format!("{}{}", self.transport.endpoint().base_url(), path))
            .json(&serde_json::json!({}));
        #[cfg(not(target_arch = "wasm32"))]
        let request = request.timeout(std::time::Duration::from_secs(310));
        let response = gateway_http::apply_request_context(request, self.transport.endpoint())
            .send()
            .await
            .map_err(|err| gateway_http::transport_error("POST", self.transport.endpoint(), &path, err))?;
        gateway_http::parse_json_response("POST", self.transport.endpoint(), &path, response).await
    }

    pub async fn acp_diagnostic(&self, id: &str, config: &Value) -> Result<String, String> {
        self.transport.post_json(&format!("/v1/desktop/acp-diagnostic/{id}"), &[], config).await
    }

    pub async fn acp_diagnostic_cancel(&self, id: &str) -> Result<bool, String> {
        self.transport.delete_json(&format!("/v1/desktop/acp-diagnostic/{id}"), &[], &serde_json::json!({}))
            .await
    }

    /// 通过 gateway 调用本机 CLI service 生命周期命令。
    pub async fn desktop_service_command(
        &self,
        command: &str,
    ) -> Result<DesktopServiceCommandResponseDto, String> {
        self.desktop_service_command_typed(DesktopServiceCommand::try_from(command)?).await
    }

    /// 执行经过类型约束的 service 生命周期命令。
    pub async fn desktop_service_command_typed(
        &self,
        command: DesktopServiceCommand,
    ) -> Result<DesktopServiceCommandResponseDto, String> {
        self.transport.post_json(
            &format!("/v1/desktop/service/{}", command.as_str()),
            &[],
            &serde_json::json!({}),
        )
        .await
    }
}
