//! SHIJU 账户与身份服务的上游客户端。

use std::fmt;
use std::time::Duration;

use reqwest::{StatusCode, Url};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use vw_gateway_client::{
    ShijuAccountDepartmentDto, ShijuAccountMeDto, ShijuAccountPasswordUpdateRequest,
    ShijuAccountProfileDto, ShijuAccountRoleDto, ShijuAccountUpdateRequest,
    ShijuAccountUpdateResponseDto,
};

const PASSPORT_BASE_URL: &str = "https://passport.shiju.cloud";
const GATEWAY_BASE_URL: &str = "https://gateway.shiju.cloud/sj/";
const LOGIN_PATH: &str = "/api/v1/plugin/login";
const POLL_PATH: &str = "/api/v1/plugin/auth/token";
const PROFILE_PATH: &str = "/iam/api/v1/dashboard/basic-info";
const ACCOUNT_ME_PATH: &str = "/iam/api/v1/me";
const ACCOUNT_PASSWORD_PATH: &str = "/iam/api/v1/me/password";
const LOGIN_SOURCE: &str = "shiju-cli";
const AUTHORIZATION_HOST: &str = "auth.shiju.cloud";

#[path = "model_token.rs"]
mod model_token;

#[derive(Debug, Clone, PartialEq, Eq)]
/// SHIJU 账户接口的输入或上游响应错误。
pub(crate) enum ShijuAccountError {
    Message(&'static str),
    Upstream { status: StatusCode, message: String },
}

impl fmt::Display for ShijuAccountError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(message) => formatter.write_str(message),
            Self::Upstream { message, .. } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ShijuAccountError {}

impl ShijuAccountError {
    /// 返回上游 HTTP 状态；本地校验错误没有状态码。
    pub(crate) fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Message(_) => None,
            Self::Upstream { status, .. } => Some(*status),
        }
    }
}

#[derive(Clone)]
/// 浏览器授权开始后用于轮询的挑战信息。
pub(crate) struct ShijuLoginChallenge {
    state: String,
    authorization_url: Url,
    poll_interval_seconds: u64,
}

impl fmt::Debug for ShijuLoginChallenge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ShijuLoginChallenge")
            .field("state", &"<redacted>")
            .field("authorization_url", &"<redacted>")
            .field("poll_interval_seconds", &self.poll_interval_seconds)
            .finish()
    }
}

impl ShijuLoginChallenge {
    /// 返回绑定本次登录流程的状态标识。
    pub(crate) fn state(&self) -> &str {
        &self.state
    }

    /// 返回已通过主机与协议校验的授权地址。
    pub(crate) fn authorization_url(&self) -> &str {
        self.authorization_url.as_str()
    }

    /// 返回上游建议的轮询间隔秒数。
    pub(crate) fn poll_interval_seconds(&self) -> u64 {
        self.poll_interval_seconds
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 登录轮询尚在等待或已经取得凭证的结果。
pub(crate) enum ShijuAccountPoll {
    Pending,
    Approved(String),
}

#[derive(Clone)]
/// SHIJU Passport 与账户网关客户端。
pub(crate) struct ShijuAccountClient {
    http: reqwest::Client,
    passport_base: Url,
    gateway_base: Url,
}

impl ShijuAccountClient {
    /// 使用固定生产端点创建客户端。
    pub(crate) fn production() -> Result<Self, ShijuAccountError> {
        Self::new(PASSPORT_BASE_URL, GATEWAY_BASE_URL)
    }

    #[cfg(test)]
    /// 使用显式基础地址创建客户端，供受控环境与测试使用。
    pub(crate) fn with_base_urls(
        passport_base: &str,
        gateway_base: &str,
    ) -> Result<Self, ShijuAccountError> {
        Self::new(passport_base, gateway_base)
    }

    fn new(passport_base: &str, gateway_base: &str) -> Result<Self, ShijuAccountError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| ShijuAccountError::Message("failed to create SHIJU account client"))?;
        Ok(Self {
            http,
            passport_base: parse_base_url(passport_base)?,
            gateway_base: parse_base_url(gateway_base)?,
        })
    }

    /// 发起浏览器授权登录并返回轮询挑战。
    pub(crate) async fn start_login(&self) -> Result<ShijuLoginChallenge, ShijuAccountError> {
        let response = self
            .http
            .post(endpoint(&self.passport_base, LOGIN_PATH)?)
            .json(&serde_json::json!({"source": LOGIN_SOURCE}))
            .send()
            .await
            .map_err(|_| ShijuAccountError::Message("SHIJU login request failed"))?;
        let data: LoginData = decode_response(response, "SHIJU login request failed").await?;
        let state = required(data.state, "SHIJU login response is missing state")?;
        let authorization_url = validate_authorization_url(&data.auth_url)?;
        Ok(ShijuLoginChallenge {
            state,
            authorization_url,
            poll_interval_seconds: data.poll_interval.clamp(1, 10),
        })
    }

    /// 使用状态标识轮询登录结果。
    pub(crate) async fn poll_login(
        &self,
        state: &str,
    ) -> Result<ShijuAccountPoll, ShijuAccountError> {
        if state.trim().is_empty() {
            return Err(ShijuAccountError::Message("SHIJU login state cannot be empty"));
        }
        let response = self
            .http
            .get(endpoint(&self.passport_base, POLL_PATH)?)
            .query(&[("state", state)])
            .send()
            .await
            .map_err(|_| ShijuAccountError::Message("SHIJU login poll failed"))?;
        let data: PollData = decode_response(response, "SHIJU login poll failed").await?;
        if !data.api_key.trim().is_empty() {
            return Ok(ShijuAccountPoll::Approved(data.api_key.trim().to_string()));
        }
        match data.status.trim().to_ascii_lowercase().as_str() {
            "pending" | "waiting" => Ok(ShijuAccountPoll::Pending),
            "approved" | "authorized" | "success" | "completed" => {
                Err(ShijuAccountError::Message("SHIJU approved login is missing credential"))
            }
            "denied" | "cancelled" | "canceled" => {
                Err(ShijuAccountError::Message("SHIJU login authorization was cancelled"))
            }
            "expired" => Err(ShijuAccountError::Message("SHIJU login authorization expired")),
            _ => Err(ShijuAccountError::Message("SHIJU login status is invalid")),
        }
    }

    /// 使用凭证读取当前账户基础资料。
    pub(crate) async fn profile(
        &self,
        credential: &str,
    ) -> Result<ShijuAccountProfileDto, ShijuAccountError> {
        if credential.trim().is_empty() {
            return Err(ShijuAccountError::Message("SHIJU credential cannot be empty"));
        }
        let response = self
            .http
            .get(endpoint(&self.gateway_base, PROFILE_PATH)?)
            .header("X-Api-Key", credential)
            .send()
            .await
            .map_err(|_| ShijuAccountError::Message("SHIJU account profile request failed"))?;
        let data: ProfileData =
            decode_response(response, "SHIJU account profile request failed").await?;
        Ok(ShijuAccountProfileDto {
            company_name: required(
                data.company.company_name,
                "SHIJU profile is missing company name",
            )?,
            company_status: required(
                data.company.status,
                "SHIJU profile is missing company status",
            )?,
            name: required(data.current_user.name, "SHIJU profile is missing user name")?,
            account_name: required(
                data.current_user.account_name,
                "SHIJU profile is missing account name",
            )?,
            user_status: required(
                data.current_user.status,
                "SHIJU profile is missing user status",
            )?,
            departments: normalized_values(data.current_user.departments),
            roles: normalized_values(data.current_user.roles),
        })
    }

    /// 读取当前用户、角色与部门信息。
    pub(crate) async fn current_user(
        &self,
        credential: &str,
    ) -> Result<ShijuAccountMeDto, ShijuAccountError> {
        if credential.trim().is_empty() {
            return Err(ShijuAccountError::Message("SHIJU credential cannot be empty"));
        }
        let response = self
            .http
            .get(endpoint(&self.gateway_base, ACCOUNT_ME_PATH)?)
            .header("X-Api-Key", credential)
            .send()
            .await
            .map_err(|_| ShijuAccountError::Message("SHIJU account details request failed"))?;
        let data: AccountMeData =
            decode_response(response, "SHIJU account details request failed").await?;
        account_me_dto(data)
    }

    /// 校验并更新当前用户可编辑资料。
    pub(crate) async fn update_current_user(
        &self,
        credential: &str,
        request: &ShijuAccountUpdateRequest,
    ) -> Result<ShijuAccountUpdateResponseDto, ShijuAccountError> {
        if credential.trim().is_empty() {
            return Err(ShijuAccountError::Message("SHIJU credential cannot be empty"));
        }
        if request.name.trim().is_empty() {
            return Err(ShijuAccountError::Message("SHIJU account name cannot be empty"));
        }
        if request.username.trim().is_empty() {
            return Err(ShijuAccountError::Message("SHIJU account username cannot be empty"));
        }
        let response = self
            .http
            .put(endpoint(&self.gateway_base, ACCOUNT_ME_PATH)?)
            .header("X-Api-Key", credential)
            .json(request)
            .send()
            .await
            .map_err(|_| ShijuAccountError::Message("SHIJU account update request failed"))?;
        let data: AccountUpdateData =
            decode_response(response, "SHIJU account update request failed").await?;
        Ok(ShijuAccountUpdateResponseDto {
            user_id: required(data.user_id, "SHIJU account update is missing user ID")?,
            company_id: required(data.company_id, "SHIJU account update is missing company ID")?,
            name: required(data.name, "SHIJU account update is missing name")?,
            status: required(data.status, "SHIJU account update is missing status")?,
        })
    }

    /// 校验并更新当前账户密码。
    pub(crate) async fn update_password(
        &self,
        credential: &str,
        request: &ShijuAccountPasswordUpdateRequest,
    ) -> Result<(), ShijuAccountError> {
        if credential.trim().is_empty() {
            return Err(ShijuAccountError::Message("SHIJU credential cannot be empty"));
        }
        validate_password_update_request(request)?;
        let response = self
            .http
            .put(endpoint(&self.gateway_base, ACCOUNT_PASSWORD_PATH)?)
            .header("X-Api-Key", credential)
            .json(request)
            .send()
            .await
            .map_err(|_| ShijuAccountError::Message("SHIJU password update request failed"))?;
        decode_empty_response(response, "SHIJU password update request failed").await
    }
}

/// 校验密码更新请求的必填项、一致性与最小长度。
pub(crate) fn validate_password_update_request(
    request: &ShijuAccountPasswordUpdateRequest,
) -> Result<(), ShijuAccountError> {
    if request.old_password.trim().is_empty() {
        return Err(ShijuAccountError::Message("SHIJU current password cannot be empty"));
    }
    let length = request.new_password.chars().count();
    let has_letter = request.new_password.chars().any(|character| character.is_ascii_alphabetic());
    let has_number = request.new_password.chars().any(|character| character.is_ascii_digit());
    if !(8..=64).contains(&length) || !has_letter || !has_number {
        return Err(ShijuAccountError::Message(
            "SHIJU new password must be 8-64 characters and contain letters and numbers",
        ));
    }
    Ok(())
}

fn parse_base_url(raw: &str) -> Result<Url, ShijuAccountError> {
    let mut url =
        Url::parse(raw).map_err(|_| ShijuAccountError::Message("SHIJU upstream URL is invalid"))?;
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

fn endpoint(base: &Url, path: &str) -> Result<Url, ShijuAccountError> {
    base.join(path.trim_start_matches('/'))
        .map_err(|_| ShijuAccountError::Message("SHIJU upstream URL is invalid"))
}

fn required(value: String, message: &'static str) -> Result<String, ShijuAccountError> {
    let value = value.trim();
    if value.is_empty() { Err(ShijuAccountError::Message(message)) } else { Ok(value.to_string()) }
}

fn normalized_values(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|value| {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_string())
        })
        .collect()
}

fn validate_authorization_url(raw: &str) -> Result<Url, ShijuAccountError> {
    let url = Url::parse(raw)
        .map_err(|_| ShijuAccountError::Message("SHIJU authorization URL is invalid"))?;
    let trusted = url.scheme() == "https"
        && url.host_str() == Some(AUTHORIZATION_HOST)
        && url.username().is_empty()
        && url.password().is_none()
        && matches!(url.port(), None | Some(443));
    if !trusted {
        return Err(ShijuAccountError::Message("SHIJU authorization URL is not trusted"));
    }
    Ok(url)
}

async fn decode_response<T: DeserializeOwned>(
    response: reqwest::Response,
    operation_error: &'static str,
) -> Result<T, ShijuAccountError> {
    if !response.status().is_success() {
        return Err(ShijuAccountError::Message(operation_error));
    }
    let envelope = response
        .json::<ApiEnvelope<T>>()
        .await
        .map_err(|_| ShijuAccountError::Message(operation_error))?;
    if envelope.code != 200 {
        return Err(ShijuAccountError::Message(operation_error));
    }
    envelope.data.ok_or(ShijuAccountError::Message(operation_error))
}

async fn decode_empty_response(
    response: reqwest::Response,
    operation_error: &'static str,
) -> Result<(), ShijuAccountError> {
    let http_status = response.status();
    let envelope = response
        .json::<ApiEnvelope<serde_json::Value>>()
        .await
        .map_err(|_| ShijuAccountError::Message(operation_error))?;
    if envelope.code != 200 {
        return Err(upstream_response_error(
            business_error_status(envelope.code),
            envelope.message,
            operation_error,
        ));
    }
    if !http_status.is_success() {
        return Err(upstream_response_error(http_status, envelope.message, operation_error));
    }
    Ok(())
}

#[derive(Deserialize)]
struct ApiEnvelope<T> {
    code: i64,
    #[serde(default)]
    message: String,
    data: Option<T>,
}

fn business_error_status(code: i64) -> StatusCode {
    code.to_string()
        .get(..3)
        .and_then(|prefix| prefix.parse::<u16>().ok())
        .and_then(|status| StatusCode::from_u16(status).ok())
        .filter(|status| status.is_client_error() || status.is_server_error())
        .unwrap_or(StatusCode::BAD_REQUEST)
}

fn upstream_response_error(
    status: StatusCode,
    message: String,
    operation_error: &'static str,
) -> ShijuAccountError {
    let message = message.trim();
    let message =
        if !message.is_empty() { message.to_string() } else { operation_error.to_string() };
    ShijuAccountError::Upstream { status, message }
}

#[derive(Deserialize)]
struct LoginData {
    #[serde(default)]
    state: String,
    #[serde(rename = "authUrl", default)]
    auth_url: String,
    #[serde(default, alias = "interval", alias = "pollInterval")]
    poll_interval: u64,
}

#[derive(Deserialize)]
struct PollData {
    #[serde(default)]
    status: String,
    #[serde(default, alias = "apiKey")]
    api_key: String,
}

#[derive(Deserialize)]
struct ProfileData {
    company: CompanyData,
    current_user: UserData,
}

#[derive(Deserialize)]
struct CompanyData {
    company_name: String,
    status: String,
}

#[derive(Deserialize)]
struct UserData {
    name: String,
    account_name: String,
    status: String,
    #[serde(default)]
    departments: Vec<String>,
    #[serde(default)]
    roles: Vec<String>,
}

#[derive(Deserialize)]
struct AccountMeData {
    user_id: String,
    company_id: String,
    account_id: String,
    account_name: String,
    name: String,
    source_type: String,
    external_user_id: String,
    allow_console_login: bool,
    status: String,
    #[serde(default)]
    departments: Vec<AccountDepartmentData>,
    #[serde(default)]
    roles: Vec<AccountRoleData>,
}

#[derive(Deserialize)]
struct AccountDepartmentData {
    department_id: String,
    department_name: String,
    parent_department_id: String,
}

#[derive(Deserialize)]
struct AccountRoleData {
    role_id: String,
    role_name: String,
    role_code: String,
}

#[derive(Deserialize)]
struct AccountUpdateData {
    user_id: String,
    company_id: String,
    name: String,
    status: String,
}

fn account_me_dto(data: AccountMeData) -> Result<ShijuAccountMeDto, ShijuAccountError> {
    Ok(ShijuAccountMeDto {
        user_id: required(data.user_id, "SHIJU account details are missing user ID")?,
        company_id: required(data.company_id, "SHIJU account details are missing company ID")?,
        account_id: required(data.account_id, "SHIJU account details are missing account ID")?,
        account_name: required(
            data.account_name,
            "SHIJU account details are missing account name",
        )?,
        name: required(data.name, "SHIJU account details are missing name")?,
        source_type: data.source_type.trim().to_string(),
        external_user_id: data.external_user_id.trim().to_string(),
        allow_console_login: data.allow_console_login,
        status: required(data.status, "SHIJU account details are missing status")?,
        departments: data
            .departments
            .into_iter()
            .filter_map(|department| {
                let department_name = department.department_name.trim();
                (!department_name.is_empty()).then(|| ShijuAccountDepartmentDto {
                    department_id: department.department_id.trim().to_string(),
                    department_name: department_name.to_string(),
                    parent_department_id: department.parent_department_id.trim().to_string(),
                })
            })
            .collect(),
        roles: data
            .roles
            .into_iter()
            .filter_map(|role| {
                let role_name = role.role_name.trim();
                (!role_name.is_empty()).then(|| ShijuAccountRoleDto {
                    role_id: role.role_id.trim().to_string(),
                    role_name: role_name.to_string(),
                    role_code: role.role_code.trim().to_string(),
                })
            })
            .collect(),
    })
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod account_tests;
