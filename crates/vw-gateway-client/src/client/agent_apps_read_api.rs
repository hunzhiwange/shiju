//! 所选网关的应用读取；客户端不探测执行主机的文件系统。
use super::GatewayClient;
use serde::de::DeserializeOwned;
use vw_api_types::agent_apps::{AppCatalogResponse, AppId, AppReadResponse};

/// 应用读取失败，保留网关状态及结构化业务错误。
#[derive(Debug, Clone, PartialEq)]
pub enum AgentAppsReadError {
    Transport(String),
    Http { status: u16, body: serde_json::Value },
    Decode(String),
}
impl std::fmt::Display for AgentAppsReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(message) | Self::Decode(message) => f.write_str(message),
            Self::Http { status, body } => write!(f, "application read HTTP {status}: {body}"),
        }
    }
}
impl std::error::Error for AgentAppsReadError {}

impl GatewayClient {
    /// 读取授权主机的应用目录。
    pub async fn agent_apps_catalog(
        &self,
        target_host_id: &str,
    ) -> Result<AppCatalogResponse, AgentAppsReadError> {
        self.agent_apps_read("catalog", target_host_id, None, None).await
    }
    /// 读取应用安装实例。
    pub async fn agent_apps_installations(
        &self,
        target_host_id: &str,
        app_id: AppId,
    ) -> Result<AppReadResponse, AgentAppsReadError> {
        self.agent_apps_read("installations", target_host_id, Some(app_id), None).await
    }
    /// 读取应用诊断。
    pub async fn agent_apps_diagnostics(
        &self,
        target_host_id: &str,
        app_id: AppId,
    ) -> Result<AppReadResponse, AgentAppsReadError> {
        self.agent_apps_read("diagnostics", target_host_id, Some(app_id), None).await
    }
    /// 读取应用版本。
    pub async fn agent_apps_versions(
        &self,
        target_host_id: &str,
        app_id: AppId,
    ) -> Result<AppReadResponse, AgentAppsReadError> {
        self.agent_apps_read("versions", target_host_id, Some(app_id), None).await
    }
    /// Read the catalog using the independently granted application credential.
    pub async fn agent_apps_catalog_authorized(
        &self,
        host: &str,
        token: &str,
    ) -> Result<AppCatalogResponse, AgentAppsReadError> {
        self.agent_apps_read("catalog", host, None, Some(token)).await
    }
    /// Read installations using the independently granted application credential.
    pub async fn agent_apps_installations_authorized(
        &self,
        host: &str,
        app: AppId,
        token: &str,
    ) -> Result<AppReadResponse, AgentAppsReadError> {
        self.agent_apps_read("installations", host, Some(app), Some(token)).await
    }
    async fn agent_apps_read<T: DeserializeOwned>(
        &self,
        resource: &str,
        target_host_id: &str,
        app_id: Option<AppId>,
        token: Option<&str>,
    ) -> Result<T, AgentAppsReadError> {
        let mut query = vec![("targetHostId", target_host_id.to_owned())];
        if let Some(app_id) = app_id {
            let serde_json::Value::String(id) = serde_json::to_value(app_id)
                .map_err(|err| AgentAppsReadError::Decode(err.to_string()))?
            else {
                return Err(AgentAppsReadError::Decode("invalid application identity".into()));
            };
            query.push(("appId", id));
        }
        self.agent_apps_read_request(&format!("/v1/agent-apps/{resource}"), &query, token).await
    }

    pub(super) async fn agent_apps_read_request<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
        token: Option<&str>,
    ) -> Result<T, AgentAppsReadError> {
        #[cfg(target_arch = "wasm32")]
        if token.is_some() {
            return Err(AgentAppsReadError::Transport(
                "authorized application reads require native transport".into(),
            ));
        }
        #[cfg(not(target_arch = "wasm32"))]
        let transport = &self.dynamic_host_client;
        #[cfg(target_arch = "wasm32")]
        let transport = &self.client;
        let request = transport.get(format!("{}{path}", self.endpoint.base_url())).query(query);
        let request = if let Some(token) = token {
            let mut value = reqwest::header::HeaderValue::from_str(token).map_err(|_| {
                AgentAppsReadError::Transport("invalid application credential".into())
            })?;
            value.set_sensitive(true);
            request.header("x-agent-apps-token", value)
        } else {
            request
        };
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|err| AgentAppsReadError::Transport(err.to_string()))?;
        let status = response.status();
        let bytes =
            response.bytes().await.map_err(|err| AgentAppsReadError::Transport(err.to_string()))?;
        if !status.is_success() {
            let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned())
            });
            return Err(AgentAppsReadError::Http { status: status.as_u16(), body });
        }
        serde_json::from_slice(&bytes).map_err(|err| AgentAppsReadError::Decode(err.to_string()))
    }
}
