use super::*;

#[tokio::test]
async fn installation_rejects_unknown_agent_without_running_commands() {
    assert!(install_builtin_agent("custom; echo unsafe").await.is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn installer_reports_process_failures() {
    let mut log = InstallLog::default();
    assert!(run_installer(&mut log, "test", "sh", &["-c", "exit 0"]).await.is_ok());
    let result = run_installer(
        &mut log,
        "test",
        "sh",
        &["-c", "printf 'download started\n'; printf 'failed to extract\n' >&2; exit 7"],
    )
    .await;
    assert!(result.unwrap_err().contains("7"));
    assert!(log.steps.iter().any(|step| step == "stdout\ndownload started"));
    assert!(log.steps.iter().any(|step| step == "stderr\nfailed to extract"));
    assert!(!sanitize_output("npm ERR! token=not-a-real-secret").contains("not-a-real-secret"));
}
