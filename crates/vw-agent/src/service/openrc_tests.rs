//! 验证 OpenRC 脚本、账户约束、状态迁移与探测命令构造。

use super::openrc::generate_openrc_script;
use super::openrc::test_support::{
    binary_path_appears_in_home, copy_dir_recursive, migrate_openrc_runtime_state_with,
    validate_vibewindow_user_entry,
};

#[cfg(unix)]
use super::openrc::{
    build_openrc_writability_probe_command, current_uid, install_linux_openrc, is_root,
    shell_single_quote,
};

#[test]
fn generate_openrc_script_contains_runtime_user_paths_and_logs() {
    let script =
        generate_openrc_script("/usr/local/bin/vibewindow".as_ref(), "/etc/vibewindow".as_ref());

    assert!(script.starts_with("#!/sbin/openrc-run\n"));
    assert!(script.contains("name=\"vibewindow\""));
    assert!(script.contains("description=\"VibeWindow daemon\""));
    assert!(script.contains("command=\"/usr/local/bin/vibewindow\""));
    assert!(script.contains("command_args=\"--config-dir /etc/vibewindow daemon\""));
    assert!(script.contains("command_background=\"yes\""));
    assert!(script.contains("command_user=\"vibewindow:vibewindow\""));
    assert!(script.contains("pidfile=\"/run/${RC_SVCNAME}.pid\""));
    assert!(script.contains("umask 027"));
    assert!(script.contains("output_log=\"/var/log/vibewindow/access.log\""));
    assert!(script.contains("error_log=\"/var/log/vibewindow/error.log\""));
    assert!(script.contains("depend()"));
    assert!(script.contains("need net"));
    assert!(script.contains("after firewall"));
}

#[test]
fn generate_openrc_script_inlines_supplied_paths() {
    let script = generate_openrc_script(
        "/opt/Vibe Window/bin/vibewindow".as_ref(),
        "/etc/vibe window/config".as_ref(),
    );

    assert!(script.contains("command=\"/opt/Vibe Window/bin/vibewindow\""));
    assert!(script.contains("command_args=\"--config-dir /etc/vibe window/config daemon\""));
    assert!(!script.contains("VIBEWINDOW_CONFIG_DIR"));
    assert!(!script.contains("VIBEWINDOW_WORKSPACE"));
}

#[test]
fn user_entry_validation_accepts_short_and_restricted_accounts() {
    for entry in [
        "malformed-entry",
        "vibewindow:x:999:999::/var/lib/vibewindow:/sbin/nologin",
        "vibewindow:x:998:998::/nonexistent:/bin/false",
    ] {
        validate_vibewindow_user_entry(entry, "delete-user", "add-user")
            .expect("restricted service account should be accepted");
    }
}

#[test]
fn user_entry_validation_treats_unparseable_uid_as_restricted_account() {
    validate_vibewindow_user_entry(
        "vibewindow:x:not-a-number:999::/nonexistent:/sbin/nologin",
        "delete-user",
        "add-user",
    )
    .expect("an unparseable UID follows the existing restricted-account fallback");
}

#[test]
fn user_entry_validation_rejects_non_system_uid_with_recovery_commands() {
    let err = validate_vibewindow_user_entry(
        "vibewindow:x:1000:1000::/var/lib/vibewindow:/sbin/nologin",
        "delete-user",
        "add-user",
    )
    .expect_err("regular user UID should be rejected");

    let message = err.to_string();
    assert!(message.contains("unexpected UID 1000"));
    assert!(message.contains("sudo delete-user && sudo add-user"));
}

#[test]
fn user_entry_validation_rejects_login_shell_with_recovery_commands() {
    let err = validate_vibewindow_user_entry(
        "vibewindow:x:999:999::/var/lib/vibewindow:/bin/bash",
        "delete-user",
        "add-user",
    )
    .expect_err("interactive shell should be rejected");

    let message = err.to_string();
    assert!(message.contains("unexpected shell '/bin/bash'"));
    assert!(message.contains("sudo delete-user && sudo add-user"));
}

#[test]
fn user_entry_validation_does_not_reject_unexpected_home() {
    validate_vibewindow_user_entry(
        "vibewindow:x:999:999::/srv/vibewindow:/sbin/nologin",
        "delete-user",
        "add-user",
    )
    .expect("unexpected home is a warning rather than a rejection");
}

#[test]
fn copy_dir_recursive_copies_nested_files_without_overwriting_existing_files() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    std::fs::create_dir_all(source.join("nested")).expect("create source tree");
    std::fs::create_dir_all(&target).expect("create target");
    std::fs::write(source.join("settings.json"), "source-settings").expect("write source file");
    std::fs::write(source.join("nested/state.json"), "nested-state")
        .expect("write nested source file");
    std::fs::write(target.join("settings.json"), "administrator-settings")
        .expect("write existing target file");

    copy_dir_recursive(&source, &target).expect("copy directory tree");

    assert_eq!(
        std::fs::read_to_string(target.join("settings.json")).expect("read preserved file"),
        "administrator-settings"
    );
    assert_eq!(
        std::fs::read_to_string(target.join("nested/state.json")).expect("read copied file"),
        "nested-state"
    );
}

#[test]
fn copy_dir_recursive_creates_target_for_empty_source() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    std::fs::create_dir_all(&source).expect("create empty source directory");

    copy_dir_recursive(&source, &target).expect("copy empty directory");

    assert!(target.is_dir());
    assert_eq!(std::fs::read_dir(target).expect("read target").count(), 0);
}

#[test]
fn copy_dir_recursive_reports_unreadable_source_context() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source_file = temp.path().join("source-file");
    let target = temp.path().join("target");
    std::fs::write(&source_file, "not a directory").expect("write source file");

    let err = copy_dir_recursive(&source_file, &target)
        .expect_err("a file cannot be read as a source directory");

    assert!(err.to_string().contains("Failed to read directory"));
    assert!(err.to_string().contains(source_file.to_string_lossy().as_ref()));
}

#[test]
fn copy_dir_recursive_reports_target_creation_context() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source");
    let blocking_file = temp.path().join("blocking-file");
    let target = blocking_file.join("target");
    std::fs::create_dir_all(&source).expect("create source directory");
    std::fs::write(&blocking_file, "not a directory").expect("write blocking file");

    let err = copy_dir_recursive(&source, &target)
        .expect_err("target below a regular file cannot be created");

    assert!(err.to_string().contains("Failed to create directory"));
    assert!(err.to_string().contains(target.to_string_lossy().as_ref()));
}

#[cfg(unix)]
#[test]
fn copy_dir_recursive_does_not_follow_symbolic_links() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    let external = temp.path().join("external-secret");
    std::fs::create_dir_all(&source).expect("create source directory");
    std::fs::write(&external, "must not be copied").expect("write external file");
    symlink(&external, source.join("linked-secret")).expect("create source symlink");

    copy_dir_recursive(&source, &target).expect("copy source directory");

    assert!(!target.join("linked-secret").exists());
}

#[test]
fn runtime_state_migration_reuses_existing_config_without_resolving_source() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let config_dir = temp.path().join("config");
    let target_config = config_dir.join("vibewindow.json");
    std::fs::create_dir_all(&config_dir).expect("create config directory");
    std::fs::write(&target_config, "existing-config").expect("write existing config");

    migrate_openrc_runtime_state_with(&config_dir, || {
        panic!("source resolver must not run when target config already exists")
    })
    .expect("reuse existing config");

    assert_eq!(
        std::fs::read_to_string(target_config).expect("read existing config"),
        "existing-config"
    );
}

#[test]
fn runtime_state_migration_without_source_is_a_noop() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let config_dir = temp.path().join("config");

    migrate_openrc_runtime_state_with(&config_dir, || None).expect("missing source is allowed");

    assert!(!config_dir.exists());
}

#[test]
fn runtime_state_migration_skips_source_without_config_file() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source");
    let config_dir = temp.path().join("config");
    std::fs::create_dir_all(&source).expect("create source directory");
    std::fs::write(source.join("unrelated.json"), "unrelated").expect("write unrelated file");

    migrate_openrc_runtime_state_with(&config_dir, || Some(source.clone()))
        .expect("source without config is allowed");

    assert!(!config_dir.exists());
}

#[test]
fn runtime_state_migration_copies_state_and_preserves_existing_target_files() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let source = temp.path().join("source");
    let config_dir = temp.path().join("config");
    std::fs::create_dir_all(source.join("sessions")).expect("create source tree");
    std::fs::create_dir_all(&config_dir).expect("create config directory");
    std::fs::write(source.join("vibewindow.json"), "source-config").expect("write source config");
    std::fs::write(source.join("settings.json"), "source-settings").expect("write source settings");
    std::fs::write(source.join("sessions/state.json"), "session-state")
        .expect("write source state");
    std::fs::write(config_dir.join("settings.json"), "administrator-settings")
        .expect("write existing settings");

    migrate_openrc_runtime_state_with(&config_dir, || Some(source.clone()))
        .expect("migrate runtime state");

    assert_eq!(
        std::fs::read_to_string(config_dir.join("vibewindow.json")).expect("read migrated config"),
        "source-config"
    );
    assert_eq!(
        std::fs::read_to_string(config_dir.join("settings.json")).expect("read preserved settings"),
        "administrator-settings"
    );
    assert_eq!(
        std::fs::read_to_string(config_dir.join("sessions/state.json"))
            .expect("read migrated session state"),
        "session-state"
    );
}

#[test]
fn binary_home_detection_distinguishes_user_and_system_paths() {
    assert!(binary_path_appears_in_home("/home/alice/bin/vibewindow".as_ref()));
    assert!(binary_path_appears_in_home("/opt/alice/.cargo/bin/vibewindow".as_ref()));
    assert!(!binary_path_appears_in_home("/usr/local/bin/vibewindow".as_ref()));
    assert!(!binary_path_appears_in_home("/opt/vibewindow/bin/vibewindow".as_ref()));
}

#[cfg(unix)]
#[test]
fn shell_single_quote_wraps_empty_plain_and_apostrophe_values() {
    assert_eq!(shell_single_quote(""), "''");
    assert_eq!(shell_single_quote("/etc/vibewindow"), "'/etc/vibewindow'");
    assert_eq!(shell_single_quote("/tmp/one two"), "'/tmp/one two'");
    assert_eq!(shell_single_quote("/tmp/weird'path"), "'/tmp/weird'\"'\"'path'");
    assert_eq!(shell_single_quote("a'b'c"), "'a'\"'\"'b'\"'\"'c'");
}

#[cfg(unix)]
#[test]
fn build_probe_command_uses_runuser_when_available() {
    let (program, args) =
        build_openrc_writability_probe_command("/etc/vibewindow/work space".as_ref(), true);

    assert_eq!(program, "runuser");
    assert_eq!(
        args,
        vec![
            "-u".to_string(),
            "vibewindow".to_string(),
            "--".to_string(),
            "sh".to_string(),
            "-c".to_string(),
            "test -w '/etc/vibewindow/work space'".to_string(),
        ]
    );
}

#[cfg(unix)]
#[test]
fn build_probe_command_falls_back_to_su_with_quoted_path() {
    let (program, args) =
        build_openrc_writability_probe_command("/tmp/vibe'window".as_ref(), false);

    assert_eq!(program, "su");
    assert_eq!(
        args,
        vec![
            "-s".to_string(),
            "/bin/sh".to_string(),
            "-c".to_string(),
            "test -w '/tmp/vibe'\"'\"'window'".to_string(),
            "vibewindow".to_string(),
        ]
    );
}

#[cfg(unix)]
#[test]
fn root_detection_matches_current_uid() {
    assert_eq!(is_root(), current_uid() == Some(0));
}

#[cfg(unix)]
#[test]
fn openrc_install_requires_root_before_touching_system_paths() {
    if is_root() {
        return;
    }

    let err = install_linux_openrc(&crate::app::agent::config::Config::default())
        .expect_err("non-root OpenRC install should stop before side effects");

    assert!(err.to_string().contains("requires root privileges"));
    assert!(err.to_string().contains("sudo vibewindow service install"));
}
