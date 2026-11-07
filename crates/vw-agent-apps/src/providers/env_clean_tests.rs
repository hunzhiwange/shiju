use super::env_clean::{clean, macos::issue};
use super::env_diagnose::diagnose_at;
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{env_clean::*, env_diagnose::*},
};
fn diagnosis(home: &std::path::Path) -> EnvDiagnosis {
    let system = tempfile::tempdir().unwrap();
    issue(
        home,
        AppId::Claude,
        "tenant/host",
        diagnose_at(home, system.path(), AppId::Claude, []).unwrap(),
    )
    .unwrap()
}
fn select(report: &EnvDiagnosis, file: UserShellFile, line: usize) -> EnvCleanSelection {
    report
        .entries
        .iter()
        .find(|e| e.source == EnvSource::UserShell { file, line })
        .unwrap()
        .cleanup
        .clone()
        .unwrap()
}
#[test]
fn cc04_env_clean_removes_only_selected_definition() {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let contents = "# keep\r\nexport ANTHROPIC_API_KEY='first-secret' # selected\r\nexport OTHER=value\r\nexport ANTHROPIC_API_KEY=second-secret\r\n# ANTHROPIC_API_KEY=comment";
    fs::write(home.path().join(".zshrc"), contents).unwrap();
    fs::write(home.path().join(".profile"), "ANTHROPIC_API_KEY=other-source\n").unwrap();
    let report = diagnosis(home.path());
    let selection = select(&report, UserShellFile::Zshrc, 2);
    let result =
        clean(home.path(), private.path(), AppId::Claude, "tenant/host", &[selection]).unwrap();
    assert_eq!(
        fs::read_to_string(home.path().join(".zshrc")).unwrap(),
        contents.replace("export ANTHROPIC_API_KEY='first-secret' ", "")
    );
    assert_eq!(
        fs::read_to_string(home.path().join(".profile")).unwrap(),
        "ANTHROPIC_API_KEY=other-source\n"
    );
    assert_eq!(result.removed_definitions, 1);
}
#[test]
fn cc04_env_clean_backup_precedes_mutation() {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = home.path().join(".zshrc");
    let content = "ANTHROPIC_API_KEY=backup-secret\n";
    fs::write(&path, content).unwrap();
    let report = diagnosis(home.path());
    let selection = select(&report, UserShellFile::Zshrc, 1);
    // A non-directory in the fixed backup location must prevent any source write.
    fs::write(private.path().join("environment-backups"), "obstruction").unwrap();
    assert!(
        clean(
            home.path(),
            private.path(),
            AppId::Claude,
            "tenant/host",
            std::slice::from_ref(&selection)
        )
        .is_err()
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), content);
    fs::remove_file(private.path().join("environment-backups")).unwrap();
    let result =
        clean(home.path(), private.path(), AppId::Claude, "tenant/host", &[selection]).unwrap();
    let backup =
        private.path().join("environment-backups").join(format!("{}.json", result.backup_id));
    let bytes = fs::read(&backup).unwrap();
    let saved: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(saved["files"][0]["original"], content);
    assert_eq!(fs::metadata(backup).unwrap().permissions().mode() & 0o777, 0o600);
    assert_eq!(
        fs::metadata(private.path().join("environment-backups")).unwrap().permissions().mode()
            & 0o777,
        0o700
    );
    assert_eq!(fs::read_to_string(path).unwrap(), "");
}
#[test]
fn cc04_env_clean_stale_forged_and_foreign_receipts_have_no_side_effects() {
    let home = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = home.path().join(".profile");
    let content = "ANTHROPIC_API_KEY=fixture\n";
    fs::write(&path, content).unwrap();
    fs::write(other.path().join(".profile"), content).unwrap();
    let report = diagnosis(home.path());
    let selection = select(&report, UserShellFile::Profile, 1);
    for (target, app, scope) in [
        (other.path(), AppId::Claude, "tenant/host"),
        (home.path(), AppId::Codex, "tenant/host"),
        (home.path(), AppId::Claude, "foreign/host"),
    ] {
        assert!(
            clean(target, private.path(), app, scope, std::slice::from_ref(&selection)).is_err()
        );
    }
    let mut forged = selection.clone();
    forged.diagnostic_id = "forged".into();
    assert!(clean(home.path(), private.path(), AppId::Claude, "tenant/host", &[forged]).is_err());
    let mut forged = selection.clone();
    forged.base_digest = "forged".into();
    assert!(clean(home.path(), private.path(), AppId::Claude, "tenant/host", &[forged]).is_err());
    assert!(clean(home.path(), private.path(), AppId::Claude, "tenant/host", &[]).is_err());
    assert!(
        clean(
            home.path(),
            private.path(),
            AppId::Claude,
            "tenant/host",
            &[selection.clone(), selection.clone()]
        )
        .is_err()
    );
    fs::write(&path, "ANTHROPIC_API_KEY=external\n").unwrap();
    assert!(
        clean(home.path(), private.path(), AppId::Claude, "tenant/host", &[selection]).is_err()
    );
    assert_eq!(fs::read_to_string(path).unwrap(), "ANTHROPIC_API_KEY=external\n");
    assert_eq!(fs::read_dir(private.path()).unwrap().count(), 0);
}
#[test]
fn cc04_env_clean_durable_backup_exists_before_publication_and_external_change_is_preserved() {
    use super::env_clean::macos::execute;
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = home.path().join(".zshrc");
    let content = "ANTHROPIC_API_KEY=before\n";
    fs::write(&path, content).unwrap();
    let report = diagnosis(home.path());
    let selection = select(&report, UserShellFile::Zshrc, 1);
    let result = execute(
        home.path(),
        private.path(),
        AppId::Claude,
        "tenant/host",
        std::slice::from_ref(&selection),
        |id| {
            let saved: serde_json::Value = serde_json::from_slice(
                &fs::read(private.path().join("environment-backups").join(format!("{id}.json")))
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(saved["files"][0]["original"], content);
            assert_eq!(fs::read_to_string(&path).unwrap(), content);
            Err("fixture interruption")
        },
    );
    assert_eq!(result.unwrap_err(), "fixture interruption");
    assert_eq!(fs::read_to_string(&path).unwrap(), content);
    let result =
        execute(home.path(), private.path(), AppId::Claude, "tenant/host", &[selection], |_| {
            fs::write(&path, "ANTHROPIC_API_KEY=external\n").unwrap();
            Ok(())
        });
    assert_eq!(result.unwrap_err(), "environment diagnosis changed");
    assert_eq!(fs::read_to_string(path).unwrap(), "ANTHROPIC_API_KEY=external\n");
    assert_eq!(fs::read_dir(private.path().join("environment-backups")).unwrap().count(), 2);
}
#[test]
fn cc04_env_clean_validates_all_sources_before_any_mutation_and_cleans_multiple_files() {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let a = home.path().join(".bashrc");
    let b = home.path().join(".profile");
    fs::write(&a, "ANTHROPIC_API_KEY=a\nOTHER=keep").unwrap();
    fs::write(&b, "# keep\nANTHROPIC_API_KEY=b").unwrap();
    let report = diagnosis(home.path());
    let selections =
        [select(&report, UserShellFile::Bashrc, 1), select(&report, UserShellFile::Profile, 2)];
    fs::write(&b, "# keep\nANTHROPIC_API_KEY=external").unwrap();
    assert!(clean(home.path(), private.path(), AppId::Claude, "tenant/host", &selections).is_err());
    assert_eq!(fs::read_to_string(&a).unwrap(), "ANTHROPIC_API_KEY=a\nOTHER=keep");
    assert_eq!(fs::read_dir(private.path()).unwrap().count(), 0);
    let report = diagnosis(home.path());
    let selections =
        [select(&report, UserShellFile::Bashrc, 1), select(&report, UserShellFile::Profile, 2)];
    let result =
        clean(home.path(), private.path(), AppId::Claude, "tenant/host", &selections).unwrap();
    assert_eq!(result.removed_definitions, 2);
    assert_eq!(fs::read_to_string(a).unwrap(), "OTHER=keep");
    assert_eq!(fs::read_to_string(b).unwrap(), "# keep\n");
}
#[test]
fn cc04_env_clean_rejects_link_substitution_and_unsafe_backups() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let path = home.path().join(".profile");
    let content = "ANTHROPIC_API_KEY=fixture\n";
    fs::write(&path, content).unwrap();
    let report = diagnosis(home.path());
    let selection = select(&report, UserShellFile::Profile, 1);
    fs::rename(&path, home.path().join("saved")).unwrap();
    symlink(home.path().join("saved"), &path).unwrap();
    assert!(
        clean(home.path(), private.path(), AppId::Claude, "tenant/host", &[selection]).is_err()
    );
    assert_eq!(fs::read_to_string(home.path().join("saved")).unwrap(), content);
    fs::remove_file(&path).unwrap();
    fs::rename(home.path().join("saved"), &path).unwrap();
    let report = diagnosis(home.path());
    let selection = select(&report, UserShellFile::Profile, 1);
    symlink(elsewhere.path(), private.path().join("environment-backups")).unwrap();
    assert!(
        clean(
            home.path(),
            private.path(),
            AppId::Claude,
            "tenant/host",
            std::slice::from_ref(&selection)
        )
        .is_err()
    );
    assert_eq!(fs::read_dir(elsewhere.path()).unwrap().count(), 0);
    fs::remove_file(private.path().join("environment-backups")).unwrap();
    fs::create_dir(private.path().join("environment-backups")).unwrap();
    fs::set_permissions(
        private.path().join("environment-backups"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    assert!(
        clean(home.path(), private.path(), AppId::Claude, "tenant/host", &[selection]).is_err()
    );
    assert_eq!(fs::read_to_string(path).unwrap(), content);
}
#[test]
fn cc04_env_clean_replaced_inode_with_same_bytes_requires_rediagnosis() {
    let home = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = home.path().join(".profile");
    let content = "ANTHROPIC_API_KEY=fixture\n";
    fs::write(&path, content).unwrap();
    let report = diagnosis(home.path());
    let selection = select(&report, UserShellFile::Profile, 1);
    fs::write(home.path().join("replacement"), content).unwrap();
    fs::rename(home.path().join("replacement"), &path).unwrap();
    assert!(
        clean(home.path(), private.path(), AppId::Claude, "tenant/host", &[selection]).is_err()
    );
    assert_eq!(fs::read_to_string(path).unwrap(), content);
    assert_eq!(fs::read_dir(private.path()).unwrap().count(), 0);
}
#[test]
fn cc04_env_clean_home_replacement_after_backup_preserves_both_directories() {
    use super::env_clean::macos::execute;
    let parent = tempfile::tempdir().unwrap();
    let home = parent.path().join("home");
    fs::create_dir(&home).unwrap();
    let private = tempfile::tempdir().unwrap();
    fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let content = "ANTHROPIC_API_KEY=fixture\n";
    fs::write(home.join(".profile"), content).unwrap();
    let report = diagnosis(&home);
    let selection = select(&report, UserShellFile::Profile, 1);
    let result = execute(&home, private.path(), AppId::Claude, "tenant/host", &[selection], |_| {
        fs::rename(&home, parent.path().join("original")).unwrap();
        fs::create_dir(&home).unwrap();
        fs::write(home.join(".profile"), "ANTHROPIC_API_KEY=new-home\n").unwrap();
        Ok(())
    });
    assert_eq!(result.unwrap_err(), "environment diagnosis changed");
    assert_eq!(fs::read_to_string(parent.path().join("original/.profile")).unwrap(), content);
    assert_eq!(fs::read_to_string(home.join(".profile")).unwrap(), "ANTHROPIC_API_KEY=new-home\n");
}
