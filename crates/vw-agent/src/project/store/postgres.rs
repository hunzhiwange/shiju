//! 项目元数据的 PostgreSQL 持久化与行映射实现。

use std::convert::TryFrom;

use anyhow::{Context, Result, bail};
use postgres::{GenericClient, Row};
use serde::de::DeserializeOwned;
use vw_api_types::id::{ProjectScope, TenantScope};
use vw_shared::project::{Info, TimeInfo};

use super::{
    from_sql_timestamp, parse_vcs, serialize_optional_json, serialize_vcs, to_sql_timestamp,
    validate_matching_info,
};
use crate::global_database::GlobalDatabase;

const PROJECTS_SCHEMA: &str = r#"
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
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    initialized_at_ms BIGINT,
    sandboxes_json TEXT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id)
);

CREATE INDEX IF NOT EXISTS idx_projects_tenant_worktree
ON projects(platform_id, company_id, worktree);
"#;

const PROJECT_COLUMNS: &str = "project_id, worktree, vcs, name, icon_json, commands_json,
     created_at_ms, updated_at_ms, initialized_at_ms, sandboxes_json";

#[derive(Debug, Clone)]
/// 项目注册表的 PostgreSQL 后端。
pub(super) struct PostgresProjectStore {
    database: GlobalDatabase,
}

impl PostgresProjectStore {
    /// 初始化租户化项目表后创建后端。
    pub(super) fn new(database: GlobalDatabase) -> Result<Self> {
        database.initialize_postgres_schema(|transaction| {
            transaction
                .batch_execute(PROJECTS_SCHEMA)
                .context("initialize PostgreSQL projects schema")?;
            Ok(())
        })?;
        Ok(Self { database })
    }

    /// 在租户作用域内创建项目。
    pub(super) fn create(&self, tenant: &TenantScope, info: &Info) -> Result<()> {
        let mut connection = self.database.connect_postgres()?;
        insert_project(&mut *connection, tenant, info)
            .context("create scoped PostgreSQL project")?;
        Ok(())
    }

    /// 在事务中导入精确项目快照并删除不再存在的记录。
    pub(super) fn import_exact_registry(
        &self,
        tenant: &TenantScope,
        projects: &[Info],
    ) -> Result<usize> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL project registry import")?;
        let mut imported = 0;
        for project in projects {
            let existing = query_project(
                &mut transaction,
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                &project.id,
                false,
            )?;
            match existing {
                Some(existing) if existing == *project => {}
                Some(_) => bail!("project registry conflict for id: {}", project.id),
                None => {
                    insert_project(&mut transaction, tenant, project)?;
                    imported += 1;
                }
            }
        }

        let target_count: i64 = transaction
            .query_one(
                "SELECT COUNT(*) FROM projects WHERE platform_id = $1 AND company_id = $2",
                &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref()],
            )
            .context("count PostgreSQL project registry rows")?
            .try_get(0)
            .context("read PostgreSQL project registry count")?;
        let source_count = i64::try_from(projects.len())
            .context("project registry source count exceeds PostgreSQL BIGINT range")?;
        if target_count != source_count {
            bail!(
                "project registry count verification failed: source={}, target={target_count}",
                projects.len()
            );
        }

        for project in projects {
            let verified = query_project(
                &mut transaction,
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                &project.id,
                false,
            )?
            .with_context(|| {
                format!("project registry content verification missing id: {}", project.id)
            })?;
            if verified != *project {
                bail!("project registry content verification failed for id: {}", project.id);
            }
        }
        transaction.commit().context("commit PostgreSQL project registry import")?;
        Ok(imported)
    }

    /// 按完整项目作用域读取项目。
    pub(super) fn get(&self, scope: &ProjectScope) -> Result<Option<Info>> {
        let mut connection = self.database.connect_postgres()?;
        query_project(
            &mut *connection,
            scope.platform_id().as_ref(),
            scope.company_id().as_ref(),
            scope.project_id().as_ref(),
            false,
        )
        .context("read scoped PostgreSQL project")
    }

    /// 列出指定租户的项目。
    pub(super) fn list(&self, tenant: &TenantScope) -> Result<Vec<Info>> {
        let mut connection = self.database.connect_postgres()?;
        let sql = format!(
            "SELECT {PROJECT_COLUMNS}
             FROM projects
             WHERE platform_id = $1 AND company_id = $2
             ORDER BY project_id"
        );
        connection
            .query(&sql, &[&tenant.platform_id().as_ref(), &tenant.company_id().as_ref()])
            .context("list scoped PostgreSQL projects")?
            .iter()
            .map(map_project)
            .collect()
    }

    /// 更新完整项目记录并报告是否命中。
    pub(super) fn update(&self, scope: &ProjectScope, info: &Info) -> Result<bool> {
        let mut connection = self.database.connect_postgres()?;
        update_project(&mut *connection, scope, info).context("update scoped PostgreSQL project")
    }

    /// 删除指定作用域项目。
    pub(super) fn delete(&self, scope: &ProjectScope) -> Result<bool> {
        let mut connection = self.database.connect_postgres()?;
        let changed = connection
            .execute(
                "DELETE FROM projects
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3",
                &[
                    &scope.platform_id().as_ref(),
                    &scope.company_id().as_ref(),
                    &scope.project_id().as_ref(),
                ],
            )
            .context("delete scoped PostgreSQL project")?;
        Ok(changed == 1)
    }

    /// 对单个项目执行事务化读改写。
    pub(super) fn mutate(
        &self,
        scope: &ProjectScope,
        mutate: impl FnOnce(&mut Info),
    ) -> Result<Option<Info>> {
        let mut connection = self.database.connect_postgres()?;
        let mut transaction =
            connection.transaction().context("begin PostgreSQL project mutation")?;
        let project = query_project(
            &mut transaction,
            scope.platform_id().as_ref(),
            scope.company_id().as_ref(),
            scope.project_id().as_ref(),
            true,
        )?;
        let Some(mut project) = project else {
            transaction.commit().context("commit empty PostgreSQL project mutation")?;
            return Ok(None);
        };
        mutate(&mut project);
        validate_matching_info(scope, &project)?;
        if !update_project(&mut transaction, scope, &project)? {
            bail!("PostgreSQL project disappeared during locked mutation");
        }
        transaction.commit().context("commit PostgreSQL project mutation")?;
        Ok(Some(project))
    }
}

fn insert_project(
    connection: &mut impl GenericClient,
    tenant: &TenantScope,
    info: &Info,
) -> Result<()> {
    let icon_json = serialize_optional_json(info.icon.as_ref())?;
    let commands_json = serialize_optional_json(info.commands.as_ref())?;
    let created_at_ms = to_sql_timestamp(info.time.created)?;
    let updated_at_ms = to_sql_timestamp(info.time.updated)?;
    let initialized_at_ms = info.time.initialized.map(to_sql_timestamp).transpose()?;
    let sandboxes_json = serde_json::to_string(&info.sandboxes)?;
    connection.execute(
        "INSERT INTO projects (
            platform_id, company_id, project_id, worktree, vcs, name,
            icon_json, commands_json, created_at_ms, updated_at_ms,
            initialized_at_ms, sandboxes_json
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        &[
            &tenant.platform_id().as_ref(),
            &tenant.company_id().as_ref(),
            &info.id,
            &info.worktree,
            &serialize_vcs(info.vcs.as_ref()),
            &info.name,
            &icon_json,
            &commands_json,
            &created_at_ms,
            &updated_at_ms,
            &initialized_at_ms,
            &sandboxes_json,
        ],
    )?;
    Ok(())
}

fn query_project(
    connection: &mut impl GenericClient,
    platform_id: &str,
    company_id: &str,
    project_id: &str,
    for_update: bool,
) -> Result<Option<Info>> {
    let lock = if for_update { " FOR UPDATE" } else { "" };
    let sql = format!(
        "SELECT {PROJECT_COLUMNS}
         FROM projects
         WHERE platform_id = $1 AND company_id = $2 AND project_id = $3{lock}"
    );
    connection
        .query_opt(&sql, &[&platform_id, &company_id, &project_id])?
        .map(|row| map_project(&row))
        .transpose()
}

fn update_project(
    connection: &mut impl GenericClient,
    scope: &ProjectScope,
    info: &Info,
) -> Result<bool> {
    let icon_json = serialize_optional_json(info.icon.as_ref())?;
    let commands_json = serialize_optional_json(info.commands.as_ref())?;
    let created_at_ms = to_sql_timestamp(info.time.created)?;
    let updated_at_ms = to_sql_timestamp(info.time.updated)?;
    let initialized_at_ms = info.time.initialized.map(to_sql_timestamp).transpose()?;
    let sandboxes_json = serde_json::to_string(&info.sandboxes)?;
    let changed = connection.execute(
        "UPDATE projects
         SET worktree = $1, vcs = $2, name = $3, icon_json = $4,
             commands_json = $5, created_at_ms = $6, updated_at_ms = $7,
             initialized_at_ms = $8, sandboxes_json = $9
         WHERE platform_id = $10 AND company_id = $11 AND project_id = $12",
        &[
            &info.worktree,
            &serialize_vcs(info.vcs.as_ref()),
            &info.name,
            &icon_json,
            &commands_json,
            &created_at_ms,
            &updated_at_ms,
            &initialized_at_ms,
            &sandboxes_json,
            &scope.platform_id().as_ref(),
            &scope.company_id().as_ref(),
            &scope.project_id().as_ref(),
        ],
    )?;
    Ok(changed == 1)
}

fn map_project(row: &Row) -> Result<Info> {
    let vcs = row.try_get::<_, Option<String>>(2)?.map(|value| parse_vcs(&value)).transpose()?;
    let icon = parse_optional_json(row.try_get(4)?)?;
    let commands = parse_optional_json(row.try_get(5)?)?;
    let created = from_sql_timestamp(row.try_get(6)?)?;
    let updated = from_sql_timestamp(row.try_get(7)?)?;
    let initialized = row.try_get::<_, Option<i64>>(8)?.map(from_sql_timestamp).transpose()?;
    let sandboxes_json: String = row.try_get(9)?;
    let sandboxes = serde_json::from_str(&sandboxes_json)?;
    Ok(Info {
        id: row.try_get(0)?,
        worktree: row.try_get(1)?,
        vcs,
        name: row.try_get(3)?,
        icon,
        commands,
        time: TimeInfo { created, updated, initialized },
        sandboxes,
    })
}

fn parse_optional_json<T: DeserializeOwned>(value: Option<String>) -> Result<Option<T>> {
    value.map(|json| serde_json::from_str(&json)).transpose().map_err(Into::into)
}

#[cfg(test)]
#[path = "postgres_tests.rs"]
mod postgres_tests;
