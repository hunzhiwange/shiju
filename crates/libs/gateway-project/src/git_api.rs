use vw_api_git::{
    GitCommandRequest, GitCommandResponse, GitCommitRequest, GitCommitResponse, GitMergeRequest,
    GitMergeResponse,
};

use super::GatewayProjectClient;

impl GatewayProjectClient {
    pub async fn git_repository(
        &self,
        request: &vw_api_git::GitRepositoryRequest,
    ) -> Result<vw_api_git::GitRepositoryDto, String> {
        self.transport.post_json("/v1/git/repository", &[], request).await
    }
    pub async fn git_history_detail(
        &self,
        request: &vw_api_git::GitHistoryDetailRequest,
    ) -> Result<vw_api_git::GitHistoryDetailDto, String> {
        self.transport.post_json("/v1/git/history/detail", &[], request).await
    }
    pub async fn git_remote(
        &self,
        request: &vw_api_git::GitRemoteRequest,
    ) -> Result<vw_api_git::GitRemoteResponse, String> {
        let path = "/v1/git/remote";
        let builder =
            self.transport.client().post(format!("{}{}", self.transport.endpoint().base_url(), path)).json(request);
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder.timeout(std::time::Duration::from_secs(150));
        let response = gateway_http::apply_request_context(builder, self.transport.endpoint())
            .send()
            .await
            .map_err(|error| gateway_http::transport_error("POST", self.transport.endpoint(), path, error))?;
        gateway_http::parse_json_response("POST", self.transport.endpoint(), path, response).await
    }

    /// 通过网关触发一次 Git 提交。
    pub async fn git_commit(
        &self,
        request: &GitCommitRequest,
    ) -> Result<GitCommitResponse, String> {
        self.transport.post_json("/v1/git/commit", &[], request).await
    }

    /// 通过网关执行一次受限 Git 命令。
    pub async fn git_command(
        &self,
        request: &GitCommandRequest,
    ) -> Result<GitCommandResponse, String> {
        self.transport.post_json("/v1/git/command", &[], request).await
    }

    /// 通过网关触发一次 Git 分支合并。
    pub async fn git_merge(&self, request: &GitMergeRequest) -> Result<GitMergeResponse, String> {
        self.transport.post_json("/v1/git/merge", &[], request).await
    }
}

#[cfg(test)]
#[path = "git_api_tests.rs"]
mod git_api_tests;
