//! 系统状态与发现接口。
//!
//! 每个公开方法固定映射一个只读 Gateway 路由，避免向调用方暴露任意路径请求能力。

use serde_json::Value;

use super::GatewayClient;
use crate::http::{
    apply_request_context, directory_query, log_request, log_request_succeeded, response_error,
    transport_error,
};

impl GatewayClient {
    /// 获取无需业务上下文的基础健康探针。
    pub async fn system_health(&self) -> Result<Value, String> {
        self.system_json("/v1/health", None).await
    }

    /// 获取 Prometheus 文本指标。
    pub async fn system_metrics(&self) -> Result<String, String> {
        const PATH: &str = "/v1/metrics";

        log_request::<()>("GET", &self.endpoint, PATH, &[], None);
        let request = self.client.get(format!("{}{}", self.endpoint.base_url(), PATH));
        let response = apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|error| transport_error("GET", &self.endpoint, PATH, error))?;
        if !response.status().is_success() {
            return Err(response_error("GET", &self.endpoint, PATH, response).await);
        }
        log_request_succeeded("GET", &self.endpoint, PATH);
        response.text().await.map_err(|error| error.to_string())
    }

    /// 获取 Gateway 运行状态。
    pub async fn system_status(&self) -> Result<Value, String> {
        self.system_json("/v1/status", None).await
    }

    /// 获取 Gateway 工具目录。
    pub async fn system_tools(&self) -> Result<Value, String> {
        self.system_json("/v1/tools", None).await
    }

    /// 检查当前 Gateway 可见的 MCP 服务。
    pub async fn system_mcp_inspect(&self) -> Result<Value, String> {
        self.system_json("/v1/mcp/inspect", None).await
    }

    /// 获取受管 API 健康摘要。
    pub async fn system_api_health(&self) -> Result<Value, String> {
        self.system_json("/v1/api-health", None).await
    }

    /// 获取全局实例健康状态。
    pub async fn system_global_health(&self) -> Result<Value, String> {
        self.system_json("/v1/global/health", None).await
    }

    /// 获取 OpenAI 兼容模型目录。
    pub async fn system_models(&self) -> Result<Value, String> {
        self.system_json("/v1/models", None).await
    }

    /// 获取目录上下文对应的路径信息。
    pub async fn system_path(&self, directory: Option<&str>) -> Result<Value, String> {
        self.system_json("/v1/path", directory).await
    }

    /// 获取目录上下文对应的版本控制状态。
    pub async fn system_vcs(&self, directory: Option<&str>) -> Result<Value, String> {
        self.system_json("/v1/vcs", directory).await
    }

    /// 获取目录上下文对应的命令目录。
    pub async fn system_commands(&self, directory: Option<&str>) -> Result<Value, String> {
        self.system_json("/v1/command", directory).await
    }

    /// 获取目录上下文对应的代理目录。
    pub async fn system_agents(&self, directory: Option<&str>) -> Result<Value, String> {
        self.system_json("/v1/agent", directory).await
    }

    /// 获取目录上下文对应的技能目录。
    pub async fn system_skills(&self, directory: Option<&str>) -> Result<Value, String> {
        self.system_json("/v1/skill", directory).await
    }

    /// 获取 Gateway 可发现的 CLI 工具目录。
    pub async fn system_cli_tools(&self) -> Result<Value, String> {
        self.system_json("/v1/cli-tools", None).await
    }

    async fn system_json(&self, path: &str, directory: Option<&str>) -> Result<Value, String> {
        self.get_json(path, &directory_query(directory)).await
    }
}
