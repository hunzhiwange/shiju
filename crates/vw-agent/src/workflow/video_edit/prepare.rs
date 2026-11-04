//! Idempotent single-video preparation. Unknown mutations remain fenced in the ledger.
use super::{VideoEditRequest, manifest::*};
use crate::{
    global_database::{GlobalDatabase, run_isolated},
    tools::ToolRuntimeContext,
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::Metadata,
    io::Read,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

/// Intentionally has neither Debug nor Serialize: presigned URLs are memory-only secrets.
pub struct VideoUploadTicket {
    pub upload_id: String,
    pub(super) url: String,
    pub(super) headers: std::collections::BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMediaInfo {
    pub has_video: bool,
    pub width: u32,
    pub height: u32,
    pub duration_sec: f64,
    pub has_audio: bool,
}
#[derive(Debug, Clone)]
pub enum VideoImportResult {
    Pending(String),
    Ready(String),
}
/// Only the external cloud boundary is replaceable. Parsing, permissions, ledger and
/// source validation are always executed by prepare_cloud_video.
#[async_trait::async_trait]
pub trait VideoPrepareCloud: Send + Sync {
    fn context(&self) -> &ToolRuntimeContext;
    async fn authorized_scopes(&self) -> Result<Vec<CloudVideoScope>>;
    async fn choose_scope(&self, scopes: &[CloudVideoScope]) -> Result<CloudVideoScope> {
        select_scope(scopes, None, self.context()).await
    }
    async fn create_project(&self, run_id: &str, scope: &CloudVideoScope) -> Result<String>;
    async fn find_project(&self, run_id: &str, scope: &CloudVideoScope) -> Result<Option<String>>;
    async fn create_upload(
        &self,
        project: &str,
        source: &VideoSourceIdentity,
    ) -> Result<VideoUploadTicket>;
    async fn upload(&self, ticket: &VideoUploadTicket, source: &VideoSourceIdentity) -> Result<()>;
    async fn finalize(
        &self,
        project: &str,
        upload: &str,
        job: Option<&str>,
    ) -> Result<VideoImportResult>;
    async fn probe(&self, project: &str, asset: &str) -> Result<VideoMediaInfo>;
}
#[derive(Debug, Clone)]
/// Snapshot only: the preparation lease is released on return. A following stage
/// must claim the same tenant ledger key rather than reuse this lease owner.
pub struct PreparedCloudVideo {
    pub manifest: VideoEditRunManifest,
    pub media: VideoMediaInfo,
}
pub(super) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}
pub(super) fn cancelled(context: &ToolRuntimeContext) -> Result<()> {
    ensure!(
        !context.tool_use_context().abort_token().is_some_and(|t| t.is_cancelled()),
        "cancelled"
    );
    Ok(())
}
fn metadata_identity(path: &Path, m: &Metadata) -> Result<VideoSourceIdentity> {
    ensure!(m.is_file() && m.len() > 0, "source must be a nonempty regular file");
    let modified_ns = m.modified()?.duration_since(UNIX_EPOCH)?.as_nanos();
    #[cfg(unix)]
    let file_id = {
        use std::os::unix::fs::MetadataExt;
        format!("{}:{}:{}:{}", m.dev(), m.ino(), m.ctime(), m.ctime_nsec())
    };
    #[cfg(not(unix))]
    let file_id = format!("{:?}", m.created().ok());
    Ok(VideoSourceIdentity {
        path: path.into(),
        bytes: m.len(),
        modified_ns,
        file_id,
        sha256: String::new(),
    })
}
/// Stream at most 128 KiB at once and reject replacement, mutation or symlink retargeting.
pub fn identify_video_source(
    path: &Path,
    context: &ToolRuntimeContext,
) -> Result<VideoSourceIdentity> {
    cancelled(context)?;
    let canonical = path.canonicalize()?;
    ensure!(
        context.path_security_policy().is_resolved_path_allowed(&canonical),
        "native policy denies source"
    );
    let before_path = metadata_identity(&canonical, &std::fs::symlink_metadata(&canonical)?)?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options.open(&canonical)?;
    let mut identity = metadata_identity(&canonical, &file.metadata()?)?;
    let before = identity.clone();
    ensure!(
        before == before_path && canonical.canonicalize()? == canonical,
        "source replaced before reading"
    );
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    loop {
        cancelled(context)?;
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    ensure!(
        metadata_identity(&canonical, &file.metadata()?)? == before
            && metadata_identity(&canonical, &canonical.metadata()?)? == before
            && path.canonicalize()? == canonical,
        "source replaced or changed while reading"
    );
    identity.sha256 = hex::encode(hash.finalize());
    Ok(identity)
}
fn digest(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}

pub(super) async fn select_scope(
    scopes: &[CloudVideoScope],
    selected: Option<&CloudVideoScope>,
    context: &ToolRuntimeContext,
) -> Result<CloudVideoScope> {
    cancelled(context)?;
    if let Some(s) = selected {
        ensure!(scopes.contains(s), "selected scope is no longer authorized");
        return Ok(s.clone());
    }
    match scopes {
        [] => bail!("no authorized cloud space"),
        [only] => Ok(only.clone()),
        _ => {
            let labels = scopes
                .iter()
                .map(|s| {
                    format!(
                        "brand={} store={}",
                        s.brand_id.as_deref().unwrap_or("-"),
                        s.store_id.as_deref().unwrap_or("-")
                    )
                })
                .collect::<Vec<_>>();
            let input = crate::question::AskInput {
                session_id: context.session.clone(),
                tool: None,
                questions: vec![crate::question::Info {
                    question: "请选择本次视频的云端归属".into(),
                    header: "云端归属".into(),
                    options: labels
                        .iter()
                        .map(|label| crate::question::OptionInfo {
                            label: label.clone(),
                            description: "使用该已授权空间准备工程与原片".into(),
                            preview: None,
                        })
                        .collect(),
                    multiple: Some(false),
                    custom: Some(false),
                }],
            };
            let abort = context.tool_use_context().abort_token().cloned().unwrap_or_default();
            let answers = tokio::select! {
                result = crate::session::ui_store::with_tenant_scope(context.tool_use_context().tenant_scope(), crate::question::ask(input)) => result.context("cloud space choice cancelled")?,
                _ = abort.cancelled() => bail!("cancelled"),
            };
            ensure!(answers.len() == 1 && answers[0].len() == 1, "invalid cloud scope answer");
            let index = labels
                .iter()
                .position(|l| *l == answers[0][0])
                .context("unauthorized cloud scope answer")?;
            Ok(scopes[index].clone())
        }
    }
}
struct RunLease {
    store: VideoEditManifestStore,
    run: VideoEditRunManifest,
}
impl RunLease {
    async fn save(&mut self) -> Result<()> {
        let store = self.store.clone();
        let mut run = self.run.clone();
        self.run = run_isolated(move || {
            store.save(&mut run, now_ms())?;
            Ok::<_, anyhow::Error>(run)
        })
        .await??;
        Ok(())
    }
}
impl Drop for RunLease {
    fn drop(&mut self) {
        let store = self.store.clone();
        let run = self.run.clone();
        if crate::global_database::run_sync_database(move || store.release(&run)).is_err() {
            tracing::warn!("video preparation lease release failed; expiry required");
        }
    }
}
/// Template and model snapshots are immutable inputs; neither business text nor credentials
/// are persisted. No project binding or writable source directory is required.
pub async fn prepare_cloud_video(
    database: GlobalDatabase,
    cloud: &dyn VideoPrepareCloud,
    request: &VideoEditRequest,
    template: &[u8],
    model: &str,
    selected: Option<&CloudVideoScope>,
) -> Result<PreparedCloudVideo> {
    let context = cloud.context();
    ensure!(
        request.tenant_scope == context.tool_use_context().tenant_scope(),
        "native tenant mismatch"
    );
    let path = request.video_path.clone();
    let ctx = context.clone();
    let source = run_isolated(move || identify_video_source(&path, &ctx)).await??;
    let scopes = cloud.authorized_scopes().await?;
    let scope = if selected.is_none() && scopes.len() > 1 {
        let choice = cloud.choose_scope(&scopes).await?;
        ensure!(scopes.contains(&choice), "unauthorized scope answer");
        choice
    } else {
        select_scope(&scopes, selected, context).await?
    };
    let template_sha256 = digest(template);
    let model_sha256 = digest(model.as_bytes());
    let requirements_sha256 = digest(request.requirements.as_deref().unwrap_or("").as_bytes());
    let run_key = digest(&serde_json::to_vec(&(
        &source,
        &template_sha256,
        &model_sha256,
        &requirements_sha256,
        &scope,
    ))?);
    let candidate = VideoEditRunManifest {
        tenant: request.tenant_scope.clone(),
        run_id: uuid::Uuid::new_v4().to_string(),
        run_key,
        source,
        template_sha256,
        model_sha256,
        requirements_sha256,
        cloud_scope: scope,
        project_id: None,
        upload_id: None,
        asset_id: None,
        jobs: vec![],
        asset_jobs: vec![],
        asset_peak: 0,
        asset_cancelled: false,
        transcript: None,
        state: VideoPrepareState::New,
        revision: 0,
        lease_owner: String::new(),
        lease_until_ms: 0,
    };
    let (store, run) = run_isolated(move || {
        let store = VideoEditManifestStore::new(database)?;
        let run = store.claim(candidate, &uuid::Uuid::new_v4().to_string(), now_ms())?;
        Ok::<_, anyhow::Error>((store, run))
    })
    .await??;
    let mut lease = RunLease { store, run };
    let mut ticket = None;
    loop {
        cancelled(context)?;
        match lease.run.state {
            VideoPrepareState::New => {
                lease.run.state = VideoPrepareState::CreatingProject;
                lease.save().await?;
                match cloud.create_project(&lease.run.run_id, &lease.run.cloud_scope).await {
                    Ok(project) => lease.run.project_id = Some(project),
                    Err(error) => {
                        if error
                            .downcast_ref::<super::cloud_failure::CloudOperationFailure>()
                            .is_some_and(|failure| failure.permits_resubmission())
                        {
                            lease.run.state = VideoPrepareState::New;
                            lease.save().await?;
                            return Err(error.context(
                                "project creation had no effects; explicit retry allowed",
                            ));
                        }
                        return Err(
                            error.context("project creation unknown; reconcile before retry")
                        );
                    }
                }
                lease.run.state = VideoPrepareState::ProjectReady;
                lease.save().await?;
            }
            VideoPrepareState::CreatingProject => {
                lease.run.project_id =
                    cloud.find_project(&lease.run.run_id, &lease.run.cloud_scope).await?;
                ensure!(
                    lease.run.project_id.is_some(),
                    "project creation unknown; no confirmed matching project, refusing resubmission"
                );
                lease.run.state = VideoPrepareState::ProjectReady;
                lease.save().await?;
            }
            VideoPrepareState::ProjectReady => {
                lease.run.state = VideoPrepareState::CreatingUpload;
                lease.save().await?;
                let t = match cloud
                    .create_upload(
                        lease.run.project_id.as_deref().context("missing project")?,
                        &lease.run.source,
                    )
                    .await
                {
                    Ok(ticket) => ticket,
                    Err(error) => {
                        if error
                            .downcast_ref::<super::cloud_failure::CloudOperationFailure>()
                            .is_some_and(|failure| failure.permits_resubmission())
                        {
                            lease.run.state = VideoPrepareState::ProjectReady;
                            lease.save().await?;
                            return Err(error
                                .context("upload session had no effects; explicit retry allowed"));
                        }
                        return Err(error.context("upload session unknown; refusing resubmission"));
                    }
                };
                lease.run.upload_id = Some(t.upload_id.clone());
                ticket = Some(t);
                lease.run.state = VideoPrepareState::UploadReady;
                lease.save().await?;
            }
            VideoPrepareState::CreatingUpload => {
                bail!("upload session unknown; pending reconciliation, refusing resubmission")
            }
            VideoPrepareState::UploadReady => {
                let t = ticket.take().context(
                    "upload ticket unavailable; pending reconciliation, refusing a new session",
                )?;
                let path = lease.run.source.path.clone();
                let ctx = context.clone();
                ensure!(
                    run_isolated(move || identify_video_source(&path, &ctx)).await??
                        == lease.run.source,
                    "source replaced before upload"
                );
                lease.run.state = VideoPrepareState::Uploading;
                lease.save().await?;
                cloud
                    .upload(&t, &lease.run.source)
                    .await
                    .context("upload outcome unknown; reconcile original upload, do not resend")?;
                lease.run.state = VideoPrepareState::Finalizing;
                lease.save().await?;
            }
            VideoPrepareState::Uploading | VideoPrepareState::Finalizing => {
                // finalize is documented idempotent, also probes an already-completed PUT.
                let job = lease.run.jobs.last().map(|j| j.id.as_str());
                match cloud
                    .finalize(
                        lease.run.project_id.as_deref().context("missing project")?,
                        lease.run.upload_id.as_deref().context("missing upload")?,
                        job,
                    )
                    .await
                    .context("import pending reconciliation; refusing new upload")?
                {
                    VideoImportResult::Pending(id) => {
                        lease.run.jobs = vec![VideoRemoteJob { id, state: "running".into() }];
                        lease.run.state = VideoPrepareState::Finalizing;
                        lease.save().await?;
                        bail!("import still running; resume the same upload/job");
                    }
                    VideoImportResult::Ready(asset) => {
                        for job in &mut lease.run.jobs {
                            job.state = "succeeded".into();
                        }
                        lease.run.asset_id = Some(asset);
                        lease.run.state = VideoPrepareState::Probing;
                        lease.save().await?;
                    }
                }
            }
            VideoPrepareState::Probing | VideoPrepareState::Ready => {
                let media = cloud
                    .probe(
                        lease.run.project_id.as_deref().context("missing project")?,
                        lease.run.asset_id.as_deref().context("missing asset")?,
                    )
                    .await?;
                if !media.has_video
                    || media.width == 0
                    || media.height == 0
                    || !media.duration_sec.is_finite()
                    || media.duration_sec <= 0.0
                {
                    lease.run.state = VideoPrepareState::Rejected;
                    lease.save().await?;
                    bail!("cloud probe rejected non-video or invalid media");
                }
                let path = lease.run.source.path.clone();
                let ctx = context.clone();
                ensure!(
                    run_isolated(move || identify_video_source(&path, &ctx)).await??
                        == lease.run.source,
                    "source replaced during preparation"
                );
                lease.run.state = VideoPrepareState::Ready;
                lease.save().await?;
                return Ok(PreparedCloudVideo { manifest: lease.run.clone(), media });
            }
            VideoPrepareState::Rejected => bail!("source previously rejected by cloud probe"),
        }
    }
}
