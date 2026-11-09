//! Query settings persisted in the gateway database, scoped to the authorized execution target.

use serde::{Deserialize, Serialize};
use vw_agent_apps::{providers::validate_identity, usage::scripts::config::UsageTemplate};
pub use vw_api_types::apps_usage::settings::UsageCredentialId;
use vw_api_types::{
    agent_apps::AppId,
    apps_usage::settings::{MAX_REFRESH_INTERVAL_MINUTES, UsageCodingPlan, valid_usage_team_id},
    id::ProjectScope,
};

use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};

const NAMESPACE: &str = "agent_apps.usage_settings.v1";

/// The caller supplies these identities after authorizing the project, host and resources.
#[derive(Clone, Copy)]
pub struct UsageSettingsScope<'a> {
    pub project: &'a ProjectScope,
    pub principal: &'a str,
    pub target_host: &'a str,
    pub app: AppId,
    pub provider: &'a str,
    pub account: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageDisplay {
    #[default]
    Left,
    Used,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UsageSigningCredentialRefs {
    pub access_key_id_ref: UsageCredentialId,
    pub secret_access_key_ref: UsageCredentialId,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UsageCredentialRefs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_ref: Option<UsageCredentialId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token_ref: Option<UsageCredentialId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing: Option<UsageSigningCredentialRefs>,
}

/// Stores only the template selection and references. Script source and
/// resolved credentials are supplied separately on the execution host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UsageSettings {
    pub template: UsageTemplate,
    pub enabled: bool,
    /// Zero disables automatic refresh.
    pub refresh_interval_minutes: u16,
    #[serde(default)]
    pub display: UsageDisplay,
    #[serde(default)]
    pub credentials: UsageCredentialRefs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coding_plan: Option<UsageCodingPlan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UsageSettingsError {
    #[error("invalid usage settings scope")]
    InvalidScope,
    #[error("invalid native usage configuration")]
    InvalidCodingPlan,
    #[error("usage refresh interval must be between 0 and 1440 minutes")]
    InvalidRefreshInterval,
    #[error("usage settings storage is unavailable")]
    StorageUnavailable,
}

pub struct UsageSettingsStore(GatewaySettingsStore);

impl UsageSettingsStore {
    pub fn new(database: GlobalDatabase) -> Result<Self, UsageSettingsError> {
        GatewaySettingsStore::new(database)
            .map(Self)
            .map_err(|_| UsageSettingsError::StorageUnavailable)
    }

    pub fn get(
        &self,
        scope: UsageSettingsScope<'_>,
    ) -> Result<Option<UsageSettings>, UsageSettingsError> {
        let key = scope.config_key()?;
        let payload = self
            .0
            .get(scope.project.tenant_scope(), NAMESPACE, &key)
            .map_err(|_| UsageSettingsError::StorageUnavailable)?;
        payload
            .as_deref()
            .map(|json| {
                let settings: UsageSettings = serde_json::from_str(json)
                    .map_err(|_| UsageSettingsError::StorageUnavailable)?;
                settings.validate().map_err(|_| UsageSettingsError::StorageUnavailable)?;
                Ok(settings)
            })
            .transpose()
    }

    /// Replace one complete, validated settings record atomically.
    pub fn save(
        &self,
        scope: UsageSettingsScope<'_>,
        settings: &UsageSettings,
    ) -> Result<(), UsageSettingsError> {
        let key = scope.config_key()?;
        settings.validate()?;
        let payload =
            serde_json::to_string(settings).map_err(|_| UsageSettingsError::StorageUnavailable)?;
        self.0
            .upsert(scope.project.tenant_scope(), NAMESPACE, &key, &payload)
            .map_err(|_| UsageSettingsError::StorageUnavailable)
    }
}

impl UsageSettings {
    fn validate(&self) -> Result<(), UsageSettingsError> {
        if self.refresh_interval_minutes > MAX_REFRESH_INTERVAL_MINUTES {
            return Err(UsageSettingsError::InvalidRefreshInterval);
        }
        if let Some(plan) = &self.coding_plan {
            if self.template != UsageTemplate::CodingPlan
                || self.credentials.api_key_ref.is_some()
                || self.credentials.access_token_ref.is_some()
            {
                return Err(UsageSettingsError::InvalidCodingPlan);
            }
            let valid = match plan {
                UsageCodingPlan::ZhipuTeam { organization_id, project_id } => {
                    valid_usage_team_id(organization_id)
                        && valid_usage_team_id(project_id)
                        && self.credentials.signing.is_none()
                }
                UsageCodingPlan::Volcengine => {
                    self.credentials.signing.map_or(!self.enabled, |refs| {
                        refs.access_key_id_ref != refs.secret_access_key_ref
                    })
                }
            };
            if !valid {
                return Err(UsageSettingsError::InvalidCodingPlan);
            }
        }
        Ok(())
    }
}

impl UsageSettingsScope<'_> {
    fn config_key(self) -> Result<String, UsageSettingsError> {
        if self.principal.trim().is_empty() || self.principal.chars().any(char::is_control) {
            return Err(UsageSettingsError::InvalidScope);
        }
        for id in [Some(self.target_host), Some(self.provider), self.account].into_iter().flatten()
        {
            validate_identity(id).map_err(|_| UsageSettingsError::InvalidScope)?;
        }
        // Encode the tuple without delimiter collisions and distinguish absent accounts.
        serde_json::to_string(&(
            self.project.project_id(),
            self.principal,
            self.target_host,
            self.app,
            self.provider,
            self.account,
        ))
        .map_err(|_| UsageSettingsError::StorageUnavailable)
    }
}
