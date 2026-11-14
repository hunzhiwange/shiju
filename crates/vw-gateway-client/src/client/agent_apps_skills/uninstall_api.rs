use super::super::GatewayClient;
use vw_api_types::agent_apps_skills::uninstall_api::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillUninstallError { Transport, Decode, InvalidId, Http { status: u16 } }
impl std::fmt::Display for SkillUninstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => f.write_str("skill uninstall transport failed"),
            Self::Decode => f.write_str("skill uninstall response invalid"),
            Self::InvalidId => f.write_str("invalid skill uninstall identity"),
            Self::Http { status } => write!(f, "skill uninstall HTTP {status}"),
        }
    }
}
impl std::error::Error for SkillUninstallError {}
impl GatewayClient {
    pub async fn agent_apps_skills_uninstall(&self, id: &str, request: &UninstallSkillRequest, token: &str)
        -> Result<UninstallSkillResponse, SkillUninstallError> {
        let id = encoded_id(id)?;
        let client = self.native_transport().map_err(|_| SkillUninstallError::Transport)?;
        let url = format!("{}/v1/agent-apps/skills/{id}", self.endpoint.base_url());
        let mut credential = reqwest::header::HeaderValue::from_str(token)
            .map_err(|_| SkillUninstallError::Transport)?;
        credential.set_sensitive(true);
        let response = crate::http::apply_request_context(
            client.delete(url).json(request).header("x-agent-apps-token", credential), &self.endpoint)
            .send().await.map_err(|_| SkillUninstallError::Transport)?;
        if !response.status().is_success() {
            return Err(SkillUninstallError::Http { status: response.status().as_u16() });
        }
        response.json().await.map_err(|_| SkillUninstallError::Decode)
    }
}
fn encoded_id(id: &str) -> Result<String, SkillUninstallError> {
    if id.is_empty() || id.len() > 255 || id.starts_with('.')
        || id.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':')) {
        return Err(SkillUninstallError::InvalidId);
    }
    Ok(id.bytes().map(|b| {
        if b.is_ascii_alphanumeric() || b"_-".contains(&b) { (b as char).to_string() }
        else { format!("%{b:02X}") }
    }).collect())
}
