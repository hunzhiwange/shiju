use crate::GatewayAppSkillsClient;
use vw_api_app_skills::bindings_api::*;

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
impl GatewayAppSkillsClient {
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
        let request = self.transport.native_transport().map_err(|_| SkillBindingError::Transport)?
            .post(format!("{}/v1/agent-apps/skills/{path}", self.transport.endpoint().base_url()))
            .json(query);
        let response = gateway_http::apply_request_context(request, self.transport.endpoint())
            .send().await.map_err(|_| SkillBindingError::Transport)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|_| SkillBindingError::Transport)?;
        if !status.is_success() { return Err(SkillBindingError::Http { status: status.as_u16() }); }
        serde_json::from_slice(&bytes).map_err(|_| SkillBindingError::Decode)
    }
}

impl GatewayAppSkillsClient {
    pub async fn agent_apps_skill_binding_targets(&self, scope: &vw_api_app_skills::repos_api::SkillReposQuery,
        change: Option<&SkillBindingPreferences>, token: &str) -> Result<SkillBindingSnapshot, SkillBindingError> {
        let url = format!("{}/v1/agent-apps/skills/binding-targets", self.transport.endpoint().base_url());
        let client = self.transport.native_transport().map_err(|_| SkillBindingError::Transport)?;
        let request = match change { Some(value) => client.put(url).json(value), None => client.get(url).query(scope) };
        let response = gateway_http::apply_request_context(request, self.transport.endpoint())
            .send().await.map_err(|_| SkillBindingError::Transport)?;
        let status = response.status();
        if !status.is_success() { return Err(SkillBindingError::Http { status: status.as_u16() }); }
        response.json().await.map_err(|_| SkillBindingError::Decode)
    }
}
