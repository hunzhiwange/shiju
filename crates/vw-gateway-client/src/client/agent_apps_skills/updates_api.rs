use super::super::GatewayClient;
use vw_api_types::agent_apps_skills::{updates_api::*, repos_api::SkillReposQuery};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillUpdateError { Transport, Decode, InvalidId, Http { status: u16 } }
impl std::fmt::Display for SkillUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => f.write_str("skill update transport failed"),
            Self::Decode => f.write_str("skill update response invalid"),
            Self::InvalidId => f.write_str("invalid skill update identity"),
            Self::Http { status } => write!(f, "skill update HTTP {status}"),
        }
    }
}
impl std::error::Error for SkillUpdateError {}
impl GatewayClient {
    pub async fn agent_apps_skills_check_updates(&self, request: &CheckSkillUpdatesRequest, token: &str)
        -> Result<CheckSkillUpdatesResponse, SkillUpdateError> {
        self.skill_update_send("/agent-apps/skills/check-updates", Some(request), None, token).await
    }
    pub async fn agent_apps_skills_update(&self, id: &str, request: &UpdateSkillRequest, token: &str)
        -> Result<SkillUpdateTask, SkillUpdateError> {
        let id = encoded_id(id)?;
        self.skill_update_send(&format!("/agent-apps/skills/{id}/update"), Some(request), None, token).await
    }
    pub async fn agent_apps_skills_update_task(&self, id: &str, scope: &SkillReposQuery, token: &str)
        -> Result<SkillUpdateTask, SkillUpdateError> {
        let id = encoded_id(id)?;
        self.skill_update_send::<(), _>(&format!("/agent-apps/skills/update-tasks/{id}"), None, Some(scope), token).await
    }
    async fn skill_update_send<Q: serde::Serialize, R: serde::de::DeserializeOwned>(&self,
        path: &str, body: Option<&Q>, scope: Option<&SkillReposQuery>, token: &str,
    ) -> Result<R, SkillUpdateError> {
        let client = self.native_transport().map_err(|_| SkillUpdateError::Transport)?;
        let url = format!("{}/v1{path}", self.endpoint.base_url());
        let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| SkillUpdateError::Transport)?;
        credential.set_sensitive(true);
        let request = match body { Some(body) => client.post(url).json(body), None => client.get(url).query(&scope) };
        let response = crate::http::apply_request_context(request.header("x-agent-apps-token", credential), &self.endpoint)
            .send().await.map_err(|_| SkillUpdateError::Transport)?;
        if !response.status().is_success() {
            return Err(SkillUpdateError::Http { status: response.status().as_u16() });
        }
        response.json().await.map_err(|_| SkillUpdateError::Decode)
    }
}
fn encoded_id(id: &str) -> Result<String, SkillUpdateError> {
    if id.is_empty() || id.len() > 255 || id.starts_with('.')
        || id.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':')) {
        return Err(SkillUpdateError::InvalidId);
    }
    Ok(id.bytes().map(|b| {
        if b.is_ascii_alphanumeric() || b"_-".contains(&b) { (b as char).to_string() }
        else { format!("%{b:02X}") }
    }).collect())
}
