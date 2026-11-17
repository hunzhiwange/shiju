use crate::GatewayAppSkillsClient;
use vw_api_app_skills::install_api::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillInstallError {
    Transport,
    Decode,
    Http { status: u16 },
}
impl std::fmt::Display for SkillInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => f.write_str("skill installation transport failed"),
            Self::Decode => f.write_str("skill installation response invalid"),
            Self::Http { status } => write!(f, "skill installation HTTP {status}"),
        }
    }
}
impl std::error::Error for SkillInstallError {}
impl GatewayAppSkillsClient {
    pub async fn agent_apps_skills_install(&self, query: &InstallSkillRequest, token: &str)
        -> Result<InstallSkillResponse, SkillInstallError> {
        let request = self.transport.native_transport().map_err(|_| SkillInstallError::Transport)?
            .post(format!("{}/v1/agent-apps/skills/install", self.transport.endpoint().base_url()))
            .json(query);
        let response = gateway_http::apply_request_context(request, self.transport.endpoint())
            .send().await.map_err(|_| SkillInstallError::Transport)?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|_| SkillInstallError::Transport)?;
        if !status.is_success() {
            return Err(SkillInstallError::Http { status: status.as_u16() });
        }
        serde_json::from_slice(&bytes).map_err(|_| SkillInstallError::Decode)
    }
}
