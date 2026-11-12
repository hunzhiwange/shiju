//! 定时任务网关接口及其传输对象。
//!
//! 除当前增删改查路由外，本模块还保留旧网关变更接口的有限回退，以兼容版本滚动升级。

use crate::client::GatewayClient;
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

fn legacy_mutation_success(err: &str) -> Option<CronJobMutationResponse> {
    if err.contains("error decoding response body") || err.contains("EOF while parsing a value") {
        Some(CronJobMutationResponse { status: "ok".to_string(), job: None })
    } else {
        None
    }
}

fn is_method_not_allowed(err: &str) -> bool {
    err.contains("405 Method Not Allowed")
}

fn is_not_found(err: &str) -> bool {
    err.contains("404 Not Found")
}

impl GatewayClient {
    /// Returns an explicit error on gateways without the schedule preview contract.
    pub async fn cron_schedule_preview(
        &self,
        request: &CronPreviewRequest,
    ) -> Result<CronPreviewResponse, String> {
        self.post_json("/v1/cron/preview", &[], request).await.map_err(|err| {
            // Old /cron/{id} update routes can capture "preview" and return an update error or DTO.
            if is_not_found(&err)
                || is_method_not_allowed(&err)
                || err.contains("Failed to update cron job")
                || legacy_mutation_success(&err).is_some()
                || (request.local_at.is_some()
                    && err.contains("RFC3339 time is required for at schedule"))
            {
                "当前网关不支持定时任务预览及扩展时间字段，请升级网关".to_string()
            } else {
                err
            }
        })
    }

    async fn require_schedule_api(&self) -> Result<(), String> {
        // Old update routes reject local_at without absolute at before applying any patch.
        self.cron_schedule_preview(&CronPreviewRequest {
            schedule_kind: "at".into(),
            local_at: Some("9999-01-01T00:00".into()),
            timezone: Some("UTC".into()),
            ..Default::default()
        })
        .await
        .map(|_| ())
    }

    pub async fn cron_jobs_get(&self) -> Result<Vec<CronJobDto>, String> {
        let response: CronJobsResponse = self.get_json("/v1/cron", &[]).await?;
        Ok(response.jobs)
    }

    pub async fn cron_history_get(&self) -> Result<CronHistoryResponse, String> {
        self.get_json("/v1/cron/history", &[]).await
    }

    pub async fn cron_job_runs_get(&self, job_id: &str) -> Result<Vec<CronRunDto>, String> {
        let job_id = required_job_id(job_id)?;
        let job_id = encode_path_segment(job_id);
        let primary_path = format!("/v1/cron/{job_id}/runs");
        let response = match self.get_json::<CronRunsResponse>(&primary_path, &[]).await {
            Ok(response) => response,
            Err(err) if is_not_found(&err) => {
                let fallback_path = format!("/v1/cron/runs/{job_id}");
                match self.get_json::<CronRunsResponse>(&fallback_path, &[]).await {
                    Ok(response) => response,
                    Err(fallback_err) if is_not_found(&fallback_err) => {
                        return Err(
                            "当前网关不支持定时任务历史接口，请重启 VibeWindow gateway/daemon"
                                .to_string(),
                        );
                    }
                    Err(fallback_err) => return Err(fallback_err),
                }
            }
            Err(err) => return Err(err),
        };
        Ok(response.runs)
    }

    pub async fn cron_extended_api_supported(&self) -> Result<bool, String> {
        match self.get_json::<CronRunsResponse>("/v1/cron/runs/__probe__", &[]).await {
            Ok(_) => Ok(true),
            Err(err) if is_not_found(&err) => Ok(false),
            Err(err) => Err(err),
        }
    }

    pub async fn cron_job_add(
        &self,
        request: &CronAddRequest,
    ) -> Result<CronJobMutationResponse, String> {
        if request.schedule_kind == "biweekly"
            || request.timezone.is_some()
            || request.local_at.is_some()
            || request.anchor_date.is_some()
            || request.weekdays.is_some()
            || request.hour.is_some()
            || request.minute.is_some()
            || request.starts_at.is_some()
            || request.ends_at.is_some()
            || request.start_date.is_some()
            || request.end_date.is_some()
        {
            self.require_schedule_api().await?;
            return self.post_json("/v1/cron", &[], request).await;
        }
        match self.post_json("/v1/cron", &[], request).await {
            Ok(response) => Ok(response),
            Err(err) => legacy_mutation_success(&err).ok_or(err),
        }
    }

    pub async fn cron_job_update(
        &self,
        job_id: &str,
        request: &CronUpdateRequest,
    ) -> Result<CronJobMutationResponse, String> {
        let job_id = required_job_id(job_id)?;
        let job_id = encode_path_segment(job_id);
        let path = format!("/v1/cron/{job_id}");
        if request.schedule_kind.as_deref() == Some("biweekly")
            || request.timezone.is_some()
            || request.local_at.is_some()
            || request.anchor_date.is_some()
            || request.weekdays.is_some()
            || request.hour.is_some()
            || request.minute.is_some()
            || request.starts_at.is_some()
            || request.ends_at.is_some()
            || request.start_date.is_some()
            || request.end_date.is_some()
        {
            self.require_schedule_api().await?;
            return self.patch_json(&path, &[], request).await;
        }
        match self.patch_json(&path, &[], request).await {
            Ok(response) => Ok(response),
            Err(err) if is_method_not_allowed(&err) => {
                match self.post_json(&path, &[], request).await {
                    Ok(response) => Ok(response),
                    Err(err) if is_method_not_allowed(&err) => {
                        match self.put_json(&path, &[], request).await {
                            Ok(response) => Ok(response),
                            Err(err) => legacy_mutation_success(&err).ok_or(err),
                        }
                    }
                    Err(err) => legacy_mutation_success(&err).ok_or(err),
                }
            }
            Err(err) => legacy_mutation_success(&err).ok_or(err),
        }
    }

    pub async fn cron_job_delete(&self, job_id: &str) -> Result<(), String> {
        let job_id = required_job_id(job_id)?;
        self.delete_empty(&format!("/v1/cron/{}", encode_path_segment(job_id)), &[]).await
    }
}

fn required_job_id(job_id: &str) -> Result<&str, String> {
    let job_id = job_id.trim();
    if job_id.is_empty() {
        return Err("cron job id must not be empty".to_string());
    }
    Ok(job_id)
}

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
#[path = "cron_api_tests.rs"]
mod cron_api_tests;
