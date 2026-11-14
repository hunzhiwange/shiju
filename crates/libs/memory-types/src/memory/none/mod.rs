//! 空操作内存后端模块
//!
//! 本模块提供了 `NoneMemory` 类型，这是一个显式的空操作内存后端实现。
//! 当配置 `memory.backend = "none"` 时使用此后端，用于在禁用持久化的同时
//! 保持运行时连接的稳定性。
//!
//! # 设计目的
//!
//! - **配置占位**：允许用户显式禁用记忆功能，而非使用隐式回退
//! - **测试隔离**：在单元测试中提供无副作用的内存后端
//! - **资源节约**：在不需要记忆功能的场景下避免不必要的资源消耗
//! - **接口一致**：实现完整的 `Memory` trait，确保调用方无需特殊处理
//!
//! # 行为特性
//!
//! - 所有写入操作（`store`、`forget`）静默成功但不实际存储
//! - 所有读取操作（`recall`、`get`、`list`）返回空结果
//! - 计数操作（`count`）始终返回 0
//! - 健康检查（`health_check`）始终返回 `true`

use super::traits::{Memory, MemoryCategory, MemoryEntry};
use async_trait::async_trait;

/// 显式空操作内存后端
///
/// 该结构体是一个零大小的类型（ZST），实现了 `Memory` trait 的空操作版本。
/// 所有方法调用都立即返回成功或空结果，不执行任何实际存储操作。
///
/// # 特性
///
/// - **零开销**：`NoneMemory` 是零大小类型，不占用运行时内存
/// - **无状态**：不维护任何内部状态，所有实例完全等价
/// - **Copy 语义**：实现了 `Copy` trait，可以自由复制
/// - **线程安全**：由于无状态，天然线程安全
///
/// # 使用示例
///
/// ```rust
/// use memory_types::memory::{Memory, NoneMemory};
///
/// let memory = NoneMemory::new();
///
/// // 存储操作静默成功
/// memory.store("key", "value", MemoryCategory::General, None).await?;
///
/// // 读取操作返回空结果
/// let entries = memory.recall("query", 10, None).await?;
/// assert!(entries.is_empty());
/// ```
///
/// # 适用场景
///
/// - 配置中显式禁用记忆功能
/// - 单元测试中需要无副作用的内存后端
/// - 临时调试或性能分析时隔离记忆子系统
#[derive(Debug, Default, Clone, Copy)]
pub struct NoneMemory;

impl NoneMemory {
    /// 创建新的 `NoneMemory` 实例
    ///
    /// 由于 `NoneMemory` 是零大小类型，所有实例完全等价。
    /// 此方法主要提供显式的构造语法，增强代码可读性。
    ///
    /// # 返回值
    ///
    /// 返回一个 `NoneMemory` 实例。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use memory_types::memory::NoneMemory;
    ///
    /// let memory = NoneMemory::new();
    /// ```
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Memory for NoneMemory {
    /// 返回后端名称标识
    ///
    /// 返回固定字符串 `"none"`，用于日志记录和调试时识别后端类型。
    ///
    /// # 返回值
    ///
    /// 始终返回 `&"none"`。
    fn name(&self) -> &str {
        "none"
    }

    /// 空操作存储方法
    ///
    /// 静默忽略所有存储请求，立即返回成功。
    /// 不执行任何实际的持久化操作。
    ///
    /// # 参数
    ///
    /// - `_key`: 记忆条目的键（被忽略）
    /// - `_content`: 记忆条目的内容（被忽略）
    /// - `_category`: 记忆条目的分类（被忽略）
    /// - `_session_id`: 可选的会话标识符（被忽略）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(())`。
    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        // 空操作：静默忽略存储请求
        Ok(())
    }

    /// 空操作召回方法
    ///
    /// 始终返回空的记忆条目列表，不执行任何实际的检索操作。
    ///
    /// # 参数
    ///
    /// - `_query`: 检索查询字符串（被忽略）
    /// - `_limit`: 返回结果的最大数量（被忽略）
    /// - `_session_id`: 可选的会话标识符（被忽略）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(Vec::new())`，即空向量。
    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        // 空操作：返回空结果集
        Ok(Vec::new())
    }

    /// 空操作获取方法
    ///
    /// 始终返回 `None`，表示未找到指定键的记忆条目。
    ///
    /// # 参数
    ///
    /// - `_key`: 要获取的记忆条目键（被忽略）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(None)`。
    async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        // 空操作：表示条目不存在
        Ok(None)
    }

    /// 空操作列表方法
    ///
    /// 始终返回空的记忆条目列表，不执行任何实际的列表操作。
    ///
    /// # 参数
    ///
    /// - `_category`: 可选的分类过滤器（被忽略）
    /// - `_session_id`: 可选的会话标识符（被忽略）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(Vec::new())`，即空向量。
    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        // 空操作：返回空列表
        Ok(Vec::new())
    }

    /// 空操作删除方法
    ///
    /// 静默忽略所有删除请求，返回 `false` 表示未删除任何条目。
    ///
    /// # 参数
    ///
    /// - `_key`: 要删除的记忆条目键（被忽略）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(false)`，表示未找到并删除条目。
    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        // 空操作：表示条目不存在，未执行删除
        Ok(false)
    }

    /// 空操作计数方法
    ///
    /// 始终返回 0，表示存储中没有记忆条目。
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(0)`。
    async fn count(&self) -> anyhow::Result<usize> {
        // 空操作：存储始终为空
        Ok(0)
    }

    /// 健康检查方法
    ///
    /// 由于空操作后端不依赖任何外部资源，始终返回 `true`。
    ///
    /// # 返回值
    ///
    /// 始终返回 `true`。
    async fn health_check(&self) -> bool {
        // 空操作后端始终健康
        true
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
