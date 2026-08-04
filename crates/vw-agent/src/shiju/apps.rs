//! SHIJU application-platform upstream client.

use std::fmt;
use std::time::Duration;

use reqwest::Url;
use reqwest::header::{HeaderValue, SET_COOKIE};
use serde::de::DeserializeOwned;
use vw_gateway_client::{ShijuAppCatalogDto, ShijuAppLaunchDto, ShijuAppListDto};

const PRODUCTION_BASE_URL: &str = "https://gateway.shiju.cloud/app-platform/api/";
const CATALOG_PATH: &str = "catalog";
const APPS_PATH: &str = "apps";
const BROWSER_SESSION_PATH: &str = "runtime/browser-session";
const API_KEY_HEADER: &str = "X-Api-Key";
const RUNTIME_SESSION_COOKIE_NAME: &str = "app_runtime_session";
const RUNTIME_HOST: &str = "app.shiju.cloud";
const MAX_INSTALLATION_ID_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShijuAppErrorKind {
    InvalidInput,
    Upstream,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShijuAppError {
    kind: ShijuAppErrorKind,
    message: &'static str,
}

impl ShijuAppError {
    fn invalid_input(message: &'static str) -> Self {
        Self {
            kind: ShijuAppErrorKind::InvalidInput,
            message,
        }
    }

    fn upstream(message: &'static str) -> Self {
        Self {
            kind: ShijuAppErrorKind::Upstream,
            message,
        }
    }

    pub(crate) fn kind(&self) -> ShijuAppErrorKind {
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
pub(crate) struct ShijuAppClient {
    http: reqwest::Client,
    base_url: Url,
}

impl ShijuAppClient {
    pub(crate) fn production() -> Result<Self, ShijuAppError> {
        Self::new(PRODUCTION_BASE_URL)
    }

    #[cfg(test)]
    pub(crate) fn with_base_url(base_url: &str) -> Result<Self, ShijuAppError> {
        Self::new(base_url)
    }

    fn new(base_url: &str) -> Result<Self, ShijuAppError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| ShijuAppError::upstream("failed to create SHIJU application client"))?;
        let base_url = parse_base_url(base_url)?;
        Ok(Self { http, base_url })
    }

    pub(crate) async fn catalog(
        &self,
        credential: &str,
    ) -> Result<ShijuAppCatalogDto, ShijuAppError> {
        self.get_json(CATALOG_PATH, credential).await
    }

    pub(crate) async fn installed(
        &self,
        credential: &str,
    ) -> Result<ShijuAppListDto, ShijuAppError> {
        self.get_json(APPS_PATH, credential).await
    }

    pub(crate) async fn prepare_launch(
        &self,
        credential: &str,
        installation_id: &str,
    ) -> Result<ShijuAppLaunchDto, ShijuAppError> {
        if !is_valid_installation_id(installation_id) {
            return Err(ShijuAppError::invalid_input(
                "SHIJU application installation ID is invalid",
            ));
        }

        let installed = self.installed(credential).await?;
        let app = installed
            .apps
            .into_iter()
            .find(|app| app.installation_id == installation_id)
            .ok_or_else(|| {
                ShijuAppError::invalid_input("SHIJU application installation is not owned")
            })?;
        if app.reconcile_status != "ready" || app.runtime_status != "ready" {
            return Err(ShijuAppError::invalid_input(
                "SHIJU application runtime is not ready",
            ));
        }
        validate_runtime_url(&app.runtime_url, installation_id)?;

        let binding_path =
            format!("tenant/installations/{installation_id}/api-key-binding");
        self.get_success(&binding_path, credential).await?;
        let runtime_set_cookie = self.establish_browser_session(credential).await?;

        Ok(ShijuAppLaunchDto {
            installation_id: app.installation_id,
            name: app.name,
            runtime_url: app.runtime_url,
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

    async fn get_success(&self, path: &str, credential: &str) -> Result<(), ShijuAppError> {
        self.get(path, credential).await.map(|_| ())
    }

    async fn establish_browser_session(
        &self,
        credential: &str,
    ) -> Result<String, ShijuAppError> {
        let endpoint = self.base_url.join(BROWSER_SESSION_PATH).map_err(|_| {
            ShijuAppError::upstream("SHIJU application upstream URL is invalid")
        })?;
        let response = self
            .http
            .post(endpoint)
            .header(API_KEY_HEADER, credential_header(credential)?)
            .send()
            .await
            .map_err(|_| ShijuAppError::upstream("SHIJU browser session request failed"))?;
        if !response.status().is_success() {
            return Err(ShijuAppError::upstream(
                "SHIJU browser session request failed",
            ));
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

    async fn get(
        &self,
        path: &str,
        credential: &str,
    ) -> Result<reqwest::Response, ShijuAppError> {
        let endpoint = self
            .base_url
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
            return Err(ShijuAppError::upstream(
                "SHIJU application upstream request failed",
            ));
        }
        Ok(response)
    }
}

pub(crate) fn is_valid_installation_id(installation_id: &str) -> bool {
    !installation_id.is_empty()
        && installation_id.len() <= MAX_INSTALLATION_ID_BYTES
        && installation_id
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
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
        return Err(ShijuAppError::invalid_input(
            "SHIJU application credential is invalid",
        ));
    }
    HeaderValue::from_str(credential).map_err(|_| {
        ShijuAppError::invalid_input("SHIJU application credential is invalid")
    })
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
        && attributes
            .iter()
            .any(|attribute| attribute.eq_ignore_ascii_case("SameSite=None"))
}

fn validate_runtime_url(runtime_url: &str, installation_id: &str) -> Result<(), ShijuAppError> {
    let url = Url::parse(runtime_url)
        .map_err(|_| ShijuAppError::invalid_input("SHIJU application runtime URL is not trusted"))?;
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
        return Err(ShijuAppError::invalid_input(
            "SHIJU application runtime URL is not trusted",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "apps_tests.rs"]
mod apps_tests;
