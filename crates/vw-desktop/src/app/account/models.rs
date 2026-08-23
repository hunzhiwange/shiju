//! 定义账号凭据、登录挑战与资料编辑所需的领域模型及校验规则。

use reqwest::Url;
use std::fmt;
use std::time::Duration;
use vw_gateway_client::{
    ShijuAccountMeDto, ShijuAccountPasswordUpdateRequest, ShijuAccountProfileDto,
    ShijuAccountUpdateRequest, ShijuCredential, ShijuLoginChallengeDto, ShijuLoginPollDto,
};

const AUTHORIZATION_HOST: &str = "auth.shiju.cloud";

#[derive(Clone, PartialEq, Eq)]
/// 已校验的账号密钥；调试输出始终脱敏，只有显式调用才能读取原值。
pub(crate) struct SecretSkey(String);

impl SecretSkey {
    /// 去除首尾空白并拒绝空密钥，成功后才建立可信凭据类型。
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, AccountModelError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(AccountModelError::new("skey 不能为空"));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// 显式暴露原始密钥，仅供构造受认证请求或安全持久化使用。
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }

    pub(crate) fn gateway_credential(&self) -> Result<ShijuCredential, AccountModelError> {
        ShijuCredential::new(self.0.clone()).map_err(AccountModelError::new)
    }
}

impl fmt::Debug for SecretSkey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
/// 登录表单中的未校验密钥输入；与可持久化的 [`SecretSkey`] 分离。
pub(crate) struct ManualSkeyInput(String);

impl ManualSkeyInput {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }

    pub(crate) fn into_secret(self) -> Result<SecretSkey, AccountModelError> {
        SecretSkey::new(self.0)
    }
}

impl fmt::Debug for ManualSkeyInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 账号所属公司的稳定身份信息。
pub(crate) struct AccountIdentity {
    company_id: String,
    company_name: String,
    founder_user_id: String,
    status: String,
}

impl AccountIdentity {
    pub(crate) fn new(
        company_id: impl Into<String>,
        company_name: impl Into<String>,
        founder_user_id: impl Into<String>,
        status: impl Into<String>,
    ) -> Result<Self, AccountModelError> {
        Ok(Self {
            company_id: company_id.into().trim().to_string(),
            company_name: required(company_name.into(), "company_name")?,
            founder_user_id: founder_user_id.into().trim().to_string(),
            status: required(status.into(), "status")?,
        })
    }

    /// 从网关资料构造身份，并校验界面依赖的必填字段。
    pub(crate) fn from_profile(
        company_name: impl Into<String>,
        status: impl Into<String>,
    ) -> Result<Self, AccountModelError> {
        Ok(Self {
            company_id: String::new(),
            company_name: required(company_name.into(), "company.company_name")?,
            founder_user_id: String::new(),
            status: required(status.into(), "company.status")?,
        })
    }

    pub(crate) fn company_id(&self) -> &str {
        &self.company_id
    }

    pub(crate) fn company_name(&self) -> &str {
        &self.company_name
    }

    pub(crate) fn founder_user_id(&self) -> &str {
        &self.founder_user_id
    }

    pub(crate) fn status(&self) -> &str {
        &self.status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 当前账号的用户资料及公司状态快照。
pub(crate) struct AccountProfile {
    company_status: String,
    name: String,
    account_name: String,
    user_status: String,
    departments: Vec<String>,
    roles: Vec<String>,
}

impl AccountProfile {
    pub(crate) fn new(
        company_status: impl Into<String>,
        name: impl Into<String>,
        account_name: impl Into<String>,
        user_status: impl Into<String>,
        departments: Vec<String>,
        roles: Vec<String>,
    ) -> Result<Self, AccountModelError> {
        Ok(Self {
            company_status: required(company_status.into(), "company.status")?,
            name: required(name.into(), "current_user.name")?,
            account_name: required(account_name.into(), "current_user.account_name")?,
            user_status: required(user_status.into(), "current_user.status")?,
            departments: normalized_values(departments),
            roles: normalized_values(roles),
        })
    }

    pub(crate) fn company_status(&self) -> &str {
        &self.company_status
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn account_name(&self) -> &str {
        &self.account_name
    }

    pub(crate) fn user_status(&self) -> &str {
        &self.user_status
    }

    pub(crate) fn departments(&self) -> &[String] {
        &self.departments
    }

    pub(crate) fn roles(&self) -> &[String] {
        &self.roles
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// 资料编辑器草稿；生成更新请求时再次执行必填字段校验。
pub(crate) struct AccountProfileDraft {
    name: String,
    username: String,
}

impl AccountProfileDraft {
    pub(crate) fn from_me(dto: &ShijuAccountMeDto) -> Result<Self, AccountModelError> {
        Self::new(dto.name.clone(), dto.account_name.clone())
    }

    pub(crate) fn from_profile(profile: &AccountProfile) -> Self {
        Self { name: profile.name().to_string(), username: profile.account_name().to_string() }
    }

    pub(crate) fn new(
        name: impl Into<String>,
        username: impl Into<String>,
    ) -> Result<Self, AccountModelError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(AccountModelError::new("显示名称不能为空"));
        }
        let username = username.into();
        if username.trim().is_empty() {
            return Err(AccountModelError::new("登录账号不能为空"));
        }
        Ok(Self { name: name.trim().to_string(), username: username.trim().to_string() })
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn username(&self) -> &str {
        &self.username
    }

    pub(crate) fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub(crate) fn set_username(&mut self, username: String) {
        self.username = username;
    }

    /// 将草稿转换为网关请求，拒绝空姓名或空账号名。
    pub(crate) fn update_request(&self) -> Result<ShijuAccountUpdateRequest, AccountModelError> {
        let normalized = Self::new(self.name.clone(), self.username.clone())?;
        Ok(ShijuAccountUpdateRequest { name: normalized.name, username: normalized.username })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 密码编辑器中可独立更新的输入字段。
pub(crate) enum AccountPasswordField {
    Current,
    New,
    Confirmation,
}

#[derive(Clone, Default, PartialEq, Eq)]
/// 密码表单的敏感输入包装；调试输出不会泄露内容。
pub(crate) struct AccountPasswordInput(String);

impl AccountPasswordInput {
    pub(crate) fn new(value: String) -> Self {
        Self(value)
    }

    pub(crate) fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Debug for AccountPasswordInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
/// 修改密码所需的当前密码、新密码和确认值。
pub(crate) struct AccountPasswordDraft {
    current_password: String,
    new_password: String,
    confirmation: String,
}

impl AccountPasswordDraft {
    #[cfg(test)]
    pub(crate) fn new(
        current_password: impl Into<String>,
        new_password: impl Into<String>,
        confirmation: impl Into<String>,
    ) -> Self {
        Self {
            current_password: current_password.into(),
            new_password: new_password.into(),
            confirmation: confirmation.into(),
        }
    }

    pub(crate) fn current_password(&self) -> &str {
        &self.current_password
    }

    pub(crate) fn new_password(&self) -> &str {
        &self.new_password
    }

    pub(crate) fn confirmation(&self) -> &str {
        &self.confirmation
    }

    pub(crate) fn set(&mut self, field: AccountPasswordField, value: AccountPasswordInput) {
        let value = value.into_inner();
        match field {
            AccountPasswordField::Current => self.current_password = value,
            AccountPasswordField::New => self.new_password = value,
            AccountPasswordField::Confirmation => self.confirmation = value,
        }
    }

    /// 校验长度、字符组成和确认值后生成修改密码请求。
    pub(crate) fn update_request(
        &self,
    ) -> Result<ShijuAccountPasswordUpdateRequest, AccountModelError> {
        if self.current_password.trim().is_empty() {
            return Err(AccountModelError::new("请输入当前密码"));
        }
        let length = self.new_password.chars().count();
        let has_letter = self.new_password.chars().any(|character| character.is_ascii_alphabetic());
        let has_number = self.new_password.chars().any(|character| character.is_ascii_digit());
        if !(8..=64).contains(&length) || !has_letter || !has_number {
            return Err(AccountModelError::new("新密码需为 8–64 个字符，并同时包含字母和数字"));
        }
        if self.new_password != self.confirmation {
            return Err(AccountModelError::new("两次输入的新密码不一致"));
        }
        Ok(ShijuAccountPasswordUpdateRequest {
            old_password: self.current_password.clone(),
            new_password: self.new_password.clone(),
        })
    }
}

impl fmt::Debug for AccountPasswordDraft {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AccountPasswordDraft")
            .field("current_password", &"<redacted>")
            .field("new_password", &"<redacted>")
            .field("confirmation", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 已由网关验证、可进入账号仓储的账号聚合。
pub(crate) struct VerifiedAccount {
    identity: AccountIdentity,
    profile: Option<AccountProfile>,
    skey: SecretSkey,
}

impl VerifiedAccount {
    #[cfg(test)]
    pub(crate) fn from_parts(identity: AccountIdentity, skey: SecretSkey) -> Self {
        Self { identity, profile: None, skey }
    }

    #[cfg(test)]
    pub(crate) fn from_parts_with_profile(
        identity: AccountIdentity,
        profile: AccountProfile,
        skey: SecretSkey,
    ) -> Self {
        Self { identity, profile: Some(profile), skey }
    }

    pub(crate) fn from_gateway_profile(
        dto: ShijuAccountProfileDto,
        skey: SecretSkey,
    ) -> Result<Self, AccountModelError> {
        let identity = AccountIdentity::from_profile(dto.company_name, dto.company_status.clone())?;
        let profile = AccountProfile::new(
            dto.company_status,
            dto.name,
            dto.account_name,
            dto.user_status,
            dto.departments,
            dto.roles,
        )?;
        Ok(Self { identity, profile: Some(profile), skey })
    }

    #[cfg(test)]
    pub(crate) fn identity(&self) -> &AccountIdentity {
        &self.identity
    }

    #[cfg(test)]
    pub(crate) fn profile(&self) -> Option<&AccountProfile> {
        self.profile.as_ref()
    }

    pub(crate) fn into_parts(self) -> (AccountIdentity, Option<AccountProfile>, SecretSkey) {
        (self.identity, self.profile, self.skey)
    }
}

#[derive(Clone)]
/// 浏览器登录挑战；授权地址只接受受信任的 HTTPS 主机。
pub(crate) struct LoginChallenge {
    state: String,
    authorization_url: Url,
    poll_interval: Duration,
}

impl TryFrom<ShijuLoginChallengeDto> for LoginChallenge {
    type Error = AccountModelError;

    fn try_from(dto: ShijuLoginChallengeDto) -> Result<Self, Self::Error> {
        let state = required(dto.state().to_string(), "state")?;
        let authorization_url = validate_authorization_url(dto.authorization_url())?;
        Ok(Self {
            state,
            authorization_url,
            poll_interval: Duration::from_secs(dto.poll_interval_seconds.clamp(1, 10)),
        })
    }
}

impl fmt::Debug for LoginChallenge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LoginChallenge")
            .field("state", &"<redacted>")
            .field("authorization_url", &"<redacted>")
            .field("poll_interval", &self.poll_interval)
            .finish()
    }
}

impl LoginChallenge {
    pub(crate) fn state(&self) -> &str {
        &self.state
    }

    pub(crate) fn authorization_url(&self) -> &Url {
        &self.authorization_url
    }

    pub(crate) fn poll_interval(&self) -> Duration {
        self.poll_interval
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 浏览器授权轮询的领域结果。
pub(crate) enum LoginPoll {
    /// 授权尚未完成，调用方应按挑战间隔继续轮询。
    Pending,
    /// 授权完成并返回可验证的密钥。
    Approved(SecretSkey),
}

impl From<ShijuLoginPollDto> for LoginPoll {
    fn from(dto: ShijuLoginPollDto) -> Self {
        match dto {
            ShijuLoginPollDto::Pending => Self::Pending,
            ShijuLoginPollDto::Approved { credential } => {
                Self::Approved(SecretSkey(credential.expose().to_string()))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 账号模型校验失败；消息可直接映射为用户可理解的提示。
pub(crate) struct AccountModelError {
    message: String,
}

impl AccountModelError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl fmt::Display for AccountModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AccountModelError {}

fn required(value: String, field: &str) -> Result<String, AccountModelError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AccountModelError::new(format!("账户响应缺少 {field}")))
    } else {
        Ok(value.to_string())
    }
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

fn validate_authorization_url(raw: &str) -> Result<Url, AccountModelError> {
    // 授权链接来自远端响应，固定协议与主机可避免用户凭据被导向第三方。
    let url = Url::parse(raw).map_err(|_| AccountModelError::new("授权地址无效"))?;
    let trusted = url.scheme() == "https"
        && url.host_str() == Some(AUTHORIZATION_HOST)
        && url.username().is_empty()
        && url.password().is_none()
        && matches!(url.port(), None | Some(443));
    if !trusted {
        return Err(AccountModelError::new("授权地址不受信任"));
    }
    Ok(url)
}
