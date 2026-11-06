use vw_api_types::git::{
    GitCommandRequest, GitCommandResponse, GitCommitRequest, GitCommitResponse, GitMergeRequest,
    GitMergeResponse,
};

use super::GatewayClient;

impl GatewayClient {
    pub async fn git_repository(
        &self,
        request: &vw_api_types::git::GitRepositoryRequest,
    ) -> Result<vw_api_types::git::GitRepositoryDto, String> {
        self.post_json("/v1/git/repository", &[], request).await
    }
    pub async fn git_history_detail(
        &self,
        request: &vw_api_types::git::GitHistoryDetailRequest,
    ) -> Result<vw_api_types::git::GitHistoryDetailDto, String> {
        self.post_json("/v1/git/history/detail", &[], request).await
    }
    pub async fn git_remote(
        &self,
        request: &vw_api_types::git::GitRemoteRequest,
    ) -> Result<vw_api_types::git::GitRemoteResponse, String> {
        let path = "/v1/git/remote";
        let builder =
            self.client.post(format!("{}{}", self.endpoint.base_url(), path)).json(request);
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder.timeout(std::time::Duration::from_secs(150));
        let response = crate::http::apply_request_context(builder, &self.endpoint)
            .send()
            .await
            .map_err(|error| crate::http::transport_error("POST", &self.endpoint, path, error))?;
        crate::http::parse_json_response("POST", &self.endpoint, path, response).await
    }

    /// 通过网关触发一次 Git 提交。
    pub async fn git_commit(
        &self,
        request: &GitCommitRequest,
    ) -> Result<GitCommitResponse, String> {
        self.post_json("/v1/git/commit", &[], request).await
    }

    /// 通过网关执行一次受限 Git 命令。
    pub async fn git_command(
        &self,
        request: &GitCommandRequest,
    ) -> Result<GitCommandResponse, String> {
        self.post_json("/v1/git/command", &[], request).await
    }

    /// 通过网关触发一次 Git 分支合并。
    pub async fn git_merge(&self, request: &GitMergeRequest) -> Result<GitMergeResponse, String> {
        self.post_json("/v1/git/merge", &[], request).await
    }
}

#[cfg(test)]
#[path = "git_api_tests.rs"]
mod git_api_tests;
