//! # SQLite 记忆存储模块测试
//!
//! 本模块包含 `SqliteMemory` 实现的完整测试套件，覆盖以下方面：
//!
//! - **基础操作**：存储、检索、删除记忆条目
//! - **全文搜索**：FTS5 搜索功能与排名
//! - **嵌入缓存**：内容哈希与缓存一致性
//! - **Schema 验证**：表结构与列存在性检查
//! - **会话隔离**：基于 session_id 的数据隔离
//! - **并发安全**：读写冲突与数据完整性
//! - **边界条件**：空值、Unicode、超长内容等边缘情况
//! - **持久化**：数据库重开与数据保留
//!
//! ## 测试组织
//!
//! - 辅助函数：`temp_sqlite()` 用于创建临时测试实例
//! - 按职责拆分到多个子模块，便于定位和维护

use super::*;
pub(super) use crate::memory::MemoryCategory;
pub(super) use crate::memory::embeddings;
use crate::memory::traits::Memory;
use std::sync::Arc;
use tempfile::TempDir;

/// 创建临时的 SQLite 记忆实例用于测试
///
/// # 返回值
///
/// 返回一个元组 `(TempDir, SqliteMemory)`：
/// - `TempDir`：临时目录句柄，离开作用域时自动清理
/// - `SqliteMemory`：初始化完成的 SQLite 记忆实例
///
/// # 示例
///
/// ```ignore
/// let (_tmp, mem) = temp_sqlite();
/// // 使用 mem 进行测试...
/// // _tmp 离开作用域时会自动删除临时文件
/// ```
fn temp_sqlite() -> (TempDir, SqliteMemory) {
    let tmp = TempDir::new().unwrap();
    let mem = SqliteMemory::new(tmp.path()).unwrap();
    (tmp, mem)
}

mod concurrency;
mod lifecycle;
mod schema;
mod search;
mod sessions;
mod storage;
