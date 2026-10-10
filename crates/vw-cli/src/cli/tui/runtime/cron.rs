//! Cron 快照详情与受限运行历史投影。

use vw_gateway_client::{CronJobDto, CronRunDto, CronUpdateRequest};

use super::gateway::{GatewayUiRuntime, block_on_gateway};
use crate::cli::tui::model::integration::sanitize_integration_text;
use crate::cli::tui::model::schedule_panel::{SchedulePanel, SchedulePanelItem};

const MAX_ID_CHARS: usize = 160;
const MAX_LABEL_CHARS: usize = 160;
const MAX_DETAIL_CHARS: usize = 512;
const MAX_OUTPUT_CHARS: usize = 512;
const MAX_FALLBACKS: usize = 8;
const MAX_HISTORY_RUNS: usize = 20;

/// 从 canonical Cron 列表复制的安全、定长详情，不保留传输句柄。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CronJobSnapshot {
    owned_id: String,
    id: String,
    name: Option<String>,
    job_type: String,
    schedule_kind: String,
    expression: String,
    at: Option<String>,
    every_ms: Option<u64>,
    command: String,
    prompt: Option<String>,
    model: Option<String>,
    agent: Option<String>,
    acp_agent: Option<String>,
    project_path: Option<String>,
    wake: String,
    fallbacks: Vec<String>,
    full_access: String,
    task_pool: String,
    delivery_mode: String,
    delivery_channel: Option<String>,
    delivery_to: Option<String>,
    delivery_best_effort: String,
    delete_after_run: String,
    next_run: String,
    last_run: Option<String>,
    last_status: Option<String>,
    last_output: Option<String>,
    enabled: String,
}

impl CronJobSnapshot {
    pub(crate) fn from_dto(job: &CronJobDto) -> Self {
        Self {
            owned_id: job.id.clone(),
            id: bounded_text(job.id.as_str(), MAX_ID_CHARS),
            name: bounded_optional(job.name.as_deref(), MAX_LABEL_CHARS),
            job_type: bounded_text(job.job_type.as_str(), MAX_LABEL_CHARS),
            schedule_kind: bounded_text(job.schedule_kind.as_str(), MAX_LABEL_CHARS),
            expression: bounded_text(job.expression.as_str(), MAX_DETAIL_CHARS),
            at: bounded_optional(job.at.as_deref(), MAX_DETAIL_CHARS),
            every_ms: job.every_ms,
            command: bounded_text(job.command.as_str(), MAX_DETAIL_CHARS),
            prompt: bounded_optional(job.prompt.as_deref(), MAX_DETAIL_CHARS),
            model: bounded_optional(job.model.as_deref(), MAX_LABEL_CHARS),
            agent: bounded_optional(job.agent.as_deref(), MAX_LABEL_CHARS),
            acp_agent: bounded_optional(job.acp_agent.as_deref(), MAX_LABEL_CHARS),
            project_path: bounded_optional(job.project_path.as_deref(), MAX_DETAIL_CHARS),
            wake: yes_no(job.wake).to_string(),
            fallbacks: job
                .fallbacks
                .iter()
                .take(MAX_FALLBACKS)
                .map(|fallback| bounded_text(fallback, MAX_LABEL_CHARS))
                .collect(),
            full_access: yes_no(job.full_access).to_string(),
            task_pool: yes_no(job.task_pool).to_string(),
            delivery_mode: bounded_text(job.delivery_mode.as_str(), MAX_LABEL_CHARS),
            delivery_channel: bounded_optional(job.delivery_channel.as_deref(), MAX_LABEL_CHARS),
            delivery_to: bounded_optional(job.delivery_to.as_deref(), MAX_LABEL_CHARS),
            delivery_best_effort: yes_no(job.delivery_best_effort).to_string(),
            delete_after_run: yes_no(job.delete_after_run).to_string(),
            next_run: bounded_text(job.next_run.as_str(), MAX_DETAIL_CHARS),
            last_run: bounded_optional(job.last_run.as_deref(), MAX_DETAIL_CHARS),
            last_status: bounded_optional(job.last_status.as_deref(), MAX_LABEL_CHARS),
            last_output: bounded_optional(job.last_output.as_deref(), MAX_OUTPUT_CHARS),
            enabled: if job.enabled { "enabled" } else { "disabled" }.to_string(),
        }
    }

    pub(crate) fn owns_id(&self, job_id: &str) -> bool {
        self.owned_id == job_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CronMutation {
    SetEnabled(bool),
    Delete,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct CronSourceRefresh {
    pub(crate) schedules: SchedulePanel,
    pub(crate) cron_jobs: Vec<CronJobSnapshot>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CronRunSummary {
    id: i64,
    started_at: String,
    finished_at: String,
    status: String,
    output: Option<String>,
    duration_ms: Option<i64>,
}

impl CronRunSummary {
    fn from_dto(run: CronRunDto) -> Self {
        Self {
            id: run.id,
            started_at: bounded_text(run.started_at.as_str(), MAX_DETAIL_CHARS),
            finished_at: bounded_text(run.finished_at.as_str(), MAX_DETAIL_CHARS),
            status: bounded_text(run.status.as_str(), MAX_LABEL_CHARS),
            output: bounded_optional(run.output.as_deref(), MAX_OUTPUT_CHARS),
            duration_ms: run.duration_ms,
        }
    }

    fn render_text(&self) -> String {
        format!(
            "#{} · {} · {} → {} · {}\n  输出：{}",
            self.id,
            display(&self.status),
            display(&self.started_at),
            display(&self.finished_at),
            self.duration_ms.map_or_else(|| "耗时未知".to_string(), |value| format!("{value}ms")),
            display_optional(self.output.as_deref()),
        )
    }
}

/// 可直接进入 transcript 的 Cron 详情与历史。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CronDetailView {
    job: CronJobSnapshot,
    runs: Vec<CronRunSummary>,
}

impl CronDetailView {
    pub(crate) fn render_text(&self) -> String {
        let fallbacks = if self.job.fallbacks.is_empty() {
            "—".to_string()
        } else {
            self.job.fallbacks.join(", ")
        };
        let history = if self.runs.is_empty() {
            "暂无运行记录".to_string()
        } else {
            self.runs.iter().map(CronRunSummary::render_text).collect::<Vec<_>>().join("\n")
        };
        format!(
            concat!(
                "Cron 详情\n",
                "名称：{}\nID：{}\n状态：{}\n",
                "任务类型：{}\n调度类型：{}\n表达式：{}\n指定时间：{}\n间隔：{}\n",
                "命令：{}\n提示：{}\n模型：{}\nAgent：{}\nACP Agent：{}\n项目路径：{}\n",
                "唤醒：{}\n回退：{}\n完整访问：{}\n任务池：{}\n",
                "投递模式：{}\n投递频道：{}\n投递目标：{}\n尽力投递：{}\n",
                "运行后删除：{}\n下次运行：{}\n上次运行：{}\n上次状态：{}\n上次输出：{}\n",
                "最近运行（最多 {} 条）：\n{}"
            ),
            display_optional(self.job.name.as_deref()),
            display(&self.job.id),
            self.job.enabled,
            display(&self.job.job_type),
            display(&self.job.schedule_kind),
            display(&self.job.expression),
            display_optional(self.job.at.as_deref()),
            self.job.every_ms.map_or_else(|| "—".to_string(), |value| format!("{value}ms")),
            display(&self.job.command),
            display_optional(self.job.prompt.as_deref()),
            display_optional(self.job.model.as_deref()),
            display_optional(self.job.agent.as_deref()),
            display_optional(self.job.acp_agent.as_deref()),
            display_optional(self.job.project_path.as_deref()),
            self.job.wake,
            fallbacks,
            self.job.full_access,
            self.job.task_pool,
            display(&self.job.delivery_mode),
            display_optional(self.job.delivery_channel.as_deref()),
            display_optional(self.job.delivery_to.as_deref()),
            self.job.delivery_best_effort,
            self.job.delete_after_run,
            display(&self.job.next_run),
            display_optional(self.job.last_run.as_deref()),
            display_optional(self.job.last_status.as_deref()),
            display_optional(self.job.last_output.as_deref()),
            MAX_HISTORY_RUNS,
            history,
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_owned_cron_detail_blocking(
    runtime: &GatewayUiRuntime,
    snapshot: &[CronJobSnapshot],
    job_id: &str,
) -> Result<CronDetailView, String> {
    let job_id = job_id.trim();
    let job =
        snapshot.iter().find(|job| job.owned_id == job_id).cloned().ok_or_else(|| {
            "Cron detail requires an ID from the current Cron snapshot".to_string()
        })?;
    let runs = block_on_gateway(async { runtime.client().cron_job_runs_get(job_id).await })
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
pub(crate) fn mutate_owned_cron_and_refresh_blocking(
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
                runtime.client().cron_job_update(job_id, &request).await.map(|_| ())?;
            }
            CronMutation::Delete => runtime.client().cron_job_delete(job_id).await?,
        }
        runtime.client().cron_jobs_get().await.map(CronSourceRefresh::from_jobs)
    })
}

fn bounded_optional(value: Option<&str>, max_chars: usize) -> Option<String> {
    value.map(|value| bounded_text(value, max_chars)).filter(|value| !value.is_empty())
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    let sanitized = sanitize_integration_text(redact_bearer_value(value).as_str());
    if sanitized.chars().count() <= max_chars {
        return sanitized;
    }
    if max_chars <= 3 {
        return sanitized.chars().take(max_chars).collect();
    }
    format!("{}...", sanitized.chars().take(max_chars.saturating_sub(3)).collect::<String>())
}

fn redact_bearer_value(value: &str) -> String {
    let mut redact_next = false;
    value
        .split_whitespace()
        .map(|word| {
            if redact_next {
                redact_next = false;
                return "[redacted]";
            }
            let normalized = word.to_ascii_lowercase();
            if normalized == "bearer"
                || (normalized.contains("authorization=") && normalized.ends_with("bearer"))
            {
                redact_next = true;
            }
            word
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn display(value: &str) -> &str {
    if value.is_empty() { "—" } else { value }
}

fn display_optional(value: Option<&str>) -> &str {
    value.filter(|value| !value.is_empty()).unwrap_or("—")
}

fn yes_no(value: bool) -> &'static str {
    if value { "是" } else { "否" }
}
