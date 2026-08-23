//! 测试调度存储门面的初始化、后端选择与事务边界。

use std::path::PathBuf;

use anyhow::Result;

use super::{AgentSchedulerStorageContext, AgentSchedulerStore, scope_params};

#[test]
fn path_storage_context_uses_stable_project_specific_scope() -> Result<()> {
    let first_path = PathBuf::from("/tmp/vibe-window-store-scope-first");
    let second_path = PathBuf::from("/tmp/vibe-window-store-scope-second");

    let path_scope = first_path.as_path().storage_scope()?;
    let path_buf_scope = first_path.storage_scope()?;
    let other_scope = second_path.storage_scope()?;

    assert_eq!(path_scope, path_buf_scope);
    assert_ne!(path_scope.project_scope().project_id(), other_scope.project_scope().project_id());
    assert_eq!(first_path.storage_project_path(), first_path.as_path());

    Ok(())
}

#[test]
fn explicit_database_path_and_scope_parameters_are_preserved() -> Result<()> {
    let database_path = PathBuf::from("/tmp/vibe-window-explicit-scheduler.sqlite3");
    let store = AgentSchedulerStore::from_database_path(database_path.clone());
    assert_eq!(store.database_path(), database_path);

    let scope = PathBuf::from("/tmp/vibe-window-store-scope-params").storage_scope()?;
    let project = scope.project_scope();
    let (platform_id, company_id, project_id) = scope_params(&scope);
    assert_eq!(platform_id, project.platform_id().as_ref());
    assert_eq!(company_id, project.company_id().as_ref());
    assert_eq!(project_id, project.project_id().as_ref());

    Ok(())
}
