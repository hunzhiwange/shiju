use serde_json::json;

#[tokio::test]
async fn video_edit_asset_jobs_native_adapter_reconciles_pages_and_reads_reference() {
    let (_dir, ctx, skill) = super::cloud_tests::fixture("ok");
    let provider = super::cloud::discover(
        ctx,
        std::slice::from_ref(&skill),
        Some(json!(["bash", "bash_wait", "bash_cancel"])),
    )
    .await
    .unwrap();
    let root = skill.location.unwrap().parent().unwrap().join("cutvideo");
    workflow_video_edit::test_support::contracts::video_edit_asset_jobs_native_adapter_reconciles_pages_and_reads_reference(&provider, _dir.path(), root).await;
}
