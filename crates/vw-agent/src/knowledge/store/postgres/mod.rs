//! PostgreSQL 知识库后端的模块装配与模式初始化入口。

mod cache;
mod chunks;
mod datasets;
mod documents;
mod indexing;
mod retrieval;
mod schema;

use anyhow::Result;

use crate::global_database::GlobalDatabase;

/// 持有全局数据库句柄的 PostgreSQL 知识库后端。
#[derive(Debug, Clone)]
pub(super) struct PostgresKnowledgeStore {
    database: GlobalDatabase,
}

impl PostgresKnowledgeStore {
    /// 初始化租户化表结构后创建后端实例。
    pub(super) fn new(database: GlobalDatabase) -> Result<Self> {
        schema::initialize_schema(&database)?;
        Ok(Self { database })
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
