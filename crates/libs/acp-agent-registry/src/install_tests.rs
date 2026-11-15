use super::*;

#[tokio::test]
async fn installation_rejects_unknown_agent_without_running_commands() {
    assert!(install_builtin_agent("custom; echo unsafe").await.is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn installer_reports_process_failures_and_timeout() {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    assert!(run_installer("sh", &["-c", "exit 0"], deadline).await.is_ok());
    assert!(run_installer("sh", &["-c", "exit 7"], deadline).await.is_err());
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(30);
    assert!(run_installer("sh", &["-c", "sleep 30"], deadline).await.is_err());
}
