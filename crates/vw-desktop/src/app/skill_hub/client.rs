//! 封装带账号凭据的技能中心 HTTP 请求与响应错误。

use super::models::{ShijuPackageId, ShijuPackageRelease};
use crate::app::account::SecretSkey;
use reqwest::{StatusCode, Url};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt;
use std::time::Duration;

/// 生产技能中心 API 的固定 HTTPS 根地址。
pub(super) const SKILL_HUB_BASE_URL: &str = "https://gateway.shiju.cloud/sj/skill-hub/api/v1/";

#[derive(Clone)]
/// 使用账号密钥访问技能中心的最小 HTTP 客户端。
pub(crate) struct SkillHubClient {
    http: reqwest::Client,
    base_url: Url,
    credential: SecretSkey,
}

impl SkillHubClient {
    /// 使用生产根地址和固定请求超时创建客户端。
    pub(crate) fn new(credential: SecretSkey) -> Result<Self, SkillHubError> {
        Self::build(credential, SKILL_HUB_BASE_URL)
    }

    #[cfg(test)]
    pub(super) fn with_base_url(
        credential: SecretSkey,
        base_url: &str,
    ) -> Result<Self, SkillHubError> {
        Self::build(credential, base_url)
    }

    fn build(credential: SecretSkey, base_url: &str) -> Result<Self, SkillHubError> {
        let base_url = Url::parse(base_url).map_err(|error| {
            SkillHubError::localized("技能中心地址无效: %s", vec![error.to_string()])
        })?;
        let http = reqwest::Client::builder().timeout(Duration::from_secs(20)).build().map_err(
            |error| SkillHubError::localized("无法创建技能中心客户端: %s", vec![error.to_string()]),
        )?;
        Ok(Self { http, base_url, credential })
    }

    /// 获取指定识矩包的最新发布信息。
    pub(crate) async fn latest_shiju_package(
        &self,
        package_id: ShijuPackageId,
    ) -> Result<ShijuPackageRelease, SkillHubError> {
        self.get("skills/hub/latest", &[("package_id", package_id.as_str().to_string())]).await
    }

    async fn get<T>(&self, path: &str, query: &[(&str, String)]) -> Result<T, SkillHubError>
    where
        T: DeserializeOwned,
    {
        let url = self.base_url.join(path).map_err(|error| {
            SkillHubError::localized("技能中心请求地址无效: %s", vec![error.to_string()])
        })?;
        let response = self
            .http
            .get(url)
            .header("X-Api-Key", self.credential.expose())
            .query(query)
            .send()
            .await
            .map_err(|error| {
                SkillHubError::localized("技能中心请求失败: %s", vec![error.to_string()])
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(SkillHubError::http_status(status));
        }
        let envelope = response.json::<ApiEnvelope<T>>().await.map_err(|error| {
            SkillHubError::localized("技能中心响应无法解析: %s", vec![error.to_string()])
        })?;
        if envelope.code != 200 {
            return Err(SkillHubError::business(envelope.code));
        }
        envelope.data.ok_or_else(|| SkillHubError::localized("技能中心响应缺少 data", vec![]))
    }
}

#[derive(Deserialize)]
struct ApiEnvelope<T> {
    code: i64,
    data: Option<T>,
}

#[derive(Clone, PartialEq, Eq)]
/// 技能中心地址、HTTP 状态、业务码或响应解码错误。
pub(crate) struct SkillHubError {
    message: String,
    localized: Option<(String, Vec<String>)>,
}

impl SkillHubError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), localized: None }
    }

    pub(crate) fn localized(source: &str, arguments: Vec<String>) -> Self {
        let message =
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese)
                .format(source, &arguments.iter().cloned().map(Into::into).collect::<Vec<_>>())
                .expect("valid client error");
        Self { message, localized: Some((source.into(), arguments)) }
    }

    pub(crate) fn message(&self) -> crate::app::i18n::format::Message {
        match &self.localized {
            Some((source, arguments)) => crate::app::i18n::format::Message::new(
                source.clone(),
                arguments.iter().cloned().map(Into::into).collect(),
            ),
            None => crate::app::i18n::format::Message::literal(self.message.clone()),
        }
    }

    pub(crate) fn render(&self, i18n: crate::app::i18n::I18n) -> String {
        match &self.localized {
            Some((source, arguments)) => i18n
                .format(source, &arguments.iter().cloned().map(Into::into).collect::<Vec<_>>())
                .expect("valid client error"),
            None => self.message.clone(),
        }
    }

    fn http_status(status: StatusCode) -> Self {
        Self::localized("技能中心请求失败: HTTP %s", vec![status.as_u16().to_string()])
    }

    fn business(code: i64) -> Self {
        Self::localized("技能中心接口返回业务错误 %s", vec![code.to_string()])
    }
}

impl From<String> for SkillHubError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl fmt::Debug for SkillHubError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("SkillHubError").field(&self.message).finish()
    }
}

impl fmt::Display for SkillHubError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SkillHubError {}

impl std::ops::Deref for SkillHubError {
    type Target = str;
    fn deref(&self) -> &str {
        &self.message
    }
}
impl From<&str> for SkillHubError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}
impl PartialEq<str> for SkillHubError {
    fn eq(&self, other: &str) -> bool {
        self.message == other
    }
}
impl PartialEq<&str> for SkillHubError {
    fn eq(&self, other: &&str) -> bool {
        self.message == *other
    }
}

impl From<anyhow::Error> for SkillHubError {
    fn from(error: anyhow::Error) -> Self {
        error.downcast_ref::<Self>().cloned().unwrap_or_else(|| Self::new(error.to_string()))
    }
}
