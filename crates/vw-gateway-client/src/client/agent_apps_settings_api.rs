use super::GatewayClient;
use gateway_app_settings::{AgentAppsSettingsError, GatewayAppSettingsClient};
use vw_api_app_identity::{AppId, AppPreferencesPatch, AppPreferencesResult, LaunchIdentity, LaunchRequest, LaunchResult, LaunchSessionStatus};

impl GatewayClient {
    pub async fn agent_apps_preferences(
        &self,
        host: &str,
    ) -> Result<AppPreferencesResult, AgentAppsSettingsError> {
        GatewayAppSettingsClient::new(self.transport.clone()).agent_apps_preferences(host).await
    }
    pub async fn agent_apps_preferences_update(
        &self,
        host: &str,
        request: &AppPreferencesPatch,
    ) -> Result<AppPreferencesResult, AgentAppsSettingsError> {
        GatewayAppSettingsClient::new(self.transport.clone()).agent_apps_preferences_update(host, request).await
    }
    pub async fn agent_apps_preferences_authorized(
        &self,
        host: &str,
        token: &str,
    ) -> Result<AppPreferencesResult, AgentAppsSettingsError> {
        GatewayAppSettingsClient::new(self.transport.clone()).agent_apps_preferences_authorized(host, token).await
    }
    pub async fn agent_apps_preferences_update_authorized(
        &self,
        host: &str,
        request: &AppPreferencesPatch,
        token: &str,
    ) -> Result<AppPreferencesResult, AgentAppsSettingsError> {
        GatewayAppSettingsClient::new(self.transport.clone()).agent_apps_preferences_update_authorized(host, request, token).await
    }
    pub async fn agent_apps_launch(
        &self,
        host: &str,
        request: &LaunchRequest,
        provider_id: Option<&str>,
        account_id: Option<&str>,
    ) -> Result<LaunchResult, AgentAppsSettingsError> {
        GatewayAppSettingsClient::new(self.transport.clone()).agent_apps_launch(host, request, provider_id, account_id).await
    }
    pub async fn agent_apps_launch_authorized(
        &self,
        host: &str,
        request: &LaunchRequest,
        token: &str,
    ) -> Result<LaunchResult, AgentAppsSettingsError> {
        GatewayAppSettingsClient::new(self.transport.clone()).agent_apps_launch_authorized(host, request, token).await
    }
    pub async fn agent_apps_launch_identities(
        &self,
        host: &str,
        app: AppId,
        token: &str,
    ) -> Result<Vec<LaunchIdentity>, AgentAppsSettingsError> {
        GatewayAppSettingsClient::new(self.transport.clone()).agent_apps_launch_identities(host, app, token).await
    }
    pub async fn agent_apps_launch_identity_authorized(
        &self,
        host: &str,
        request: &LaunchRequest,
        identity: &LaunchIdentity,
        token: &str,
    ) -> Result<LaunchResult, AgentAppsSettingsError> {
        GatewayAppSettingsClient::new(self.transport.clone()).agent_apps_launch_identity_authorized(host, request, identity, token).await
    }
    pub async fn agent_apps_launch_session(
        &self,
        host: &str,
        id: &str,
        token: &str,
    ) -> Result<LaunchSessionStatus, AgentAppsSettingsError> {
        GatewayAppSettingsClient::new(self.transport.clone()).agent_apps_launch_session(host, id, token).await
    }
}
