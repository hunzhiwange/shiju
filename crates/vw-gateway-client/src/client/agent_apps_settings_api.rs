//! Settings and launch requests never perform local filesystem or OS actions.
use super::GatewayClient;
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::agent_apps::*;

/// Transport, malformed responses and original gateway business errors stay distinct.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentAppsSettingsError {
    Transport(String),
    Http { status: u16, body: serde_json::Value },
    Decode(String),
}
impl std::fmt::Display for AgentAppsSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(message) | Self::Decode(message) => f.write_str(message),
            Self::Http { status, body } => {
                write!(f, "application settings/launch HTTP {status}: {body}")
            }
        }
    }
}
impl std::error::Error for AgentAppsSettingsError {}

impl GatewayClient {
    /// Read preferences on the selected execution host.
    pub async fn agent_apps_preferences(
        &self,
        host: &str,
    ) -> Result<AppPreferencesResult, AgentAppsSettingsError> {
        self.agent_apps_settings_request::<(), _>(
            reqwest::Method::GET,
            "preferences",
            host,
            None,
            None,
        )
        .await
    }
    /// Update one preferences domain without restarting applications.
    pub async fn agent_apps_preferences_update(
        &self,
        host: &str,
        request: &AppPreferencesPatch,
    ) -> Result<AppPreferencesResult, AgentAppsSettingsError> {
        self.agent_apps_settings_request(
            reqwest::Method::PATCH,
            "preferences",
            host,
            Some(request),
            None,
        )
        .await
    }
    /// Read using the independently granted application credential.
    pub async fn agent_apps_preferences_authorized(
        &self,
        host: &str,
        token: &str,
    ) -> Result<AppPreferencesResult, AgentAppsSettingsError> {
        self.agent_apps_settings_request::<(), _>(
            reqwest::Method::GET,
            "preferences",
            host,
            None,
            Some(token),
        )
        .await
    }
    /// Save one domain with no retries or implicit restart.
    pub async fn agent_apps_preferences_update_authorized(
        &self,
        host: &str,
        request: &AppPreferencesPatch,
        token: &str,
    ) -> Result<AppPreferencesResult, AgentAppsSettingsError> {
        self.agent_apps_settings_request(
            reqwest::Method::PATCH,
            "preferences",
            host,
            Some(request),
            Some(token),
        )
        .await
    }
    /// Launch once on the execution host. Paths and argv remain opaque to this client.
    /// Provider/account identities are resolved by the gateway; unsupported selections stay errors.
    pub async fn agent_apps_launch(
        &self,
        host: &str,
        request: &LaunchRequest,
        provider_id: Option<&str>,
        account_id: Option<&str>,
    ) -> Result<LaunchResult, AgentAppsSettingsError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Request<'a> {
            #[serde(flatten)]
            launch: &'a LaunchRequest,
            #[serde(skip_serializing_if = "Option::is_none")]
            provider_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            account_id: Option<&'a str>,
        }
        self.agent_apps_settings_request(
            reqwest::Method::POST,
            "launch",
            host,
            Some(&Request { launch: request, provider_id, account_id }),
            None,
        )
        .await
    }
    /// Launch the selected instance with the separately granted application credential.
    pub async fn agent_apps_launch_authorized(
        &self,
        host: &str,
        request: &LaunchRequest,
        token: &str,
    ) -> Result<LaunchResult, AgentAppsSettingsError> {
        self.agent_apps_settings_request(
            reqwest::Method::POST,
            "launch",
            host,
            Some(request),
            Some(token),
        )
        .await
    }
    /// Sanitized identities from the selected host's tenant-owned stores.
    pub async fn agent_apps_launch_identities(
        &self,
        host: &str,
        app: AppId,
        token: &str,
    ) -> Result<Vec<LaunchIdentity>, AgentAppsSettingsError> {
        let app = serde_json::to_value(app).expect("AppId serialization");
        self.agent_apps_settings_request::<(), _>(
            reqwest::Method::GET,
            &format!("launch-identities?appId={}", app.as_str().expect("AppId string")),
            host,
            None,
            Some(token),
        )
        .await
    }
    pub async fn agent_apps_launch_identity_authorized(
        &self,
        host: &str,
        request: &LaunchRequest,
        identity: &LaunchIdentity,
        token: &str,
    ) -> Result<LaunchResult, AgentAppsSettingsError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Selected<'a> {
            #[serde(flatten)]
            launch: &'a LaunchRequest,
            #[serde(skip_serializing_if = "Option::is_none")]
            provider_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            account_id: Option<&'a str>,
        }
        let body = Selected {
            launch: request,
            provider_id: (identity.kind == LaunchIdentityKind::Provider)
                .then_some(identity.id.as_str()),
            account_id: (identity.kind == LaunchIdentityKind::OfficialAccount)
                .then_some(identity.id.as_str()),
        };
        self.agent_apps_settings_request(
            reqwest::Method::POST,
            "launch",
            host,
            Some(&body),
            Some(token),
        )
        .await
    }
    /// Read status without ever replaying the launch POST.
    pub async fn agent_apps_launch_session(
        &self,
        host: &str,
        id: &str,
        token: &str,
    ) -> Result<LaunchSessionStatus, AgentAppsSettingsError> {
        if id.is_empty()
            || id.len() > 64
            || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(AgentAppsSettingsError::Transport(
                "invalid terminal session identity".into(),
            ));
        }
        self.agent_apps_settings_request::<(), _>(
            reqwest::Method::GET,
            &format!("launch-sessions/{id}"),
            host,
            None,
            Some(token),
        )
        .await
    }
    async fn agent_apps_settings_request<B: Serialize, T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        resource: &str,
        host: &str,
        body: Option<&B>,
        token: Option<&str>,
    ) -> Result<T, AgentAppsSettingsError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (method, resource, host, body, token);
            Err(AgentAppsSettingsError::Transport(
                "settings/launch requests require a native transport with redirects and retries disabled"
                    .into(),
            ))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            // This existing transport disables redirects and retry, including on ambiguous writes.
            let mut request = self
                .dynamic_host_client
                .request(method, format!("{}/v1/agent-apps/{resource}", self.endpoint.base_url()))
                .query(&[("targetHostId", host)]);
            if let Some(token) = token {
                request = request.header("x-agent-apps-token", token);
            }
            if let Some(body) = body {
                request = request.json(body);
            }
            let response = crate::http::apply_request_context(request, &self.endpoint)
                .send()
                .await
                .map_err(|err| AgentAppsSettingsError::Transport(err.to_string()))?;
            let status = response.status();
            let bytes = response
                .bytes()
                .await
                .map_err(|err| AgentAppsSettingsError::Transport(err.to_string()))?;
            if !status.is_success() {
                let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                    serde_json::Value::String(String::from_utf8_lossy(&bytes).into_owned())
                });
                return Err(AgentAppsSettingsError::Http { status: status.as_u16(), body });
            }
            serde_json::from_slice(&bytes)
                .map_err(|err| AgentAppsSettingsError::Decode(err.to_string()))
        }
    }
}
