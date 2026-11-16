use super::{manifest::*, prepare::*, *};
use crate::{global_database::GlobalDatabase, tools::ToolRuntimeContext};
use anyhow::Result;

type Cloud = workflow_video_edit::test_support::prepare_fixture::Cloud<ToolRuntimeContext>;
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
    let tenant = vw_api_transport::TenantScope::new("other-platform", "other-company").unwrap();
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
