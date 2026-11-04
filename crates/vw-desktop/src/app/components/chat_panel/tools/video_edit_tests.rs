use super::*;
use serde_json::json;
#[test]
fn video_edit_ui_progress_keeps_failure_and_budget_visible() {
    let progress = vw_gateway_client::vw_api_types::workflow::VideoEditProgress {
        run_id: "run-1".into(),
        sequence: 8,
        stage: "合并".into(),
        total: 7,
        queued: 1,
        running: 1,
        completed: 4,
        failed: 1,
        peak: 5,
        limit: 5,
        template_sha256: "digest".into(),
        jobs_failed: 1,
        ..Default::default()
    };
    let summary =
        video_edit_summary(&json!({"metadata":{"video_edit_progress":progress}})).unwrap();
    assert!(summary.contains("失败 1"));
    assert!(summary.contains("峰值 5/5"));
    assert!(summary.contains("digest"));
    assert!(video_edit_summary(&json!({"metadata":{"token":"secret"}})).is_none());
}
