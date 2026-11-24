//! CLI workbench requests use the endpoint's authenticated tenant and user.
use gateway_http::apply_request_context;
use serde::{Serialize, de::DeserializeOwned};
use vw_api_types::cli_workbench::*;

use super::GatewayClient;

impl GatewayClient {
    pub async fn cli_workbench_list(
        &self,
        project_id: &str,
    ) -> Result<Vec<CliWorkbenchSession>, CliWorkbenchError> {
        self.cli_workbench_request(reqwest::Method::GET, &workbench_path(project_id), None::<&()>)
            .await
    }

    pub async fn cli_workbench_start(
        &self,
        project_id: &str,
        request: &StartCliWorkbenchSession,
    ) -> Result<CliWorkbenchSession, CliWorkbenchError> {
        self.cli_workbench_request(
            reqwest::Method::POST,
            &workbench_path(project_id),
            Some(request),
        )
        .await
    }

    pub async fn cli_workbench_stop(
        &self,
        project_id: &str,
        session_id: &str,
    ) -> Result<CliWorkbenchSession, CliWorkbenchError> {
        let path = format!("{}/stop", session_path(project_id, session_id));
        self.cli_workbench_request(reqwest::Method::POST, &path, None::<&()>).await
    }

    pub async fn cli_workbench_send(
        &self,
        project_id: &str,
        session_id: &str,
        request: &SendCliWorkbenchMessage,
    ) -> Result<CliWorkbenchMessage, CliWorkbenchError> {
        let path = format!("{}/messages", session_path(project_id, session_id));
        self.cli_workbench_request(reqwest::Method::POST, &path, Some(request)).await
    }

    pub async fn cli_workbench_receive(
        &self,
        project_id: &str,
        session_id: &str,
        request_id: &str,
    ) -> Result<CliWorkbenchMessage, CliWorkbenchError> {
        let path = message_path(project_id, session_id, request_id);
        self.cli_workbench_request(reqwest::Method::GET, &path, None::<&()>).await
    }

    /// Wait for a newer snapshot or a terminal state. An unchanged revision is a heartbeat.
    pub async fn cli_workbench_subscribe(
        &self,
        project_id: &str,
        session_id: &str,
        request_id: &str,
        after_revision: u64,
    ) -> Result<CliWorkbenchMessage, CliWorkbenchError> {
        let path = format!(
            "{}/events?after_revision={after_revision}",
            message_path(project_id, session_id, request_id)
        );
        self.cli_workbench_request(reqwest::Method::GET, &path, None::<&()>).await
    }

    async fn cli_workbench_request<B: Serialize, T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, CliWorkbenchError> {
        let endpoint = self.transport.endpoint();
        let mut request =
            self.transport.client().request(method, format!("{}{path}", endpoint.base_url()));
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = apply_request_context(request, endpoint).send().await.map_err(|_| {
            client_error(CliWorkbenchErrorCode::Disconnected, "gateway connection failed")
        })?;
        let status = response.status();
        if status.is_success() {
            return response.json().await.map_err(|_| {
                client_error(CliWorkbenchErrorCode::GatewayFailure, "invalid gateway response")
            });
        }
        Err(response.json().await.unwrap_or_else(|_| CliWorkbenchError {
            code: if status == reqwest::StatusCode::NOT_FOUND {
                CliWorkbenchErrorCode::NotFound
            } else {
                CliWorkbenchErrorCode::GatewayFailure
            },
            message: format!("gateway returned HTTP {status}"),
        }))
    }
}

fn client_error(code: CliWorkbenchErrorCode, message: &str) -> CliWorkbenchError {
    CliWorkbenchError { code, message: message.into() }
}

fn workbench_path(project_id: &str) -> String {
    let project = gateway_project::encode_path_segment(project_id);
    format!("/v1/projects/{project}/cli-workbench/sessions")
}

fn session_path(project_id: &str, session_id: &str) -> String {
    let session = gateway_project::encode_path_segment(session_id);
    format!("{}/{session}", workbench_path(project_id))
}

fn message_path(project_id: &str, session_id: &str, request_id: &str) -> String {
    let request = gateway_project::encode_path_segment(request_id);
    format!("{}/messages/{request}", session_path(project_id, session_id))
}
