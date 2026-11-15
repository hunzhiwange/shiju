//! 工作流兼容门面。
use super::GatewayClient;
use gateway_workflow::GatewayWorkflowClient;
pub use gateway_workflow::{GatewayWorkflowDefaultStreamRequest, GatewayWorkflowResponseMode, GatewayWorkflowRunOutcome, GatewayWorkflowStreamError, GatewayWorkflowStreamEvent, GatewayWorkflowStreamRequest};
use vw_api_workflow::{WorkflowRecord, WorkflowRecordDeleteResponse, WorkflowRecordSummary, WorkflowRecordUpsertBody, WorkflowResumeRequest, WorkflowRunResponse};

impl GatewayClient {
    pub async fn workflow_resume(
        &self,
        body: &WorkflowResumeRequest,
    ) -> Result<WorkflowRunResponse, String> {
        GatewayWorkflowClient::new(self.transport.clone()).workflow_resume(body).await
    }

    pub async fn workflow_applications_list(&self) -> Result<Vec<WorkflowRecordSummary>, String> {
        GatewayWorkflowClient::new(self.transport.clone()).workflow_applications_list().await
    }

    pub async fn workflow_application_get(&self, uuid: &str) -> Result<WorkflowRecord, String> {
        GatewayWorkflowClient::new(self.transport.clone()).workflow_application_get(uuid).await
    }

    pub async fn workflow_application_create(
        &self,
        body: &WorkflowRecordUpsertBody,
    ) -> Result<WorkflowRecord, String> {
        GatewayWorkflowClient::new(self.transport.clone()).workflow_application_create(body).await
    }

    pub async fn workflow_application_update(
        &self,
        uuid: &str,
        body: &WorkflowRecordUpsertBody,
    ) -> Result<WorkflowRecord, String> {
        GatewayWorkflowClient::new(self.transport.clone()).workflow_application_update(uuid, body).await
    }

    pub async fn workflow_template_update(
        &self,
        uuid: &str,
        body: &WorkflowRecordUpsertBody,
        expected_updated_at_ms: u64,
    ) -> Result<WorkflowRecord, String> {
        GatewayWorkflowClient::new(self.transport.clone()).workflow_template_update(uuid, body, expected_updated_at_ms).await
    }

    pub async fn workflow_template_restore(
        &self,
        uuid: &str,
        expected_updated_at_ms: u64,
        confirmed: bool,
    ) -> Result<WorkflowRecord, String> {
        GatewayWorkflowClient::new(self.transport.clone()).workflow_template_restore(uuid, expected_updated_at_ms, confirmed).await
    }

    pub async fn workflow_application_delete(
        &self,
        uuid: &str,
    ) -> Result<WorkflowRecordDeleteResponse, String> {
        GatewayWorkflowClient::new(self.transport.clone()).workflow_application_delete(uuid).await
    }

    pub async fn stream_default_workflow_application(
        &self,
        body: &GatewayWorkflowDefaultStreamRequest,
        on_event: impl FnMut(GatewayWorkflowStreamEvent) -> bool,
    ) -> Result<(), String> {
        GatewayWorkflowClient::new(self.transport.clone()).stream_default_workflow_application(body, on_event).await
    }

    pub async fn stream_workflow_application(
        &self,
        uuid: &str,
        body: &GatewayWorkflowStreamRequest,
        on_event: impl FnMut(GatewayWorkflowStreamEvent) -> bool,
    ) -> Result<(), String> {
        GatewayWorkflowClient::new(self.transport.clone()).stream_workflow_application(uuid, body, on_event).await
    }

    pub async fn run_workflow_application(
        &self,
        uuid: &str,
        body: &GatewayWorkflowStreamRequest,
    ) -> Result<GatewayWorkflowRunOutcome, String> {
        GatewayWorkflowClient::new(self.transport.clone()).run_workflow_application(uuid, body).await
    }
}

#[cfg(test)]
#[path = "workflow_facade_tests.rs"]
mod workflow_facade_tests;
