//! Verified video_generate boundary. No media pool or timeline mutation is exposed.
use super::*;
use crate::workflow::video_edit::assets::*;
use anyhow::{Context, Result, ensure};

impl ShijuCloudVideoToolProvider {
    async fn asset_call(&self, operation: &str, args: Value) -> Result<Value> {
        ensure!(
            ["video_generate", "generation_list", "generation_status", "media_probe"]
                .contains(&operation),
            "unsupported asset operation"
        );
        // The typed caller is the only route to generation; no model-facing arbitrary commands.
        let schema: Value = serde_json::from_str(
            &self
                .occ_capture(&["schema".into(), operation.into()])
                .await
                .map_err(anyhow::Error::msg)?,
        )?;
        ensure!(
            schema["name"] == operation && schema["inputSchema"]["type"] == "object",
            "asset schema identity changed"
        );
        let properties =
            schema["inputSchema"]["properties"].as_object().context("missing asset schema")?;
        for (name, value) in args.as_object().context("invalid asset arguments")? {
            let field = properties.get(name).context("asset schema field changed")?;
            ensure!(
                match field["type"].as_str() {
                    Some("string") => value.is_string(),
                    Some("boolean") => value.is_boolean(),
                    Some("integer") => value.is_u64(),
                    _ => false,
                },
                "asset schema type changed"
            );
            if let Some(choices) = field["enum"].as_array() {
                ensure!(choices.contains(value), "asset schema enum changed");
            }
        }
        if let Some(required) = schema["inputSchema"]["required"].as_array() {
            ensure!(
                required.iter().all(|v| v.as_str().is_some_and(|k| args.get(k).is_some())),
                "asset schema requires unsupported field"
            );
        }
        let value =
            self.memory_exchange(json!({"kind":"call","operation":operation,"args":args})).await?;
        ensure!(
            value.is_object() && value.get("error").is_none(),
            "asset transport returned an error; reconcile before retry"
        );
        Ok(value)
    }
    async fn asset_status(&self, task: &str) -> Result<VideoAssetRemoteState> {
        let value = self.asset_call("generation_status", json!({"taskId":task})).await?;
        ensure!(value["taskId"] == task, "asset task identity mismatch");
        Ok(match value["status"].as_str() {
            Some("queued" | "running") => VideoAssetRemoteState::Running(task.into()),
            Some("succeeded") => VideoAssetRemoteState::Succeeded {
                task_id: task.into(),
                reference: required_string(&value, "ref").map_err(anyhow::Error::msg)?.into(),
            },
            Some("failed") => VideoAssetRemoteState::Failed { task_id: task.into() },
            _ => VideoAssetRemoteState::Unknown,
        })
    }
}
#[async_trait::async_trait]
impl VideoAssetCloud for ShijuCloudVideoToolProvider {
    async fn submit(
        &self,
        request: &VideoAssetRequest,
        key: &str,
    ) -> Result<VideoAssetRemoteState> {
        validate_recipe(&request.recipe)?;
        let r = &request.recipe;
        let value = self.asset_call("video_generate", json!({"projectId":request.project_id,"idempotencyKey":key,"prompt":r.prompt,"kind":r.kind,"provider":"wan3","model":r.model,"durationSec":r.duration_sec,"ratio":r.ratio,"resolution":r.resolution,"audio":false})).await?;
        let task = required_string(&value, "taskId").map_err(anyhow::Error::msg)?;
        ensure!(
            matches!(value["status"].as_str(), Some("queued" | "running")),
            "unexpected generation submission receipt"
        );
        Ok(VideoAssetRemoteState::Running(task.into()))
    }
    async fn reconcile(&self, job: &VideoAssetJob) -> Result<VideoAssetRemoteState> {
        if let Some(task) = &job.task_id {
            return self.asset_status(task).await;
        }
        let mut args = json!({"projectId":job.request.project_id,"mine":true,"tool":"video_generate","limit":50,"refresh":true});
        let mut cursors = std::collections::BTreeSet::new();
        let mut found = None;
        loop {
            let value = self.asset_call("generation_list", args.clone()).await?;
            for item in value["items"].as_array().context("invalid generation list")? {
                if item["idempotencyKey"] == job.idempotency_key {
                    ensure!(
                        found.is_none()
                            && item["projectId"] == job.request.project_id
                            && item["tool"] == "video_generate",
                        "ambiguous generation identity"
                    );
                    found = Some(item.clone());
                }
            }
            match value["nextBefore"].as_str().filter(|s| !s.is_empty()) {
                None => break,
                Some(cursor) => {
                    ensure!(
                        cursors.insert(cursor.to_owned()),
                        "generation pagination did not advance"
                    );
                    args["before"] = json!(cursor);
                }
            }
        }
        let Some(item) = found else {
            return Ok(VideoAssetRemoteState::Unknown);
        };
        if let Some(task) = item["taskId"].as_str().filter(|s| !s.is_empty()) {
            return self.asset_status(task).await;
        }
        if item["submittedToProvider"] == false
            && item["status"] == "failed"
            && item["billing"] == "not_submitted"
        {
            return Ok(VideoAssetRemoteState::NotSubmitted);
        }
        Ok(VideoAssetRemoteState::Unknown)
    }
    async fn verify(&self, project: &str, reference: &str) -> Result<()> {
        let value = super::probe::normalize_media_probe(
            self.asset_call("media_probe", json!({"projectId":project,"url":reference})).await?,
        )?;
        ensure!(
            value["streams"]
                .as_array()
                .is_some_and(|a| a.iter().any(|s| s["codec_type"] == "video"))
                && value["durationSec"].as_f64().is_some_and(|n| n.is_finite() && n > 0.0),
            "generated video cannot be read back"
        );
        Ok(())
    }
}
