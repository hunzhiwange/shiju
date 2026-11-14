use super::super::GatewayClient;
use vw_api_types::agent_apps_skills::discovery_api::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillDiscoveryError {
    Transport,
    Decode,
    Http { status: u16 },
}
impl std::fmt::Display for SkillDiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => f.write_str("skill discovery transport failed"),
            Self::Decode => f.write_str("skill discovery response invalid"),
            Self::Http { status } => write!(f, "skill discovery HTTP {status}"),
        }
    }
}
impl std::error::Error for SkillDiscoveryError {}
impl GatewayClient {
    pub async fn agent_apps_skills_discover(&self, query: &DiscoverSkillsRequest, token: &str)
        -> Result<DiscoverSkillsResponse, SkillDiscoveryError> {
        let mut credential = reqwest::header::HeaderValue::from_str(token)
            .map_err(|_| SkillDiscoveryError::Transport)?;
        credential.set_sensitive(true);
        let request = self.native_transport().map_err(|_| SkillDiscoveryError::Transport)?
            .post(format!("{}/v1/agent-apps/skills/discover", self.endpoint.base_url()))
            .json(query).header("x-agent-apps-token", credential);
        let response = crate::http::apply_request_context(request, &self.endpoint)
            .send().await.map_err(|_| SkillDiscoveryError::Transport)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|_| SkillDiscoveryError::Transport)?;
        if !status.is_success() {
            return Err(SkillDiscoveryError::Http { status: status.as_u16() });
        }
        serde_json::from_slice(&bytes).map_err(|_| SkillDiscoveryError::Decode)
    }
}
