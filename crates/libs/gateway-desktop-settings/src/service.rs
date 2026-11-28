use crate::{DesktopServiceCommand, DesktopServiceCommandResponseDto};
use serde_json::Value;
use crate::GatewayDesktopSettingsClient;

/// 网关主机返回适用于自身平台的安装命令。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcpInstallInstructions {
    pub supported: bool,
    pub shell: String,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcpInstallReport {
    pub success: bool,
    pub message: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AcpInstallProgress {
    pub stage: String,
    pub download: Option<AcpInstallDownload>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AcpInstallDownload {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub bytes_per_second: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum McpExtension {
    RustCodeGraph,
    ChromeDevTools,
}

impl McpExtension {
    pub const fn id(self) -> &'static str {
        match self {
            Self::RustCodeGraph => "rustcodegraph",
            Self::ChromeDevTools => "chrome-devtools",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        [Self::RustCodeGraph, Self::ChromeDevTools].into_iter().find(|kind| kind.id() == id)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpInstallInstructions {
    pub version: String,
    pub shell: String,
    pub commands: Vec<String>,
    pub configuration: String,
}

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

    /// 等待网关完成安装。
    pub async fn acp_install(&self, agent: &str) -> Result<String, String> {
        self.request_acp_install(agent, false, None).await
    }

    pub async fn acp_install_instructions(&self, agent: &str) -> Result<AcpInstallInstructions, String> {
        let path = acp_install_path(agent)?;
        self.transport.get_json(&path, &[]).await
    }

    pub async fn acp_install_report(&self, agent: &str) -> Result<AcpInstallReport, String> {
        self.acp_install_report_with_id(agent, None).await
    }

    pub async fn acp_install_report_with_id(
        &self,
        agent: &str,
        install_id: Option<&str>,
    ) -> Result<AcpInstallReport, String> {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum Reply {
            Report(AcpInstallReport),
            Legacy(String),
        }

        match self.request_acp_install(agent, true, install_id).await? {
            Reply::Report(report) => Ok(report),
            Reply::Legacy(message) => Ok(AcpInstallReport { success: true, message, steps: Vec::new() }),
        }
    }

    pub async fn acp_install_progress(
        &self,
        agent: &str,
        install_id: &str,
    ) -> Result<Option<AcpInstallProgress>, String> {
        let path = format!("{}/progress", acp_install_path(agent)?);
        self.transport.get_json(&path, &[("install_id".into(), install_id.into())]).await
    }

    pub async fn acp_install_cancel(&self, agent: &str, install_id: &str) -> Result<bool, String> {
        let path = acp_install_path(agent)?;
        self.transport.delete_json(&path, &[("install_id".into(), install_id.into())], &serde_json::json!({})).await
    }

    async fn request_acp_install<T: serde::de::DeserializeOwned>(
        &self,
        agent: &str,
        report: bool,
        install_id: Option<&str>,
    ) -> Result<T, String> {
        let path = acp_install_path(agent)?;
        let request = self.transport.long_running_client()?
            .post(format!("{}{}", self.transport.endpoint().base_url(), path))
            .json(&serde_json::json!({}));
        let request = if report { request.query(&[("report", true)]) } else { request };
        let request = if let Some(id) = install_id {
            request.query(&[("install_id", id)])
        } else {
            request
        };
        let response = gateway_http::apply_request_context(request, self.transport.endpoint())
            .send()
            .await
            .map_err(|err| gateway_http::transport_error("POST", self.transport.endpoint(), &path, err))?;
        gateway_http::parse_json_response("POST", self.transport.endpoint(), &path, response).await
    }

    pub async fn mcp_install_instructions(
        &self,
        extension: McpExtension,
    ) -> Result<McpInstallInstructions, String> {
        self.transport.get_json(&format!("/v1/desktop/mcp-install/{}", extension.id()), &[]).await
    }

    pub async fn mcp_install_progress(
        &self,
        extension: McpExtension,
        id: &str,
    ) -> Result<Option<AcpInstallProgress>, String> {
        let path = format!("/v1/desktop/mcp-install/{}/progress", extension.id());
        self.transport.get_json(&path, &[("install_id".into(), id.into())]).await
    }

    pub async fn mcp_install(
        &self,
        extension: McpExtension,
        id: &str,
    ) -> Result<AcpInstallReport, String> {
        let path = format!("/v1/desktop/mcp-install/{}", extension.id());
        let request = self.transport.long_running_client()?
            .post(format!("{}{}", self.transport.endpoint().base_url(), path))
            .query(&[("install_id", id)])
            .json(&serde_json::json!({}));
        let response = gateway_http::apply_request_context(request, self.transport.endpoint())
            .send().await
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


fn acp_install_path(agent: &str) -> Result<String, String> {
    if agent.is_empty() || !agent.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
        return Err("invalid ACP install name".into());
    }
    Ok(format!("/v1/desktop/acp-install/{agent}"))
}
