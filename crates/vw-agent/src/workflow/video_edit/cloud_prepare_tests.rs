//! External occ process fixture; native policy, schemas, pipes, sockets and typed adapter are real.
use serde_json::json;

#[tokio::test]
async fn video_edit_prepare_native_receipts_keep_signed_tickets_in_memory() {
    let (dir, ctx, skill) = super::cloud_tests::fixture("ok");
    let provider = super::cloud::discover(
        ctx.clone(),
        std::slice::from_ref(&skill),
        Some(json!(["bash", "bash_wait", "bash_cancel"])),
    )
    .await
    .unwrap();
    let hub = skill.location.unwrap().parent().unwrap().to_path_buf();
    let root = hub.join("cutvideo");
    let ctx = super::prepare::NativeVideoContext(ctx);
    workflow_video_edit::test_support::contracts::video_edit_prepare_native_receipts_keep_signed_tickets_in_memory(&provider, dir.path(), root, &ctx).await;
}
#[tokio::test]
async fn video_edit_prepare_cloud_identity_is_independent_of_local_tenant_and_rejects_mismatch() {
    let (_dir, ctx, skill) = super::cloud_tests::fixture("ok");
    assert_ne!(ctx.tool_use_context().tenant_scope().company_id().as_ref(), "10011");
    let provider = super::cloud::discover(ctx, std::slice::from_ref(&skill), None)
        .await
        .unwrap();
    let root = skill.location.unwrap().parent().unwrap().join("cutvideo");
    workflow_video_edit::test_support::contracts::video_edit_prepare_cloud_identity_is_independent_of_local_tenant_and_rejects_mismatch(&provider, _dir.path(), root).await;
}
