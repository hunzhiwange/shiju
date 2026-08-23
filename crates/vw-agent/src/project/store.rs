//! 项目元数据的 SQLite 持久化实现与数据库无关入口。
//!
//! 所有读写均以租户和项目作用域为边界，PostgreSQL 操作委托给子模块。

use std::collections::HashSet;
use std::convert::TryFrom;

use anyhow::{Context, Result, bail};
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use serde::de::DeserializeOwned;
use vw_api_types::id::{ProjectScope, TenantScope};
use vw_shared::project::{Info, TimeInfo, Vcs};

use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;

mod postgres;

use postgres::PostgresProjectStore;

const PROJECTS_SCHEMA: &str = r#"
BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS platform (
    platform_id TEXT PRIMARY KEY CHECK (trim(platform_id) <> ''),
    name TEXT NOT NULL CHECK (trim(name) <> '')
);

CREATE TABLE IF NOT EXISTS company (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    name TEXT NOT NULL CHECK (trim(name) <> ''),
    PRIMARY KEY (platform_id, company_id),
    FOREIGN KEY (platform_id) REFERENCES platform(platform_id) ON DELETE RESTRICT
);

INSERT INTO platform (platform_id, name)
VALUES
    ('10000', '默认平台'),
    ('10001', '测试平台1'),
    ('10002', '测试平台2')
ON CONFLICT (platform_id) DO NOTHING;

INSERT INTO company (platform_id, company_id, name)
VALUES
    ('10000', '100000000', '默认公司'),
    ('10000', '100000001', '测试公司1'),
    ('10000', '100000002', '测试公司2'),
    ('10000', '100000003', '测试公司3'),
    ('10000', '100000004', '测试公司4'),
    ('10000', '100000005', '测试公司5'),
    ('10001', '100010000', '默认公司'),
    ('10001', '100010001', '测试公司1'),
    ('10001', '100010002', '测试公司2'),
    ('10001', '100010003', '测试公司3'),
    ('10001', '100010004', '测试公司4'),
    ('10001', '100010005', '测试公司5'),
    ('10002', '100020000', '默认公司'),
    ('10002', '100020001', '测试公司1'),
    ('10002', '100020002', '测试公司2'),
    ('10002', '100020003', '测试公司3'),
    ('10002', '100020004', '测试公司4'),
    ('10002', '100020005', '测试公司5')
ON CONFLICT (platform_id, company_id) DO NOTHING;

CREATE TABLE IF NOT EXISTS projects (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    worktree TEXT NOT NULL CHECK (trim(worktree) <> ''),
    vcs TEXT,
    name TEXT,
    icon_json TEXT,
    commands_json TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    initialized_at_ms INTEGER,
    sandboxes_json TEXT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id)
);

CREATE INDEX IF NOT EXISTS idx_projects_tenant_worktree
ON projects(platform_id, company_id, worktree);

COMMIT;
"#;

#[derive(Debug, Clone)]
/// 按数据库提供方分派、按完整项目作用域隔离的项目注册表。
pub struct ProjectStore {
    provider: ProjectStoreProvider,
}

#[derive(Debug, Clone)]
enum ProjectStoreProvider {
    Sqlite(GlobalDatabase),
    Postgres(PostgresProjectStore),
}

impl ProjectStore {
    /// 初始化项目表结构并创建存储实例。
    pub fn new(database: GlobalDatabase) -> Result<Self> {
        let provider = match database.provider() {
            DatabaseProvider::Sqlite => {
                database.initialize_sqlite_schema(|connection| {
                    connection
                        .execute_batch(PROJECTS_SCHEMA)
                        .context("initialize SQLite projects schema")?;
                    Ok(())
                })?;
                ProjectStoreProvider::Sqlite(database)
            }
            DatabaseProvider::Postgres => {
                ProjectStoreProvider::Postgres(PostgresProjectStore::new(database)?)
            }
        };
        Ok(Self { provider })
    }

    /// 在指定租户下创建项目；重复作用域返回数据库约束错误。
    pub fn create(&self, tenant: &TenantScope, info: &Info) -> Result<()> {
        validate_info(info)?;
        match &self.provider {
            ProjectStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                insert_project(&connection, tenant, info)?;
                Ok(())
            }
            ProjectStoreProvider::Postgres(store) => store.create(tenant, info),
        }
    }

    /// 用精确注册表快照替换租户项目，并校验当前应保留的项目集合。
    pub(crate) fn import_exact_registry(
        &self,
        tenant: &TenantScope,
        projects: &[Info],
    ) -> Result<usize> {
        let mut ids = HashSet::with_capacity(projects.len());
        for project in projects {
            validate_info(project)?;
            if !ids.insert(project.id.as_str()) {
                bail!("duplicate project id in registry import: {}", project.id);
            }
        }

        match &self.provider {
            ProjectStoreProvider::Sqlite(database) => {
                import_exact_registry_sqlite(database, tenant, projects)
            }
            ProjectStoreProvider::Postgres(store) => store.import_exact_registry(tenant, projects),
        }
    }

    /// 按完整作用域读取单个项目。
    pub fn get(&self, scope: &ProjectScope) -> Result<Option<Info>> {
        match &self.provider {
            ProjectStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                connection
                    .query_row(
                        "SELECT project_id, worktree, vcs, name, icon_json, commands_json,
                                created_at_ms, updated_at_ms, initialized_at_ms, sandboxes_json
                         FROM projects
                         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3",
                        scope_params(scope),
                        map_project,
                    )
                    .optional()
                    .context("read scoped SQLite project")
            }
            ProjectStoreProvider::Postgres(store) => store.get(scope),
        }
    }

    /// 按稳定更新时间顺序列出租户项目。
    pub fn list(&self, tenant: &TenantScope) -> Result<Vec<Info>> {
        match &self.provider {
            ProjectStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let mut statement = connection.prepare(
                    "SELECT project_id, worktree, vcs, name, icon_json, commands_json,
                            created_at_ms, updated_at_ms, initialized_at_ms, sandboxes_json
                     FROM projects
                     WHERE platform_id = ?1 AND company_id = ?2
                     ORDER BY project_id",
                )?;
                statement
                    .query_map(
                        params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
                        map_project,
                    )?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .context("list SQLite tenant projects")
            }
            ProjectStoreProvider::Postgres(store) => store.list(tenant),
        }
    }

    /// 更新项目元数据；作用域与载荷标识不一致时拒绝写入。
    pub fn update(&self, scope: &ProjectScope, info: &Info) -> Result<bool> {
        validate_matching_info(scope, info)?;
        match &self.provider {
            ProjectStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let changed = connection.execute(
                    "UPDATE projects
                     SET worktree = ?1, vcs = ?2, name = ?3, icon_json = ?4,
                         commands_json = ?5, created_at_ms = ?6, updated_at_ms = ?7,
                         initialized_at_ms = ?8, sandboxes_json = ?9
                     WHERE platform_id = ?10 AND company_id = ?11 AND project_id = ?12",
                    params![
                        info.worktree,
                        serialize_vcs(info.vcs.as_ref()),
                        info.name,
                        serialize_optional_json(info.icon.as_ref())?,
                        serialize_optional_json(info.commands.as_ref())?,
                        to_sql_timestamp(info.time.created)?,
                        to_sql_timestamp(info.time.updated)?,
                        info.time.initialized.map(to_sql_timestamp).transpose()?,
                        serde_json::to_string(&info.sandboxes)?,
                        scope.platform_id().as_ref(),
                        scope.company_id().as_ref(),
                        scope.project_id().as_ref(),
                    ],
                )?;
                Ok(changed == 1)
            }
            ProjectStoreProvider::Postgres(store) => store.update(scope, info),
        }
    }

    /// 删除单个作用域项目，并报告是否实际删除。
    pub fn delete(&self, scope: &ProjectScope) -> Result<bool> {
        match &self.provider {
            ProjectStoreProvider::Sqlite(database) => {
                let connection = database.connect_sqlite()?;
                let changed = connection.execute(
                    "DELETE FROM projects
                     WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3",
                    scope_params(scope),
                )?;
                Ok(changed == 1)
            }
            ProjectStoreProvider::Postgres(store) => store.delete(scope),
        }
    }

    /// 向项目沙箱集合添加一个去重后的名称。
    pub fn add_sandbox(&self, scope: &ProjectScope, sandbox: &str) -> Result<bool> {
        Ok(self
            .mutate(scope, |project| {
                if !project.sandboxes.iter().any(|existing| existing == sandbox) {
                    project.sandboxes.push(sandbox.to_owned());
                }
            })?
            .is_some())
    }

    /// 从项目沙箱集合移除指定名称。
    pub fn remove_sandbox(&self, scope: &ProjectScope, sandbox: &str) -> Result<bool> {
        Ok(self
            .mutate(scope, |project| {
                project.sandboxes.retain(|existing| existing != sandbox);
            })?
            .is_some())
    }

    /// 在事务内加载、修改并写回项目，项目不存在时返回 `false`。
    pub(crate) fn mutate(
        &self,
        scope: &ProjectScope,
        mutate: impl FnOnce(&mut Info),
    ) -> Result<Option<Info>> {
        match &self.provider {
            ProjectStoreProvider::Sqlite(database) => mutate_sqlite(database, scope, mutate),
            ProjectStoreProvider::Postgres(store) => store.mutate(scope, mutate),
        }
    }
}

fn import_exact_registry_sqlite(
    database: &GlobalDatabase,
    tenant: &TenantScope,
    projects: &[Info],
) -> Result<usize> {
    database.with_sqlite_transaction(TransactionBehavior::Immediate, |transaction| {
        let mut imported = 0;
        for project in projects {
            let existing = transaction
                .query_row(
                    "SELECT project_id, worktree, vcs, name, icon_json, commands_json,
                                created_at_ms, updated_at_ms, initialized_at_ms, sandboxes_json
                         FROM projects
                         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3",
                    params![
                        tenant.platform_id().as_ref(),
                        tenant.company_id().as_ref(),
                        project.id,
                    ],
                    map_project,
                )
                .optional()?;
            match existing {
                Some(existing) if existing == *project => {}
                Some(_) => bail!("project registry conflict for id: {}", project.id),
                None => {
                    insert_project(transaction, tenant, project)?;
                    imported += 1;
                }
            }
        }

        let target_count: usize = transaction.query_row(
            "SELECT COUNT(*) FROM projects WHERE platform_id = ?1 AND company_id = ?2",
            params![tenant.platform_id().as_ref(), tenant.company_id().as_ref()],
            |row| row.get(0),
        )?;
        if target_count != projects.len() {
            bail!(
                "project registry count verification failed: source={}, target={target_count}",
                projects.len()
            );
        }

        for project in projects {
            let verified = transaction.query_row(
                "SELECT project_id, worktree, vcs, name, icon_json, commands_json,
                            created_at_ms, updated_at_ms, initialized_at_ms, sandboxes_json
                     FROM projects
                     WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3",
                params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), project.id,],
                map_project,
            )?;
            if verified != *project {
                bail!("project registry content verification failed for id: {}", project.id);
            }
        }
        Ok(imported)
    })
}

fn mutate_sqlite(
    database: &GlobalDatabase,
    scope: &ProjectScope,
    mutate: impl FnOnce(&mut Info),
) -> Result<Option<Info>> {
    database.with_sqlite_transaction(TransactionBehavior::Immediate, |transaction| {
        let project = transaction
            .query_row(
                "SELECT project_id, worktree, vcs, name, icon_json, commands_json,
                            created_at_ms, updated_at_ms, initialized_at_ms, sandboxes_json
                     FROM projects
                     WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3",
                scope_params(scope),
                map_project,
            )
            .optional()?;
        let Some(mut project) = project else {
            return Ok(None);
        };
        mutate(&mut project);
        validate_matching_info(scope, &project)?;
        transaction.execute(
            "UPDATE projects
                 SET worktree = ?1, vcs = ?2, name = ?3, icon_json = ?4,
                     commands_json = ?5, created_at_ms = ?6, updated_at_ms = ?7,
                     initialized_at_ms = ?8, sandboxes_json = ?9
                 WHERE platform_id = ?10 AND company_id = ?11 AND project_id = ?12",
            params![
                project.worktree,
                serialize_vcs(project.vcs.as_ref()),
                project.name,
                serialize_optional_json(project.icon.as_ref())?,
                serialize_optional_json(project.commands.as_ref())?,
                to_sql_timestamp(project.time.created)?,
                to_sql_timestamp(project.time.updated)?,
                project.time.initialized.map(to_sql_timestamp).transpose()?,
                serde_json::to_string(&project.sandboxes)?,
                scope.platform_id().as_ref(),
                scope.company_id().as_ref(),
                scope.project_id().as_ref(),
            ],
        )?;
        Ok(Some(project))
    })
}

fn insert_project(connection: &Connection, tenant: &TenantScope, info: &Info) -> Result<()> {
    connection.execute(
        "INSERT INTO projects (
            platform_id, company_id, project_id, worktree, vcs, name,
            icon_json, commands_json, created_at_ms, updated_at_ms,
            initialized_at_ms, sandboxes_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            tenant.platform_id().as_ref(),
            tenant.company_id().as_ref(),
            info.id,
            info.worktree,
            serialize_vcs(info.vcs.as_ref()),
            info.name,
            serialize_optional_json(info.icon.as_ref())?,
            serialize_optional_json(info.commands.as_ref())?,
            to_sql_timestamp(info.time.created)?,
            to_sql_timestamp(info.time.updated)?,
            info.time.initialized.map(to_sql_timestamp).transpose()?,
            serde_json::to_string(&info.sandboxes)?,
        ],
    )?;
    Ok(())
}

fn validate_info(info: &Info) -> Result<()> {
    if info.id.trim().is_empty() {
        bail!("project id must not be empty");
    }
    if info.worktree.trim().is_empty() {
        bail!("project worktree must not be empty");
    }
    Ok(())
}

fn validate_matching_info(scope: &ProjectScope, info: &Info) -> Result<()> {
    validate_info(info)?;
    if info.id != scope.project_id().as_ref() {
        bail!("project info id must match project scope");
    }
    Ok(())
}

fn scope_params(scope: &ProjectScope) -> (&str, &str, &str) {
    (scope.platform_id().as_ref(), scope.company_id().as_ref(), scope.project_id().as_ref())
}

fn map_project(row: &Row<'_>) -> rusqlite::Result<Info> {
    let vcs = row
        .get::<_, Option<String>>(2)?
        .map(|value| parse_vcs(&value))
        .transpose()
        .map_err(|error| conversion_error(2, Type::Text, error))?;
    let icon = parse_optional_json(row, 4)?;
    let commands = parse_optional_json(row, 5)?;
    let created = from_sql_timestamp(row.get(6)?)
        .map_err(|error| conversion_error(6, Type::Integer, error))?;
    let updated = from_sql_timestamp(row.get(7)?)
        .map_err(|error| conversion_error(7, Type::Integer, error))?;
    let initialized = row
        .get::<_, Option<i64>>(8)?
        .map(from_sql_timestamp)
        .transpose()
        .map_err(|error| conversion_error(8, Type::Integer, error))?;
    let sandboxes_json: String = row.get(9)?;
    let sandboxes = serde_json::from_str(&sandboxes_json)
        .map_err(|error| conversion_error(9, Type::Text, error))?;

    Ok(Info {
        id: row.get(0)?,
        worktree: row.get(1)?,
        vcs,
        name: row.get(3)?,
        icon,
        commands,
        time: TimeInfo { created, updated, initialized },
        sandboxes,
    })
}

fn serialize_vcs(vcs: Option<&Vcs>) -> Option<&'static str> {
    vcs.map(|value| match value {
        Vcs::Git => "git",
    })
}

fn parse_vcs(value: &str) -> std::io::Result<Vcs> {
    match value {
        "git" => Ok(Vcs::Git),
        other => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unsupported project vcs: {other}"),
        )),
    }
}

fn serialize_optional_json<T: serde::Serialize>(value: Option<&T>) -> Result<Option<String>> {
    value.map(serde_json::to_string).transpose().map_err(Into::into)
}

fn parse_optional_json<T: DeserializeOwned>(
    row: &Row<'_>,
    index: usize,
) -> rusqlite::Result<Option<T>> {
    row.get::<_, Option<String>>(index)?
        .map(|value| serde_json::from_str(&value))
        .transpose()
        .map_err(|error| conversion_error(index, Type::Text, error))
}

fn to_sql_timestamp(value: u64) -> Result<i64> {
    i64::try_from(value).context("project timestamp exceeds SQLite integer range")
}

fn from_sql_timestamp(value: i64) -> std::result::Result<u64, std::num::TryFromIntError> {
    u64::try_from(value)
}

fn conversion_error(
    index: usize,
    value_type: Type,
    error: impl std::error::Error + Send + Sync + 'static,
) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(index, value_type, Box::new(error))
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod store_tests;
