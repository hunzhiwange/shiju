//! 本地知识库运行时。
//!
//! 统一组织数据集、文档、分块、索引与检索能力，并按全局数据库配置选择后端。
//! 网关只通过存储门面访问这些能力，不直接依赖具体数据库实现。

mod chunker;
mod fusion;
/// 文档导入载荷的校验与文本提取。
pub(crate) mod import;
mod store;

/// 知识库对外存储门面；名称为兼容既有调用者保留。
pub use store::SqliteKnowledgeStore;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "import_tests.rs"]
mod import_tests;
