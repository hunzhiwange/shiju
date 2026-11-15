//! Cron 领域客户端兼容转发。

use super::GatewayClient;
pub use gateway_cron::*;

impl GatewayClient {
    pub async fn cron_schedule_preview(
        &self,
        request: &CronPreviewRequest,
    ) -> Result<CronPreviewResponse, String> {
        gateway_cron::GatewayCronClient::new(self.transport().clone())
            .cron_schedule_preview(request).await
    }

    pub async fn cron_jobs_get(&self) -> Result<Vec<CronJobDto>, String> {
        gateway_cron::GatewayCronClient::new(self.transport().clone())
            .cron_jobs_get().await
    }

    pub async fn cron_history_get(&self) -> Result<CronHistoryResponse, String> {
        gateway_cron::GatewayCronClient::new(self.transport().clone())
            .cron_history_get().await
    }

    pub async fn cron_job_runs_get(&self, job_id: &str) -> Result<Vec<CronRunDto>, String> {
        gateway_cron::GatewayCronClient::new(self.transport().clone())
            .cron_job_runs_get(job_id).await
    }

    pub async fn cron_extended_api_supported(&self) -> Result<bool, String> {
        gateway_cron::GatewayCronClient::new(self.transport().clone())
            .cron_extended_api_supported().await
    }

    pub async fn cron_job_add(
        &self,
        request: &CronAddRequest,
    ) -> Result<CronJobMutationResponse, String> {
        gateway_cron::GatewayCronClient::new(self.transport().clone())
            .cron_job_add(request).await
    }

    pub async fn cron_job_update(
        &self,
        job_id: &str,
        request: &CronUpdateRequest,
    ) -> Result<CronJobMutationResponse, String> {
        gateway_cron::GatewayCronClient::new(self.transport().clone())
            .cron_job_update(job_id, request).await
    }

    pub async fn cron_job_delete(&self, job_id: &str) -> Result<(), String> {
        gateway_cron::GatewayCronClient::new(self.transport().clone())
            .cron_job_delete(job_id).await
    }

}

#[cfg(test)]
#[path = "cron_api_tests.rs"]
mod cron_api_tests;
