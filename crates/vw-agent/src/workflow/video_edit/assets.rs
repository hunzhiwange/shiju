//! Durable independent asset jobs. Project imports and timeline writes are excluded.
use super::{budget::VideoEditRunBudget, manifest::*, planning::VideoEditPlan};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoAssetJobState {
    Queued,
    Unknown,
    Running,
    Succeeded,
    Failed,
}
impl VideoAssetJobState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }
}
/// Explicit generation parameters supplied by the editable workflow, never inferred from prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoAssetRecipe {
    pub prompt: String,
    pub kind: String,
    pub model: String,
    pub duration_sec: u8,
    pub ratio: String,
    pub resolution: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoAssetRequest {
    pub digest: String,
    pub project_id: String,
    pub recipe: VideoAssetRecipe,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoAssetAttempt {
    pub idempotency_key: String,
    pub task_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoAssetJob {
    pub request: VideoAssetRequest,
    pub idempotency_key: String,
    pub task_id: Option<String>,
    pub state: VideoAssetJobState,
    pub result_ref: Option<String>,
    pub attempts: u32,
    pub reconciliation_version: u64,
    #[serde(default)]
    pub failed_attempts: Vec<VideoAssetAttempt>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoAssetProgress {
    pub queued: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub peak: usize,
}
#[derive(Debug, Clone)]
pub struct VideoAssetRegistry {
    pub references: BTreeMap<String, String>,
    pub progress: VideoAssetProgress,
}
#[derive(Debug)]
pub enum VideoAssetRemoteState {
    Running(String),
    Succeeded {
        task_id: String,
        reference: String,
    },
    Failed {
        task_id: String,
    },
    Unknown,
    /// Only an explicit refusal before upstream acceptance permits a retry.
    NotSubmitted,
}
#[async_trait::async_trait]
pub trait VideoAssetCloud: Send + Sync {
    async fn submit(&self, request: &VideoAssetRequest, key: &str)
    -> Result<VideoAssetRemoteState>;
    async fn reconcile(&self, job: &VideoAssetJob) -> Result<VideoAssetRemoteState>;
    async fn verify(&self, project: &str, reference: &str) -> Result<()>;
}

pub(super) fn validate_recipe(recipe: &VideoAssetRecipe) -> Result<()> {
    ensure!(
        !recipe.prompt.trim().is_empty()
            && recipe.prompt.chars().count() <= 2000
            && ["environment", "atmosphere", "abstract", "illustration"]
                .contains(&recipe.kind.as_str())
            && ["wan3.0-video", "wan3.0-video-prime"].contains(&recipe.model.as_str())
            && (2..=30).contains(&recipe.duration_sec)
            && ["9:16", "16:9", "1:1", "3:4", "4:3", "21:9", "adaptive"]
                .contains(&recipe.ratio.as_str())
            && ["480P", "720P", "1080P"].contains(&recipe.resolution.as_str()),
        "unsupported asset generation recipe"
    );
    Ok(())
}

/// Deduplicate by content, production parameters and project scope. Missing recipes fail closed.
pub fn video_asset_requests(
    plan: &VideoEditPlan,
    project: &str,
    recipes: &BTreeMap<String, VideoAssetRecipe>,
) -> Result<Vec<VideoAssetRequest>> {
    ensure!(!project.is_empty(), "asset generation requires a prepared project");
    let mut unique = BTreeMap::new();
    for description in plan.effects.iter().flat_map(|e| &e.assets) {
        let recipe =
            recipes.get(description).context("asset description has no explicit recipe")?;
        validate_recipe(recipe)?;
        let digest = hex::encode(Sha256::digest(serde_json::to_vec(&(project, recipe))?));
        unique.entry(digest.clone()).or_insert(VideoAssetRequest {
            digest,
            project_id: project.into(),
            recipe: recipe.clone(),
        });
    }
    Ok(unique.into_values().collect())
}

fn key(run: &VideoEditRunManifest, digest: &str) -> String {
    let mut hash = Sha256::new();
    for part in [
        run.tenant.platform_id().as_ref(),
        run.tenant.company_id().as_ref(),
        run.run_id.as_str(),
        digest,
    ] {
        hash.update((part.len() as u64).to_le_bytes());
        hash.update(part.as_bytes());
    }
    format!("asset:{}", hex::encode(hash.finalize()))
}
async fn save(store: &VideoEditManifestStore, run: &mut VideoEditRunManifest) -> Result<()> {
    let store = store.clone();
    let mut snapshot = run.clone();
    *run = crate::global_database::run_isolated(move || {
        store.save(&mut snapshot, chrono::Utc::now().timestamp_millis())?;
        Ok::<_, anyhow::Error>(snapshot)
    })
    .await??;
    Ok(())
}
/// Persist cancellation even when no scheduling future is currently running.
/// Keep reconciling old jobs; this does not claim the remote service cancelled them.
pub async fn cancel_video_assets(
    store: &VideoEditManifestStore,
    run: &mut VideoEditRunManifest,
    budget: &VideoEditRunBudget,
) -> Result<()> {
    budget.cancel();
    run.asset_cancelled = true;
    save(store, run).await
}
async fn check_cancelled(
    store: &VideoEditManifestStore,
    run: &mut VideoEditRunManifest,
    budget: &VideoEditRunBudget,
) -> Result<()> {
    if budget.is_cancelled() || run.asset_cancelled {
        cancel_video_assets(store, run, budget).await?;
        bail!("video edit cancelled; unresolved asset jobs retained for reconciliation");
    }
    Ok(())
}
fn progress(run: &VideoEditRunManifest) -> VideoAssetProgress {
    let mut p = VideoAssetProgress { peak: run.asset_peak, ..Default::default() };
    for job in &run.asset_jobs {
        match job.state {
            VideoAssetJobState::Queued => p.queued += 1,
            VideoAssetJobState::Unknown | VideoAssetJobState::Running => p.running += 1,
            VideoAssetJobState::Succeeded => p.completed += 1,
            VideoAssetJobState::Failed => p.failed += 1,
        }
    }
    p
}

/// One durable scheduling/reconciliation round. None means unfinished, never an applicable plan.
/// The caller may poll again (official interval 10–15s). Unknown jobs retain named budget slots
/// even if this future is dropped; a fresh process re-reserves all old jobs before dispatch.
pub async fn prepare_video_assets(
    cloud: &dyn VideoAssetCloud,
    store: &VideoEditManifestStore,
    run: &mut VideoEditRunManifest,
    requests: &[VideoAssetRequest],
    budget: &VideoEditRunBudget,
) -> Result<Option<VideoAssetRegistry>> {
    run.asset_cancelled |= budget.is_cancelled();
    let project = run.project_id.as_deref().context("missing prepared project")?;
    let mut expected = BTreeSet::new();
    for request in requests {
        validate_recipe(&request.recipe)?;
        ensure!(
            request.project_id == project && expected.insert(&request.digest),
            "invalid asset request scope/duplicate"
        );
        let digest = hex::encode(Sha256::digest(serde_json::to_vec(&(
            &request.project_id,
            &request.recipe,
        ))?));
        ensure!(request.digest == digest, "asset request digest mismatch");
    }
    if run.asset_jobs.is_empty() {
        run.asset_jobs = requests
            .iter()
            .map(|request| VideoAssetJob {
                request: request.clone(),
                idempotency_key: key(run, &request.digest),
                task_id: None,
                state: VideoAssetJobState::Queued,
                result_ref: None,
                attempts: 0,
                reconciliation_version: 0,
                failed_attempts: vec![],
            })
            .collect();
        save(store, run).await?;
        budget.set_asset_progress(progress(run));
    }
    let ledger_digests: BTreeSet<_> = run.asset_jobs.iter().map(|j| &j.request.digest).collect();
    ensure!(
        ledger_digests == expected
            && run.asset_jobs.len() == requests.len()
            && run.asset_jobs.iter().all(|j| requests.contains(&j.request)),
        "asset ledger request mismatch"
    );
    save(store, run).await?;
    budget.set_asset_progress(progress(run));
    for index in 0..run.asset_jobs.len() {
        if run.asset_jobs[index].state.is_terminal() {
            budget.release_asset(&run.asset_jobs[index].idempotency_key);
            retry_failed(run, index);
        }
    }
    save(store, run).await?;
    budget.set_asset_progress(progress(run));
    let old: Vec<_> = run
        .asset_jobs
        .iter()
        .filter(|j| matches!(j.state, VideoAssetJobState::Unknown | VideoAssetJobState::Running))
        .cloned()
        .collect();
    ensure!(
        old.len() <= budget.limit(),
        "pending assets exceed run budget; reconcile with original limit"
    );
    // Reserve every old remote operation before any new POST or poll.
    for job in &old {
        budget.reserve_asset(&job.idempotency_key).await.map_err(anyhow::Error::msg)?;
    }
    for job in old {
        let index = run
            .asset_jobs
            .iter()
            .position(|j| j.idempotency_key == job.idempotency_key)
            .context("asset ledger changed")?;
        let state = cloud.reconcile(&job).await.unwrap_or(VideoAssetRemoteState::Unknown);
        update(run, index, state)?;
        save(store, run).await?;
        budget.set_asset_progress(progress(run));
        if !matches!(
            run.asset_jobs[index].state,
            VideoAssetJobState::Unknown | VideoAssetJobState::Running
        ) {
            budget.release_asset(&job.idempotency_key);
            retry_failed(run, index);
            save(store, run).await?;
            budget.set_asset_progress(progress(run));
        }
    }
    check_cancelled(store, run, budget).await?;
    ensure!(
        !run.asset_jobs.iter().any(|j| j.state == VideoAssetJobState::Failed),
        "asset failed; refusing partial plan application"
    );
    // Nonblocking reservation: model planning may still own part of this SAME run budget.
    let mut dispatch = Vec::new();
    for index in 0..run.asset_jobs.len() {
        let job = &run.asset_jobs[index];
        if job.state == VideoAssetJobState::Queued
            && budget.try_reserve_asset(&job.idempotency_key).map_err(anyhow::Error::msg)?
        {
            run.asset_jobs[index].state = VideoAssetJobState::Unknown;
            run.asset_jobs[index].attempts += 1;
            dispatch.push(index);
        }
    }
    run.asset_peak = run.asset_peak.max(progress(run).running);
    // Persist intent before side effects. A save failure leaves reservations held, fail closed.
    save(store, run).await?;
    budget.set_asset_progress(progress(run));
    let results = futures_util::future::join_all(dispatch.iter().map(|&index| {
        let job = &run.asset_jobs[index];
        async move {
            if budget.is_cancelled() {
                return VideoAssetRemoteState::NotSubmitted;
            }
            cloud
                .submit(&job.request, &job.idempotency_key)
                .await
                .unwrap_or(VideoAssetRemoteState::Unknown)
        }
    }))
    .await;
    for (index, state) in dispatch.into_iter().zip(results) {
        let idempotency_key = run.asset_jobs[index].idempotency_key.clone();
        update(run, index, state)?;
        save(store, run).await?;
        budget.set_asset_progress(progress(run));
        if !matches!(
            run.asset_jobs[index].state,
            VideoAssetJobState::Unknown | VideoAssetJobState::Running
        ) {
            budget.release_asset(&idempotency_key);
            retry_failed(run, index);
            save(store, run).await?;
            budget.set_asset_progress(progress(run));
        }
    }
    check_cancelled(store, run, budget).await?;
    let p = progress(run);
    ensure!(p.failed == 0, "asset failed; refusing partial plan application");
    if p.completed != run.asset_jobs.len() {
        return Ok(None);
    }
    let completed: Vec<_> = run
        .asset_jobs
        .iter()
        .map(|job| {
            Ok((
                job.request.digest.clone(),
                job.request.project_id.clone(),
                job.result_ref.clone().context("successful asset has no result")?,
            ))
        })
        .collect::<Result<_>>()?;
    let mut references = BTreeMap::new();
    for (digest, project, reference) in completed {
        let verification = cloud.verify(&project, &reference).await;
        check_cancelled(store, run, budget).await?;
        verification?;
        references.insert(digest, reference);
    }
    check_cancelled(store, run, budget).await?;
    Ok(Some(VideoAssetRegistry { references, progress: p }))
}
fn retry_failed(run: &mut VideoEditRunManifest, index: usize) {
    if run.asset_jobs[index].state != VideoAssetJobState::Failed
        || run.asset_jobs[index].attempts >= 3
    {
        return;
    }
    let next_key = format!(
        "{}:{}",
        key(run, &run.asset_jobs[index].request.digest),
        run.asset_jobs[index].attempts + 1
    );
    let job = &mut run.asset_jobs[index];
    if let Some(task_id) = job.task_id.take() {
        job.failed_attempts
            .push(VideoAssetAttempt { idempotency_key: job.idempotency_key.clone(), task_id });
        job.idempotency_key = next_key;
        job.state = VideoAssetJobState::Queued;
    }
}
fn update(
    run: &mut VideoEditRunManifest,
    index: usize,
    remote: VideoAssetRemoteState,
) -> Result<()> {
    let job = &mut run.asset_jobs[index];
    job.reconciliation_version = job
        .reconciliation_version
        .checked_add(1)
        .context("asset reconciliation version overflow")?;
    let (state, task, reference) = match remote {
        VideoAssetRemoteState::Running(id) => (VideoAssetJobState::Running, Some(id), None),
        VideoAssetRemoteState::Succeeded { task_id, reference } => {
            ensure!(
                reference.starts_with("gen:")
                    && reference.len() <= 200
                    && !reference.contains(['?', '&', '\n']),
                "invalid durable generation reference"
            );
            (VideoAssetJobState::Succeeded, Some(task_id), Some(reference))
        }
        VideoAssetRemoteState::Failed { task_id } => {
            (VideoAssetJobState::Failed, Some(task_id), None)
        }
        VideoAssetRemoteState::Unknown => (VideoAssetJobState::Unknown, job.task_id.clone(), None),
        VideoAssetRemoteState::NotSubmitted => {
            ensure!(job.task_id.is_none(), "accepted asset cannot become not-submitted");
            (
                if job.attempts < 3 {
                    VideoAssetJobState::Queued
                } else {
                    VideoAssetJobState::Failed
                },
                None,
                None,
            )
        }
    };
    if let Some(task) = &task {
        ensure!(
            !task.is_empty()
                && task.len() <= 200
                && job.task_id.as_ref().is_none_or(|old| old == task),
            "remote asset identity mismatch"
        );
    }
    job.task_id = task;
    job.state = state;
    job.result_ref = reference;
    Ok(())
}
