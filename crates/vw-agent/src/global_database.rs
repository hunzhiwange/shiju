//! 全局数据库独立库的应用入口。
pub use agent_database::*;

#[cfg(test)]
#[path = "global_database_tests.rs"]
mod global_database_tests;
