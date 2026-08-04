use super::install::{
    copy_dir_recursive_secure_for_test, disabled_marker_path, install_dir_from_config,
    install_plugin_from_source, remove_installed_plugin, set_next_post_copy_audit_result_for_test,
    set_plugin_enabled,
};
use crate::config::Config;
use serde_json::json;
use std::fs;
use std::path::Path;

fn write_plugin(plugin_root: &Path, name: &str) {
    fs::create_dir_all(plugin_root.join(".vibewindow-plugin")).expect("manifest dir");
    fs::write(
        plugin_root.join(".vibewindow-plugin/plugin.json"),
        serde_json::to_vec_pretty(&json!({
            "name": name,
            "version": "0.1.0",
            "description": format!("{name} test plugin")
        }))
        .expect("manifest json"),
    )
    .expect("write manifest");
}

#[test]
fn plugin_install_uses_manifest_name_for_destination() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source-dir-name");
    let plugins_dir = dir.path().join("plugins");
    write_plugin(&source, "manifest-plugin");

    let installed =
        install_plugin_from_source(source.to_str().expect("utf-8 source"), &plugins_dir)
            .expect("install plugin");

    assert_eq!(installed.manifest.name, "manifest-plugin");
    assert_eq!(installed.installed_dir, plugins_dir.join("manifest-plugin"));
    assert!(plugins_dir.join("manifest-plugin/.vibewindow-plugin/plugin.json").is_file());
}

#[test]
fn plugin_install_dir_uses_config_parent() {
    let mut config = Config::default();
    config.config_path = "/tmp/vibewindow-config/vibewindow.json".into();

    assert_eq!(install_dir_from_config(&config), Path::new("/tmp/vibewindow-config/plugins"));
}

#[test]
fn plugin_install_rejects_blank_and_missing_sources_without_creating_root() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugins_dir = dir.path().join("plugins");

    let blank = install_plugin_from_source(" \n ", &plugins_dir)
        .expect_err("blank source should be rejected");
    let missing = install_plugin_from_source("missing-plugin", &plugins_dir)
        .expect_err("missing source should be rejected");

    assert!(blank.to_string().contains("cannot be empty"));
    assert!(missing.to_string().contains("does not exist"));
    assert!(!plugins_dir.exists());
}

#[test]
fn plugin_install_rejects_remote_sources_before_touching_install_root() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugins_dir = dir.path().join("plugins");

    for source in [
        "https://example.test/plugin.git",
        "git@github.com:owner/plugin.git",
        "npm:package-name",
        "marketplace:plugin",
    ] {
        let error =
            install_plugin_from_source(source, &plugins_dir).expect_err("remote source must fail");
        assert!(error.to_string().contains("not supported yet"));
    }

    assert!(!plugins_dir.exists());
}

#[test]
fn plugin_install_cleans_destination_when_post_copy_audit_changes_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source");
    let plugins_dir = dir.path().join("plugins");
    write_plugin(&source, "audit-plugin");
    let mut changed = super::load_plugin_manifest(&source).expect("source manifest");
    changed.name = "changed-plugin".to_string();
    set_next_post_copy_audit_result_for_test(Ok(changed));

    let error = install_plugin_from_source(source.to_str().expect("utf-8 source"), &plugins_dir)
        .expect_err("post-copy name change must fail");

    assert!(error.to_string().contains("manifest name changed"));
    assert!(!plugins_dir.join("audit-plugin").exists());
}

#[test]
fn plugin_install_cleans_destination_when_post_copy_audit_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source");
    let plugins_dir = dir.path().join("plugins");
    write_plugin(&source, "audit-error-plugin");
    set_next_post_copy_audit_result_for_test(Err("forced audit failure".to_string()));

    let error = install_plugin_from_source(source.to_str().expect("utf-8 source"), &plugins_dir)
        .expect_err("post-copy audit failure must fail");

    let message = format!("{error:#}");
    assert!(message.contains("installed plugin failed post-copy audit"));
    assert!(message.contains("forced audit failure"));
    assert!(!plugins_dir.join("audit-error-plugin").exists());
}

#[test]
fn plugin_install_rejects_existing_destination_without_overwriting_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source");
    let plugins_dir = dir.path().join("plugins");
    let destination = plugins_dir.join("duplicate-plugin");
    write_plugin(&source, "duplicate-plugin");
    write_plugin(&destination, "duplicate-plugin");
    let sentinel = destination.join("sentinel.txt");
    fs::write(&sentinel, "keep").expect("write sentinel");

    let error = install_plugin_from_source(source.to_str().expect("utf-8 source"), &plugins_dir)
        .expect_err("existing destination should fail");

    assert!(error.to_string().contains("already exists"));
    assert_eq!(fs::read_to_string(sentinel).expect("read sentinel"), "keep");
}

#[test]
fn plugin_install_rejects_unsafe_manifest_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source");
    let plugins_dir = dir.path().join("plugins");
    write_plugin(&source, "../escape");

    let error = install_plugin_from_source(source.to_str().expect("utf-8 source"), &plugins_dir)
        .expect_err("unsafe manifest name should fail");

    assert!(error.to_string().contains("Invalid plugin name"));
    assert!(!dir.path().join("escape").exists());
    assert!(!plugins_dir.exists());
}

#[test]
fn plugin_install_rejects_non_directory_install_root() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source");
    let plugins_dir = dir.path().join("plugins-file");
    write_plugin(&source, "sample-plugin");
    fs::write(&plugins_dir, "not a directory").expect("write install root file");

    let error = install_plugin_from_source(source.to_str().expect("utf-8 source"), &plugins_dir)
        .expect_err("file install root should fail");

    assert!(error.to_string().contains("must be a directory"));
    assert_eq!(fs::read_to_string(&plugins_dir).expect("read install root"), "not a directory");
}

#[cfg(unix)]
#[test]
fn plugin_install_rejects_symlink_install_root() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source");
    let real_plugins_dir = dir.path().join("real-plugins");
    let plugins_dir = dir.path().join("plugins-link");
    write_plugin(&source, "root-link-plugin");
    fs::create_dir(&real_plugins_dir).expect("real install root");
    symlink(&real_plugins_dir, &plugins_dir).expect("symlink install root");

    let error = install_plugin_from_source(source.to_str().expect("utf-8 source"), &plugins_dir)
        .expect_err("symlink install root must fail");

    assert!(error.to_string().contains("must not be a symlink"));
    assert!(!real_plugins_dir.join("root-link-plugin").exists());
}

#[test]
fn plugin_install_reports_install_root_inspection_errors() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source");
    let blocked_parent = dir.path().join("blocked");
    let plugins_dir = blocked_parent.join("plugins");
    write_plugin(&source, "inspect-plugin");
    fs::write(&blocked_parent, "not a directory").expect("block parent");

    let error = install_plugin_from_source(source.to_str().expect("utf-8 source"), &plugins_dir)
        .expect_err("install root inspect error must fail");

    assert!(format!("{error:#}").contains("failed to inspect"));
}

#[test]
fn plugin_enable_disable_is_idempotent() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugins_dir = dir.path().join("plugins");
    write_plugin(&plugins_dir.join("toggle-plugin"), "toggle-plugin");
    let marker = disabled_marker_path(&plugins_dir, "toggle-plugin");

    set_plugin_enabled(&plugins_dir, "toggle-plugin", false).expect("disable plugin");
    set_plugin_enabled(&plugins_dir, "toggle-plugin", false).expect("disable plugin again");
    assert!(marker.is_file());

    set_plugin_enabled(&plugins_dir, "toggle-plugin", true).expect("enable plugin");
    set_plugin_enabled(&plugins_dir, "toggle-plugin", true).expect("enable plugin again");
    assert!(!marker.exists());
}

#[test]
fn plugin_enable_reports_marker_remove_errors() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugins_dir = dir.path().join("plugins");
    write_plugin(&plugins_dir.join("marker-error-plugin"), "marker-error-plugin");
    fs::create_dir(disabled_marker_path(&plugins_dir, "marker-error-plugin"))
        .expect("marker directory");

    let error = set_plugin_enabled(&plugins_dir, "marker-error-plugin", true)
        .expect_err("directory marker must fail remove_file");

    assert!(format!("{error:#}").contains("failed to remove"));
}

#[test]
fn plugin_lifecycle_rejects_invalid_or_mismatched_names() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugins_dir = dir.path().join("plugins");
    write_plugin(&plugins_dir.join("requested-name"), "manifest-name");

    let invalid =
        set_plugin_enabled(&plugins_dir, "../escape", false).expect_err("invalid name should fail");
    let mismatch = remove_installed_plugin(&plugins_dir, "requested-name")
        .expect_err("manifest mismatch should fail");

    assert!(invalid.to_string().contains("Invalid plugin name"));
    assert!(mismatch.to_string().contains("name mismatch"));
    assert!(plugins_dir.join("requested-name").is_dir());
}

#[cfg(unix)]
#[test]
fn plugin_lifecycle_rejects_symlink_installed_plugin_dir() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().expect("temp dir");
    let plugins_dir = dir.path().join("plugins");
    let real_plugin = dir.path().join("real-plugin");
    write_plugin(&real_plugin, "linked-installed-plugin");
    fs::create_dir_all(&plugins_dir).expect("plugins dir");
    symlink(&real_plugin, plugins_dir.join("linked-installed-plugin"))
        .expect("installed plugin symlink");

    let error = set_plugin_enabled(&plugins_dir, "linked-installed-plugin", false)
        .expect_err("installed symlink must fail");

    assert!(error.to_string().contains("must not be a symlink"));
}

#[test]
fn plugin_lifecycle_rejects_file_installed_plugin_dir() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugins_dir = dir.path().join("plugins");
    fs::create_dir_all(&plugins_dir).expect("plugins dir");
    fs::write(plugins_dir.join("file-plugin"), "not a directory").expect("plugin file");

    let error = set_plugin_enabled(&plugins_dir, "file-plugin", false)
        .expect_err("installed file must fail");

    assert!(error.to_string().contains("must be a directory"));
}

#[test]
fn plugin_remove_deletes_directory_and_marker() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugins_dir = dir.path().join("plugins");
    let plugin_dir = plugins_dir.join("remove-plugin");
    write_plugin(&plugin_dir, "remove-plugin");
    set_plugin_enabled(&plugins_dir, "remove-plugin", false).expect("disable plugin");

    remove_installed_plugin(&plugins_dir, "remove-plugin").expect("remove plugin");

    assert!(!plugin_dir.exists());
    assert!(!disabled_marker_path(&plugins_dir, "remove-plugin").exists());
}

#[test]
fn plugin_remove_reports_marker_remove_errors_after_deleting_plugin() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plugins_dir = dir.path().join("plugins");
    let plugin_dir = plugins_dir.join("remove-marker-error-plugin");
    write_plugin(&plugin_dir, "remove-marker-error-plugin");
    fs::create_dir(disabled_marker_path(&plugins_dir, "remove-marker-error-plugin"))
        .expect("marker directory");

    let error = remove_installed_plugin(&plugins_dir, "remove-marker-error-plugin")
        .expect_err("directory marker must fail remove_file");

    assert!(format!("{error:#}").contains("failed to remove"));
    assert!(!plugin_dir.exists());
}

#[cfg(unix)]
#[test]
fn plugin_install_rejects_symlinked_source_root_without_creating_destination() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().expect("temp dir");
    let real_source = dir.path().join("real-source");
    let linked_source = dir.path().join("linked-source");
    let plugins_dir = dir.path().join("plugins");
    write_plugin(&real_source, "source-link-plugin");
    symlink(&real_source, &linked_source).expect("source root symlink");

    let error = copy_dir_recursive_secure_for_test(&linked_source, &plugins_dir.join("copied"))
        .expect_err("symlinked source root must fail");

    assert!(error.to_string().contains("Refusing to copy symlinked plugin source path"));
    assert!(!plugins_dir.join("copied").exists());
}

#[test]
fn plugin_copy_rejects_non_directory_source_root() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("plugin-file");
    let dest = dir.path().join("copied");
    fs::write(&source, "not a directory").expect("source file");

    let error =
        copy_dir_recursive_secure_for_test(&source, &dest).expect_err("file source root must fail");

    assert!(error.to_string().contains("Plugin source must be a directory"));
    assert!(!dest.exists());
}

#[cfg(unix)]
#[test]
fn plugin_install_rejects_unsupported_special_source_entries_and_cleans_destination() {
    use std::ffi::CString;

    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source");
    let plugins_dir = dir.path().join("plugins");
    write_plugin(&source, "special-entry-plugin");
    let fifo = source.join("fifo");
    let fifo_c = CString::new(fifo.to_str().expect("utf-8 fifo path")).expect("cstring");
    let result = unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) };
    assert_eq!(result, 0, "create fifo source entry");

    let error = install_plugin_from_source(source.to_str().expect("utf-8 source"), &plugins_dir)
        .expect_err("special source entry must fail");

    assert!(error.to_string().contains("Unsupported plugin source entry"));
    assert!(!plugins_dir.join("special-entry-plugin").exists());
}

#[cfg(unix)]
#[test]
fn plugin_install_reports_file_copy_errors_and_cleans_destination() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source");
    let plugins_dir = dir.path().join("plugins");
    write_plugin(&source, "copy-error-plugin");
    let protected_file = source.join("protected.txt");
    fs::write(&protected_file, "secret").expect("protected file");
    fs::set_permissions(&protected_file, fs::Permissions::from_mode(0o000))
        .expect("protect source file");

    let error = install_plugin_from_source(source.to_str().expect("utf-8 source"), &plugins_dir)
        .expect_err("unreadable source file must fail copy");

    assert!(format!("{error:#}").contains("failed to copy plugin file"));
    assert!(!plugins_dir.join("copy-error-plugin").exists());
}

#[cfg(unix)]
#[test]
fn plugin_install_rejects_symlink_inside_source_and_cleans_destination() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().expect("temp dir");
    let source = dir.path().join("source");
    let plugins_dir = dir.path().join("plugins");
    write_plugin(&source, "linked-plugin");
    fs::write(dir.path().join("outside.txt"), "outside").expect("write outside file");
    symlink(dir.path().join("outside.txt"), source.join("linked.txt")).expect("create symlink");

    let error = install_plugin_from_source(source.to_str().expect("utf-8 source"), &plugins_dir)
        .expect_err("source symlink should fail");

    assert!(error.to_string().contains("Refusing to copy symlink"));
    assert!(!plugins_dir.join("linked-plugin").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("outside.txt")).expect("read outside file"),
        "outside"
    );
}
