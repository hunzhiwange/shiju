//! 项目与调度存储的外键集成测试。
use anyhow::Result;
use agent_project::store::ProjectStore;
use project_types::{Info, TimeInfo, Vcs, Icon, Commands};
use vw_api_transport::id::{TenantScope, ProjectScope, ProjectId, DEFAULT_PLATFORM_ID, DEFAULT_COMPANY_ID};
use vw_api_agent_scheduler::AgentSchedulerScope;
use crate::agent_scheduler::domain::AgentSchedulerContext;
use crate::agent_scheduler::store::AgentSchedulerStore;
use crate::global_database::GlobalDatabase;

#[test]
// 调度记录仍引用项目时拒绝删除，用来锁定数据库外键边界。
fn project_store_restricts_delete_while_scheduler_rows_exist() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let database_path = temp_dir.path().join("global").join("_index.sqlite3");
    let database = GlobalDatabase::new(&database_path)?;
    let store = ProjectStore::new(database.clone())?;
    let tenant = TenantScope::default();
    let scope = project_scope(&tenant, "scheduled-project")?;
    let worktree = temp_dir.path().join("scheduled-project");
    std::fs::create_dir_all(&worktree)?;
    store.create(
        &tenant,
        &full_info("scheduled-project", &worktree.to_string_lossy(), "Scheduled"),
    )?;
    let scheduler = AgentSchedulerStore::from_database_path(&database_path);
    let context = AgentSchedulerContext::new(AgentSchedulerScope::new(scope.clone()), worktree)?;
    scheduler.initialize_project(&context)?;
    database.connect()?.execute(
        "INSERT INTO scheduler_tasks (
             platform_id, company_id, project_id, id, kind, status, title,
             created_at_ms, updated_at_ms
         ) VALUES (
             ?1, ?2, 'scheduled-project', 'fixture', 'finite', 'draft',
             'kept', 1, 1
         )",
        rusqlite::params![DEFAULT_PLATFORM_ID, DEFAULT_COMPANY_ID],
    )?;

    let error = store.delete(&scope).expect_err("scheduler foreign key must restrict delete");
    assert!(error.to_string().contains("FOREIGN KEY constraint failed"));
    assert!(store.get(&scope)?.is_some());
    Ok(())
}

fn project_scope(tenant: &TenantScope, project_id: &str) -> Result<ProjectScope> {
    Ok(ProjectScope::new(tenant.clone(), ProjectId::from(project_id))?)
}

fn full_info(id: &str, worktree: &str, name: &str) -> Info {
    Info {
        id: id.to_owned(),
        worktree: worktree.to_owned(),
        vcs: Some(Vcs::Git),
        name: Some(name.to_owned()),
        icon: Some(Icon {
            url: Some(format!("https://example.test/{id}.png")),
            override_icon: Some("rocket".to_owned()),
            color: Some("#123456".to_owned()),
        }),
        commands: Some(Commands { start: Some("cargo run".to_owned()) }),
        time: TimeInfo { created: 10, updated: 20, initialized: Some(15) },
        sandboxes: vec![format!("{worktree}-sandbox")],
    }
}
