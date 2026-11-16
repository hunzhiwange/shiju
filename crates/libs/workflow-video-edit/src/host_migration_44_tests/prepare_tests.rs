use super::{manifest::*, prepare::*, *};
use agent_database::GlobalDatabase;
use super::TestContext as ToolRuntimeContext;
use anyhow::Result;
use std::sync::Arc;

type Cloud = crate::test_support::prepare_fixture::Cloud<ToolRuntimeContext>;
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

#[tokio::test]
async fn video_edit_prepare_proven_non_submission_retries_same_ledger_without_duplicate_project() {
    for failure in ["not-sent", "rejected", "upload-not-sent", "upload-rejected"] {
        let (_root, db, cloud, req) = fixture();
        cloud.remote.lock().unwrap().fail = failure.into();
        assert!(prepare(db.clone(), &cloud, &req).await.is_err());
        assert_eq!(
            cloud.remote.lock().unwrap().projects,
            usize::from(failure.starts_with("upload-"))
        );
        assert_eq!(cloud.remote.lock().unwrap().sessions, 0);
        let result = prepare(db.clone(), &cloud, &req).await;
        assert!(result.is_ok(), "proven {failure} must not remain CreatingProject: {result:?}");
        assert_eq!(cloud.remote.lock().unwrap().projects, 1);
        assert_eq!(
            prepare(db, &cloud, &req).await.unwrap().manifest.run_id,
            result.unwrap().manifest.run_id
        );
        assert_eq!(cloud.remote.lock().unwrap().projects, 1);
    }
}

#[tokio::test]
async fn video_edit_prepare_unknown_without_matching_project_never_reposts() {
    let (_root, db, cloud, req) = fixture();
    cloud.remote.lock().unwrap().fail = "unknown-no-project".into();
    assert!(prepare(db.clone(), &cloud, &req).await.is_err());
    assert!(
        prepare(db, &cloud, &req).await.unwrap_err().to_string().contains("refusing resubmission")
    );
    assert_eq!(cloud.remote.lock().unwrap().projects, 1);
}
