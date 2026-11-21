//! 识矩应用网关传输契约。
//!
//! 汇总应用目录、已安装应用、API 密钥绑定和启动信息，同时对敏感字段使用脱敏调试表示。

use std::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Serialize};

use super::GatewayShijuAppClient;
use gateway_shiju_account::ShijuCredential;

const CATALOG_PATH: &str = "/v1/shiju/apps/catalog";
const APPS_PATH: &str = "/v1/shiju/apps";
const API_KEYS_PATH: &str = "/v1/shiju/apps/api-keys";

/// Application catalog returned by the local gateway.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuAppCatalogDto {
    #[serde(alias = "items")]
    #[serde(default, deserialize_with = "null_list")]
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
    #[serde(flatten)]
    pub presentation: ShijuAppPresentation,
}

/// Applications installed for the current SHIJU account.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuAppListDto {
    #[serde(alias = "items")]
    #[serde(default, deserialize_with = "null_list")]
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
    #[serde(default)]
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
    #[serde(default)]
    pub reconcile_status: String,
    #[serde(default)]
    pub runtime_status: String,
    #[serde(default)]
    pub runtime_url: String,
    #[serde(default, alias = "installedAt")]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(flatten)]
    pub presentation: ShijuAppPresentation,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub status: String,
}

impl ShijuInstalledAppDto {
    /// Reports runtime status independently of launch authorization and URL validation.
    pub fn runtime_ready(&self) -> bool {
        !self.installation_id.is_empty()
            && self.reconcile_status == "ready"
            && self.runtime_status == "ready"
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShijuAppPresentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_user_name: Option<String>,
    #[serde(alias = "createdBy")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_user_id: Option<String>,
    #[serde(alias = "category")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_name_en: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub is_favorite: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub can_manage: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub can_copy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overview: Option<ShijuAppOverview>,
}

fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShijuAppOverview {
    #[serde(deserialize_with = "null_list")]
    pub features: Vec<ShijuAppFeature>,
    #[serde(deserialize_with = "null_list")]
    pub scenarios: Vec<String>,
    pub before_use: String,
    pub data_notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuAppFeature {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShijuAppCategory {
    pub code: String,
    pub name: String,
    pub name_en: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShijuFeaturedApp {
    pub app: ShijuInstalledAppDto,
    pub cover_url: String,
    pub background_url: Option<String>,
    #[serde(default)]
    pub open_catalog_detail: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct ShijuAppItems<T> {
    #[serde(deserialize_with = "null_list")]
    pub items: Vec<T>,
}

fn null_list<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShijuCreatedApp {
    pub app_zid: String,
    pub app_id: String,
    pub title: String,
    pub icon_url: Option<String>,
    pub app_status: String,
    pub version_status: String,
    pub active_version: String,
    pub published_version: Option<String>,
    pub access_scope: Option<String>,
    pub installation_id: Option<String>,
    pub updated_at: Option<String>,
    pub description: Option<String>,
    pub created_by_user_name: Option<String>,
    pub created_by_user_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShijuDevelopmentDetail {
    pub app_id: String,
    pub title: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub updated_at: Option<String>,
    pub active_version: String,
    #[serde(flatten)]
    pub presentation: ShijuAppPresentation,
}

impl ShijuCreatedApp {
    pub fn lifecycle(&self) -> &'static str {
        match (self.app_status.as_str(), self.version_status.as_str()) {
            (_, "submitted" | "approved" | "under_review") => "审核中",
            (_, "draft" | "rejected") => "草稿",
            ("published", _) | (_, "published") => "已发布",
            _ => "草稿",
        }
    }
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

impl GatewayShijuAppClient {
    /// Reads the SHIJU application catalog through the local gateway.
    pub async fn shiju_app_catalog(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuAppCatalogDto, String> {
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_sensitive_request::<ShijuAppLaunchRequest, _>(
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
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_sensitive_request::<ShijuAppLaunchRequest, _>(
                reqwest::Method::GET,
                APPS_PATH,
                &[],
                None,
                Some(credential),
            )
            .await
    }

    pub async fn marketplace_list<T: serde::de::DeserializeOwned>(
        &self,
        credential: &ShijuCredential,
        list: ShijuMarketplaceList,
    ) -> Result<ShijuAppItems<T>, String> {
        let path = format!("{APPS_PATH}/{}", list.segment());
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_sensitive_request::<(), _>(
                reqwest::Method::GET,
                &path,
                &[],
                None,
                Some(credential),
            )
            .await
    }

    pub async fn set_favorite(
        &self,
        credential: &ShijuCredential,
        installation_id: &str,
        favorite: bool,
    ) -> Result<(), String> {
        if !valid_installation_id(installation_id) {
            return Err("SHIJU installation ID is invalid".into());
        }
        let path = format!("{APPS_PATH}/{installation_id}/favorite");
        let method = if favorite { reqwest::Method::PUT } else { reqwest::Method::DELETE };
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_empty_sensitive_request::<()>(method, &path, &[], None, Some(credential))
            .await
    }

    pub async fn install_catalog(
        &self,
        credential: &ShijuCredential,
        catalog_id: &str,
    ) -> Result<(), String> {
        if !valid_catalog_id(catalog_id) {
            return Err("SHIJU catalog app ID is invalid".into());
        }
        let path = format!("{CATALOG_PATH}/{catalog_id}/install");
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_empty_sensitive_request::<()>(
                reqwest::Method::POST,
                &path,
                &[],
                None,
                Some(credential),
            )
            .await
    }

    pub async fn get_development_detail(
        &self,
        credential: &ShijuCredential,
        app_zid: &str,
    ) -> Result<ShijuDevelopmentDetail, String> {
        if !valid_installation_id(app_zid) {
            return Err("SHIJU application ZID is invalid".into());
        }
        let path = format!("{APPS_PATH}/created/{app_zid}");
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_sensitive_request::<(), _>(
                reqwest::Method::GET,
                &path,
                &[],
                None,
                Some(credential),
            )
            .await
    }

    pub async fn get_installation(
        &self,
        credential: &ShijuCredential,
        installation_id: &str,
    ) -> Result<ShijuInstalledAppDto, String> {
        if !valid_installation_id(installation_id) {
            return Err("SHIJU installation ID is invalid".into());
        }
        let path = format!("{APPS_PATH}/installations/{installation_id}");
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_sensitive_request::<(), _>(
                reqwest::Method::GET,
                &path,
                &[],
                None,
                Some(credential),
            )
            .await
    }

    pub async fn copy_installation(
        &self,
        credential: &ShijuCredential,
        installation_id: &str,
        request: &ShijuAppCopyRequest,
    ) -> Result<(), String> {
        if !valid_installation_id(installation_id) {
            return Err("SHIJU installation ID is invalid".into());
        }
        let path = format!("{APPS_PATH}/{installation_id}/copy");
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_empty_sensitive_request(
                reqwest::Method::POST,
                &path,
                &[],
                Some(request),
                Some(credential),
            )
            .await
    }

    pub async fn download_companion_skill(
        &self,
        credential: &ShijuCredential,
        installation_id: &str,
    ) -> Result<Vec<u8>, String> {
        if !valid_installation_id(installation_id) {
            return Err("SHIJU installation ID is invalid".into());
        }
        let path = format!("{APPS_PATH}/{installation_id}/companion-skill");
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_bytes_sensitive_request(&path, credential)
            .await
    }

    /// Reads the personal API key binding for a SHIJU application installation.
    pub async fn shiju_app_api_key_binding(
        &self,
        credential: &ShijuCredential,
        installation_id: &str,
    ) -> Result<ShijuAppApiKeyBindingDto, String> {
        let path = format!("{APPS_PATH}/{installation_id}/api-key-binding");
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_sensitive_request::<ShijuAppLaunchRequest, _>(
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
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_sensitive_request::<ShijuAppLaunchRequest, _>(
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
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_empty_sensitive_request(
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
        gateway_shiju_account::GatewayShijuAccountClient::new(self.transport.clone())
            .shiju_account_sensitive_request::<ShijuAppLaunchRequest, _>(
                reqwest::Method::POST,
                &path,
                &[],
                None,
                Some(credential),
            )
            .await
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ShijuMarketplaceList {
    Categories,
    Featured,
    Created,
}

impl ShijuMarketplaceList {
    pub fn segment(self) -> &'static str {
        match self {
            Self::Categories => "categories",
            Self::Featured => "featured",
            Self::Created => "created",
        }
    }
}

pub fn valid_catalog_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShijuAppCopyRequest {
    pub request_id: String,
    pub expected_version: String,
    pub app_id: String,
    pub title: String,
}

pub fn valid_installation_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.bytes().all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
}
