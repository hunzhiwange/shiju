use vw_gateway_client::CronUpdateRequest;

use super::gateway::{GatewayUiRuntime, block_on_gateway};
use super::super::model::schedule_panel::SchedulePanelItem;

pub(crate) trait ScheduleRuntime {
    fn list_schedules(&mut self) -> Result<Vec<SchedulePanelItem>, String>;
    fn set_schedule_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String>;
    fn delete_schedule(&mut self, id: &str) -> Result<(), String>;
}

impl ScheduleRuntime for GatewayUiRuntime {
    fn list_schedules(&mut self) -> Result<Vec<SchedulePanelItem>, String> {
        let jobs = block_on_gateway(self.client().cron_jobs_get())?;
        Ok(jobs
            .into_iter()
            .map(|job| {
                let runs = block_on_gateway(self.client().cron_job_runs_get(job.id.as_str()));
                let (history, history_error) = match runs {
                    Ok(runs) => (
                        runs.into_iter()
                            .take(5)
                            .map(|run| {
                                format!(
                                    "{} {} {}ms",
                                    run.started_at,
                                    run.status,
                                    run.duration_ms.unwrap_or_default()
                                )
                            })
                            .collect(),
                        None,
                    ),
                    Err(error) => (Vec::new(), Some(error)),
                };
                let schedule = if !job.expression.trim().is_empty() {
                    job.expression
                } else if let Some(at) = job.at {
                    at
                } else if let Some(every_ms) = job.every_ms {
                    format!("every {every_ms}ms")
                } else {
                    job.schedule_kind
                };
                SchedulePanelItem {
                    id: job.id.clone(),
                    name: job.name.unwrap_or(job.id),
                    schedule,
                    next_run: job.next_run,
                    last_status: job.last_status,
                    enabled: job.enabled,
                    history,
                    history_error,
                }
            })
            .collect())
    }

    fn set_schedule_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        block_on_gateway(
            self.client()
                .cron_job_update(id, &CronUpdateRequest { enabled: Some(enabled), ..Default::default() }),
        )
        .and_then(|response| {
            (response.status == "ok" || response.job.is_some())
                .then_some(())
                .ok_or_else(|| "gateway rejected cron update".to_string())
        })
    }

    fn delete_schedule(&mut self, id: &str) -> Result<(), String> {
        block_on_gateway(self.client().cron_job_delete(id))
    }
}
