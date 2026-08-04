//! SHIJU application gateway transport contract.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::{GatewayClient, ShijuCredential};

const CATALOG_PATH: &str = "/v1/shiju/apps/catalog";
const APPS_PATH: &str = "/v1/shiju/apps";

/// Application catalog returned by the local gateway.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuAppCatalogDto {
    #[serde(alias = "items")]
    #[serde(default)]
    pub apps: Vec<ShijuCatalogAppDto>,
}

/// A SHIJU application available in the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShijuCatalogAppDto {
    #[serde(alias = "catalogAppId")]
    pub app_id: String,
    #[serde(alias = "title")]
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(alias = "currentListedVersion")]
    pub version: String,
    pub installed: bool,
    #[serde(default)]
    pub installation_id: Option<String>,
    #[serde(default, alias = "installedByUserId")]
    pub user_id: Option<String>,
    #[serde(default)]
    pub installed_at: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

/// Applications installed for the current SHIJU account.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuAppListDto {
    #[serde(alias = "items")]
    #[serde(default)]
    pub apps: Vec<ShijuInstalledAppDto>,
}

impl fmt::Debug for ShijuAppListDto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("ShijuAppListDto").field("apps", &self.apps).finish()
    }
}

/// An installed SHIJU application and its current runtime status.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShijuInstalledAppDto {
    pub installation_id: String,
    pub app_id: String,
    #[serde(alias = "title")]
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    pub version: String,
    #[serde(alias = "origin")]
    pub source: String,
    pub reconcile_status: String,
    pub runtime_status: String,
    pub runtime_url: String,
    #[serde(default, alias = "installedAt")]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl fmt::Debug for ShijuInstalledAppDto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ShijuInstalledAppDto")
            .field("installation_id", &self.installation_id)
            .field("app_id", &self.app_id)
            .field("name", &self.name)
            .field("description", &self.description)
            .field("icon_url", &self.icon_url)
            .field("version", &self.version)
            .field("source", &self.source)
            .field("reconcile_status", &self.reconcile_status)
            .field("runtime_status", &self.runtime_status)
            .field("runtime_url", &"<redacted>")
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// A gateway-validated SHIJU application launch target.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShijuAppLaunchDto {
    pub installation_id: String,
    pub name: String,
    pub runtime_url: String,
    pub runtime_set_cookie: String,
}

impl fmt::Debug for ShijuAppLaunchDto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ShijuAppLaunchDto")
            .field("installation_id", &self.installation_id)
            .field("name", &self.name)
            .field("runtime_url", &"<redacted>")
            .field("runtime_set_cookie", &"<redacted>")
            .finish()
    }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ShijuAppLaunchRequest {}

impl GatewayClient {
    /// Reads the SHIJU application catalog through the local gateway.
    pub async fn shiju_app_catalog(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuAppCatalogDto, String> {
        self.shiju_account_sensitive_request::<ShijuAppLaunchRequest, _>(
            reqwest::Method::GET,
            CATALOG_PATH,
            &[],
            None,
            Some(credential),
        )
        .await
    }

    /// Reads applications installed for the current SHIJU account.
    pub async fn shiju_apps(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuAppListDto, String> {
        self.shiju_account_sensitive_request::<ShijuAppLaunchRequest, _>(
            reqwest::Method::GET,
            APPS_PATH,
            &[],
            None,
            Some(credential),
        )
        .await
    }

    /// Prepares a gateway-validated SHIJU application launch.
    pub async fn shiju_app_prepare_launch(
        &self,
        credential: &ShijuCredential,
        installation_id: &str,
    ) -> Result<ShijuAppLaunchDto, String> {
        let path = format!("{APPS_PATH}/{installation_id}/launch");
        self.shiju_account_sensitive_request::<ShijuAppLaunchRequest, _>(
            reqwest::Method::POST,
            &path,
            &[],
            None,
            Some(credential),
        )
        .await
    }
}
