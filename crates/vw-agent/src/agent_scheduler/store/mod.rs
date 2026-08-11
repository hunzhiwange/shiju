use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Result, bail};
use rusqlite::Connection;
use vw_api_types::agent_scheduler::AgentSchedulerScope;
#[cfg(test)]
use vw_api_types::id::ProjectId;

use super::domain::AgentSchedulerContext;
use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;

pub mod artifacts;
pub mod dependencies;
pub mod events;
pub mod logs;
pub mod outbox;
mod postgres;
pub mod resources;
pub mod runs;
pub(super) mod schema;
pub mod tasks;
pub mod teams;
pub mod workers;

#[derive(Debug, Clone)]
pub struct AgentSchedulerStore {
    database_path: PathBuf,
    home: Option<PathBuf>,
    database: Arc<Mutex<Option<GlobalDatabase>>>,
    #[cfg(test)]
    test_home: Option<std::sync::Arc<tempfile::TempDir>>,
    #[cfg(test)]
    register_test_projects: bool,
}

pub trait AgentSchedulerStorageContext: Send + Sync {
    fn storage_scope(&self) -> Result<AgentSchedulerScope>;
    fn storage_project_path(&self) -> &Path;
}

impl AgentSchedulerStorageContext for AgentSchedulerContext {
    fn storage_scope(&self) -> Result<AgentSchedulerScope> {
        Ok(self.scope().clone())
    }

    fn storage_project_path(&self) -> &Path {
        self.project_path()
    }
}

#[cfg(test)]
static TEST_HOME: std::sync::LazyLock<std::sync::Arc<tempfile::TempDir>> =
    std::sync::LazyLock::new(|| {
        std::sync::Arc::new(tempfile::tempdir().expect("create scheduler test home"))
    });

#[cfg(test)]
fn test_scheduler_scope(project_path: &Path) -> Result<AgentSchedulerScope> {
    use std::hash::{DefaultHasher, Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    project_path.hash(&mut hasher);
    Ok(AgentSchedulerScope::for_default_tenant(ProjectId::from(format!(
        "test-project-{:016x}",
        hasher.finish()
    )))?)
}

impl AgentSchedulerStorageContext for Path {
    fn storage_scope(&self) -> Result<AgentSchedulerScope> {
        #[cfg(test)]
        {
            return test_scheduler_scope(self);
        }
        #[cfg(not(test))]
        {
            bail!("explicit agent scheduler scope is required for project path {}", self.display())
        }
    }

    fn storage_project_path(&self) -> &Path {
        self
    }
}

impl AgentSchedulerStorageContext for PathBuf {
    fn storage_scope(&self) -> Result<AgentSchedulerScope> {
        self.as_path().storage_scope()
    }

    fn storage_project_path(&self) -> &Path {
        self.as_path()
    }
}

impl Default for AgentSchedulerStore {
    fn default() -> Self {
        #[cfg(test)]
        {
            return Self::new_for_tests().expect("create temporary agent scheduler test home");
        }
        #[cfg(not(test))]
        {
            Self::for_home(&crate::app::agent::global::paths().home)
        }
    }
}

impl AgentSchedulerStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_home(home: impl AsRef<Path>) -> Self {
        let home = home.as_ref().to_path_buf();
        Self {
            database_path: vw_config_types::paths::global_database_path(&home),
            home: Some(home),
            database: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            test_home: None,
            #[cfg(test)]
            register_test_projects: false,
        }
    }

    pub fn from_database_path(database_path: impl Into<PathBuf>) -> Self {
        Self {
            database_path: database_path.into(),
            home: None,
            database: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            test_home: None,
            #[cfg(test)]
            register_test_projects: false,
        }
    }

    pub fn from_database(database: GlobalDatabase) -> Self {
        let database_path = database.sqlite_path().map(Path::to_path_buf).unwrap_or_default();
        Self {
            database_path,
            home: None,
            database: Arc::new(Mutex::new(Some(database))),
            #[cfg(test)]
            test_home: None,
            #[cfg(test)]
            register_test_projects: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_for_tests() -> Result<Self> {
        let test_home = TEST_HOME.clone();
        Ok(Self {
            database_path: vw_config_types::paths::global_database_path(test_home.path()),
            home: Some(test_home.path().to_path_buf()),
            database: Arc::new(Mutex::new(None)),
            test_home: Some(test_home),
            register_test_projects: true,
        })
    }

    #[cfg(test)]
    pub(crate) fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn initialize_project<C>(&self, context: &C) -> Result<()>
    where
        C: AgentSchedulerStorageContext + ?Sized,
    {
        initialize_provider(self, context)?;
        Ok(())
    }

    fn database(&self) -> Result<GlobalDatabase> {
        let mut initialized_database = self
            .database
            .lock()
            .map_err(|_| anyhow::anyhow!("lock agent scheduler global database"))?;
        if let Some(database) = initialized_database.as_ref() {
            return Ok(database.clone());
        }
        let database = match &self.home {
            Some(home) => GlobalDatabase::for_home(home),
            None => GlobalDatabase::new(&self.database_path),
        }?;
        *initialized_database = Some(database.clone());
        Ok(database)
    }

    #[cfg(test)]
    fn register_test_project(
        &self,
        database: &GlobalDatabase,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        scope: &AgentSchedulerScope,
    ) -> Result<()> {
        if !self.register_test_projects {
            return Ok(());
        }
        debug_assert!(self.test_home.is_some());
        let project_scope = scope.project_scope();
        let store = crate::project::store::ProjectStore::new(database.clone())?;
        let info = vw_shared::project::Info {
            id: project_scope.project_id().as_ref().to_owned(),
            worktree: context.storage_project_path().to_string_lossy().into_owned(),
            vcs: None,
            name: Some("Agent Scheduler test project".to_owned()),
            icon: None,
            commands: None,
            time: vw_shared::project::TimeInfo { created: 0, updated: 0, initialized: None },
            sandboxes: Vec::new(),
        };
        if store.get(project_scope)?.is_none() {
            store.create(project_scope.tenant_scope(), &info)?;
        }

        let runtime_store = crate::project::runtime_store()?;
        if runtime_store.get(project_scope)?.is_none() {
            runtime_store.create(project_scope.tenant_scope(), &info)?;
        }
        Ok(())
    }
}

enum InitializedAgentSchedulerProvider {
    Sqlite(GlobalDatabase),
    Postgres(postgres::PostgresAgentSchedulerStore),
}

fn initialize_provider<C>(
    store: &AgentSchedulerStore,
    context: &C,
) -> Result<InitializedAgentSchedulerProvider>
where
    C: AgentSchedulerStorageContext + ?Sized,
{
    let scope = context.storage_scope()?;
    let database = store.database()?;
    #[cfg(test)]
    store.register_test_project(&database, context, &scope)?;
    match database.provider() {
        DatabaseProvider::Sqlite => {
            database.initialize_sqlite_schema(|connection| {
                schema::initialize_scoped_schema(connection, &scope)?;
                ensure_parent_project(connection, &scope)?;
                Ok(())
            })?;
            Ok(InitializedAgentSchedulerProvider::Sqlite(database))
        }
        DatabaseProvider::Postgres => Ok(InitializedAgentSchedulerProvider::Postgres(
            postgres::PostgresAgentSchedulerStore::new(database, &scope)?,
        )),
    }
}

fn postgres_provider<C>(
    store: &AgentSchedulerStore,
    context: &C,
) -> Result<Option<postgres::PostgresAgentSchedulerStore>>
where
    C: AgentSchedulerStorageContext + ?Sized,
{
    if store.database()?.provider() != DatabaseProvider::Postgres {
        return Ok(None);
    }
    match initialize_provider(store, context)? {
        InitializedAgentSchedulerProvider::Postgres(store) => Ok(Some(store)),
        InitializedAgentSchedulerProvider::Sqlite(_) => unreachable!("provider changed during use"),
    }
}

fn open_initialized_connection<C>(store: &AgentSchedulerStore, context: &C) -> Result<Connection>
where
    C: AgentSchedulerStorageContext + ?Sized,
{
    match initialize_provider(store, context)? {
        InitializedAgentSchedulerProvider::Sqlite(database) => database.connect_sqlite(),
        InitializedAgentSchedulerProvider::Postgres(store) => store.reject_sqlite_connection(),
    }
}

fn open_locked_initialized_connection<C>(
    store: &AgentSchedulerStore,
    context: &C,
) -> Result<((), Connection)>
where
    C: AgentSchedulerStorageContext + ?Sized,
{
    Ok(((), open_initialized_connection(store, context)?))
}

fn ensure_parent_project(connection: &Connection, scope: &AgentSchedulerScope) -> Result<()> {
    let projects_table: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'projects'",
        [],
        |row| row.get(0),
    )?;
    if projects_table != 1 {
        bail!("agent scheduler parent projects table is not initialized");
    }

    let project = scope.project_scope();
    let parent_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM projects
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3",
        scope_params(scope),
        |row| row.get(0),
    )?;
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

fn scope_params(scope: &AgentSchedulerScope) -> (&str, &str, &str) {
    let project = scope.project_scope();
    (project.platform_id().as_ref(), project.company_id().as_ref(), project.project_id().as_ref())
}

#[cfg(test)]
#[path = "dependencies_tests.rs"]
mod dependencies_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
