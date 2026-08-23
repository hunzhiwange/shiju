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

#[derive(Debug, Clone)]
pub(super) struct PostgresKnowledgeStore {
    database: GlobalDatabase,
}

impl PostgresKnowledgeStore {
    pub(super) fn new(database: GlobalDatabase) -> Result<Self> {
        schema::initialize_schema(&database)?;
        Ok(Self { database })
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
