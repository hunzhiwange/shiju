//! 封装带账号凭据的技能中心 HTTP 请求与响应错误。

use super::models::{MarketplaceSkill, ShijuPackageId, ShijuPackageRelease};
use gateway_shiju_skill::ShijuMarketplaceSkillDto;
use desktop_account::SecretSkey;
use reqwest::{Method, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

/// 生产技能中心 API 的固定 HTTPS 根地址。
pub(super) const SKILL_HUB_BASE_URL: &str = "https://gateway.shiju.cloud/sj/skill-hub/api/v1/";

#[derive(Clone)]
/// 使用账号密钥访问技能中心的最小 HTTP 客户端。
pub struct SkillHubClient {
    http: reqwest::Client,
    base_url: Url,
    credential: SecretSkey,
}

impl SkillHubClient {
    /// 使用生产根地址和固定请求超时创建客户端。
    pub fn new(credential: SecretSkey) -> Result<Self, SkillHubError> {
        Self::build(credential, SKILL_HUB_BASE_URL)
    }

    #[cfg(test)]
    pub(super) fn with_base_url(
        credential: SecretSkey,
        base_url: &str,
    ) -> Result<Self, SkillHubError> {
        Self::build(credential, base_url)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn build(credential: SecretSkey, base_url: &str) -> Result<Self, SkillHubError> {
        let base_url = Url::parse(base_url).map_err(|error| {
            SkillHubError::localized("技能中心地址无效: %s", vec![error.to_string()])
        })?;
        let builder = reqwest::Client::builder().timeout(Duration::from_secs(20));
        let builder = builder.redirect(reqwest::redirect::Policy::none());
        let http = builder.build().map_err(
            |error| SkillHubError::localized("无法创建技能中心客户端: %s", vec![error.to_string()]),
        )?;
        Ok(Self { http, base_url, credential })
    }

    #[cfg(target_arch = "wasm32")]
    fn build(_credential: SecretSkey, _base_url: &str) -> Result<Self, SkillHubError> {
        // Browser Fetch through reqwest cannot enforce the no-redirect policy for API keys.
        Err(SkillHubError::localized("当前平台不支持原生技能中心连接", vec![]))
    }

    /// 获取指定识矩包的最新发布信息。
    pub async fn latest_shiju_package(
        &self,
        package_id: ShijuPackageId,
    ) -> Result<ShijuPackageRelease, SkillHubError> {
        self.get("skills/hub/latest", &[("package_id", package_id.as_str().to_string())]).await
    }

    pub async fn recommendations(
        &self,
        exclude_ids: Vec<u64>,
    ) -> Result<Vec<MarketplaceSkill>, SkillHubError> {
        #[derive(Deserialize)]
        struct Batch {
            items: Vec<ShijuMarketplaceSkillDto>,
        }

        let mut query = vec![("limit", "4".into())];
        if !exclude_ids.is_empty() {
            let ids = exclude_ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
            query.push(("exclude_ids", ids));
        }
        let batch: Batch = self.get("skills/recommendations", &query).await?;
        Ok(batch.items.into_iter().map(MarketplaceSkill::from).collect())
    }

    pub async fn mutate_maintained_skill(
        &self,
        id: u64,
        action: super::MaintainedSkillAction,
        all: bool,
        department_ids: Vec<String>,
        user_ids: Vec<String>,
    ) -> Result<(), SkillHubError> {
        #[derive(Serialize)]
        struct Scope {
            department_ids: Option<Vec<String>>,
            user_ids: Option<Vec<String>>,
        }
        let path = format!("me/maintained-skills/{id}");
        let data: serde_json::Value = match action {
            super::MaintainedSkillAction::Visibility => self.request(
                Method::PATCH,
                &format!("{path}/visibility"),
                &[],
                Some(&Scope {
                    department_ids: (!all).then_some(department_ids),
                    user_ids: (!all).then_some(user_ids),
                }),
            ).await?,
            super::MaintainedSkillAction::Unpublish => self.request(
                Method::POST, &format!("{path}/unpublish"), &[], None::<&()>,
            ).await?,
            super::MaintainedSkillAction::Delete => self.request(
                Method::DELETE, &path, &[], None::<&()>,
            ).await?,
        };
        if action == super::MaintainedSkillAction::Delete
            && data.get("deleted").and_then(serde_json::Value::as_bool) != Some(true)
        {
            return Err(SkillHubError::localized("技能删除未完成", vec![]));
        }
        Ok(())
    }

    async fn get<T>(&self, path: &str, query: &[(&str, String)]) -> Result<T, SkillHubError>
    where
        T: DeserializeOwned,
    {
        self.request(Method::GET, path, query, None::<&()>).await
    }

    async fn request<T, B>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<&B>,
    ) -> Result<T, SkillHubError>
    where
        T: DeserializeOwned,
        B: Serialize,
    {
        let url = self.base_url.join(path).map_err(|error| {
            SkillHubError::localized("技能中心请求地址无效: %s", vec![error.to_string()])
        })?;
        let request = self
            .http
            .request(method, url)
            .header("X-Api-Key", self.credential.expose())
            .query(query);
        let request = if let Some(body) = body { request.json(body) } else { request };
        let response = request.send()
            .await
            .map_err(|error| {
                SkillHubError::localized("技能中心请求失败: %s", vec![error.without_url().to_string()])
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
pub struct SkillHubError {
    message: String,
    localized: Option<(String, Vec<String>)>,
}

impl SkillHubError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), localized: None }
    }

    pub fn localized(source: &str, arguments: Vec<String>) -> Self {
        let message =
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese)
                .format(source, &arguments.iter().cloned().map(Into::into).collect::<Vec<_>>())
                .expect("valid client error");
        Self { message, localized: Some((source.into(), arguments)) }
    }

    pub fn message(&self) -> desktop_i18n::format::Message {
        match &self.localized {
            Some((source, arguments)) => desktop_i18n::format::Message::new(
                source.clone(),
                arguments.iter().cloned().map(Into::into).collect(),
            ),
            None => desktop_i18n::format::Message::literal(self.message.clone()),
        }
    }

    pub fn render(&self, i18n: desktop_i18n::I18n) -> String {
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
