use super::*;

#[tokio::test]
async fn installer_reports_nonzero_exit_and_timeout() {
    let mut success = tokio_command("rustc");
    success.arg("--version");
    assert!(run(success, tokio::time::Instant::now() + Duration::from_secs(10)).await.is_ok());
    let mut failure = tokio_command("rustc");
    failure.arg("--definitely-invalid-browser-install-test");
    assert!(run(failure, tokio::time::Instant::now() + Duration::from_secs(10)).await.is_err());
    #[cfg(unix)]
    {
        let mut slow = tokio_command("sleep");
        slow.arg("30");
        let result = run(slow, tokio::time::Instant::now() + Duration::from_millis(50)).await;
        assert!(result.unwrap_err().contains("超时"));
    }
}

#[test]
fn managed_browser_is_used_only_after_successful_installation() {
    const CHILD: &str = "SHIJU_BROWSER_INSTALL_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let home = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!(
                    "{}::managed_browser_is_used_only_after_successful_installation",
                    module_path!().split_once("::").unwrap().1
                ),
            ])
            .env(CHILD, "1")
            .env("HOME", home.path())
            .env("USERPROFILE", home.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        return;
    }
    let root = install_root().unwrap();
    let script = entry(&root);
    std::fs::create_dir_all(script.parent().unwrap()).unwrap();
    std::fs::write(&script, b"// fixture").unwrap();
    assert!(agent_browser_command().as_std().get_args().next().is_none());
    std::fs::write(root.join("ready"), b"0.42.0").unwrap();
    let command = agent_browser_command();
    assert_eq!(command.as_std().get_args().next(), Some(script.as_os_str()));
    std::fs::remove_file(script).unwrap();
    assert!(agent_browser_command().as_std().get_args().next().is_none());
}
