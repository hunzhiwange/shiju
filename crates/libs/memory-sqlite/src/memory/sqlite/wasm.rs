//! WASM 目标下的 SQLite 记忆后端占位实现。
//!
//! 浏览器或 WASM 运行环境当前没有启用本地 SQLite 存储，因此这里提供显式的空实现。
//! 这样上层可以在编译期保持统一 trait 形状，同时不会伪装成持久化可用。

#[cfg(target_arch = "wasm32")]
use super::SqliteMemory;
#[cfg(target_arch = "wasm32")]
use crate::memory::traits::{Memory, MemoryCategory, MemoryEntry};
#[cfg(target_arch = "wasm32")]
use async_trait::async_trait;

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl Memory for SqliteMemory {
    /// 返回该记忆后端的稳定名称。
    fn name(&self) -> &str {
        "sqlite"
    }

    /// WASM 环境暂不持久化记忆，写入请求会被安全地忽略。
    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// WASM 环境没有本地索引，因此召回始终返回空列表。
    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    /// WASM 环境没有本地记录，因此按键读取始终为空。
    async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(None)
    }

    /// WASM 环境没有本地记录，因此列表始终为空。
    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    /// WASM 环境没有本地记录，因此删除始终返回 `false`。
    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    /// WASM 环境没有本地记录，因此计数始终为 0。
    async fn count(&self) -> anyhow::Result<usize> {
        Ok(0)
    }

    /// WASM 环境未提供真实 SQLite 连接，因此健康检查返回 `false`。
    async fn health_check(&self) -> bool {
        false
    }
}

#[cfg(test)]
#[path = "wasm_tests.rs"]
mod wasm_tests;
