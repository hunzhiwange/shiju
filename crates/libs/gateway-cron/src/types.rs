//! 定时任务网关接口及其传输对象。
//!
//! 除当前增删改查路由外，本模块还保留旧网关变更接口的有限回退，以兼容版本滚动升级。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CronJobDto {
    pub timezone: Option<String>,
    pub anchor_date: Option<String>,
    pub weekdays: Option<Vec<u32>>,
    pub hour: Option<u32>,
    pub minute: Option<u32>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,

    pub id: String,
    pub name: Option<String>,
    #[serde(default)]
    pub job_type: String,
    #[serde(default)]
    pub schedule_kind: String,
    #[serde(default)]
    pub expression: String,
    pub at: Option<String>,
    pub every_ms: Option<u64>,
    #[serde(default)]
    pub command: String,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub agent: Option<String>,
    #[serde(default)]
    pub acp_agent: Option<String>,
    pub project_path: Option<String>,
    #[serde(default)]
    pub wake: bool,
    #[serde(default)]
    pub fallbacks: Vec<String>,
    #[serde(default)]
    pub full_access: bool,
    #[serde(default)]
    pub task_pool: bool,
    #[serde(default)]
    pub delivery_mode: String,
    pub delivery_channel: Option<String>,
    pub delivery_to: Option<String>,
    #[serde(default = "default_true")]
    pub delivery_best_effort: bool,
    #[serde(default)]
    pub delete_after_run: bool,
    pub next_run: String,
    pub last_run: Option<String>,
    pub last_status: Option<String>,
    pub last_output: Option<String>,
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct CronJobsResponse {
    #[serde(default)]
    pub jobs: Vec<CronJobDto>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CronRunDto {
    pub id: i64,
    pub job_id: String,
    pub started_at: String,
    pub finished_at: String,
    pub status: String,
    pub output: Option<String>,
    pub duration_ms: Option<i64>,
}

/// 汇总记录独立于旧单任务 DTO，未知 status 保留原值。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CronHistoryRunDto {
    pub id: Option<i64>,
    pub execution_id: Option<String>,
    pub job_id: String,
    pub name: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub status: String,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CronHistoryResponse {
    pub runs: Vec<CronHistoryRunDto>,
    pub finished_limit: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CronRunsResponse {
    #[serde(default)]
    pub runs: Vec<CronRunDto>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CronAddRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekdays: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hour: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minute: Option<u32>,
    #[serde(
        default,
        deserialize_with = "cron_window_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub starts_at: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "cron_window_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub ends_at: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "cron_window_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub start_date: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "cron_window_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub end_date: Option<Option<String>>,

    pub name: Option<String>,
    pub job_type: String,
    pub schedule_kind: String,
    pub schedule: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub every_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acp_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wake: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallbacks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_access: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_pool: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_best_effort: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_after_run: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CronUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekdays: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hour: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minute: Option<u32>,
    #[serde(
        default,
        deserialize_with = "cron_window_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub starts_at: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "cron_window_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub ends_at: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "cron_window_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub start_date: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "cron_window_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub end_date: Option<Option<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub every_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acp_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wake: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallbacks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_access: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_pool: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_best_effort: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_after_run: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

// Missing preserves a window boundary; explicit null clears it.
fn cron_window_field<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CronPreviewRequest {
    pub schedule_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub every_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekdays: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hour: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minute: Option<u32>,
    #[serde(
        default,
        deserialize_with = "cron_window_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub starts_at: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "cron_window_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub ends_at: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "cron_window_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub start_date: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "cron_window_field",
        skip_serializing_if = "Option::is_none"
    )]
    pub end_date: Option<Option<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CronPreviewResponse {
    pub normalized: CronPreviewRequest,
    pub timezone: String,
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CronJobMutationResponse {
    pub status: String,
    pub job: Option<CronJobDto>,
}
