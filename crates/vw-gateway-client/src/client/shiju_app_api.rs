//! 识矩应用网关传输契约。
//!
//! 汇总应用目录、已安装应用、API 密钥绑定和启动信息，同时对敏感字段使用脱敏调试表示。

use std::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Serialize};

use super::{GatewayClient, ShijuCredential};

const CATALOG_PATH: &str = "/v1/shiju/apps/catalog";
const APPS_PATH: &str = "/v1/shiju/apps";
const API_KEYS_PATH: &str = "/v1/shiju/apps/api-keys";

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

/// The current personal API key binding for a SHIJU application installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShijuAppApiKeyBindingDto {
    pub personal: ShijuAppPersonalApiKeyBindingDto,
}

/// Non-sensitive fields describing a personal SHIJU application API key binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShijuAppPersonalApiKeyBindingDto {
    pub configured: bool,
    pub can_edit: bool,
    #[serde(default, deserialize_with = "deserialize_optional_api_key_id")]
    pub api_key_id: Option<u64>,
    pub key_name: Option<String>,
    pub api_key_mask: Option<String>,
    pub status: Option<String>,
    pub expires_at: Option<String>,
    pub updated_at: Option<String>,
}

fn deserialize_optional_api_key_id<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ApiKeyId {
        Number(u64),
        Text(String),
    }

    match Option::<ApiKeyId>::deserialize(deserializer)? {
        None => Ok(None),
        Some(ApiKeyId::Text(value)) if value.is_empty() => Ok(None),
        Some(ApiKeyId::Number(value)) => Ok(Some(value)),
        Some(ApiKeyId::Text(value)) => value.parse::<u64>().map(Some).map_err(D::Error::custom),
    }
}

/// IAM API keys available to the current SHIJU account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShijuIamApiKeyListDto {
    #[serde(default)]
    pub api_keys: Vec<ShijuIamApiKeyDto>,
}

/// Non-sensitive metadata for one SHIJU IAM API key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShijuIamApiKeyDto {
    pub api_key_id: u64,
    pub key_name: String,
    pub api_key_mask: String,
    pub status: String,
    pub expires_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Selects an owned IAM API key for one SHIJU application installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShijuAppApiKeyBindingSaveRequest {
    pub api_key_id: u64,
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

    /// Reads the personal API key binding for a SHIJU application installation.
    pub async fn shiju_app_api_key_binding(
        &self,
        credential: &ShijuCredential,
        installation_id: &str,
    ) -> Result<ShijuAppApiKeyBindingDto, String> {
        let path = format!("{APPS_PATH}/{installation_id}/api-key-binding");
        self.shiju_account_sensitive_request::<ShijuAppLaunchRequest, _>(
            reqwest::Method::GET,
            &path,
            &[],
            None,
            Some(credential),
        )
        .await
    }

    /// Reads IAM API keys available to the current SHIJU account.
    pub async fn shiju_app_api_keys(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuIamApiKeyListDto, String> {
        self.shiju_account_sensitive_request::<ShijuAppLaunchRequest, _>(
            reqwest::Method::GET,
            API_KEYS_PATH,
            &[],
            None,
            Some(credential),
        )
        .await
    }

    /// Saves a personal API key binding for a SHIJU application installation.
    pub async fn shiju_app_save_api_key_binding(
        &self,
        credential: &ShijuCredential,
        installation_id: &str,
        request: &ShijuAppApiKeyBindingSaveRequest,
    ) -> Result<(), String> {
        let path = format!("{APPS_PATH}/{installation_id}/api-key-binding");
        self.shiju_empty_sensitive_request(
            reqwest::Method::PUT,
            &path,
            &[],
            Some(request),
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
