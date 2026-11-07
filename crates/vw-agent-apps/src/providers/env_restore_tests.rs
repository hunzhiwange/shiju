use super::{
    env_clean::{clean, macos::issue},
    env_diagnose::diagnose_at,
    env_restore::restore,
};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{env_diagnose::*, env_restore::*},
};
struct Fixture {
    home: tempfile::TempDir,
    private: tempfile::TempDir,
    id: String,
}
const ORIGINAL: &str = "# keep\r\nexport ANTHROPIC_API_KEY='selected-secret' # chosen\r\nANTHROPIC_API_KEY=unselected\r\nOTHER=keep";
fn fixture() -> Fixture {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(home.path().join(".zshrc"), ORIGINAL).unwrap();
    fs::write(home.path().join(".profile"), "ANTHROPIC_API_KEY=other\n").unwrap();
    let system = tempfile::tempdir().unwrap();
    let report = issue(
        home.path(),
        AppId::Claude,
        "tenant/host",
        diagnose_at(home.path(), system.path(), AppId::Claude, []).unwrap(),
    )
    .unwrap();
    let selection = report
        .entries
        .iter()
        .find(|e| e.source == EnvSource::UserShell { file: UserShellFile::Zshrc, line: 2 })
        .unwrap()
        .cleanup
        .clone()
        .unwrap();
    let id = clean(home.path(), private.path(), AppId::Claude, "tenant/host", &[selection])
        .unwrap()
        .backup_id;
    Fixture { home, private, id }
}
#[test]
fn cc04_env_restore_returns_selected_definition() {
    let f = fixture();
    let result =
        restore(f.home.path(), f.private.path(), AppId::Claude, "tenant/host", &f.id).unwrap();
    assert_eq!(fs::read_to_string(f.home.path().join(".zshrc")).unwrap(), ORIGINAL);
    assert_eq!(
        result.items,
        vec![EnvRestoreItem { file: UserShellFile::Zshrc, status: EnvRestoreStatus::Restored }]
    );
    assert_eq!(
        fs::read_to_string(f.home.path().join(".profile")).unwrap(),
        "ANTHROPIC_API_KEY=other\n"
    );
    assert!(!serde_json::to_string(&result).unwrap().contains("secret"));
    let retry =
        restore(f.home.path(), f.private.path(), AppId::Claude, "tenant/host", &f.id).unwrap();
    assert_eq!(retry.items[0].status, EnvRestoreStatus::AlreadyRestored);
}
#[test]
fn cc04_env_restore_rejects_stale_or_foreign_backup() {
    let f = fixture();
    for (app, scope, id) in [
        (AppId::Claude, "foreign/host", f.id.as_str()),
        (AppId::Codex, "tenant/host", f.id.as_str()),
        (AppId::Claude, "tenant/alias", f.id.as_str()),
        (AppId::Claude, "tenant/host", "../outside"),
        (AppId::Claude, "tenant/host", "/tmp/backup.json"),
        (AppId::Claude, "tenant/host", "forged"),
    ] {
        assert!(restore(f.home.path(), f.private.path(), app, scope, id).is_err());
    }
    fs::write(f.home.path().join(".zshrc"), "OTHER=external\n").unwrap();
    let result =
        restore(f.home.path(), f.private.path(), AppId::Claude, "tenant/host", &f.id).unwrap();
    assert_eq!(result.items[0].status, EnvRestoreStatus::Conflict);
    assert_eq!(fs::read_to_string(f.home.path().join(".zshrc")).unwrap(), "OTHER=external\n");
}
#[test]
fn cc04_env_restore_rejects_same_bytes_inode_substitution_and_home_rebinding() {
    let f = fixture();
    let path = f.home.path().join(".zshrc");
    let cleaned = fs::read(&path).unwrap();
    fs::rename(&path, f.home.path().join("retained-inode")).unwrap();
    fs::write(&path, &cleaned).unwrap();
    let result =
        restore(f.home.path(), f.private.path(), AppId::Claude, "tenant/host", &f.id).unwrap();
    assert_eq!(result.items[0].status, EnvRestoreStatus::Conflict);
    assert_eq!(fs::read(&path).unwrap(), cleaned);
    let other = tempfile::tempdir().unwrap();
    fs::write(other.path().join(".zshrc"), &cleaned).unwrap();
    assert!(restore(other.path(), f.private.path(), AppId::Claude, "tenant/host", &f.id).is_err());
    assert_eq!(fs::read(other.path().join(".zshrc")).unwrap(), cleaned);
}
#[test]
fn cc04_env_restore_links_missing_sources_and_unsafe_backups_never_write() {
    use std::os::unix::fs::symlink;
    let f = fixture();
    let path = f.home.path().join(".zshrc");
    let cleaned = fs::read(&path).unwrap();
    fs::rename(&path, f.home.path().join("saved")).unwrap();
    symlink(f.home.path().join("saved"), &path).unwrap();
    let result =
        restore(f.home.path(), f.private.path(), AppId::Claude, "tenant/host", &f.id).unwrap();
    assert_eq!(result.items[0].status, EnvRestoreStatus::Unavailable);
    assert_eq!(fs::read(f.home.path().join("saved")).unwrap(), cleaned);
    fs::remove_file(&path).unwrap();
    let result =
        restore(f.home.path(), f.private.path(), AppId::Claude, "tenant/host", &f.id).unwrap();
    assert_eq!(result.items[0].status, EnvRestoreStatus::Conflict);
    assert!(!path.exists());
    fs::rename(f.home.path().join("saved"), &path).unwrap();
    let backup = f.private.path().join("environment-backups").join(format!("{}.json", f.id));
    let saved = fs::read(&backup).unwrap();
    fs::rename(&backup, f.private.path().join("saved-backup")).unwrap();
    symlink(f.private.path().join("saved-backup"), &backup).unwrap();
    assert!(restore(f.home.path(), f.private.path(), AppId::Claude, "tenant/host", &f.id).is_err());
    assert_eq!(fs::read(f.private.path().join("saved-backup")).unwrap(), saved);
    fs::remove_file(&backup).unwrap();
    fs::rename(f.private.path().join("saved-backup"), &backup).unwrap();
    fs::set_permissions(&backup, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(restore(f.home.path(), f.private.path(), AppId::Claude, "tenant/host", &f.id).is_err());
    fs::set_permissions(&backup, fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(
        f.private.path().join("environment-backups"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    assert!(restore(f.home.path(), f.private.path(), AppId::Claude, "tenant/host", &f.id).is_err());
    assert_eq!(fs::read(&path).unwrap(), cleaned);
}
#[test]
fn cc04_env_restore_reports_partial_conflicts_and_preserves_file_modes() {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let system = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    for name in [".profile", ".zshrc"] {
        fs::write(home.path().join(name), "ANTHROPIC_API_KEY=fixture\nOTHER=keep").unwrap();
        fs::set_permissions(home.path().join(name), fs::Permissions::from_mode(0o640)).unwrap();
    }
    let report = issue(
        home.path(),
        AppId::Claude,
        "tenant/host",
        diagnose_at(home.path(), system.path(), AppId::Claude, []).unwrap(),
    )
    .unwrap();
    let selections: Vec<_> = report.entries.into_iter().filter_map(|e| e.cleanup).collect();
    let id = clean(home.path(), private.path(), AppId::Claude, "tenant/host", &selections)
        .unwrap()
        .backup_id;
    fs::write(home.path().join(".profile"), "OTHER=external\n").unwrap();
    let result = restore(home.path(), private.path(), AppId::Claude, "tenant/host", &id).unwrap();
    assert_eq!(result.items.len(), 2);
    assert!(result.items.contains(&EnvRestoreItem {
        file: UserShellFile::Profile,
        status: EnvRestoreStatus::Conflict
    }));
    assert!(result.items.contains(&EnvRestoreItem {
        file: UserShellFile::Zshrc,
        status: EnvRestoreStatus::Restored
    }));
    assert_eq!(fs::read_to_string(home.path().join(".profile")).unwrap(), "OTHER=external\n");
    assert_eq!(
        fs::read_to_string(home.path().join(".zshrc")).unwrap(),
        "ANTHROPIC_API_KEY=fixture\nOTHER=keep"
    );
    assert_eq!(
        fs::metadata(home.path().join(".zshrc")).unwrap().permissions().mode() & 0o777,
        0o640
    );
    let retry = restore(home.path(), private.path(), AppId::Claude, "tenant/host", &id).unwrap();
    assert!(retry.items.contains(&EnvRestoreItem {
        file: UserShellFile::Zshrc,
        status: EnvRestoreStatus::AlreadyRestored
    }));
}
#[test]
fn cc04_env_restore_interrupted_cleanup_is_not_falsely_restored() {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let system = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(home.path().join(".zshrc"), ORIGINAL).unwrap();
    let report = issue(
        home.path(),
        AppId::Claude,
        "tenant/host",
        diagnose_at(home.path(), system.path(), AppId::Claude, []).unwrap(),
    )
    .unwrap();
    let selection = report
        .entries
        .into_iter()
        .find(|e| e.source == EnvSource::UserShell { file: UserShellFile::Zshrc, line: 2 })
        .unwrap()
        .cleanup
        .unwrap();
    let mut id = String::new();
    let outcome = super::env_clean::macos::execute(
        home.path(),
        private.path(),
        AppId::Claude,
        "tenant/host",
        &[selection],
        |value| {
            id = value.into();
            Err("interrupted")
        },
    );
    assert_eq!(outcome.unwrap_err(), "interrupted");
    let result = restore(home.path(), private.path(), AppId::Claude, "tenant/host", &id).unwrap();
    assert_eq!(result.items[0].status, EnvRestoreStatus::NotCleaned);
    assert_eq!(fs::read_to_string(home.path().join(".zshrc")).unwrap(), ORIGINAL);
}
#[test]
fn cc04_env_restore_rechecks_home_identity_at_publication() {
    let f = fixture();
    let parent = tempfile::tempdir().unwrap();
    let home = parent.path().join("home");
    // Moving the directory preserves its recorded physical identity.
    fs::rename(f.home.path(), &home).unwrap();
    let cleaned = fs::read(home.join(".zshrc")).unwrap();
    let result = super::env_restore::macos::execute(
        &home,
        f.private.path(),
        AppId::Claude,
        "tenant/host",
        &f.id,
        || {
            fs::rename(&home, parent.path().join("old")).unwrap();
            fs::create_dir(&home).unwrap();
            fs::write(home.join(".zshrc"), "OTHER=new-home").unwrap();
        },
    )
    .unwrap();
    assert_eq!(result.items[0].status, EnvRestoreStatus::Conflict);
    assert_eq!(fs::read(parent.path().join("old/.zshrc")).unwrap(), cleaned);
    assert_eq!(fs::read_to_string(home.join(".zshrc")).unwrap(), "OTHER=new-home");
}
#[test]
fn cc04_env_restore_rejects_unbound_corrupt_and_coordinate_bearing_backups() {
    let f = fixture();
    let backup = f.private.path().join("environment-backups").join(format!("{}.json", f.id));
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&backup).unwrap()).unwrap();
    let cleaned = fs::read(f.home.path().join(".zshrc")).unwrap();
    let mut version = saved.clone();
    version["version"] = serde_json::json!(1);
    let mut unbound = saved.clone();
    unbound.as_object_mut().unwrap().remove("homeIdentity");
    let mut coordinate = saved.clone();
    coordinate["files"][0]["file"] = serde_json::json!("../../outside");
    let mut injected = saved.clone();
    injected["files"][0]["path"] = serde_json::json!("/tmp/outside");
    let mut repeated = saved.clone();
    repeated["files"].as_array_mut().unwrap().push(saved["files"][0].clone());
    for value in [version, unbound, coordinate, injected, repeated] {
        fs::write(&backup, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(
            restore(f.home.path(), f.private.path(), AppId::Claude, "tenant/host", &f.id).is_err()
        );
        assert_eq!(fs::read(f.home.path().join(".zshrc")).unwrap(), cleaned);
    }
    fs::write(&backup, b"{corrupt").unwrap();
    assert!(restore(f.home.path(), f.private.path(), AppId::Claude, "tenant/host", &f.id).is_err());
    assert_eq!(fs::read(f.home.path().join(".zshrc")).unwrap(), cleaned);
}
