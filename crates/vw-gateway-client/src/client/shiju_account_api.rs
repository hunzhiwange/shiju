//! SHIJU account gateway transport contract.

use std::fmt;

use serde::de::{DeserializeOwned, Error as _};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::GatewayClient;
use crate::http::{apply_request_context, log_request, log_request_succeeded, transport_error};

const LOGIN_PATH: &str = "/v1/shiju/account/login";
const LOGIN_POLL_PATH: &str = "/v1/shiju/account/login/poll";
const PROFILE_PATH: &str = "/v1/shiju/account/profile";
const ACCOUNT_ME_PATH: &str = "/v1/shiju/account/me";
const ACCOUNT_PASSWORD_PATH: &str = "/v1/shiju/account/me/password";

/// Internal header carrying a single-request SHIJU credential to the local gateway.
pub const SHIJU_CREDENTIAL_HEADER: &str = "x-vibewindow-shiju-credential";

/// A non-empty SHIJU credential whose debug representation never exposes its value.
#[derive(Clone, PartialEq, Eq)]
pub struct ShijuCredential(String);

impl ShijuCredential {
    /// Creates a credential after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("SHIJU credential cannot be empty".to_string());
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Exposes the credential only for controlled request or persistence boundaries.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ShijuCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

impl Serialize for ShijuCredential {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.expose())
    }
}

impl<'de> Deserialize<'de> for ShijuCredential {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

/// Browser-login challenge returned by the local gateway.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuLoginChallengeDto {
    state: String,
    authorization_url: String,
    pub poll_interval_seconds: u64,
}

impl ShijuLoginChallengeDto {
    /// Returns the opaque state used by the polling request.
    pub fn state(&self) -> &str {
        &self.state
    }

    /// Returns the trusted authorization URL validated by the gateway.
    pub fn authorization_url(&self) -> &str {
        &self.authorization_url
    }
}

impl fmt::Debug for ShijuLoginChallengeDto {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ShijuLoginChallengeDto")
            .field("state", &"<redacted>")
            .field("authorization_url", &"<redacted>")
            .field("poll_interval_seconds", &self.poll_interval_seconds)
            .finish()
    }
}

/// Result of polling a SHIJU browser-login challenge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ShijuLoginPollDto {
    Pending,
    Approved { credential: ShijuCredential },
}

/// Account fields currently consumed by the desktop account state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuAccountProfileDto {
    pub company_name: String,
    pub company_status: String,
    pub name: String,
    pub account_name: String,
    pub user_status: String,
    #[serde(default)]
    pub departments: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuAccountDepartmentDto {
    pub department_id: String,
    pub department_name: String,
    pub parent_department_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuAccountRoleDto {
    pub role_id: String,
    pub role_name: String,
    pub role_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuAccountMeDto {
    pub user_id: String,
    pub company_id: String,
    pub account_id: String,
    pub account_name: String,
    pub name: String,
    pub source_type: String,
    pub external_user_id: String,
    pub allow_console_login: bool,
    pub status: String,
    #[serde(default)]
    pub departments: Vec<ShijuAccountDepartmentDto>,
    #[serde(default)]
    pub roles: Vec<ShijuAccountRoleDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShijuAccountUpdateRequest {
    pub name: String,
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShijuAccountUpdateResponseDto {
    pub user_id: String,
    pub company_id: String,
    pub name: String,
    pub status: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShijuAccountPasswordUpdateRequest {
    pub old_password: String,
    pub new_password: String,
}

impl fmt::Debug for ShijuAccountPasswordUpdateRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ShijuAccountPasswordUpdateRequest")
            .field("old_password", &"<redacted>")
            .field("new_password", &"<redacted>")
            .finish()
    }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ShijuLoginRequest {}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ShijuLoginPollRequest<'a> {
    state: &'a str,
}

impl GatewayClient {
    /// Starts a SHIJU browser-login challenge through the local gateway.
    pub async fn shiju_account_login(&self) -> Result<ShijuLoginChallengeDto, String> {
        self.shiju_account_sensitive_request(
            reqwest::Method::POST,
            LOGIN_PATH,
            &[],
            Some(&ShijuLoginRequest {}),
            None,
        )
        .await
    }

    /// Polls a SHIJU login challenge without placing its state in the URL or logs.
    pub async fn shiju_account_login_poll(&self, state: &str) -> Result<ShijuLoginPollDto, String> {
        if state.trim().is_empty() {
            return Err("SHIJU login state cannot be empty".to_string());
        }
        self.shiju_account_sensitive_request(
            reqwest::Method::POST,
            LOGIN_POLL_PATH,
            &[],
            Some(&ShijuLoginPollRequest { state }),
            None,
        )
        .await
    }

    /// Reads the SHIJU account profile using a dedicated internal credential header.
    pub async fn shiju_account_profile(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuAccountProfileDto, String> {
        self.shiju_account_sensitive_request::<ShijuLoginRequest, _>(
            reqwest::Method::GET,
            PROFILE_PATH,
            &[],
            None,
            Some(credential),
        )
        .await
    }

    /// Reads the editable SHIJU account profile.
    pub async fn shiju_account_me(
        &self,
        credential: &ShijuCredential,
    ) -> Result<ShijuAccountMeDto, String> {
        self.shiju_account_sensitive_request::<ShijuLoginRequest, _>(
            reqwest::Method::GET,
            ACCOUNT_ME_PATH,
            &[],
            None,
            Some(credential),
        )
        .await
    }

    /// Updates the editable SHIJU account profile.
    pub async fn shiju_account_update(
        &self,
        credential: &ShijuCredential,
        body: &ShijuAccountUpdateRequest,
    ) -> Result<ShijuAccountUpdateResponseDto, String> {
        self.shiju_account_sensitive_request(
            reqwest::Method::PUT,
            ACCOUNT_ME_PATH,
            &[],
            Some(body),
            Some(credential),
        )
        .await
    }

    /// Updates the current SHIJU account password without logging the request body.
    pub async fn shiju_account_password_update(
        &self,
        credential: &ShijuCredential,
        body: &ShijuAccountPasswordUpdateRequest,
    ) -> Result<(), String> {
        self.shiju_empty_sensitive_request(
            reqwest::Method::PUT,
            ACCOUNT_PASSWORD_PATH,
            &[],
            Some(body),
            Some(credential),
        )
        .await
    }

    pub(super) async fn shiju_account_sensitive_request<B, T>(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(String, String)],
        body: Option<&B>,
        credential: Option<&ShijuCredential>,
    ) -> Result<T, String>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        let response =
            self.shiju_sensitive_request(method.clone(), path, query, body, credential).await?;
        response.json().await.map_err(|error| {
            let message = error.to_string();
            tracing::error!(
                target: "vw_gateway_client",
                method = method.as_str(),
                endpoint = %self.endpoint.describe(),
                path,
                error = %message,
                "SHIJU gateway response JSON decode failed"
            );
            message
        })
    }

    pub(super) async fn shiju_empty_sensitive_request<B>(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(String, String)],
        body: Option<&B>,
        credential: Option<&ShijuCredential>,
    ) -> Result<(), String>
    where
        B: Serialize,
    {
        self.shiju_sensitive_request(method, path, query, body, credential).await?;
        Ok(())
    }

    async fn shiju_sensitive_request<B>(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(String, String)],
        body: Option<&B>,
        credential: Option<&ShijuCredential>,
    ) -> Result<reqwest::Response, String>
    where
        B: Serialize,
    {
        log_request::<()>(method.as_str(), &self.endpoint, path, query, None);
        let mut request = self
            .client
            .request(method.clone(), format!("{}{}", self.endpoint.base_url(), path))
            .query(query);
        if let Some(body) = body {
            request = request.json(body);
        }
        if let Some(credential) = credential {
            request = request.header(SHIJU_CREDENTIAL_HEADER, credential.expose());
        }
        let response = apply_request_context(request, &self.endpoint)
            .send()
            .await
            .map_err(|error| transport_error(method.as_str(), &self.endpoint, path, error))?;
        let status = response.status();
        if !status.is_success() {
            tracing::warn!(
                target: "vw_gateway_client",
                method = method.as_str(),
                endpoint = %self.endpoint.describe(),
                path,
                status = %status,
                "SHIJU gateway request returned non-success status"
            );
            let message = response
                .json::<ShijuGatewayError>()
                .await
                .ok()
                .and_then(|body| {
                    let error = body.error.trim();
                    (!error.is_empty()).then(|| error.to_string())
                })
                .unwrap_or_else(|| format!("SHIJU gateway request failed: {status}"));
            return Err(message);
        }
        log_request_succeeded(method.as_str(), &self.endpoint, path);
        Ok(response)
    }
}

#[derive(Deserialize)]
struct ShijuGatewayError {
    #[serde(default)]
    error: String,
}
