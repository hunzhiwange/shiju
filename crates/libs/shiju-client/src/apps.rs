//! SHIJU 应用平台与 IAM 接口的上游客户端。

use std::fmt;
use std::time::Duration;

use gateway_shiju_app::{
    ShijuAppCategory, ShijuAppCopyRequest, ShijuAppItems, ShijuCreatedApp, ShijuFeaturedApp,
    valid_catalog_id,
};
use reqwest::header::{HeaderValue, SET_COOKIE};
use reqwest::{Method, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use {
    gateway_shiju_app::ShijuAppApiKeyBindingDto, gateway_shiju_app::ShijuAppCatalogDto,
    gateway_shiju_app::ShijuAppLaunchDto, gateway_shiju_app::ShijuAppListDto,
    gateway_shiju_app::ShijuIamApiKeyDto, gateway_shiju_app::ShijuIamApiKeyListDto,
};

const PRODUCTION_APP_BASE_URL: &str = "https://gateway.shiju.cloud/sj/app-platform/api/";
const PRODUCTION_IAM_BASE_URL: &str = "https://gateway.shiju.cloud/sj/iam/api/v1/";
const CATALOG_PATH: &str = "catalog";
const APPS_PATH: &str = "apps";
const IAM_API_KEYS_PATH: &str = "api-keys";
const BROWSER_SESSION_PATH: &str = "/api/runtime/browser-session";
const API_KEY_HEADER: &str = "X-Api-Key";
const RUNTIME_SESSION_COOKIE_NAME: &str = "app_runtime_session";
const RUNTIME_HOST: &str = "app.shiju.cloud";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// SHIJU 应用接口错误的稳定分类。
pub enum ShijuAppErrorKind {
    InvalidInput,
    Upstream,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// SHIJU 应用接口的本地校验或上游错误。
pub struct ShijuAppError {
    kind: ShijuAppErrorKind,
    message: &'static str,
}

impl ShijuAppError {
    fn invalid_input(message: &'static str) -> Self {
        Self { kind: ShijuAppErrorKind::InvalidInput, message }
    }

    fn upstream(message: &'static str) -> Self {
        Self { kind: ShijuAppErrorKind::Upstream, message }
    }

    /// 返回调用方可用于映射响应的错误分类。
    pub fn kind(&self) -> ShijuAppErrorKind {
        self.kind
    }
}

impl fmt::Display for ShijuAppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for ShijuAppError {}

#[derive(Clone)]
/// SHIJU 应用平台与 IAM 客户端。
pub struct ShijuAppClient {
    http: reqwest::Client,
    app_base_url: Url,
    iam_base_url: Url,
    browser_session_endpoint: Url,
}

impl ShijuAppClient {
    /// 使用固定生产端点创建客户端。
    pub fn production() -> Result<Self, ShijuAppError> {
        Self::new(
            PRODUCTION_APP_BASE_URL,
            PRODUCTION_IAM_BASE_URL,
            &format!("https://{RUNTIME_HOST}"),
        )
    }

    #[cfg(any(test, feature = "test-support"))]
    /// 使用显式应用与 IAM 基础地址创建客户端。
    pub fn with_base_urls(app_base_url: &str, iam_base_url: &str) -> Result<Self, ShijuAppError> {
        Self::new(app_base_url, iam_base_url, app_base_url)
    }

    fn new(
        app_base_url: &str,
        iam_base_url: &str,
        runtime_base_url: &str,
    ) -> Result<Self, ShijuAppError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| ShijuAppError::upstream("failed to create SHIJU application client"))?;
        let app_base_url = parse_base_url(app_base_url)?;
        let iam_base_url = parse_base_url(iam_base_url)?;
        let browser_session_endpoint = parse_base_url(runtime_base_url)?
            .join(BROWSER_SESSION_PATH)
            .map_err(|_| ShijuAppError::upstream("SHIJU browser session endpoint is invalid"))?;
        Ok(Self { http, app_base_url, iam_base_url, browser_session_endpoint })
    }

    /// 读取当前凭证可见的应用目录。
    pub async fn catalog(&self, credential: &str) -> Result<ShijuAppCatalogDto, ShijuAppError> {
        self.get_json(CATALOG_PATH, credential).await
    }

    /// 读取当前凭证已安装的应用。
    pub async fn installed(&self, credential: &str) -> Result<ShijuAppListDto, ShijuAppError> {
        self.get_json(APPS_PATH, credential).await
    }

    pub async fn categories(
        &self,
        credential: &str,
    ) -> Result<ShijuAppItems<ShijuAppCategory>, ShijuAppError> {
        self.get_json("marketplace/categories", credential).await
    }

    pub async fn featured(
        &self,
        credential: &str,
    ) -> Result<ShijuAppItems<ShijuFeaturedApp>, ShijuAppError> {
        self.get_json("marketplace/featured?limit=4", credential).await
    }

    pub async fn created(
        &self,
        credential: &str,
    ) -> Result<ShijuAppItems<ShijuCreatedApp>, ShijuAppError> {
        self.get_json("apps/development?managedOnly=true", credential).await
    }

    pub async fn set_favorite(
        &self,
        credential: &str,
        id: &str,
        favorite: bool,
    ) -> Result<(), ShijuAppError> {
        validate_installation_id(id)?;
        let method = if favorite { Method::PUT } else { Method::DELETE };
        self.write(method, &format!("installations/{id}/favorite"), credential).await
    }

    pub async fn install_catalog(&self, credential: &str, id: &str) -> Result<(), ShijuAppError> {
        if !valid_catalog_id(id) {
            return Err(ShijuAppError::invalid_input("SHIJU catalog app ID is invalid"));
        }
        self.write(Method::POST, &format!("tenant/catalog/{id}/install"), credential).await
    }

    pub async fn development_detail(
        &self,
        credential: &str,
        app_zid: &str,
    ) -> Result<gateway_shiju_app::ShijuDevelopmentDetail, ShijuAppError> {
        validate_installation_id(app_zid)?;
        self.get_json(&format!("apps/{app_zid}"), credential).await
    }

    pub async fn installation_detail(
        &self,
        credential: &str,
        installation_id: &str,
    ) -> Result<gateway_shiju_app::ShijuInstalledAppDto, ShijuAppError> {
        validate_installation_id(installation_id)?;
        let app: gateway_shiju_app::ShijuInstalledAppDto =
            self.get_json(&format!("tenant/installations/{installation_id}"), credential).await?;
        if app.installation_id != installation_id {
            return Err(ShijuAppError::upstream(
                "SHIJU installation detail does not match the requested ID",
            ));
        }
        Ok(app)
    }

    pub async fn copy_installation(
        &self,
        credential: &str,
        id: &str,
        request: &ShijuAppCopyRequest,
    ) -> Result<(), ShijuAppError> {
        validate_installation_id(id)?;
        let endpoint = self
            .app_base_url
            .join(&format!("tenant/installations/{id}/copy"))
            .map_err(|_| ShijuAppError::upstream("SHIJU application upstream URL is invalid"))?;
        let response = self
            .http
            .post(endpoint)
            .header(API_KEY_HEADER, credential_header(credential)?)
            .json(request)
            .send()
            .await
            .map_err(|_| ShijuAppError::upstream("SHIJU application copy failed"))?;
        if !response.status().is_success() {
            return Err(ShijuAppError::upstream("SHIJU application copy failed"));
        }
        Ok(())
    }

    pub async fn download_companion_skill(
        &self,
        credential: &str,
        id: &str,
    ) -> Result<Vec<u8>, ShijuAppError> {
        const MAX_BYTES: usize = 32 * 1024 * 1024;
        validate_installation_id(id)?;
        let mut response =
            self.get(&format!("tenant/installations/{id}/companion-skill"), credential).await?;
        if response.content_length().is_some_and(|length| length > MAX_BYTES as u64) {
            return Err(ShijuAppError::upstream(
                "SHIJU companion skill archive exceeds the download limit",
            ));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| ShijuAppError::upstream("SHIJU companion skill download failed"))?
        {
            if bytes.len() + chunk.len() > MAX_BYTES {
                return Err(ShijuAppError::upstream(
                    "SHIJU companion skill archive exceeds the download limit",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        if !bytes.starts_with(b"PK\x03\x04") {
            return Err(ShijuAppError::upstream("SHIJU companion skill archive is invalid"));
        }
        Ok(bytes)
    }

    async fn write(
        &self,
        method: Method,
        path: &str,
        credential: &str,
    ) -> Result<(), ShijuAppError> {
        let endpoint = self
            .app_base_url
            .join(path)
            .map_err(|_| ShijuAppError::upstream("SHIJU application upstream URL is invalid"))?;
        let response = self
            .http
            .request(method, endpoint)
            .header(API_KEY_HEADER, credential_header(credential)?)
            .send()
            .await
            .map_err(|_| ShijuAppError::upstream("SHIJU application upstream request failed"))?;
        if !response.status().is_success() {
            return Err(ShijuAppError::upstream("SHIJU application upstream request failed"));
        }
        Ok(())
    }

    /// 读取指定安装实例绑定的 API 密钥。
    pub async fn api_key_binding(
        &self,
        credential: &str,
        installation_id: &str,
    ) -> Result<ShijuAppApiKeyBindingDto, ShijuAppError> {
        validate_installation_id(installation_id)?;
        let path = binding_path(installation_id);
        self.get_json(&path, credential).await
    }

    /// 列出当前账户可用的 IAM API 密钥。
    pub async fn api_keys(&self, credential: &str) -> Result<ShijuIamApiKeyListDto, ShijuAppError> {
        let response = self.get_from(&self.iam_base_url, IAM_API_KEYS_PATH, credential).await?;
        let envelope = response
            .json::<IamApiKeyEnvelope>()
            .await
            .map_err(|_| ShijuAppError::upstream("SHIJU IAM API key response is invalid"))?;
        if envelope.code != 200 {
            return Err(ShijuAppError::upstream("SHIJU IAM API key request failed"));
        }
        let api_keys = envelope
            .data
            .list
            .into_iter()
            .map(IamApiKeyRecord::into_dto)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ShijuIamApiKeyListDto { api_keys })
    }

    /// 校验安装标识并保存 API 密钥绑定。
    pub async fn save_api_key_binding(
        &self,
        credential: &str,
        installation_id: &str,
        api_key_id: u64,
    ) -> Result<(), ShijuAppError> {
        validate_installation_id(installation_id)?;
        let binding = self.api_key_binding(credential, installation_id).await?;
        if !binding.personal.can_edit {
            return Err(ShijuAppError::invalid_input(
                "SHIJU application API key binding cannot be edited",
            ));
        }

        let key = self
            .api_keys(credential)
            .await?
            .api_keys
            .into_iter()
            .find(|key| key.api_key_id == api_key_id)
            .ok_or_else(|| {
                ShijuAppError::invalid_input("SHIJU application API key is not owned")
            })?;
        if key.status != "enabled" {
            return Err(ShijuAppError::invalid_input("SHIJU application API key is not enabled"));
        }

        let endpoint = self
            .app_base_url
            .join(&binding_path(installation_id))
            .map_err(|_| ShijuAppError::upstream("SHIJU application upstream URL is invalid"))?;
        let response = self
            .http
            .put(endpoint)
            .header(API_KEY_HEADER, credential_header(credential)?)
            .json(&ApiKeyBindingSaveRecord::from(key))
            .send()
            .await
            .map_err(|_| ShijuAppError::upstream("SHIJU API key binding request failed"))?;
        if !response.status().is_success() {
            return Err(ShijuAppError::upstream("SHIJU API key binding request failed"));
        }
        Ok(())
    }

    /// 建立浏览器会话并返回已校验的应用运行地址。
    ///
    /// 返回地址必须使用 HTTPS、匹配固定运行主机且绑定请求的安装标识。
    pub async fn prepare_launch(
        &self,
        credential: &str,
        installation_id: &str,
    ) -> Result<ShijuAppLaunchDto, ShijuAppError> {
        validate_installation_id(installation_id)?;

        let installed = self.installed(credential).await.map_err(|_| {
            ShijuAppError::upstream("SHIJU launch failed to load installed applications")
        })?;
        let app = installed
            .apps
            .into_iter()
            .find(|app| app.installation_id == installation_id)
            .ok_or_else(|| {
                ShijuAppError::invalid_input("SHIJU application installation is not owned")
            })?;
        // 平台已移除 runtimeStatus，Web 版以平台返回的 runtimeUrl 作为启动入口。
        let app = if app.runtime_url.trim().is_empty() {
            self.installation_detail(credential, installation_id).await.map_err(|_| {
                ShijuAppError::upstream("SHIJU launch failed to load installation detail")
            })?
        } else {
            app
        };
        let runtime_url = app.runtime_url.trim();
        if runtime_url.is_empty() {
            return Err(ShijuAppError::invalid_input("SHIJU application runtime URL is missing"));
        }
        validate_runtime_url(runtime_url, installation_id)?;

        let binding = self.api_key_binding(credential, installation_id).await.map_err(|_| {
            ShijuAppError::upstream("SHIJU launch failed to read the application API key binding")
        })?;
        if !binding.personal.configured {
            return Err(ShijuAppError::invalid_input(
                "SHIJU application API key binding is not configured",
            ));
        }
        let runtime_set_cookie = self.establish_browser_session(credential).await?;

        Ok(ShijuAppLaunchDto {
            installation_id: app.installation_id,
            name: app.name,
            runtime_url: runtime_url.to_owned(),
            runtime_set_cookie,
        })
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        credential: &str,
    ) -> Result<T, ShijuAppError> {
        let response = self.get(path, credential).await?;
        response
            .json::<T>()
            .await
            .map_err(|_| ShijuAppError::upstream("SHIJU application response is invalid"))
    }

    async fn establish_browser_session(&self, credential: &str) -> Result<String, ShijuAppError> {
        let response = self
            .http
            .post(self.browser_session_endpoint.clone())
            .bearer_auth(credential)
            .send()
            .await
            .map_err(|_| ShijuAppError::upstream("SHIJU browser session request failed"))?;
        if !response.status().is_success() {
            let message = match response.status() {
                reqwest::StatusCode::UNAUTHORIZED => "SHIJU browser session rejected credentials (401)",
                reqwest::StatusCode::FORBIDDEN => "SHIJU browser session access denied (403)",
                reqwest::StatusCode::NOT_FOUND => "SHIJU browser session endpoint not found (404)",
                _ => "SHIJU browser session returned an unsuccessful status",
            };
            return Err(ShijuAppError::upstream(message));
        }
        response
            .headers()
            .get_all(SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find(|value| value.starts_with(&format!("{RUNTIME_SESSION_COOKIE_NAME}=")))
            .map(str::to_string)
            .filter(|value| is_valid_runtime_set_cookie(value))
            .ok_or_else(|| {
                ShijuAppError::upstream("SHIJU browser session cookie is missing or invalid")
            })
    }

    async fn get(&self, path: &str, credential: &str) -> Result<reqwest::Response, ShijuAppError> {
        self.get_from(&self.app_base_url, path, credential).await
    }

    async fn get_from(
        &self,
        base_url: &Url,
        path: &str,
        credential: &str,
    ) -> Result<reqwest::Response, ShijuAppError> {
        let endpoint = base_url
            .join(path)
            .map_err(|_| ShijuAppError::upstream("SHIJU application upstream URL is invalid"))?;
        let response = self
            .http
            .get(endpoint)
            .header(API_KEY_HEADER, credential_header(credential)?)
            .send()
            .await
            .map_err(|_| ShijuAppError::upstream("SHIJU application upstream request failed"))?;
        if !response.status().is_success() {
            return Err(ShijuAppError::upstream("SHIJU application upstream request failed"));
        }
        Ok(response)
    }
}

#[derive(Deserialize)]
struct IamApiKeyEnvelope {
    code: i64,
    data: IamApiKeyData,
}

#[derive(Deserialize)]
struct IamApiKeyData {
    list: Vec<IamApiKeyRecord>,
}

#[derive(Deserialize)]
struct IamApiKeyRecord {
    api_key_id: String,
    key_name: String,
    api_key_mask: String,
    status: String,
    expires_at: Option<String>,
    updated_at: Option<String>,
}

impl IamApiKeyRecord {
    fn into_dto(self) -> Result<ShijuIamApiKeyDto, ShijuAppError> {
        let api_key_id = self
            .api_key_id
            .parse::<u64>()
            .map_err(|_| ShijuAppError::upstream("SHIJU IAM API key response is invalid"))?;
        Ok(ShijuIamApiKeyDto {
            api_key_id,
            key_name: self.key_name,
            api_key_mask: self.api_key_mask,
            status: self.status,
            expires_at: self.expires_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiKeyBindingSaveRecord {
    api_key_id: String,
    key_name: String,
    api_key_mask: String,
    status: String,
    expires_at: Option<String>,
    target: &'static str,
}

impl From<ShijuIamApiKeyDto> for ApiKeyBindingSaveRecord {
    fn from(key: ShijuIamApiKeyDto) -> Self {
        Self {
            api_key_id: key.api_key_id.to_string(),
            key_name: key.key_name,
            api_key_mask: key.api_key_mask,
            status: key.status,
            expires_at: key.expires_at,
            target: "personal",
        }
    }
}

fn binding_path(installation_id: &str) -> String {
    format!("tenant/installations/{installation_id}/api-key-binding")
}

fn validate_installation_id(installation_id: &str) -> Result<(), ShijuAppError> {
    if !is_valid_installation_id(installation_id) {
        return Err(ShijuAppError::invalid_input("SHIJU application installation ID is invalid"));
    }
    Ok(())
}

fn is_valid_installation_id(installation_id: &str) -> bool {
    gateway_shiju_app::valid_installation_id(installation_id)
}

fn parse_base_url(raw: &str) -> Result<Url, ShijuAppError> {
    let mut url = Url::parse(raw)
        .map_err(|_| ShijuAppError::upstream("SHIJU application upstream URL is invalid"))?;
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

fn credential_header(credential: &str) -> Result<HeaderValue, ShijuAppError> {
    if credential.trim().is_empty() || credential.trim() != credential {
        return Err(ShijuAppError::invalid_input("SHIJU application credential is invalid"));
    }
    HeaderValue::from_str(credential)
        .map_err(|_| ShijuAppError::invalid_input("SHIJU application credential is invalid"))
}

fn is_valid_runtime_set_cookie(value: &str) -> bool {
    let mut parts = value.split(';').map(str::trim);
    let Some(pair) = parts.next() else {
        return false;
    };
    let Some((name, token)) = pair.split_once('=') else {
        return false;
    };
    if name != RUNTIME_SESSION_COOKIE_NAME
        || token.is_empty()
        || token.chars().any(char::is_control)
    {
        return false;
    }

    let attributes = parts.collect::<Vec<_>>();
    attributes.iter().any(|attribute| attribute.eq_ignore_ascii_case("Path=/"))
        && attributes.iter().any(|attribute| attribute.eq_ignore_ascii_case("HttpOnly"))
        && attributes.iter().any(|attribute| attribute.eq_ignore_ascii_case("Secure"))
        && attributes.iter().any(|attribute| attribute.eq_ignore_ascii_case("SameSite=None"))
}

fn validate_runtime_url(runtime_url: &str, installation_id: &str) -> Result<(), ShijuAppError> {
    let url = Url::parse(runtime_url).map_err(|_| {
        ShijuAppError::invalid_input("SHIJU application runtime URL is not trusted")
    })?;
    let expected_path = format!("/app/{installation_id}/");
    let trusted = url.scheme() == "https"
        && url.host_str() == Some(RUNTIME_HOST)
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.path() == expected_path
        && url.query().is_none()
        && url.fragment().is_none();
    if !trusted {
        return Err(ShijuAppError::invalid_input("SHIJU application runtime URL is not trusted"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "apps_tests.rs"]
mod apps_tests;
