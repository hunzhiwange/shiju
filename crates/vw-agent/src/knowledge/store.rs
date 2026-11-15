//! 基于全局数据库配置的知识存储与检索实现。
//!
//! 该模块承载 SQLite 主实现，并把 PostgreSQL 路径委托给对应后端模块。

pub use agent_knowledge::store::SqliteKnowledgeStore;

// 实现与原有私有辅助测试已迁入 agent-knowledge。
