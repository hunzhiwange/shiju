use anyhow::{Context, Result, bail};
use vw_api_types::agent_scheduler::AgentSchedulerScope;

use crate::global_database::GlobalDatabase;

mod artifacts;
mod events;
mod logs;
mod outbox;
mod resources;
mod runs;
mod schema;
mod tasks;
mod teams;
mod workers;

#[derive(Debug)]
pub(super) struct PostgresAgentSchedulerStore {
    database: GlobalDatabase,
}

impl PostgresAgentSchedulerStore {
    pub(super) fn new(database: GlobalDatabase, scope: &AgentSchedulerScope) -> Result<Self> {
        crate::global_database::run_sync_database(|| {
            database.initialize_postgres_schema(|transaction| {
                schema::reject_unsupported_schema(transaction)?;
                ensure_parent_project(transaction, scope)?;
                schema::initialize_schema(transaction)?;
                Ok(())
            })?;
            Ok(Self { database })
        })
    }

    pub(super) fn reject_sqlite_connection(self) -> Result<rusqlite::Connection> {
        debug_assert_eq!(
            self.database.provider(),
            crate::global_database::config::DatabaseProvider::Postgres
        );
        bail!("PostgreSQL agent scheduler operation provider is not implemented")
    }
}

fn ensure_parent_project(
    connection: &mut impl ::postgres::GenericClient,
    scope: &AgentSchedulerScope,
) -> Result<()> {
    let projects_table: bool = connection
        .query_one(
            "SELECT EXISTS(
                 SELECT 1
                 FROM pg_catalog.pg_class AS class
                 JOIN pg_catalog.pg_namespace AS namespace
                   ON namespace.oid = class.relnamespace
                 WHERE namespace.nspname = current_schema()
                   AND class.relname = 'projects'
                   AND class.relkind IN ('r', 'p')
             )",
            &[],
        )
        .context("inspect PostgreSQL agent scheduler parent projects table")?
        .get(0);
    if !projects_table {
        bail!("agent scheduler parent projects table is not initialized");
    }

    let project = scope.project_scope();
    let parent_count: i64 = connection
        .query_one(
            "SELECT COUNT(*) FROM projects
             WHERE platform_id = $1 AND company_id = $2 AND project_id = $3",
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
            ],
        )
        .context("inspect scoped PostgreSQL agent scheduler parent project")?
        .get(0);
    if parent_count != 1 {
        bail!(
            "agent scheduler parent project does not exist: {}/{}/{}",
            project.platform_id().as_ref(),
            project.company_id().as_ref(),
            project.project_id().as_ref()
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
