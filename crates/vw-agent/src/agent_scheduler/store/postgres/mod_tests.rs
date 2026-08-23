//! 测试 PostgreSQL 调度存储的连接、事务和作用域初始化。

use anyhow::Result;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};

use super::PostgresAgentSchedulerStore;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;

#[test]
fn postgres_store_requires_exact_parent_project_scope() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let database = fixture.database().clone();
    let projects = ProjectStore::new(database.clone())?;
    let scope = scheduler_scope("platform-a", "company-a", "project-a")?;

    let error = PostgresAgentSchedulerStore::new(database, &scope)
        .expect_err("missing scoped parent project must fail");
    assert!(error.to_string().contains("parent project does not exist"));

    drop(projects);
    fixture.cleanup()
}

#[test]
fn postgres_store_initializes_for_parent_and_rejects_sqlite_connection() -> Result<()> {
    let fixture = crate::global_database::test_support::postgres_test_database!();
    let database = fixture.database().clone();
    let projects = ProjectStore::new(database.clone())?;
    let tenant = TenantScope::new("platform-a", "company-a")?;
    projects.create(&tenant, &project_info("project-a"))?;
    let scope = AgentSchedulerScope::new(ProjectScope::new(tenant, ProjectId::from("project-a"))?);

    let store = PostgresAgentSchedulerStore::new(database, &scope)?;
    let error = store
        .reject_sqlite_connection()
        .expect_err("PostgreSQL provider must reject SQLite connection access");
    assert_eq!(
        error.to_string(),
        "PostgreSQL agent scheduler operation provider is not implemented"
    );

    drop(projects);
    fixture.cleanup()
}

fn scheduler_scope(
    platform_id: &str,
    company_id: &str,
    project_id: &str,
) -> Result<AgentSchedulerScope> {
    Ok(AgentSchedulerScope::new(ProjectScope::new(
        TenantScope::new(platform_id, company_id)?,
        ProjectId::from(project_id),
    )?))
}

fn project_info(project_id: &str) -> ProjectInfo {
    ProjectInfo {
        id: project_id.to_string(),
        worktree: "/tmp/postgres-store-project".to_string(),
        vcs: None,
        name: Some("PostgreSQL store project".to_string()),
        icon: None,
        commands: None,
        time: ProjectTimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    }
}
