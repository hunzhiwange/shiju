//! 记忆加载器模块
//!
//! 本模块提供了从记忆系统中加载和过滤上下文信息的能力。记忆加载器负责
//! 根据用户消息检索相关的历史记忆条目，并根据相关性分数进行过滤，
//! 最终生成格式化的上下文字符串供代理使用。
//!
//! # 主要组件
//!
//! - [`MemoryLoader`]: 定义记忆加载行为的核心 trait
//! - [`DefaultMemoryLoader`]: 默认实现，提供基于相关性分数的过滤功能
//!
//! # 架构说明
//!
//! 模块采用 trait 抽象以支持不同的加载策略。默认实现会：
//! 1. 根据用户消息查询相关记忆
//! 2. 过滤掉助手自动保存的条目
//! 3. 根据最小相关性分数过滤低质量结果
//! 4. 生成格式化的上下文字符串

use agent_memory::{self as memory, Memory};
use async_trait::async_trait;
use std::fmt::Write;

/// 记忆加载器边界 trait（非 WASM 目标）
///
/// 在非 WebAssembly 目标平台上，要求实现者必须是线程安全的（Send + Sync），
/// 以支持多线程并发访问。
#[cfg(not(target_arch = "wasm32"))]
pub trait MemoryLoaderBounds: Send + Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> MemoryLoaderBounds for T {}

/// 记忆加载器边界 trait（WASM 目标）
///
/// 在 WebAssembly 目标平台上，由于 WASM 单线程模型的限制，
/// 不要求实现者满足 Send 和 Sync 约束。
#[cfg(target_arch = "wasm32")]
pub trait MemoryLoaderBounds {}
#[cfg(target_arch = "wasm32")]
impl<T> MemoryLoaderBounds for T {}

/// 记忆加载器 trait
///
/// 定义了从记忆系统加载上下文信息的标准接口。实现者负责
/// 根据用户消息检索相关记忆，并返回格式化的上下文字符串。
///
/// # 线程安全性
///
/// - 在非 WASM 目标上，实现必须是线程安全的（Send + Sync）
/// - 在 WASM 目标上，不强制线程安全要求
///
/// # 示例
///
/// ```rust,ignore
/// use agent_memory::Memory;
///
/// async fn load_memory(
///     loader: &dyn MemoryLoader,
///     memory: &dyn Memory,
///     query: &str,
/// ) -> anyhow::Result<String> {
///     loader.load_context(memory, query).await
/// }
/// ```
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait MemoryLoader: MemoryLoaderBounds {
    /// 从记忆系统加载上下文信息
    ///
    /// 根据用户消息查询相关记忆条目，经过过滤后返回格式化的上下文字符串。
    ///
    /// # 参数
    ///
    /// - `memory`: 记忆系统的引用，用于查询历史条目
    /// - `user_message`: 用户输入消息，用作查询关键词
    ///
    /// # 返回
    ///
    /// - `Ok(String)`: 格式化的上下文字符串，如果无相关记忆则返回空字符串
    /// - `Err`: 当记忆查询失败时返回错误
    ///
    /// # 上下文格式
    ///
    /// 返回的字符串格式通常为：
    /// ```text
    /// [Memory context]
    /// - key1: content1
    /// - key2: content2
    ///
    /// ```
    async fn load_context(&self, memory: &dyn Memory, user_message: &str)
    -> anyhow::Result<String>;
}

/// 默认记忆加载器
///
/// 提供基于相关性分数过滤的记忆加载实现。该加载器会：
/// 1. 从记忆系统中检索指定数量的相关条目
/// 2. 过滤掉助手自动保存的条目
/// 3. 根据最小相关性分数过滤低质量结果
/// 4. 生成格式化的上下文字符串
///
/// # 配置参数
///
/// - `limit`: 最大检索条目数量（最小值为 1）
/// - `min_relevance_score`: 最小相关性分数阈值（0.0 到 1.0）
///
/// # 示例
///
/// ```rust,ignore
/// let loader = DefaultMemoryLoader::new(10, 0.5);
/// let context = loader.load_context(&memory, "查询内容").await?;
/// ```
pub struct DefaultMemoryLoader {
    /// 最大检索条目数量
    limit: usize,
    /// 最小相关性分数阈值，低于此分数的条目将被过滤
    min_relevance_score: f64,
}

impl Default for DefaultMemoryLoader {
    /// 返回默认配置的加载器实例
    ///
    /// 默认配置：
    /// - 最大条目数：5
    /// - 最小相关性分数：0.4
    fn default() -> Self {
        Self { limit: 5, min_relevance_score: 0.4 }
    }
}

impl DefaultMemoryLoader {
    /// 创建新的记忆加载器实例
    ///
    /// # 参数
    ///
    /// - `limit`: 最大检索条目数量，实际值会被限制为至少 1
    /// - `min_relevance_score`: 最小相关性分数阈值
    ///
    /// # 返回
    ///
    /// 返回配置好的 `DefaultMemoryLoader` 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // 创建最多返回 10 条、相关性分数 >= 0.6 的加载器
    /// let loader = DefaultMemoryLoader::new(10, 0.6);
    /// ```
    pub fn new(limit: usize, min_relevance_score: f64) -> Self {
        Self { limit: limit.max(1), min_relevance_score }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl MemoryLoader for DefaultMemoryLoader {
    /// 从记忆系统加载上下文信息
    ///
    /// # 实现细节
    ///
    /// 1. 调用记忆系统的 `recall` 方法检索相关条目
    /// 2. 遍历结果并进行两重过滤：
    ///    - 跳过助手自动保存的条目（避免循环引用）
    ///    - 跳过相关性分数低于阈值的条目
    /// 3. 将符合条件的条目格式化为列表形式
    /// 4. 如果所有条目都被过滤，返回空字符串
    async fn load_context(
        &self,
        memory: &dyn Memory,
        user_message: &str,
    ) -> anyhow::Result<String> {
        // 从记忆系统中检索相关条目
        let entries = memory.recall(user_message, self.limit, None).await?;

        // 如果没有检索到任何条目，直接返回空字符串
        if entries.is_empty() {
            return Ok(String::new());
        }

        // 构建上下文字符串，以固定前缀开头
        let mut context = String::from("[Memory context]\n");

        for entry in entries {
            // 跳过助手自动保存的条目，避免在响应中重复包含
            if memory::is_assistant_autosave_key(&entry.key) {
                continue;
            }

            // 检查相关性分数，跳过低于阈值的条目
            if let Some(score) = entry.score {
                if score < self.min_relevance_score {
                    continue;
                }
            }

            // 将条目添加到上下文中，格式："- key: content"
            let _ = writeln!(context, "- {}: {}", entry.key, entry.content);
        }

        // 如果所有条目都被过滤掉（context 仍为初始值），返回空字符串
        if context == "[Memory context]\n" {
            return Ok(String::new());
        }

        // 添加末尾空行，改善可读性
        context.push('\n');
        Ok(context)
    }
}

#[cfg(test)]
#[path = "memory_loader_tests.rs"]
mod memory_loader_tests;
#[cfg(test)]
#[path = "tests/memory_loader.rs"]
mod tests;
