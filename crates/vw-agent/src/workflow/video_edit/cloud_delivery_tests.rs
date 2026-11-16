use serde_json::json;
async fn native_scenario(
    mode: &str,
) -> (anyhow::Result<super::preview::VideoEditDelivery>, String) {
    let (_root, ctx, skill) = super::cloud_tests::fixture("ok");
    let provider = super::cloud::discover(
        ctx,
        std::slice::from_ref(&skill),
        Some(json!(["bash", "bash_wait", "bash_cancel"])),
    )
    .await
    .unwrap();
    let cut = skill.location.as_ref().unwrap().parent().unwrap().join("cutvideo");
    workflow_video_edit::test_support::contracts::delivery_scenario(&provider, cut, _root.path(), mode).await
}

#[tokio::test]
async fn video_edit_delivery_native_adapter_applies_three_types_and_reads_real_contract_shapes() {
    let (delivery, _) = native_scenario("normal").await;
    let delivery = delivery.unwrap();
    assert_eq!(delivery.timeline.effect_ids.len(), 3);
    assert_eq!(delivery.source_mapping[1].timeline_start, 5.0);
}
#[tokio::test]
async fn video_edit_delivery_native_adapter_rejects_partial_caption_failure_and_pending_draft() {
    for mode in ["partial", "pending"] {
        let (result, _) = native_scenario(mode).await;
        assert!(result.is_err(), "accepted {mode}");
    }
}
#[tokio::test]
async fn video_edit_delivery_native_adapter_records_verified_empty_cleanup_separately() {
    let (result, _) = native_scenario("noop").await;
    let delivery = result.unwrap();
    assert_eq!(delivery.source_mapping.len(), 1);
    assert_eq!(delivery.timeline.duration_sec, 20.0);
}

#[tokio::test]
async fn video_edit_delivery_native_finalize_accepts_observed_complete_import_receipt() {
    let (_root, ctx, skill) = super::cloud_tests::fixture("ok");
    let provider = super::cloud::discover(
        ctx,
        std::slice::from_ref(&skill),
        Some(json!(["bash", "bash_wait", "bash_cancel"])),
    )
    .await
    .unwrap();
    let cut = skill.location.as_ref().unwrap().parent().unwrap().join("cutvideo");
    workflow_video_edit::test_support::contracts::finalize_receipt(&provider, cut).await;
}
