//! Repository SDK; normal client authentication and tenant context are preserved.
use super::super::GatewayClient;
use vw_api_types::agent_apps_skills::repos_api::*;

impl GatewayClient {
    pub async fn skill_repos_list(&self, scope: &SkillReposQuery) -> Result<SkillReposResponse, String> {
        let project = scope.project_id.as_ref().to_owned();
        self.get_json("/v1/agent-apps/skills/repos", &[
            ("projectId".into(), project), ("targetHostId".into(), scope.target_host_id.clone()),
        ]).await
    }

    /// Saving an existing repository also changes its enabled state.
    pub async fn skill_repo_save(&self, request: &SaveSkillRepoRequest) -> Result<SkillReposResponse, String> {
        self.post_json("/v1/agent-apps/skills/repos", &[], request).await
    }

    pub async fn skill_repo_remove(&self, owner: &str, name: &str,
        request: &RemoveSkillRepoRequest,
    ) -> Result<SkillReposResponse, String> {
        if owner.is_empty() || name.is_empty() || !owner.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)) {
            return Err("invalid skill repository identity".into());
        }
        self.delete_json(&format!("/v1/agent-apps/skills/repos/{owner}:{name}"), &[], request).await
    }
}
