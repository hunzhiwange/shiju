//! 所选网关的应用读取；客户端不探测执行主机的文件系统。
use super::GatewayClient;
use vw_api_app_identity::{AppCatalogResponse, AppId, AppReadResponse};

pub use gateway_http::AgentAppsReadError;

impl GatewayClient {
    /// 读取授权主机的应用目录。
    pub async fn agent_apps_catalog(
        &self,
        target_host_id: &str,
    ) -> Result<AppCatalogResponse, AgentAppsReadError> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone())
            .agent_apps_catalog(target_host_id).await
    }
    /// 读取应用安装实例。
    pub async fn agent_apps_installations(
        &self,
        target_host_id: &str,
        app_id: AppId,
    ) -> Result<AppReadResponse, AgentAppsReadError> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone())
            .agent_apps_installations(target_host_id, app_id).await
    }
    /// 读取应用诊断。
    pub async fn agent_apps_diagnostics(
        &self,
        target_host_id: &str,
        app_id: AppId,
    ) -> Result<AppReadResponse, AgentAppsReadError> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone())
            .agent_apps_diagnostics(target_host_id, app_id).await
    }
    /// 读取应用版本。
    pub async fn agent_apps_versions(
        &self,
        target_host_id: &str,
        app_id: AppId,
    ) -> Result<AppReadResponse, AgentAppsReadError> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone())
            .agent_apps_versions(target_host_id, app_id).await
    }
    /// Read the catalog using the independently granted application credential.
    pub async fn agent_apps_catalog_authorized(
        &self,
        host: &str,
        token: &str,
    ) -> Result<AppCatalogResponse, AgentAppsReadError> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone())
            .agent_apps_catalog_authorized(host, token).await
    }
    /// Read installations using the independently granted application credential.
    pub async fn agent_apps_installations_authorized(
        &self,
        host: &str,
        app: AppId,
        token: &str,
    ) -> Result<AppReadResponse, AgentAppsReadError> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone())
            .agent_apps_installations_authorized(host, app, token).await
    }
    pub(super) async fn agent_apps_read_request<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
        token: Option<&str>,
    ) -> Result<T, AgentAppsReadError> {
        gateway_app_read::GatewayAppReadClient::new(self.transport.clone())
            .agent_apps_read_request(path, query, token).await
    }
}
