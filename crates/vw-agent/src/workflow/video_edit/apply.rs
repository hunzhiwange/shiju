//! Serial, reconciled project writes. Only the external cloud boundary is replaceable.
use super::{assets::VideoAssetRegistry, planning::*, preview::*};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoSourceWindow {
    pub source_start: f64,
    pub source_end: f64,
    pub timeline_start: f64,
    pub timeline_end: f64,
}
/// Windows come from cloud read-back after cleanup, never an accumulated-duration guess.
pub fn validate_video_mapping(windows: &[VideoSourceWindow], duration: f64) -> Result<()> {
    ensure!(duration.is_finite() && duration > 0.0 && !windows.is_empty(), "empty mapping");
    let mut end = 0.0;
    let mut source_end = 0.0;
    for w in windows {
        ensure!(
            [w.source_start, w.source_end, w.timeline_start, w.timeline_end]
                .iter()
                .all(|n| n.is_finite())
                && w.source_start >= source_end
                && w.source_end > w.source_start
                && w.source_end <= duration
                && (w.timeline_start - end).abs() < 0.000001
                && w.timeline_end > w.timeline_start
                && ((w.timeline_end - w.timeline_start) - (w.source_end - w.source_start)).abs()
                    < 0.000001,
            "invalid source-to-timeline mapping; retimed/reordered source unsupported"
        );
        source_end = w.source_end;
        end = w.timeline_end;
    }
    Ok(())
}
pub fn video_timeline_range(
    windows: &[VideoSourceWindow],
    start: f64,
    end: f64,
) -> Result<(f64, f64)> {
    ensure!(start.is_finite() && end.is_finite() && start < end, "invalid source interval");
    let w = windows
        .iter()
        .find(|w| start >= w.source_start && end <= w.source_end)
        .context("effect crosses a removed source window; replan required")?;
    Ok((w.timeline_start + start - w.source_start, w.timeline_start + end - w.source_start))
}
/// A template selection must be resolved before writes. Free-text asset descriptions are not IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoEffectMaterial {
    GeneratedVideo { digest: String },
    MotionGraphic { template_name: String },
    Keyword,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoProjectWrite {
    Import {
        reference: String,
    },
    Original {
        asset_id: String,
    },
    Transcript {
        asset_id: String,
        words: Vec<super::transcript::VideoTranscriptWord>,
    },
    /// Verified service safe cleanup; no retakes or final export.
    Cleanup,
    /// Uses the service's default caption preset.
    Captions,
    Effect {
        effect: VideoEditEffect,
        material: VideoEffectMaterial,
        start: f64,
        end: f64,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoWriteStatus {
    Applied,
    /// Deterministic cleanup found no eligible changes; not a successful write.
    VerifiedNoOp,
    /// Unknown acceptance is fenced; never retransmit it without reconciliation.
    Unknown,
    NoChange,
    Failed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoWriteEntry {
    pub key: String,
    pub write: VideoProjectWrite,
    pub status: VideoWriteStatus,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoApplyLedger {
    pub plan_sha256: String,
    pub template_sha256: String,
    pub entries: Vec<VideoWriteEntry>,
    pub mapping: Vec<VideoSourceWindow>,
    pub cancelled: bool,
}
/// Persist before every side effect. The caller must hold a tenant/project database fence
/// across this entire function, including preview. A run-key lease alone is insufficient.
#[async_trait::async_trait]
pub trait VideoProjectCloud: Send + Sync {
    async fn read_timeline(&self, project: &str) -> Result<VideoTimelineEvidence>;
    async fn write(
        &self,
        project: &str,
        key: &str,
        write: &VideoProjectWrite,
    ) -> Result<VideoWriteStatus>;
    async fn reconcile(&self, project: &str, entry: &VideoWriteEntry) -> Result<VideoWriteStatus>;
    async fn preview(&self, project: &str, revision: &str) -> Result<VideoPreviewEvidence>;
    async fn viewer_url(&self, project: &str) -> Result<String>;
}
#[async_trait::async_trait]
pub trait VideoApplyJournal: Send + Sync {
    /// Must reject another active writer for the SAME tenant/project, across runs/processes.
    async fn claim(&self, project: &str) -> Result<VideoApplyLedger>;
    async fn save(&self, project: &str, ledger: &VideoApplyLedger) -> Result<()>;
    async fn release(&self, project: &str) -> Result<()>;
}

pub struct VideoApplyInput<'a> {
    pub project_id: &'a str,
    pub template_sha256: &'a str,
    pub plan_sha256: &'a str,
    pub plan: &'a VideoEditPlan,
    pub transcript: &'a super::transcript::VideoTranscript,
    pub segments: &'a [super::segments::VideoSegment],
    pub requirements: &'a VideoPlanRequirements,
    pub template_snapshot: &'a str,
    pub registry: &'a VideoAssetRegistry,
    pub materials: &'a BTreeMap<String, VideoEffectMaterial>,
    pub cancel: &'a CancellationToken,
}
pub fn video_plan_sha256(plan: &VideoEditPlan) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(plan)?)))
}
fn validate_input(input: &VideoApplyInput<'_>) -> Result<()> {
    ensure!(
        !input.project_id.is_empty() && !input.plan.asset_id.is_empty(),
        "missing project/source"
    );
    ensure!(
        input.template_sha256.len() == 64 && hex::decode(input.template_sha256).is_ok(),
        "invalid template digest"
    );
    ensure!(video_plan_sha256(input.plan)? == input.plan_sha256, "plan digest mismatch");
    ensure!(
        input.plan.duration_sec.is_finite() && input.plan.duration_sec > 0.0,
        "invalid source duration"
    );
    ensure!(
        input.registry.progress.failed == 0
            && input.registry.progress.running == 0
            && input.registry.progress.queued == 0,
        "asset registry not ready"
    );
    ensure!(
        hex::encode(Sha256::digest(input.template_snapshot.as_bytes())) == input.template_sha256,
        "template snapshot digest mismatch"
    );
    let chunks = input
        .segments
        .iter()
        .map(|segment| VideoChunkPlan {
            chunk_id: segment.chunk_id.clone(),
            effects: input
                .plan
                .effects
                .iter()
                .filter(|effect| effect.chunk_id == segment.chunk_id)
                .cloned()
                .collect(),
        })
        .collect();
    let verified = super::merge::merge_video_edit_plans(
        input.transcript,
        input.segments,
        chunks,
        input.requirements,
    )?;
    ensure!(verified == *input.plan, "plan content differs from source-validated plan");
    let mut ids = BTreeSet::new();
    for e in &input.plan.effects {
        ensure!(
            !e.id.is_empty()
                && ids.insert(&e.id)
                && !e.anchor.trim().is_empty()
                && e.word_index <= e.end_word_index
                && e.start.is_finite()
                && e.end.is_finite()
                && e.start >= 0.0
                && e.start < e.end
                && e.end <= input.plan.duration_sec
                && !e.assets.is_empty()
                && e.assets.iter().all(|s| !s.trim().is_empty()),
            "invalid effect content"
        );
        let material = input.materials.get(&e.id).context("missing resolved effect material")?;
        match (e.visual_type, material) {
            (VisualType::Keyword, VideoEffectMaterial::Keyword) => {}
            (VisualType::BRoll, VideoEffectMaterial::GeneratedVideo { digest }) => {
                ensure!(
                    e.assets.len() == 1,
                    "multiple-material placement requires an explicit composition contract"
                );
                let reference =
                    input.registry.references.get(digest).context("missing generated material")?;
                ensure!(
                    reference.starts_with("gen:")
                        && reference.len() > 4
                        && !reference.contains(['?', '&', '\n']),
                    "invalid generated reference"
                );
            }
            (VisualType::MotionGraphic, VideoEffectMaterial::MotionGraphic { template_name }) => {
                ensure!(
                    !template_name.trim().is_empty()
                        && e.assets.len() == 1
                        && e.assets[0] == *template_name,
                    "missing/mismatched exact motion graphic template"
                );
            }
            _ => bail!("unsupported effect/material combination"),
        }
    }
    ensure!(input.materials.len() == ids.len(), "unexpected material outside plan");
    Ok(())
}
fn entry_complete(entry: &VideoWriteEntry) -> bool {
    matches!(entry.status, VideoWriteStatus::Applied)
        || (matches!(entry.status, VideoWriteStatus::VerifiedNoOp)
            && matches!(entry.write, VideoProjectWrite::Cleanup))
}
async fn step(
    cloud: &dyn VideoProjectCloud,
    journal: &dyn VideoApplyJournal,
    input: &VideoApplyInput<'_>,
    ledger: &mut VideoApplyLedger,
    write: VideoProjectWrite,
) -> Result<()> {
    ensure!(!input.cancel.is_cancelled() && !ledger.cancelled, "video application cancelled");
    let key = hex::encode(Sha256::digest(serde_json::to_vec(&(
        input.project_id,
        input.plan_sha256,
        &write,
    ))?));
    if let Some(index) = ledger.entries.iter().position(|e| e.key == key) {
        if entry_complete(&ledger.entries[index]) {
            return Ok(());
        }
        ensure!(
            matches!(ledger.entries[index].status, VideoWriteStatus::Unknown),
            "terminal no-change/failed write requires a new validated plan"
        );
        let status = cloud
            .reconcile(input.project_id, &ledger.entries[index])
            .await
            .unwrap_or(VideoWriteStatus::Unknown);
        ledger.entries[index].status = status;
        journal.save(input.project_id, ledger).await?;
        ensure!(
            entry_complete(&ledger.entries[index]),
            "write unresolved/no change/failed; refusing retransmission"
        );
        return Ok(());
    }
    // An earlier unresolved write may not be bypassed by another operation.
    ensure!(ledger.entries.iter().all(entry_complete), "previous write unresolved");
    ledger.entries.push(VideoWriteEntry {
        key: key.clone(),
        write: write.clone(),
        status: VideoWriteStatus::Unknown,
    });
    journal.save(input.project_id, ledger).await?;
    let status =
        cloud.write(input.project_id, &key, &write).await.unwrap_or(VideoWriteStatus::Unknown);
    ledger.entries.last_mut().context("missing write intent")?.status = status;
    journal.save(input.project_id, ledger).await?;
    ensure!(
        ledger.entries.last().is_some_and(entry_complete),
        "write unresolved/no change/failed; reconcile before continuing"
    );
    Ok(())
}

pub async fn apply_video_edit(
    cloud: &dyn VideoProjectCloud,
    journal: &dyn VideoApplyJournal,
    input: VideoApplyInput<'_>,
) -> Result<VideoEditDelivery> {
    validate_input(&input)?;
    ensure!(!input.cancel.is_cancelled(), "video application cancelled");
    let mut ledger = journal.claim(input.project_id).await?;
    let result = apply_claimed(cloud, journal, &input, &mut ledger).await;
    // Cancellation is durable, but does not assert that remote operations were cancelled.
    if input.cancel.is_cancelled() {
        ledger.cancelled = true;
        journal.save(input.project_id, &ledger).await?;
    }
    journal.release(input.project_id).await?;
    result
}
async fn apply_claimed(
    cloud: &dyn VideoProjectCloud,
    journal: &dyn VideoApplyJournal,
    input: &VideoApplyInput<'_>,
    ledger: &mut VideoApplyLedger,
) -> Result<VideoEditDelivery> {
    if ledger.plan_sha256.is_empty() {
        let current = cloud.read_timeline(input.project_id).await?;
        ensure!(
            current.project_id == input.project_id
                && current.windows.is_empty()
                && current.effect_ids.is_empty()
                && current.duration_sec == 0.0,
            "existing project content requires reconciliation; refusing overwrite"
        );
        ledger.plan_sha256 = input.plan_sha256.into();
        ledger.template_sha256 = input.template_sha256.into();
        journal.save(input.project_id, ledger).await?;
    }
    ensure!(
        ledger.plan_sha256 == input.plan_sha256 && ledger.template_sha256 == input.template_sha256,
        "application ledger digest mismatch"
    );
    for reference in input
        .materials
        .values()
        .filter_map(|m| match m {
            VideoEffectMaterial::GeneratedVideo { digest } => input.registry.references.get(digest),
            _ => None,
        })
        .collect::<BTreeSet<_>>()
    {
        step(
            cloud,
            journal,
            input,
            ledger,
            VideoProjectWrite::Import { reference: reference.clone() },
        )
        .await?;
    }
    step(
        cloud,
        journal,
        input,
        ledger,
        VideoProjectWrite::Original { asset_id: input.plan.asset_id.clone() },
    )
    .await?;
    step(
        cloud,
        journal,
        input,
        ledger,
        VideoProjectWrite::Transcript {
            asset_id: input.plan.asset_id.clone(),
            words: input.transcript.words.clone(),
        },
    )
    .await?;
    step(cloud, journal, input, ledger, VideoProjectWrite::Cleanup).await?;
    let cleaned = cloud.read_timeline(input.project_id).await?;
    ensure!(
        cleaned.project_id == input.project_id
            && cleaned.source_asset_id == input.plan.asset_id
            && cleaned.video_readable
            && cleaned.original_audio_present,
        "cleanup lost source video/original audio"
    );
    validate_video_mapping(&cleaned.windows, input.plan.duration_sec)?;
    if ledger.mapping.is_empty() {
        ledger.mapping = cleaned.windows.clone();
        journal.save(input.project_id, ledger).await?;
    }
    ensure!(ledger.mapping == cleaned.windows, "cloud source mapping changed outside ledger");
    step(cloud, journal, input, ledger, VideoProjectWrite::Captions).await?;
    for effect in &input.plan.effects {
        let (start, end) = video_timeline_range(&ledger.mapping, effect.start, effect.end)?;
        step(
            cloud,
            journal,
            input,
            ledger,
            VideoProjectWrite::Effect {
                effect: effect.clone(),
                material: input.materials[&effect.id].clone(),
                start,
                end,
            },
        )
        .await?;
    }
    ensure!(!input.cancel.is_cancelled(), "video application cancelled");
    let timeline = cloud.read_timeline(input.project_id).await?;
    ensure!(
        timeline.project_id == input.project_id
            && timeline.source_asset_id == input.plan.asset_id
            && timeline.windows == ledger.mapping
            && input.plan.effects.iter().all(|e| timeline.effect_ids.contains(&e.id)),
        "incomplete cloud application read-back"
    );
    let preview = cloud.preview(input.project_id, &timeline.revision).await?;
    validate_preview(&timeline, &preview)?;
    ensure!(!input.cancel.is_cancelled(), "video application cancelled");
    let viewer_url = cloud.viewer_url(input.project_id).await?;
    validate_viewer_url(&viewer_url)?;
    ensure!(!input.cancel.is_cancelled(), "video application cancelled");
    Ok(VideoEditDelivery {
        project_id: input.project_id.into(),
        template_sha256: input.template_sha256.into(),
        plan_sha256: input.plan_sha256.into(),
        source_mapping: ledger.mapping.clone(),
        timeline,
        preview,
        viewer_url,
    })
}
