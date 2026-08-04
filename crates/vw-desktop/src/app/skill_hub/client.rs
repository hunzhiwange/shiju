use super::models::{ShijuPackageId, ShijuPackageRelease};
use crate::app::account::SecretSkey;
use reqwest::{StatusCode, Url};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt;
use std::time::Duration;

const SKILL_HUB_BASE_URL: &str = "https://gateway.shiju.cloud/skill-hub/api/v1/";

#[derive(Clone)]
pub(crate) struct SkillHubClient {
    http: reqwest::Client,
    base_url: Url,
    credential: SecretSkey,
}

impl SkillHubClient {
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
        let base_url = Url::parse(base_url)
            .map_err(|error| SkillHubError::new(format!("技能中心地址无效: {error}")))?;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|error| SkillHubError::new(format!("无法创建技能中心客户端: {error}")))?;
        Ok(Self { http, base_url, credential })
    }

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
        let url = self
            .base_url
            .join(path)
            .map_err(|error| SkillHubError::new(format!("技能中心请求地址无效: {error}")))?;
        let response = self
            .http
            .get(url)
            .header("X-Api-Key", self.credential.expose())
            .query(query)
            .send()
            .await
            .map_err(|error| SkillHubError::new(format!("技能中心请求失败: {error}")))?;
        let status = response.status();
        if !status.is_success() {
            return Err(SkillHubError::http_status(status));
        }
        let envelope = response
            .json::<ApiEnvelope<T>>()
            .await
            .map_err(|error| SkillHubError::new(format!("技能中心响应无法解析: {error}")))?;
        if envelope.code != 200 {
            return Err(SkillHubError::business(envelope.code));
        }
        envelope.data.ok_or_else(|| SkillHubError::new("技能中心响应缺少 data"))
    }
}

#[derive(Deserialize)]
struct ApiEnvelope<T> {
    code: i64,
    data: Option<T>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SkillHubError {
    message: String,
}

impl SkillHubError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    fn http_status(status: StatusCode) -> Self {
        Self::new(format!("技能中心请求失败: HTTP {}", status.as_u16()))
    }

    fn business(code: i64) -> Self {
        Self::new(format!("技能中心接口返回业务错误 {code}"))
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
