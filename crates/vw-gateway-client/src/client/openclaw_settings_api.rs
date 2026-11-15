use super::{GatewayClient, OpenClawSettingsError, OpenClawSettingsScope};
use vw_api_openclaw_settings::*;

impl GatewayClient {
    /// Read the section without substituting defaults for null or unknown fields.
    pub async fn openclaw_settings_model(
        &self,
        scope: &OpenClawSettingsScope,
        token: &str,
    ) -> Result<Option<OpenClawDefaultModel>, OpenClawSettingsError> {
        gateway_openclaw_settings::GatewayOpenClawSettingsClient::new(self.transport.clone())
            .openclaw_settings_model(scope, token).await
    }

    /// Preserve write warnings; callers retain their draft if the write fails.
    pub async fn set_openclaw_settings_model(
        &self,
        scope: &OpenClawSettingsScope,
        value: &OpenClawDefaultModel,
        token: &str,
    ) -> Result<OpenClawWriteOutcome, OpenClawSettingsError> {
        gateway_openclaw_settings::GatewayOpenClawSettingsClient::new(self.transport.clone())
            .set_openclaw_settings_model(scope, value, token).await
    }

    /// Read the section without substituting defaults for null or unknown fields.
    pub async fn openclaw_settings_catalog(
        &self,
        scope: &OpenClawSettingsScope,
        token: &str,
    ) -> Result<Option<OpenClawModelCatalog>, OpenClawSettingsError> {
        gateway_openclaw_settings::GatewayOpenClawSettingsClient::new(self.transport.clone())
            .openclaw_settings_catalog(scope, token).await
    }

    /// Preserve write warnings; callers retain their draft if the write fails.
    pub async fn set_openclaw_settings_catalog(
        &self,
        scope: &OpenClawSettingsScope,
        value: &OpenClawModelCatalog,
        token: &str,
    ) -> Result<OpenClawWriteOutcome, OpenClawSettingsError> {
        gateway_openclaw_settings::GatewayOpenClawSettingsClient::new(self.transport.clone())
            .set_openclaw_settings_catalog(scope, value, token).await
    }

    /// Read the section without substituting defaults for null or unknown fields.
    pub async fn openclaw_settings_defaults(
        &self,
        scope: &OpenClawSettingsScope,
        token: &str,
    ) -> Result<Option<OpenClawAgentsDefaults>, OpenClawSettingsError> {
        gateway_openclaw_settings::GatewayOpenClawSettingsClient::new(self.transport.clone())
            .openclaw_settings_defaults(scope, token).await
    }

    /// Preserve write warnings; callers retain their draft if the write fails.
    pub async fn set_openclaw_settings_defaults(
        &self,
        scope: &OpenClawSettingsScope,
        value: &OpenClawAgentsDefaults,
        token: &str,
    ) -> Result<OpenClawWriteOutcome, OpenClawSettingsError> {
        gateway_openclaw_settings::GatewayOpenClawSettingsClient::new(self.transport.clone())
            .set_openclaw_settings_defaults(scope, value, token).await
    }

    /// Read the section without substituting defaults for null or unknown fields.
    pub async fn openclaw_settings_env(
        &self,
        scope: &OpenClawSettingsScope,
        token: &str,
    ) -> Result<Option<OpenClawEnvConfig>, OpenClawSettingsError> {
        gateway_openclaw_settings::GatewayOpenClawSettingsClient::new(self.transport.clone())
            .openclaw_settings_env(scope, token).await
    }

    /// Preserve write warnings; callers retain their draft if the write fails.
    pub async fn set_openclaw_settings_env(
        &self,
        scope: &OpenClawSettingsScope,
        value: &OpenClawEnvConfig,
        token: &str,
    ) -> Result<OpenClawWriteOutcome, OpenClawSettingsError> {
        gateway_openclaw_settings::GatewayOpenClawSettingsClient::new(self.transport.clone())
            .set_openclaw_settings_env(scope, value, token).await
    }

    /// Read the section without substituting defaults for null or unknown fields.
    pub async fn openclaw_settings_tools(
        &self,
        scope: &OpenClawSettingsScope,
        token: &str,
    ) -> Result<Option<OpenClawToolsConfig>, OpenClawSettingsError> {
        gateway_openclaw_settings::GatewayOpenClawSettingsClient::new(self.transport.clone())
            .openclaw_settings_tools(scope, token).await
    }

    /// Preserve write warnings; callers retain their draft if the write fails.
    pub async fn set_openclaw_settings_tools(
        &self,
        scope: &OpenClawSettingsScope,
        value: &OpenClawToolsConfig,
        token: &str,
    ) -> Result<OpenClawWriteOutcome, OpenClawSettingsError> {
        gateway_openclaw_settings::GatewayOpenClawSettingsClient::new(self.transport.clone())
            .set_openclaw_settings_tools(scope, value, token).await
    }

    pub async fn openclaw_settings_health(
        &self,
        scope: &OpenClawSettingsScope,
        token: &str,
    ) -> Result<Vec<OpenClawHealthWarning>, OpenClawSettingsError> {
        gateway_openclaw_settings::GatewayOpenClawSettingsClient::new(self.transport.clone())
            .openclaw_settings_health(scope, token).await
    }
}
