use super::super::GatewayClient;
use vw_api_types::agent_apps_skills::bindings_api::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillBindingError {
    Transport,
    Decode,
    Http { status: u16 },
}
impl std::fmt::Display for SkillBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => f.write_str("skill distribution transport failed"),
            Self::Decode => f.write_str("skill distribution response invalid"),
            Self::Http { status } => write!(f, "skill distribution HTTP {status}"),
        }
    }
}
impl std::error::Error for SkillBindingError {}
impl GatewayClient {
    pub async fn agent_apps_skills_bindings(&self, id: &str, query: &SkillBindingsRequest, token: &str)
        -> Result<SkillBindingsResponse, SkillBindingError> {
        if id.is_empty() || id == "." || id == ".." || id.contains(['/', '\\', '?', '#', '%']) {
            return Err(SkillBindingError::Transport);
        }
        self.skill_distribution_post(&format!("{id}/bindings"), query, token).await
    }
    pub async fn agent_apps_skills_resync(&self, query: &SkillResyncRequest, token: &str)
        -> Result<SkillBindingsResponse, SkillBindingError> {
        self.skill_distribution_post("resync", query, token).await
    }
    async fn skill_distribution_post(&self, path: &str, query: &impl serde::Serialize, token: &str)
        -> Result<SkillBindingsResponse, SkillBindingError> {
        let mut credential = reqwest::header::HeaderValue::from_str(token)
            .map_err(|_| SkillBindingError::Transport)?;
        credential.set_sensitive(true);
        let request = self.native_transport().map_err(|_| SkillBindingError::Transport)?
            .post(format!("{}/v1/agent-apps/skills/{path}", self.endpoint.base_url()))
            .json(query).header("x-agent-apps-token", credential);
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send().await.map_err(|_| SkillBindingError::Transport)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|_| SkillBindingError::Transport)?;
        if !status.is_success() { return Err(SkillBindingError::Http { status: status.as_u16() }); }
        serde_json::from_slice(&bytes).map_err(|_| SkillBindingError::Decode)
    }
}

impl GatewayClient {
    pub async fn agent_apps_skill_binding_targets(&self, scope: &vw_api_types::agent_apps_skills::repos_api::SkillReposQuery,
        change: Option<&SkillBindingPreferences>, token: &str) -> Result<SkillBindingSnapshot, SkillBindingError> {
        let mut credential = reqwest::header::HeaderValue::from_str(token).map_err(|_| SkillBindingError::Transport)?;
        credential.set_sensitive(true);
        let url = format!("{}/v1/agent-apps/skills/binding-targets", self.endpoint.base_url());
        let client = self.native_transport().map_err(|_| SkillBindingError::Transport)?;
        let request = match change { Some(value) => client.put(url).json(value), None => client.get(url).query(scope) };
        let response = crate::http::apply_request_context(request.header("x-agent-apps-token", credential), &self.endpoint)
            .send().await.map_err(|_| SkillBindingError::Transport)?;
        let status = response.status();
        if !status.is_success() { return Err(SkillBindingError::Http { status: status.as_u16() }); }
        response.json().await.map_err(|_| SkillBindingError::Decode)
    }
}
