//! 验证旧项目注册表迁移的冲突保护、备份和失败原子性。

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use vw_api_types::id::TenantScope;
use vw_config_types::paths::{global_database_path, home_config_dir};
use vw_shared::project::{Info, TimeInfo};

use super::{ensure_unique_project_id, migrate_project_registry};
use crate::global_database::GlobalDatabase;
use crate::project::store::ProjectStore;

#[test]
fn project_registry_migrates_json_into_local_tenant_once() -> Result<()> {
    let home = tempfile::tempdir()?;
    let source_dir = source_project_dir(home.path());
    fs::create_dir_all(&source_dir)?;
    let alpha = project_info("alpha", "/workspace/alpha", 10);
    let beta = project_info("beta", "/workspace/beta", 20);
    write_project_json(&source_dir, &alpha)?;
    write_project_json(&source_dir, &beta)?;
    let store = project_store(home.path())?;

    let first = migrate_project_registry(home.path(), &store)?;

    assert_eq!(first.source_count, 2);
    assert_eq!(first.imported_count, 2);
    assert!(first.backed_up);
    assert_eq!(store.list(&TenantScope::default())?, vec![alpha.clone(), beta.clone()]);
    assert!(!source_dir.exists());
    let backup_dir = backup_project_dir(home.path());
    assert_eq!(read_project_json(&backup_dir.join("alpha.json"))?, alpha);
    assert_eq!(read_project_json(&backup_dir.join("beta.json"))?, beta);

    let second = migrate_project_registry(home.path(), &store)?;
    assert_eq!(second.source_count, 0);
    assert_eq!(second.imported_count, 0);
    assert!(!second.backed_up);
    assert_eq!(store.list(&TenantScope::default())?.len(), 2);
    Ok(())
}

#[test]
fn project_registry_migration_preserves_sources_on_conflict() -> Result<()> {
    let home = tempfile::tempdir()?;
    let source_dir = source_project_dir(home.path());
    fs::create_dir_all(&source_dir)?;
    let source = project_info("same", "/workspace/source", 10);
    write_project_json(&source_dir, &source)?;
    let store = project_store(home.path())?;
    store.create(&TenantScope::default(), &project_info("same", "/workspace/conflict", 30))?;

    let error = migrate_project_registry(home.path(), &store).expect_err("conflict must fail");

    assert!(error.to_string().contains("conflict"));
    assert_eq!(read_project_json(&source_dir.join("same.json"))?, source);
    assert!(!backup_project_dir(home.path()).exists());
    Ok(())
}

#[test]
// 任一旧记录无效时保持全部源文件不动，避免部分迁移造成数据分裂。
fn project_registry_migration_preserves_all_sources_on_invalid_input() -> Result<()> {
    let home = tempfile::tempdir()?;
    let source_dir = source_project_dir(home.path());
    fs::create_dir_all(&source_dir)?;
    let valid = project_info("valid", "/workspace/valid", 10);
    write_project_json(&source_dir, &valid)?;
    fs::write(source_dir.join("broken.json"), b"{not-json")?;
    let store = project_store(home.path())?;

    assert!(migrate_project_registry(home.path(), &store).is_err());
    assert!(source_dir.join("valid.json").exists());
    assert!(source_dir.join("broken.json").exists());
    assert!(store.list(&TenantScope::default())?.is_empty());
    assert!(!backup_project_dir(home.path()).exists());

    fs::remove_file(source_dir.join("broken.json"))?;
    let mismatched = project_info("payload-id", "/workspace/mismatch", 20);
    write_project_json_as(&source_dir, "filename-id", &mismatched)?;
    assert!(migrate_project_registry(home.path(), &store).is_err());
    assert!(source_dir.join("valid.json").exists());
    assert!(source_dir.join("filename-id.json").exists());
    assert!(store.list(&TenantScope::default())?.is_empty());
    Ok(())
}

#[test]
fn project_registry_migration_rejects_non_directory_source() -> Result<()> {
    let home = tempfile::tempdir()?;
    let source_dir = source_project_dir(home.path());
    fs::create_dir_all(source_dir.parent().expect("source directory should have a parent"))?;
    fs::write(&source_dir, b"not-a-directory")?;
    let store = project_store(home.path())?;

    let error = migrate_project_registry(home.path(), &store)
        .expect_err("a non-directory registry source must be rejected");

    assert!(error.to_string().contains("source is not a directory"));
    assert_eq!(fs::read(&source_dir)?, b"not-a-directory");
    assert!(!backup_project_dir(home.path()).exists());
    Ok(())
}

#[test]
fn project_registry_migration_rejects_existing_backup_without_moving_sources() -> Result<()> {
    let home = tempfile::tempdir()?;
    let source_dir = source_project_dir(home.path());
    fs::create_dir_all(&source_dir)?;
    let source = project_info("kept", "/workspace/kept", 10);
    write_project_json(&source_dir, &source)?;
    let backup_dir = backup_project_dir(home.path());
    fs::create_dir_all(&backup_dir)?;
    fs::write(backup_dir.join("sentinel"), b"existing-backup")?;
    let store = project_store(home.path())?;

    let error = migrate_project_registry(home.path(), &store)
        .expect_err("an existing backup must prevent migration");

    assert!(error.to_string().contains("backup already exists"));
    assert_eq!(read_project_json(&source_dir.join("kept.json"))?, source);
    assert_eq!(fs::read(backup_dir.join("sentinel"))?, b"existing-backup");
    assert!(store.list(&TenantScope::default())?.is_empty());
    Ok(())
}

#[test]
fn project_registry_migration_moves_empty_registry_with_ignored_entries() -> Result<()> {
    let home = tempfile::tempdir()?;
    let source_dir = source_project_dir(home.path());
    fs::create_dir_all(source_dir.join("nested"))?;
    fs::write(source_dir.join("notes.txt"), b"ignored")?;
    let store = project_store(home.path())?;

    let outcome = migrate_project_registry(home.path(), &store)?;

    assert_eq!(outcome.source_count, 0);
    assert_eq!(outcome.imported_count, 0);
    assert!(outcome.backed_up);
    let backup_dir = backup_project_dir(home.path());
    assert_eq!(fs::read(backup_dir.join("notes.txt"))?, b"ignored");
    assert!(backup_dir.join("nested").is_dir());
    Ok(())
}

#[test]
fn project_registry_migration_reports_backup_parent_creation_failure() -> Result<()> {
    let home = tempfile::tempdir()?;
    let source_dir = source_project_dir(home.path());
    fs::create_dir_all(&source_dir)?;
    let source = project_info("blocked-parent", "/workspace/blocked-parent", 10);
    write_project_json(&source_dir, &source)?;
    let store = project_store(home.path())?;
    let backup_parent = backup_project_dir(home.path())
        .parent()
        .expect("backup directory should have a parent")
        .to_path_buf();
    fs::write(&backup_parent, b"not-a-directory")?;

    let error = migrate_project_registry(home.path(), &store)
        .expect_err("a file at the backup parent path must prevent migration");

    assert!(error.to_string().contains("create project registry backup directory"));
    assert_eq!(read_project_json(&source_dir.join("blocked-parent.json"))?, source);
    assert!(!backup_project_dir(home.path()).exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn project_registry_migration_reports_backup_rename_failure() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let home = tempfile::tempdir()?;
    let source_dir = source_project_dir(home.path());
    fs::create_dir_all(&source_dir)?;
    let source = project_info("rename-blocked", "/workspace/rename-blocked", 10);
    write_project_json(&source_dir, &source)?;
    let store = project_store(home.path())?;
    let backup_parent = backup_project_dir(home.path())
        .parent()
        .expect("backup directory should have a parent")
        .to_path_buf();
    fs::create_dir_all(&backup_parent)?;
    let original_mode = fs::metadata(&backup_parent)?.permissions().mode();
    fs::set_permissions(&backup_parent, fs::Permissions::from_mode(original_mode & !0o222))?;

    let result = migrate_project_registry(home.path(), &store);

    fs::set_permissions(&backup_parent, fs::Permissions::from_mode(original_mode))?;
    let error = result.expect_err("a non-writable backup parent must prevent migration");
    assert!(error.to_string().contains("move project registry into backup"));
    assert_eq!(read_project_json(&source_dir.join("rename-blocked.json"))?, source);
    assert!(!backup_project_dir(home.path()).exists());
    Ok(())
}

#[test]
fn project_registry_migration_rejects_duplicate_payload_ids() -> Result<()> {
    let mut ids = HashSet::new();
    let source = project_info("duplicate", "/workspace/duplicate", 10);

    ensure_unique_project_id(&mut ids, &source)?;
    let error = ensure_unique_project_id(&mut ids, &source)
        .expect_err("a repeated project id must be rejected");

    assert!(error.to_string().contains("duplicate project id"));
    Ok(())
}

#[test]
fn project_callers_no_longer_read_project_json_storage() -> Result<()> {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    let forbidden = concat!("[", "\"project\"");

    for entry in walkdir::WalkDir::new(&source_root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !entry.file_type().is_file()
            || path.extension().and_then(|value| value.to_str()) != Some("rs")
            || path.starts_with(source_root.join("storage"))
            || path.file_name().and_then(|value| value.to_str()).is_some_and(|name| {
                name.ends_with("_tests.rs") || name == "tests.rs" || name == "mod_tests.rs"
            })
        {
            continue;
        }
        let source = fs::read_to_string(path)?;
        for (index, line) in source.lines().enumerate() {
            if line.contains(forbidden) {
                violations.push(format!("{}:{}", path.display(), index + 1));
            }
        }
    }

    assert!(violations.is_empty(), "project JSON storage references remain: {violations:?}");
    Ok(())
}

fn project_store(home: &Path) -> Result<ProjectStore> {
    ProjectStore::new(GlobalDatabase::new(global_database_path(home))?)
}

fn source_project_dir(home: &Path) -> PathBuf {
    home_config_dir(home).join("storage").join("project")
}

// 备份目录与源目录分开构造，用来验证重命名失败时的原子性。
fn backup_project_dir(home: &Path) -> PathBuf {
    home_config_dir(home).join("database").join("migration-backups").join("projects")
}

fn write_project_json(directory: &Path, info: &Info) -> Result<()> {
    write_project_json_as(directory, &info.id, info)
}

fn write_project_json_as(directory: &Path, file_stem: &str, info: &Info) -> Result<()> {
    fs::write(directory.join(format!("{file_stem}.json")), serde_json::to_vec_pretty(info)?)?;
    Ok(())
}

fn read_project_json(path: &Path) -> Result<Info> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn project_info(id: &str, worktree: &str, updated: u64) -> Info {
    Info {
        id: id.to_owned(),
        worktree: worktree.to_owned(),
        vcs: None,
        name: Some(id.to_owned()),
        icon: None,
        commands: None,
        time: TimeInfo { created: 1, updated, initialized: Some(2) },
        sandboxes: vec![format!("{worktree}-sandbox")],
    }
}
