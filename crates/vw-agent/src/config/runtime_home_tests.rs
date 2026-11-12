//! Run home selection in a child, without changing the test runner environment.
#[test]
fn model_protocol_021_config_store_uses_runtime_home() {
    const CHILD: &str = "SHIJU_PROTOCOL_CONFIG_HOME_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let root = std::path::PathBuf::from(std::env::var_os("VIBEWINDOW_TEST_ROOT").unwrap());
        assert_eq!(super::runtime_home::config_home().unwrap(), root);
        assert_eq!(crate::global::paths().home, root);
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "config::runtime_home_tests::model_protocol_021_config_store_uses_runtime_home",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("VIBEWINDOW_TEST_ROOT", root.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "isolated config child failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
}
