//! Typed ASR transport; signed extraction URLs never leave the memory boundary.
use super::*;
use crate::workflow::video_edit::transcript::VideoTranscriptCloud;
use anyhow::{Context, Result, ensure};

#[async_trait::async_trait]
impl VideoTranscriptCloud for ShijuCloudVideoToolProvider {
    fn context(&self) -> &ToolRuntimeContext {
        &self.context
    }
    async fn probe_transcript_media(&self, project: &str, asset: &str) -> Result<Value> {
        self.prepare_call("media_probe", json!({"projectId":project,"assetId":asset})).await
    }
    async fn transcribe(
        &self,
        project: &str,
        asset: &str,
        key: &str,
        task: Option<&str>,
    ) -> Result<Value> {
        let args = if let Some(task) = task {
            ensure!((8..=2048).contains(&task.len()), "invalid ASR task id");
            json!({"taskId":task})
        } else {
            let response = self
                .prepare_call(
                    "media_audio_extract",
                    json!({"projectId":project,"assetId":asset,"sampleRate":16000,"channels":1}),
                )
                .await?;
            let audio = response["audioUrl"]
                .as_str()
                .context("no completed cloud audio extraction; reconcile before retry")?;
            let url = url::Url::parse(audio)?;
            ensure!(
                url.scheme() == "https"
                    && url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none(),
                "invalid cloud audio URL"
            );
            json!({"audioUrl":audio,"idempotencyKey":key,"words":true})
        };
        self.prepare_call("asr_transcribe", args).await
    }
}
