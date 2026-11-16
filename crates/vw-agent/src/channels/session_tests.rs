//! 原生进程隔离和真实宿主端口的薄注册。
use super::*;
pub(crate) use channel_runtime::session_contracts::{registered_context, channel_message};
fn processor(req: channel_runtime::session_contracts::Request, delta: Option<tokio::sync::mpsc::Sender<String>>) -> std::pin::Pin<Box<dyn std::future::Future<Output=anyhow::Result<String>> + Send>> {
    Box::pin(run_session_processor_for_channel(req, delta))
}

#[tokio::test]
async fn run_session_processor_for_channel_accumulates_and_forwards_docs_output() {
    if isolated_processor_fixture("channels::session::session_tests::run_session_processor_for_channel_accumulates_and_forwards_docs_output").await { return; }
    channel_runtime::session_contracts::run_session_processor_for_channel_accumulates_and_forwards_docs_output(processor).await;
}

#[tokio::test]
async fn run_session_processor_for_channel_propagates_docs_error() {
    if isolated_processor_fixture("channels::session::session_tests::run_session_processor_for_channel_propagates_docs_error").await { return; }
    channel_runtime::session_contracts::run_session_processor_for_channel_propagates_docs_error(processor).await;
}

#[tokio::test]
async fn channel_project_scope_sessions_follow_snapshot_and_isolate_rooms() {
    channel_runtime::session_contracts::channel_project_scope_sessions_follow_snapshot_and_isolate_rooms(crate::channels::manager::contract_host()).await;
}

#[tokio::test]
async fn channel_project_scope_unavailable_directory_does_not_cache_fake_session() {
    channel_runtime::session_contracts::channel_project_scope_unavailable_directory_does_not_cache_fake_session(crate::channels::manager::contract_host()).await;
}

#[tokio::test]
async fn channel_project_scope_processor_uses_captured_directory() {
    if isolated_processor_fixture("channels::session::session_tests::channel_project_scope_processor_uses_captured_directory").await { return; }
    channel_runtime::session_contracts::channel_project_scope_processor_uses_captured_directory(crate::channels::manager::contract_host()).await;
}

#[tokio::test]
async fn channel_project_scope_memory_reads_only_matching_scope() {
    channel_runtime::session_contracts::channel_project_scope_memory_reads_only_matching_scope().await;
}

#[tokio::test]
async fn channel_project_scope_snapshot_rejects_different_sender() {
    channel_runtime::session_contracts::channel_project_scope_snapshot_rejects_different_sender(crate::channels::manager::contract_host()).await;
}

/// 子进程隔离模型目录与配置数据库，避免修改维护者配置或并行测试的环境。
async fn isolated_processor_fixture(test: &str) -> bool {
    if std::env::var("VW_CHANNEL_SCOPE_PROCESSOR_TEST").as_deref() == Ok(test) {
        crate::config::update(serde_json::json!({
            "providers": {"channel-scope-fixture": {"models": {"scope-model": {"status": "active"}}}}
        })).await.unwrap();
        return false;
    }
    let home = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env("VW_CHANNEL_SCOPE_PROCESSOR_TEST", test)
        .env("HOME", home.path())
        .env("VIBEWINDOW_CONFIG_DIR", home.path().join("config"))
        .env("VIBEWINDOW_WORKSPACE", home.path().join("workspace"))
        .output()
        .unwrap();
    assert!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "isolated processor fixture failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
