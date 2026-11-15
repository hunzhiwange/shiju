//! Cron 快照详情与受限运行历史投影。

use gateway_cron::{CronJobDto, CronUpdateRequest};
use cli_interaction_model::snapshots::{
    CronJobSnapshot, CronDetailView, CronRunSummary, bounded_text,
    MAX_DETAIL_CHARS, MAX_HISTORY_RUNS,
};

use cli_gateway_transport::gateway::{GatewayUiRuntime, block_on_gateway};
use cli_interaction_model::schedule_panel::{SchedulePanel, SchedulePanelItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronMutation {
    SetEnabled(bool),
    Delete,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CronSourceRefresh {
    pub schedules: SchedulePanel,
    pub cron_jobs: Vec<CronJobSnapshot>,
}

impl CronSourceRefresh {
    fn from_jobs(jobs: Vec<CronJobDto>) -> Self {
        let mut refresh = Self::default();
        for job in jobs {
            refresh.cron_jobs.push(CronJobSnapshot::from_dto(&job));
            refresh.schedules.items.push(SchedulePanelItem {
                id: job.id,
                name: job.name.unwrap_or_else(|| "unnamed".to_string()),
                schedule: if job.expression.is_empty() {
                    job.schedule_kind
                } else {
                    job.expression
                },
                next_run: job.next_run,
                last_status: job.last_status,
                enabled: job.enabled,
                history: Vec::new(),
                history_error: None,
            });
        }
        refresh
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_owned_cron_detail_blocking(
    runtime: &GatewayUiRuntime,
    snapshot: &[CronJobSnapshot],
    job_id: &str,
) -> Result<CronDetailView, String> {
    let job_id = job_id.trim();
    let job =
        snapshot.iter().find(|job| job.owns_id(job_id)).cloned().ok_or_else(|| {
            "Cron detail requires an ID from the current Cron snapshot".to_string()
        })?;
    let runs = block_on_gateway(async { gateway_cron::GatewayCronClient::new(runtime.client().transport().clone()).cron_job_runs_get(job_id).await })
        .map_err(|error| {
            format!(
                "failed to load Cron run history: {}",
                bounded_text(error.as_str(), MAX_DETAIL_CHARS)
            )
        })?;
    if runs.iter().any(|run| run.job_id != job_id) {
        return Err("Gateway Cron history contains a run for a different job ID".to_string());
    }
    Ok(CronDetailView {
        job,
        runs: runs.into_iter().take(MAX_HISTORY_RUNS).map(CronRunSummary::from_dto).collect(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn mutate_owned_cron_and_refresh_blocking(
    runtime: &GatewayUiRuntime,
    snapshot: &[CronJobSnapshot],
    job_id: &str,
    mutation: CronMutation,
) -> Result<CronSourceRefresh, String> {
    let job_id = job_id.trim();
    if !snapshot.iter().any(|job| job.owns_id(job_id)) {
        return Err("Cron mutation requires an ID from the current Cron snapshot".to_string());
    }
    block_on_gateway(async {
        match mutation {
            CronMutation::SetEnabled(enabled) => {
                let request =
                    CronUpdateRequest { enabled: Some(enabled), ..CronUpdateRequest::default() };
                gateway_cron::GatewayCronClient::new(runtime.client().transport().clone()).cron_job_update(job_id, &request).await.map(|_| ())?;
            }
            CronMutation::Delete => gateway_cron::GatewayCronClient::new(runtime.client().transport().clone()).cron_job_delete(job_id).await?,
        }
        gateway_cron::GatewayCronClient::new(runtime.client().transport().clone()).cron_jobs_get().await.map(CronSourceRefresh::from_jobs)
    })
}
