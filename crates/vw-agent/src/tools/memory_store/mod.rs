//! 记忆存储工具
//!
//! 将事实、偏好或笔记存储到长期记忆中。支持多种分类（core 永久、daily 会话、
//! conversation 对话），用于代理的持久化知识存储。

use super::traits::{Tool, ToolResult};
use crate::app::agent::memory::{Memory, MemoryCategory};
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::security::policy::ToolOperation;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// 记忆存储工具
///
/// 允许代理将事实、偏好或笔记持久化存储到长期记忆系统中。
/// 这是代理实现自我学习和知识积累的核心工具之一。
///
/// # 支持的记忆类别
///
/// - `core`: 永久性核心记忆，跨会话持久保存
/// - `daily`: 会话级记忆，通常在会话结束后清理
/// - `conversation`: 对话级记忆，用于存储聊天上下文
/// - 自定义类别: 用户可定义任意字符串作为类别名称
///
/// # 安全性
///
/// 所有存储操作都会经过安全策略检查，确保操作被授权后才执行。
pub struct MemoryStoreTool {
    /// 记忆存储后端，实现 Memory trait 的动态分发引用
    memory: Arc<dyn Memory>,
    /// 安全策略，用于检查工具操作权限
    security: Arc<SecurityPolicy>,
}

impl MemoryStoreTool {
    /// 创建新的记忆存储工具实例
    ///
    /// # 参数
    ///
    /// - `memory`: 记忆存储后端的 Arc 引用，必须是实现了 Memory trait 的类型
    /// - `security`: 安全策略的 Arc 引用，用于权限验证
    ///
    /// # 返回值
    ///
    /// 返回配置好的 MemoryStoreTool 实例
    pub fn new(memory: Arc<dyn Memory>, security: Arc<SecurityPolicy>) -> Self {
        Self { memory, security }
    }
}

/// Tool trait 实现
///
/// 为 MemoryStoreTool 实现 Tool trait，使其能够被代理运行时调用。
/// 支持 WASM 和原生两种目标平台的异步执行。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for MemoryStoreTool {
    /// 返回工具名称
    ///
    /// 工具名称 "memory_store" 用于在工具注册表中唯一标识此工具，
    /// 也是调用时使用的标识符。
    fn name(&self) -> &str {
        "memory_store"
    }

    /// 返回工具的功能描述
    ///
    /// 提供给 LLM 的工具说明，帮助模型理解何时以及如何使用此工具。
    /// 描述中包含了各类别的用途说明，便于模型选择合适的记忆类别。
    fn description(&self) -> &str {
        "Store facts, preferences, or notes in long-term memory. Use category 'core' for permanent facts, 'daily' for session notes, 'conversation' for chat context, or provide a custom category name."
    }

    /// 返回工具参数的 JSON Schema 定义
    ///
    /// 定义了工具接受的参数结构，包括：
    /// - `key`: 记忆的唯一标识键（必填）
    /// - `content`: 要存储的记忆内容（必填）
    /// - `category`: 记忆类别（可选，默认为 'core'）
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "Unique key for this memory, such as 'user_lang' or 'project_stack'"
                },
                "content": {
                    "type": "string",
                    "description": "Information to remember"
                },
                "category": {
                    "type": "string",
                    "description": "Memory category: 'core' (permanent), 'daily' (session), 'conversation' (chat), or a custom category name. Defaults to 'core'."
                }
            },
            "required": ["key", "content"]
        })
    }

    /// 执行记忆存储操作
    ///
    /// # 参数
    ///
    /// - `args`: JSON 格式的参数对象，包含 key、content 和可选的 category
    ///
    /// # 返回值
    ///
    /// 返回 `anyhow::Result<ToolResult>`，其中：
    /// - 成功时 `success` 为 true，`output` 包含存储确认信息
    /// - 失败时 `success` 为 false，`error` 包含错误描述
    ///
    /// # 错误处理
    ///
    /// - 参数缺失时返回错误
    /// - 安全策略拒绝时返回权限错误
    /// - 存储失败时返回底层错误信息
    ///
    /// # 示例
    ///
    /// ```json
    /// {
    ///     "key": "user_preferred_language",
    ///     "content": "用户偏好使用中文进行交流",
    ///     "category": "core"
    /// }
    /// ```
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 解析必填参数：记忆的唯一键
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'key' parameter"))?;

        // 解析必填参数：要存储的记忆内容
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;

        // 解析可选参数：记忆类别，默认为 Core
        // 支持预定义类别（core/daily/conversation）和自定义类别
        let category = match args.get("category").and_then(|v| v.as_str()) {
            Some("core") | None => MemoryCategory::Core,
            Some("daily") => MemoryCategory::Daily,
            Some("conversation") => MemoryCategory::Conversation,
            Some(other) => MemoryCategory::Custom(other.to_string()),
        };

        // 安全检查：验证是否允许执行 memory_store 操作
        // 如果安全策略拒绝，立即返回失败结果
        if let Err(error) = self.security.enforce_tool_operation(ToolOperation::Act, "memory_store")
        {
            return Ok(ToolResult { success: false, output: String::new(), error: Some(error) });
        }

        // 执行实际的记忆存储操作
        // None 参数表示不设置过期时间（永久存储）
        match self.memory.store(key, content, category, None).await {
            Ok(()) => Ok(ToolResult {
                success: true,
                output: format!("Stored memory: {key}"),
                error: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to store memory: {e}")),
            }),
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
