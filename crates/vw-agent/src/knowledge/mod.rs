//! 本地知识库运行时。
//!
//! 统一组织数据集、文档、分块、索引与检索能力，并按全局数据库配置选择后端。

mod chunker;
mod fusion;
pub(crate) mod import;
mod store;

pub use store::SqliteKnowledgeStore;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "import_tests.rs"]
mod import_tests;
