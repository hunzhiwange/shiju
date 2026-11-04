//! Source-time transcript normalization and leased cloud job reuse.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoTranscriptWord {
    pub word_index: u64,
    pub text: String,
    pub start: f64,
    pub end: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoTranscript {
    pub asset_id: String,
    pub duration_sec: f64,
    pub fps: f64,
    pub text: String,
    pub words: Vec<VideoTranscriptWord>,
    pub source: String,
    pub diagnostics: Vec<String>,
}
fn number(value: &Value) -> Result<f64> {
    let n = value.as_f64().context("media time must be a finite number, never bool/string")?;
    ensure!(n.is_finite() && n >= 0.0, "invalid media time");
    Ok(n)
}
fn frame_rate(probe: &Value, video: &Value) -> Result<f64> {
    let value = video
        .get("avg_frame_rate")
        .or_else(|| video.get("fps"))
        .or_else(|| probe.get("fps"))
        .context("missing probed frame rate")?;
    let fps = if let Some(ratio) = value.as_str() {
        let (n, d) =
            ratio.split_once('/').context("frame rate must be a numeric value or rational")?;
        let n: f64 = n.parse()?;
        let d: f64 = d.parse()?;
        ensure!(n.is_finite() && d.is_finite() && n > 0.0 && d > 0.0, "invalid frame rate ratio");
        n / d
    } else {
        number(value)?
    };
    ensure!(fps.is_finite() && fps > 0.0, "invalid probed frame rate");
    Ok(fps)
}
pub fn normalize_transcript(
    asset: &str,
    probe: &Value,
    receipt: &Value,
) -> Result<VideoTranscript> {
    ensure!(!asset.is_empty(), "missing cloud asset identity");
    ensure!(
        receipt["status"] == "succeeded" && receipt.get("error").is_none(),
        "cloud transcription not successful; template not applicable"
    );
    let streams = probe["streams"].as_array().context("missing probed streams")?;
    ensure!(
        streams.iter().any(|s| s["codec_type"] == "audio"),
        "source has no audio; template not applicable"
    );
    let video =
        streams.iter().find(|s| s["codec_type"] == "video").context("source has no video")?;
    let duration_sec = number(&probe["durationSec"])?;
    ensure!(duration_sec > 0.0, "invalid source duration");
    let fps = frame_rate(probe, video)?;
    let text = receipt["text"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .context("no usable transcript; template not applicable")?
        .to_owned();
    let engine = receipt["engine"]
        .as_str()
        .filter(|s| !s.is_empty())
        .context("missing transcription provenance")?;
    let mut words = Vec::new();
    let mut diagnostics = Vec::new();
    let mut ordinal = 0u64;
    let mut previous_index = None;
    let mut previous_end = 0.0;
    let mut previous_segment_end = 0.0;
    for segment in receipt["segments"].as_array().context("missing source transcript segments")? {
        let from = number(&segment["start"])?;
        let to = number(&segment["end"])?;
        ensure!(
            from <= to && to <= duration_sec && from >= previous_segment_end,
            "invalid source segment interval"
        );
        previous_segment_end = to;
        for word in segment["words"]
            .as_array()
            .context("ASR has no actual word timestamps; template not applicable")?
        {
            let index = match word.get("word_index").or_else(|| word.get("wordIndex")) {
                Some(value) => value.as_u64().context("invalid source word index")?,
                None => ordinal,
            };
            ordinal = ordinal.checked_add(1).context("word index overflow")?;
            ensure!(
                previous_index.is_none_or(|p| index > p),
                "duplicate or reversed source word index"
            );
            previous_index = Some(index);
            let start = number(&word["start"])?;
            let end = number(&word["end"])?;
            ensure!(
                start >= previous_end && start >= from && end >= start && end <= to,
                "invalid source word interval"
            );
            previous_end = end;
            let original = word["text"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .context("empty source word")?;
            if start == end {
                let punctuation = original.chars().all(|c| {
                    c.is_ascii_punctuation()
                        || "，。！？、；：…—“”‘’（）《》【】「」『』·".contains(c)
                });
                ensure!(
                    punctuation && word["punctuation"] == true,
                    "zero duration speech or unconfirmed punctuation"
                );
                diagnostics.push(format!(
                    "excluded confirmed zero-duration punctuation at source word {index}"
                ));
                continue;
            }
            words.push(VideoTranscriptWord {
                word_index: index,
                text: original.into(),
                start,
                end,
            });
        }
    }
    ensure!(!words.is_empty(), "no usable timestamped speech; template not applicable");
    Ok(VideoTranscript {
        asset_id: asset.into(),
        duration_sec,
        fps,
        text,
        words,
        source: format!("asr_transcribe:{engine}"),
        diagnostics,
    })
}

/// Persisted beside the preparation manifest; absence supports existing run records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoTranscriptState {
    Submitting,
    Pending { task_id: String },
    Ready(VideoTranscript),
    Rejected,
}
/// Only cloud transport is replaceable. The tenant ledger and normalization remain real.
#[async_trait::async_trait]
pub trait VideoTranscriptCloud: Send + Sync {
    fn context(&self) -> &crate::tools::ToolRuntimeContext;
    async fn probe_transcript_media(&self, project: &str, asset: &str) -> Result<Value>;
    async fn transcribe(
        &self,
        project: &str,
        asset: &str,
        key: &str,
        task: Option<&str>,
    ) -> Result<Value>;
}
struct TranscriptLease {
    store: super::manifest::VideoEditManifestStore,
    run: super::manifest::VideoEditRunManifest,
}
impl TranscriptLease {
    async fn save(&mut self) -> Result<()> {
        let store = self.store.clone();
        let mut run = self.run.clone();
        self.run = crate::global_database::run_isolated(move || {
            store.save(&mut run, super::prepare::now_ms())?;
            Ok::<_, anyhow::Error>(run)
        })
        .await??;
        Ok(())
    }
}
impl Drop for TranscriptLease {
    fn drop(&mut self) {
        let store = self.store.clone();
        let run = self.run.clone();
        if crate::global_database::run_sync_database(move || store.release(&run)).is_err() {
            tracing::warn!("video transcript lease release failed; expiry required");
        }
    }
}
/// One submission per logical run. Pending jobs are queried, unknown submissions fenced.
/// Callers retry Pending errors against the same prepared run; no synthetic text is returned.
pub async fn transcribe_cloud_video(
    database: crate::global_database::GlobalDatabase,
    cloud: &dyn VideoTranscriptCloud,
    prepared: &super::prepare::PreparedCloudVideo,
) -> Result<VideoTranscript> {
    use super::{
        manifest::*,
        prepare::{cancelled, now_ms},
    };
    use crate::global_database::run_isolated;
    let context = cloud.context();
    ensure!(
        context.tool_use_context().tenant_scope() == prepared.manifest.tenant,
        "native transcript tenant mismatch"
    );
    let store = run_isolated(move || VideoEditManifestStore::new(database)).await??;
    let owner = uuid::Uuid::new_v4().to_string();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
    let run = loop {
        cancelled(context)?;
        let store = store.clone();
        let candidate = prepared.manifest.clone();
        let owner = owner.clone();
        match run_isolated(move || store.claim(candidate, &owner, now_ms())).await? {
            Ok(run) => break run,
            Err(error)
                if error.to_string() == "video run busy: active lease"
                    && tokio::time::Instant::now() < deadline =>
            {
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
            Err(error) => return Err(error),
        }
    };
    let mut lease = TranscriptLease { store, run };
    ensure!(lease.run.state == VideoPrepareState::Ready, "cloud source is not prepared");
    let project = lease.run.project_id.clone().context("missing prepared project")?;
    let asset = lease.run.asset_id.clone().context("missing prepared asset")?;
    ensure!(
        prepared.manifest.project_id.as_ref() == Some(&project)
            && prepared.manifest.asset_id.as_ref() == Some(&asset),
        "prepared cloud identity changed"
    );
    match &lease.run.transcript {
        Some(VideoTranscriptState::Ready(t)) => return Ok(t.clone()),
        Some(VideoTranscriptState::Rejected) => {
            bail!("transcript rejected; template not applicable")
        }
        Some(VideoTranscriptState::Submitting) => {
            bail!("transcript submission unknown; reconcile before retry, refusing resubmission")
        }
        _ => {}
    }
    let probe = cloud.probe_transcript_media(&project, &asset).await?;
    let streams = probe["streams"].as_array().context("missing probed streams")?;
    if !streams.iter().any(|s| s["codec_type"] == "audio") {
        lease.run.transcript = Some(VideoTranscriptState::Rejected);
        lease.save().await?;
        bail!("source has no audio; template not applicable");
    }
    // Validate media before allowing any billable ASR submission.
    let video =
        streams.iter().find(|s| s["codec_type"] == "video").context("source has no video")?;
    ensure!(number(&probe["durationSec"])? > 0.0, "invalid source duration");
    frame_rate(&probe, video)?;
    let task = match &lease.run.transcript {
        Some(VideoTranscriptState::Pending { task_id }) => Some(task_id.clone()),
        None => {
            lease.run.transcript = Some(VideoTranscriptState::Submitting);
            lease.save().await?;
            None
        }
        _ => bail!("invalid transcription state"),
    };
    cancelled(context)?;
    let key = format!("video-asr:{}", lease.run.run_id);
    let response = cloud.transcribe(&project, &asset, &key, task.as_deref()).await?;
    match response["status"].as_str() {
        Some("running" | "queued") => {
            let task_id = response["taskId"]
                .as_str()
                .filter(|s| (8..=2048).contains(&s.len()))
                .context("invalid ASR task identity; reconcile before retry")?;
            ensure!(
                task.as_deref().is_none_or(|t| t == task_id),
                "ASR job identity changed; reconcile original task"
            );
            lease.run.transcript = Some(VideoTranscriptState::Pending { task_id: task_id.into() });
            lease.save().await?;
            bail!("transcription pending; resume the same cloud task");
        }
        Some("failed") => {
            lease.run.transcript = Some(VideoTranscriptState::Rejected);
            lease.save().await?;
            bail!("cloud transcription failed; template not applicable");
        }
        Some("succeeded") => {
            if let Some(id) = response.get("taskId") {
                ensure!(
                    id.as_str().is_some_and(|id| task.as_deref().is_none_or(|t| t == id)),
                    "ASR job identity changed"
                );
            }
            let transcript = match normalize_transcript(&asset, &probe, &response) {
                Ok(t) => t,
                Err(e) => {
                    lease.run.transcript = Some(VideoTranscriptState::Rejected);
                    lease.save().await?;
                    return Err(e);
                }
            };
            lease.run.transcript = Some(VideoTranscriptState::Ready(transcript.clone()));
            lease.save().await?;
            cancelled(context)?;
            Ok(transcript)
        }
        _ => bail!("unknown ASR status; reconcile original job, refusing resubmission"),
    }
}
