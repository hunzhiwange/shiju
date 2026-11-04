//! Independent delivery checks. A model's ready flag is never delivery evidence.
use super::apply::VideoSourceWindow;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTimelineEvidence {
    pub project_id: String,
    pub revision: String,
    pub source_asset_id: String,
    pub windows: Vec<VideoSourceWindow>,
    pub captions_present: bool,
    pub effect_ids: BTreeSet<String>,
    pub video_readable: bool,
    pub original_audio_present: bool,
    pub duration_sec: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoPreviewEvidence {
    pub project_id: String,
    pub revision: String,
    pub job_id: String,
    pub job_status: String,
    pub media_playable: bool,
    pub duration_sec: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEditDelivery {
    pub project_id: String,
    pub template_sha256: String,
    pub plan_sha256: String,
    pub source_mapping: Vec<VideoSourceWindow>,
    pub timeline: VideoTimelineEvidence,
    pub preview: VideoPreviewEvidence,
    pub viewer_url: String,
}

pub(super) fn validate_preview(
    timeline: &VideoTimelineEvidence,
    preview: &VideoPreviewEvidence,
) -> Result<()> {
    ensure!(
        !timeline.project_id.is_empty()
            && !timeline.revision.is_empty()
            && timeline.video_readable
            && timeline.original_audio_present
            && timeline.captions_present
            && timeline.duration_sec.is_finite()
            && timeline.duration_sec > 0.0
            && !timeline.windows.is_empty(),
        "timeline is empty, unreadable or missing original audio/captions"
    );
    let source_duration = timeline.windows.iter().map(|w| w.source_end).fold(0.0, f64::max);
    super::apply::validate_video_mapping(&timeline.windows, source_duration)?;
    ensure!(
        timeline
            .windows
            .last()
            .is_some_and(|w| (w.timeline_end - timeline.duration_sec).abs() < 0.000001),
        "timeline duration does not match source mapping"
    );
    ensure!(
        preview.project_id == timeline.project_id
            && preview.revision == timeline.revision
            && !preview.job_id.is_empty()
            && preview.job_status == "succeeded"
            && preview.media_playable
            && preview.duration_sec.is_finite()
            && (preview.duration_sec - timeline.duration_sec).abs() <= 0.1,
        "preview is unfinished, stale or unplayable"
    );
    Ok(())
}
pub(super) fn validate_viewer_url(value: &str) -> Result<()> {
    let url = reqwest::Url::parse(value)?;
    ensure!(
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path().strip_prefix("/s/").is_some_and(|s| !s.is_empty()),
        "missing viewer page"
    );
    Ok(())
}
