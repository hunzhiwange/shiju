use super::*;
use crate::providers::live::{commit::*, write_guard::lock_app};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
#[test]
fn aggregation_t48_native_directory_write_preserves_project_and_protected_files() {
    let home = tempfile::tempdir().unwrap();
    fs::set_permissions(home.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = home.path().join("private");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let tx = ProviderTransactions::new(&root, ExecutionHostId::new("host").unwrap()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let project = ProjectScope::new(tenant.clone(), ProjectId::from("project")).unwrap();
    let mut guard = lock_app(AppId::Pi, home.path()).unwrap();
    tx.commit_project_checked(
        &project,
        &mut guard,
        &tx.state(AppId::Pi).unwrap(),
        true,
        &[],
        PendingTarget::default(),
        || Ok(()),
    )
    .unwrap();
    drop(guard);
    let directory = home.path().join(".pi/agent");
    fs::create_dir_all(&directory).unwrap();
    let settings = br#"{"defaultProvider":"native","defaultModel":"selected"}"#;
    let auth = br#"{"native":{"type":"api_key","key":"synthetic-only"}}"#;
    fs::write(directory.join("settings.json"), settings).unwrap();
    fs::write(directory.join("auth.json"), auth).unwrap();
    let revision = read_native_directory(home.path(), None, None).unwrap().source_revision;
    let mut files = PiFiles::open_directory(home.path(), None, None).unwrap();
    let profile = PiProfile {
        name: "Fixture".into(),
        declaration: json!({"baseUrl":"https://fixture.invalid/v1","api":"openai-completions","models":[{"id":"first","name":"first","contextWindow":16000,"maxTokens":2048}]}),
    };
    let result = files.apply_directory_change_for_project(
        &tx,
        &project,
        &revision,
        "fixture",
        PiDirectoryChange::Insert(&profile),
        || true,
    );
    assert_eq!(result, Ok(()));
    assert_eq!(tx.state(AppId::Pi).unwrap().project_scope.as_ref(), Some(&project));
    assert_eq!(fs::read(directory.join("settings.json")).unwrap(), settings);
    assert_eq!(fs::read(directory.join("auth.json")).unwrap(), auth);
    let doc: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("models.json")).unwrap()).unwrap();
    assert_eq!(doc["providers"]["fixture"]["models"][0]["id"], "first");
    let revision = read_native_directory(home.path(), None, None).unwrap().source_revision;
    assert_eq!(
        files.apply_directory_change(
            &tx,
            &tenant,
            &revision,
            "fixture",
            PiDirectoryChange::Remove,
            || true
        ),
        Err(PiDirectoryWriteError::RevisionConflict)
    );
    let foreign = ProjectScope::new(tenant, ProjectId::from("other")).unwrap();
    assert_eq!(
        files.apply_directory_change_for_project(
            &tx,
            &foreign,
            &revision,
            "fixture",
            PiDirectoryChange::Remove,
            || true
        ),
        Err(PiDirectoryWriteError::Forbidden)
    );
    assert_eq!(fs::read(directory.join("settings.json")).unwrap(), settings);
    assert_eq!(fs::read(directory.join("auth.json")).unwrap(), auth);
}
