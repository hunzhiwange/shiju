#[test]
fn normalize_version_uses_first_nonempty_line_and_last_token() {
    assert_eq!(super::normalize_version("\n  VibeWindow v1.2.3  \nignored"), "1.2.3");
    assert_eq!(super::normalize_version("v2.0.0"), "2.0.0");
    assert_eq!(super::normalize_version("  "), "");
}

#[test]
fn versions_match_ignores_prefix_and_product_name_but_not_version_changes() {
    assert!(super::versions_match("VibeWindow v1.2.3", "1.2.3"));
    assert!(super::versions_match("\nCLI v1.2.3\n", "v1.2.3"));
    assert!(!super::versions_match("v1.2.3", "v1.2.4"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn version_output_prefers_stdout_and_trims_first_nonempty_line() {
    let version = super::extract_version_output(
        b"\n  vibewindow v1.2.3  \nignored".to_vec(),
        b"stderr version".to_vec(),
    )
    .expect("stdout should contain a version");

    assert_eq!(version, "vibewindow v1.2.3");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn version_output_falls_back_to_stderr_and_rejects_blank_output() {
    assert_eq!(
        super::extract_version_output(Vec::new(), b"  vibewindow v2.0.0\n".to_vec())
            .expect("stderr should contain a version"),
        "vibewindow v2.0.0"
    );
    assert_eq!(
        super::extract_version_output(Vec::new(), b" \n\t".to_vec()).unwrap_err(),
        "CLI 未安装或未返回版本信息"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn install_cli_binary_from_path_replaces_old_file_without_tail_bytes() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let source_bin = dir.path().join("source-shiju");
    let target_bin = dir.path().join(if cfg!(windows) { "shiju.exe" } else { "shiju" });
    std::fs::write(&source_bin, b"new").expect("source binary should be written");
    std::fs::write(&target_bin, b"old-binary-tail").expect("old binary should be written");

    super::install_cli_binary_from_path(&source_bin, &target_bin)
        .expect("CLI binary should be installed");

    assert_eq!(std::fs::read(&target_bin).expect("target binary should be readable"), b"new");
    assert!(!super::staged_cli_binary_path(&target_bin).exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn staged_cli_path_is_hidden_and_stale_file_is_removed() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let target = dir.path().join("shiju");
    let staged = super::staged_cli_binary_path(&target);

    assert_eq!(staged, dir.path().join(".shiju.installing"));
    std::fs::write(&staged, b"stale").expect("staged file should be written");
    super::remove_stale_staged_cli_binary(&staged).expect("staged file should be removed");
    assert!(!staged.exists());
    super::remove_stale_staged_cli_binary(&staged).expect("missing staged file is accepted");
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn executable_permission_is_set_on_staged_cli() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("temp dir should be created");
    let staged = dir.path().join("staged-cli");
    std::fs::write(&staged, b"binary").expect("staged file should be written");
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o600))
        .expect("initial permissions should be set");

    super::ensure_cli_binary_executable(&staged).expect("permissions should be updated");

    let mode =
        std::fs::metadata(&staged).expect("metadata should be readable").permissions().mode();
    assert_eq!(mode & 0o777, 0o755);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn missing_profile_is_created_and_path_entry_is_idempotent() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let profile = dir.path().join("nested").join("profile");
    let install_dir = dir.path().join("bin");
    assert!(!profile.exists());

    assert!(
        super::ensure_profile_contains_path(&profile, &install_dir)
            .expect("path entry should be appended")
    );
    let first = std::fs::read_to_string(&profile).expect("profile should be readable");
    assert!(first.ends_with('\n'));
    assert!(first.contains(install_dir.to_string_lossy().as_ref()));

    assert!(
        !super::ensure_profile_contains_path(&profile, &install_dir)
            .expect("existing path entry should be accepted")
    );
    assert_eq!(std::fs::read_to_string(&profile).expect("profile should remain readable"), first);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn existing_profile_without_trailing_newline_gets_a_separate_path_line() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let profile = dir.path().join("profile");
    let install_dir = dir.path().join("bin");
    std::fs::write(&profile, "existing").expect("profile should be written");

    assert!(
        super::ensure_profile_contains_path(&profile, &install_dir)
            .expect("path entry should be appended")
    );

    let contents = std::fs::read_to_string(&profile).expect("profile should be readable");
    assert!(contents.starts_with("existing\n"));
    assert!(contents.ends_with('\n'));
}

#[cfg(not(target_arch = "wasm32"))]
// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn path_profiles_are_created_only_inside_supplied_home() {
    let home = tempfile::tempdir().expect("temporary home should be created");
    let install_dir = home.path().join(".shiju-harness").join("bin");

    let updated = super::ensure_path_profiles(home.path(), &install_dir)
        .expect("isolated profiles should be updated");

    #[cfg(windows)]
    assert_eq!(updated.len(), 2);
    #[cfg(not(windows))]
    assert_eq!(updated.len(), 3);
    assert!(updated.iter().all(|path| path.starts_with(home.path().to_string_lossy().as_ref())));
    assert!(updated.iter().all(|path| std::path::Path::new(path).is_file()));
}
