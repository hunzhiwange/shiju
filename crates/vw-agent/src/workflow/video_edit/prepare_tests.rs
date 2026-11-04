use super::{manifest::*, prepare::*, *};
use crate::{global_database::GlobalDatabase, tools::ToolRuntimeContext};
use anyhow::{Result, bail};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Remote {
    projects: usize,
    sessions: usize,
    puts: usize,
    finalizes: usize,
    project: Option<String>,
    fail: String,
    nonvideo: bool,
    incomplete: bool,
    replace: Option<std::path::PathBuf>,
}
#[derive(Clone)]
struct Cloud {
    ctx: ToolRuntimeContext,
    scopes: Vec<CloudVideoScope>,
    remote: Arc<Mutex<Remote>>,
}
#[async_trait::async_trait]
impl VideoPrepareCloud for Cloud {
    fn context(&self) -> &ToolRuntimeContext {
        &self.ctx
    }
    async fn authorized_scopes(&self) -> Result<Vec<CloudVideoScope>> {
        Ok(self.scopes.clone())
    }
    async fn create_project(&self, _run: &str, _scope: &CloudVideoScope) -> Result<String> {
        let mut r = self.remote.lock().unwrap();
        r.projects += 1;
        let id = format!("project-{}", r.projects);
        r.project = Some(id.clone());
        if r.fail == "project" {
            r.fail.clear();
            bail!("connection interrupted after create");
        }
        Ok(id)
    }
    async fn find_project(&self, _run: &str, _scope: &CloudVideoScope) -> Result<Option<String>> {
        Ok(self.remote.lock().unwrap().project.clone())
    }
    async fn create_upload(
        &self,
        _project: &str,
        _source: &VideoSourceIdentity,
    ) -> Result<VideoUploadTicket> {
        let mut r = self.remote.lock().unwrap();
        r.sessions += 1;
        if let Some(path) = r.replace.take() {
            std::fs::write(path, b"replacement source").unwrap();
        }
        if r.fail == "session" {
            r.fail.clear();
            bail!("session response lost");
        }
        Ok(VideoUploadTicket {
            upload_id: format!("upload-{}", r.sessions),
            url: "https://fixture.oss.aliyuncs.com/object?Signature=private".into(),
            headers: Default::default(),
        })
    }
    async fn upload(
        &self,
        _ticket: &VideoUploadTicket,
        _source: &VideoSourceIdentity,
    ) -> Result<()> {
        let mut r = self.remote.lock().unwrap();
        r.puts += 1;
        if r.fail == "put-incomplete" {
            r.fail.clear();
            r.incomplete = true;
            bail!("PUT interrupted before completion");
        }
        if r.fail == "put" {
            r.fail.clear();
            bail!("PUT response lost");
        }
        Ok(())
    }
    async fn finalize(
        &self,
        _project: &str,
        _upload: &str,
        job: Option<&str>,
    ) -> Result<VideoImportResult> {
        let mut r = self.remote.lock().unwrap();
        r.finalizes += 1;
        if r.incomplete {
            bail!("remote upload incomplete; reconciliation required");
        }
        if r.fail == "finalize" {
            r.fail.clear();
            bail!("finalize response lost");
        }
        if r.fail == "pending" {
            r.fail.clear();
            return Ok(VideoImportResult::Pending("job-original".into()));
        }
        if let Some(job) = job {
            assert_eq!(job, "job-original");
        }
        Ok(VideoImportResult::Ready("asset-original".into()))
    }
    async fn probe(&self, _project: &str, _asset: &str) -> Result<VideoMediaInfo> {
        Ok(VideoMediaInfo {
            has_video: !self.remote.lock().unwrap().nonvideo,
            width: 1920,
            height: 1080,
            duration_sec: 45.0,
            has_audio: true,
        })
    }
}
fn fixture() -> (tempfile::TempDir, GlobalDatabase, Cloud, VideoEditRequest) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("原片.mp4");
    std::fs::write(&path, b"external cloud fixture probes this").unwrap();
    let ctx = ToolRuntimeContext::new(
        uuid::Uuid::new_v4().to_string(),
        Some(root.path().to_string_lossy().into()),
    );
    let request =
        prepare_video_edit_request(&format!("/shiju 剪辑 \"{}\" 保留重点", path.display()), &ctx)
            .unwrap()
            .unwrap();
    let db = GlobalDatabase::new(root.path().join("app/db")).unwrap();
    let cloud = Cloud {
        ctx,
        scopes: vec![CloudVideoScope { brand_id: Some("brand-1".into()), store_id: None }],
        remote: Default::default(),
    };
    (root, db, cloud, request)
}
async fn prepare(
    db: GlobalDatabase,
    cloud: &Cloud,
    req: &VideoEditRequest,
) -> Result<PreparedCloudVideo> {
    prepare_cloud_video(db, cloud, req, b"template snapshot", "model snapshot", None).await
}
#[tokio::test]
async fn video_edit_prepare_unbound_readonly_source_reuses_durable_run() {
    let (root, db, cloud, req) = fixture();
    let source_dir = root.path().join("readonly");
    std::fs::create_dir(&source_dir).unwrap();
    let path = source_dir.join("source.mp4");
    std::fs::rename(&req.video_path, &path).unwrap();
    let mut req = req;
    req.video_path = path;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&source_dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    }
    let first = prepare(db.clone(), &cloud, &req).await.unwrap();
    let second = prepare(db.clone(), &cloud, &req).await.unwrap();
    assert_eq!(first.manifest.run_id, second.manifest.run_id);
    assert!(first.media.has_audio);
    assert!(!root.path().join(".shiju/workflow_commands.json").exists());
    assert_eq!(std::fs::read_dir(&source_dir).unwrap().count(), 1);
    let r = cloud.remote.lock().unwrap();
    assert_eq!((r.projects, r.sessions, r.puts), (1, 1, 1));
    drop(r);
    let bytes = std::fs::read(db.path()).unwrap();
    assert!(!bytes.windows(9).any(|w| w == b"Signature"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&source_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}
#[tokio::test]
async fn video_edit_prepare_unknown_writes_reconcile_without_duplicate_side_effects() {
    for stage in ["project", "put", "finalize", "pending"] {
        let (_root, db, cloud, req) = fixture();
        cloud.remote.lock().unwrap().fail = stage.into();
        assert!(prepare(db.clone(), &cloud, &req).await.is_err(), "{stage}");
        // Reopen the real SQLite ledger, simulating a client restart.
        let restarted = GlobalDatabase::new(db.path()).unwrap();
        let ready = prepare(restarted, &cloud, &req).await.unwrap();
        assert_eq!(ready.manifest.state, VideoPrepareState::Ready);
        let r = cloud.remote.lock().unwrap();
        assert_eq!((r.projects, r.sessions, r.puts), (1, 1, 1), "{stage}");
    }
}
#[tokio::test]
async fn video_edit_prepare_lost_session_response_stays_pending_without_resubmission() {
    let (_root, db, cloud, req) = fixture();
    cloud.remote.lock().unwrap().fail = "session".into();
    assert!(prepare(db.clone(), &cloud, &req).await.is_err());
    let error = prepare(db, &cloud, &req).await.unwrap_err().to_string();
    assert!(error.contains("pending reconciliation"));
    let r = cloud.remote.lock().unwrap();
    assert_eq!((r.projects, r.sessions, r.puts), (1, 1, 0));
}
#[tokio::test]
async fn video_edit_prepare_source_content_and_snapshots_invalidate_cache() {
    let (_root, db, cloud, req) = fixture();
    let a = prepare(db.clone(), &cloud, &req).await.unwrap();
    std::fs::write(&req.video_path, b"different contents at the same canonical path").unwrap();
    let b = prepare(db.clone(), &cloud, &req).await.unwrap();
    assert_ne!(a.manifest.run_id, b.manifest.run_id);
    let c = prepare_cloud_video(db.clone(), &cloud, &req, b"new template", "model snapshot", None)
        .await
        .unwrap();
    assert_ne!(b.manifest.run_id, c.manifest.run_id);
    let d =
        prepare_cloud_video(db, &cloud, &req, b"new template", "new model", None).await.unwrap();
    assert_ne!(c.manifest.run_id, d.manifest.run_id);
    assert_eq!(cloud.remote.lock().unwrap().projects, 4);
}
#[tokio::test]
async fn video_edit_prepare_replacement_and_nonvideo_never_return_success() {
    let (_root, db, cloud, req) = fixture();
    cloud.remote.lock().unwrap().replace = Some(req.video_path.clone());
    assert!(prepare(db, &cloud, &req).await.unwrap_err().to_string().contains("replaced"));
    assert_eq!(cloud.remote.lock().unwrap().puts, 0);
    let (_root, db, cloud, req) = fixture();
    cloud.remote.lock().unwrap().nonvideo = true;
    assert!(prepare(db.clone(), &cloud, &req).await.unwrap_err().to_string().contains("non-video"));
    assert!(prepare(db, &cloud, &req).await.is_err());
    assert_eq!(cloud.remote.lock().unwrap().projects, 1);
}
#[tokio::test]
async fn video_edit_prepare_two_projects_need_no_binding_and_have_independent_sources() {
    let (root, db, cloud, req) = fixture();
    for n in 0..2 {
        let project = root.path().join(format!("project-{n}"));
        std::fs::create_dir(&project).unwrap();
        let path = project.join("source.mp4");
        std::fs::write(&path, b"same bytes separate source identity").unwrap();
        let mut request = req.clone();
        request.project_root = Some(project);
        request.video_path = path;
        assert!(prepare(db.clone(), &cloud, &request).await.is_ok());
    }
    assert_eq!(cloud.remote.lock().unwrap().projects, 2);
}
#[tokio::test]
async fn video_edit_prepare_no_space_and_unapproved_choice_have_no_side_effects() {
    let (_root, db, mut cloud, req) = fixture();
    cloud.scopes.clear();
    assert!(prepare(db, &cloud, &req).await.unwrap_err().to_string().contains("no authorized"));
    let (_root, db, cloud, req) = fixture();
    let other = CloudVideoScope { brand_id: Some("unauthorized".into()), store_id: None };
    assert!(prepare_cloud_video(db, &cloud, &req, b"t", "m", Some(&other)).await.is_err());
    assert_eq!(cloud.remote.lock().unwrap().projects, 0);
}
#[tokio::test]
async fn video_edit_prepare_multiple_spaces_use_real_question_and_cancel() {
    for cancel in [false, true] {
        let (_root, db, mut cloud, req) = fixture();
        cloud.scopes.push(CloudVideoScope { brand_id: None, store_id: Some("store-2".into()) });
        let responder = tokio::spawn({
            let session = cloud.ctx.session.clone();
            async move {
                tokio::time::timeout(std::time::Duration::from_secs(5), async {
                    loop {
                        if let Some(q) =
                            crate::question::list().into_iter().find(|q| q.session_id == session)
                        {
                            if cancel {
                                crate::question::reject(q.id);
                            } else {
                                crate::question::reply(crate::question::ReplyInput {
                                    request_id: q.id,
                                    answers: vec![vec![q.questions[0].options[1].label.clone()]],
                                });
                            }
                            break;
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .unwrap();
            }
        });
        let result = prepare(db, &cloud, &req).await;
        responder.await.unwrap();
        if cancel {
            assert!(result.is_err());
            assert_eq!(cloud.remote.lock().unwrap().projects, 0);
        } else {
            assert_eq!(result.unwrap().manifest.cloud_scope.store_id.as_deref(), Some("store-2"));
        }
    }
}
#[test]
fn video_edit_prepare_streaming_hash_obeys_security_and_cancellation() {
    let (root, _db, cloud, req) = fixture();
    let identity = identify_video_source(&req.video_path, &cloud.ctx).unwrap();
    assert_eq!(identity.sha256.len(), 64);
    let token = tokio_util::sync::CancellationToken::new();
    token.cancel();
    let ctx = cloud.ctx.clone().with_tool_use_context(
        cloud.ctx.tool_use_context().as_ref().clone().with_abort_token(token),
    );
    assert!(identify_video_source(&req.video_path, &ctx).is_err());
    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("source.mp4");
    std::fs::write(&path, b"outside").unwrap();
    let mut config = crate::app::agent::config::Config::default();
    config.autonomy.workspace_only = true;
    config.workspace_dir = root.path().into();
    assert!(identify_video_source(&path, &cloud.ctx.clone().with_test_config(config)).is_err());
}

#[tokio::test]
async fn video_edit_prepare_cross_tenant_runs_and_native_mismatch_are_isolated() {
    let (root, db, cloud, req) = fixture();
    let first = prepare(db.clone(), &cloud, &req).await.unwrap();
    let tenant = vw_api_types::TenantScope::new("other-platform", "other-company").unwrap();
    crate::session::ui_store::with_tenant_scope(tenant.clone(), async {
        let mut other = cloud.clone();
        other.ctx = ToolRuntimeContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(root.path().to_string_lossy().into()),
        );
        let mut request = req.clone();
        request.tenant_scope = tenant;
        assert!(
            prepare(db.clone(), &cloud, &request).await.is_err(),
            "reject mismatched native tenant"
        );
        let second = prepare(db, &other, &request).await.unwrap();
        assert_ne!(first.manifest.run_id, second.manifest.run_id);
    })
    .await;
    assert_eq!(cloud.remote.lock().unwrap().projects, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn video_edit_prepare_competing_requests_create_and_upload_only_once() {
    let (_root, db, cloud, req) = fixture();
    let barrier = Arc::new(tokio::sync::Barrier::new(8));
    let tasks = (0..8)
        .map(|_| {
            let db = db.clone();
            let cloud = cloud.clone();
            let req = req.clone();
            let barrier = barrier.clone();
            tokio::spawn(async move {
                barrier.wait().await;
                prepare(db, &cloud, &req).await
            })
        })
        .collect::<Vec<_>>();
    let mut successes = 0;
    for task in tasks {
        if task.await.unwrap().is_ok() {
            successes += 1;
        }
    }
    assert!(successes >= 1);
    assert!(prepare(db, &cloud, &req).await.is_ok());
    let r = cloud.remote.lock().unwrap();
    assert_eq!((r.projects, r.sessions, r.puts), (1, 1, 1));
}
#[tokio::test]
async fn video_edit_prepare_incomplete_put_never_becomes_success_or_reuploads() {
    let (_root, db, cloud, req) = fixture();
    cloud.remote.lock().unwrap().fail = "put-incomplete".into();
    assert!(prepare(db.clone(), &cloud, &req).await.is_err());
    for _ in 0..2 {
        assert!(prepare(db.clone(), &cloud, &req).await.is_err());
    }
    let r = cloud.remote.lock().unwrap();
    assert_eq!((r.projects, r.sessions, r.puts), (1, 1, 1));
}
#[cfg(unix)]
#[test]
fn video_edit_prepare_source_replacement_with_same_bytes_changes_identity() {
    let (_root, _db, cloud, req) = fixture();
    let first = identify_video_source(&req.video_path, &cloud.ctx).unwrap();
    let replacement = req.video_path.with_extension("replacement");
    std::fs::copy(&req.video_path, &replacement).unwrap();
    std::fs::rename(replacement, &req.video_path).unwrap();
    let second = identify_video_source(&req.video_path, &cloud.ctx).unwrap();
    assert_eq!(first.sha256, second.sha256);
    assert_ne!(first.file_id, second.file_id);
}
