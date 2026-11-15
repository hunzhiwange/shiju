use crate::{GatewayCronClient, types::*};

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

impl GatewayCronClient {
    /// Returns an explicit error on gateways without the schedule preview contract.
    pub async fn cron_schedule_preview(
        &self,
        request: &CronPreviewRequest,
    ) -> Result<CronPreviewResponse, String> {
        self.transport.post_json("/v1/cron/preview", &[], request).await.map_err(|err| {
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
        let response: CronJobsResponse = self.transport.get_json("/v1/cron", &[]).await?;
        Ok(response.jobs)
    }

    pub async fn cron_history_get(&self) -> Result<CronHistoryResponse, String> {
        self.transport.get_json("/v1/cron/history", &[]).await
    }

    pub async fn cron_job_runs_get(&self, job_id: &str) -> Result<Vec<CronRunDto>, String> {
        let job_id = required_job_id(job_id)?;
        let job_id = encode_path_segment(job_id);
        let primary_path = format!("/v1/cron/{job_id}/runs");
        let response = match self.transport.get_json::<CronRunsResponse>(&primary_path, &[]).await {
            Ok(response) => response,
            Err(err) if is_not_found(&err) => {
                let fallback_path = format!("/v1/cron/runs/{job_id}");
                match self.transport.get_json::<CronRunsResponse>(&fallback_path, &[]).await {
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
        match self.transport.get_json::<CronRunsResponse>("/v1/cron/runs/__probe__", &[]).await {
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
            return self.transport.post_json("/v1/cron", &[], request).await;
        }
        match self.transport.post_json("/v1/cron", &[], request).await {
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
            return self.transport.patch_json(&path, &[], request).await;
        }
        match self.transport.patch_json(&path, &[], request).await {
            Ok(response) => Ok(response),
            Err(err) if is_method_not_allowed(&err) => {
                match self.transport.post_json(&path, &[], request).await {
                    Ok(response) => Ok(response),
                    Err(err) if is_method_not_allowed(&err) => {
                        match self.transport.put_json(&path, &[], request).await {
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
        self.transport.delete_empty(&format!("/v1/cron/{}", encode_path_segment(job_id)), &[]).await
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
