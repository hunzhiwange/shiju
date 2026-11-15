//! 内存加载器测试模块
//!
//! 本模块提供 `DefaultMemoryLoader` 的单元测试，验证内存上下文加载行为：
//! - 格式化记忆条目为可注入提示的上下文字符串
//! - 过滤遗留的助手自动保存条目以防止虚构信息污染
//!
//! 测试使用 Mock 实现来隔离 `Memory` trait 的行为，避免依赖真实存储后端。

use super::*;
use agent_memory::{Memory, MemoryCategory, MemoryEntry};
use std::sync::Arc;

/// 空记忆 Mock 实现
///
/// 提供最小化的 `Memory` trait 实现，用于测试不需要预置数据的场景：
/// - `store` 为空操作
/// - `recall` 返回固定单条测试记录（当 limit > 0）
/// - `get`/`list` 返回空结果
/// - `count` 返回 0
struct MockMemory;

/// 带预设条目的记忆 Mock 实现
///
/// 使用 `Arc<Vec<MemoryEntry>>` 存储预设的记忆条目，用于测试：
/// - `recall` 直接返回预设条目（忽略查询与限制参数）
/// - `count` 返回预设条目数量
struct MockMemoryWithEntries {
    /// 预设的记忆条目集合
    entries: Arc<Vec<MemoryEntry>>,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Memory for MockMemory {
    /// 存储（空操作）
    ///
    /// Mock 实现不实际存储数据，直接返回成功。
    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// 回忆（返回固定测试记录）
    ///
    /// # 参数
    /// - `limit`: 返回条数限制；若为 0 则返回空
    /// - `_query`: 忽略查询字符串（Mock 实现）
    /// - `_session_id`: 忽略会话 ID（Mock 实现）
    ///
    /// # 返回
    /// 当 limit > 0 时返回单条硬编码的对话记忆条目；否则返回空。
    async fn recall(
        &self,
        _query: &str,
        limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        // limit 为 0 时直接返回空结果
        if limit == 0 {
            return Ok(vec![]);
        }
        // 返回一条固定的测试记忆条目
        Ok(vec![MemoryEntry {
            id: "1".into(),
            key: "k".into(),
            content: "v".into(),
            category: MemoryCategory::Conversation,
            timestamp: "now".into(),
            session_id: None,
            score: None,
        }])
    }

    /// 按键获取（空实现）
    ///
    /// Mock 实现始终返回 `None`。
    async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(None)
    }

    /// 列出条目（空实现）
    ///
    /// Mock 实现始终返回空列表。
    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(vec![])
    }

    /// 删除条目（空实现）
    ///
    /// Mock 实现始终返回 `true`（假装删除成功）。
    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(true)
    }

    /// 计数（空实现）
    ///
    /// Mock 实现始终返回 0。
    async fn count(&self) -> anyhow::Result<usize> {
        Ok(0)
    }

    /// 健康检查
    ///
    /// Mock 实现始终返回 `true`。
    async fn health_check(&self) -> bool {
        true
    }

    /// 获取后端名称
    ///
    /// 返回标识符 `"mock"` 用于调试和日志。
    fn name(&self) -> &str {
        "mock"
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Memory for MockMemoryWithEntries {
    /// 存储（空操作）
    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// 回忆（返回预设条目）
    ///
    /// 直接返回 `self.entries` 中的所有条目，忽略查询参数和限制。
    /// 用于测试加载器对特定条目的处理逻辑（如过滤）。
    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        // 克隆预设条目返回
        Ok(self.entries.as_ref().clone())
    }

    /// 按键获取（空实现）
    async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(None)
    }

    /// 列出条目（空实现）
    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(vec![])
    }

    /// 删除条目（空实现）
    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(true)
    }

    /// 计数（返回预设条目数量）
    async fn count(&self) -> anyhow::Result<usize> {
        Ok(self.entries.len())
    }

    /// 健康检查
    async fn health_check(&self) -> bool {
        true
    }

    /// 获取后端名称
    fn name(&self) -> &str {
        "mock-with-entries"
    }
}

/// 测试默认加载器格式化上下文
///
/// 验证 `DefaultMemoryLoader::load_context` 能正确：
/// 1. 调用记忆后端的 `recall` 方法
/// 2. 将返回的记忆条目格式化为包含 `[Memory context]` 标记的字符串
/// 3. 以 `- key: content` 格式展示条目
#[tokio::test]
async fn default_loader_formats_context() {
    // 使用默认配置创建加载器
    let loader = DefaultMemoryLoader::default();
    // 加载上下文，查询字符串为 "hello"
    let context = loader.load_context(&MockMemory, "hello").await.unwrap();
    // 验证输出包含上下文标记
    assert!(context.contains("[Memory context]"));
    // 验证条目以正确格式展示
    assert!(context.contains("- k: v"));
}

/// 测试默认加载器跳过遗留的助手自动保存条目
///
/// 验证 `DefaultMemoryLoader` 会过滤掉以 `assistant_resp_legacy` 为键的记忆条目。
/// 这些遗留条目可能包含虚构的细节，不应注入到提示上下文中。
///
/// # 背景
/// 旧版本可能自动保存助手的响应作为记忆，但这些内容不可靠。
/// 加载器应识别并排除这类条目，防止虚构信息污染后续对话。
#[tokio::test]
async fn default_loader_skips_legacy_assistant_autosave_entries() {
    // 创建加载器：最多 5 条，阈值 0.0（接受所有分数）
    let loader = DefaultMemoryLoader::new(5, 0.0);
    // 构造包含遗留条目和正常条目的 Mock 记忆后端
    let memory = MockMemoryWithEntries {
        entries: Arc::new(vec![
            // 遗留的助手自动保存条目，应被过滤
            MemoryEntry {
                id: "1".into(),
                key: "assistant_resp_legacy".into(),
                content: "fabricated detail".into(),
                category: MemoryCategory::Daily,
                timestamp: "now".into(),
                session_id: None,
                score: Some(0.95),
            },
            // 正常的用户事实条目，应被保留
            MemoryEntry {
                id: "2".into(),
                key: "user_fact".into(),
                content: "User prefers concise answers".into(),
                category: MemoryCategory::Conversation,
                timestamp: "now".into(),
                session_id: None,
                score: Some(0.9),
            },
        ]),
    };

    // 加载上下文
    let context = loader.load_context(&memory, "answer style").await.unwrap();
    // 验证正常条目被包含
    assert!(context.contains("user_fact"));
    // 验证遗留条目的键被排除
    assert!(!context.contains("assistant_resp_legacy"));
    // 验证遗留条目的内容被排除
    assert!(!context.contains("fabricated detail"));
}
