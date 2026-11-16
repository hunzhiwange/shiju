use serde_json::json;
#[tokio::test]
async fn video_edit_transcript_native_adapter_keeps_audio_ticket_private_and_queries_exact_task() {
    let (dir, context, skill) = super::cloud_tests::fixture("ok");
    let provider = super::cloud::discover(
        context,
        std::slice::from_ref(&skill),
        Some(json!(["bash", "bash_wait", "bash_cancel"])),
    )
    .await
    .unwrap();
    let root = skill.location.unwrap().parent().unwrap().join("cutvideo");
    workflow_video_edit::test_support::contracts::video_edit_transcript_native_adapter_keeps_audio_ticket_private_and_queries_exact_task(&provider, dir.path(), root).await;
}
