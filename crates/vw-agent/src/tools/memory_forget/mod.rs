//! 记忆删除工具
//!
//! 本模块提供从长期记忆中删除指定键条目的功能。
//!
//! # 主要功能
//!
//! - 按键删除记忆条目
//! - 删除过时的事实或敏感数据
//! - 支持安全策略检查
//!
//! # 使用场景
//!
//! - 清理不再需要的记忆项
//! - 移除过期的信息
//! - 删除敏感数据以满足隐私要求
//!
//! # 安全性
//!
//! 所有删除操作都需通过安全策略检查，
//! 只有被授权的操作才会被执行。

use super::traits::{Tool, ToolResult};
use crate::app::agent::memory::Memory;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::security::policy::ToolOperation;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// 记忆遗忘工具
///
/// 允许代理从长期记忆中删除指定键的条目。
/// 主要用于：
/// - 删除过时的事实或数据
/// - 移除敏感信息
/// - 清理不再需要的记忆项
pub struct MemoryForgetTool {
    /// 记忆存储后端的共享引用
    memory: Arc<dyn Memory>,
    /// 安全策略检查器，用于验证操作权限
    security: Arc<SecurityPolicy>,
}

impl MemoryForgetTool {
    /// 创建新的记忆遗忘工具实例
    ///
    /// # 参数
    ///
    /// * `memory` - 记忆存储后端的共享引用
    /// * `security` - 安全策略检查器的共享引用
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// let tool = MemoryForgetTool::new(memory, security);
    /// ```
    pub fn new(memory: Arc<dyn Memory>, security: Arc<SecurityPolicy>) -> Self {
        Self { memory, security }
    }
}

/// 实现 Tool trait，提供记忆遗忘功能
///
/// 该工具通过安全策略检查后，从记忆后端删除指定键的条目。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for MemoryForgetTool {
    /// 返回工具名称
    ///
    /// 该名称用于工具注册和调用时的标识符
    fn name(&self) -> &str {
        "memory_forget"
    }

    /// 返回工具描述
    ///
    /// 描述说明工具的用途和行为，供代理理解和决策使用
    fn description(&self) -> &str {
        "Remove a memory by key. Use it to delete outdated facts or sensitive data. Returns whether the memory was found and removed."
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// 定义工具接受参数的结构：
    /// - `key`: 要删除的记忆键（必需）
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "Key of the memory to forget"
                }
            },
            "required": ["key"]
        })
    }

    /// 执行记忆遗忘操作
    ///
    /// # 参数
    ///
    /// * `args` - 包含 `key` 字段的 JSON 参数对象
    ///
    /// # 返回值
    ///
    /// 返回 `ToolResult`，包含：
    /// - `success`: 操作是否成功（即使未找到记忆也返回 true）
    /// - `output`: 操作结果描述
    /// - `error`: 错误信息（如有）
    ///
    /// # 执行流程
    ///
    /// 1. 提取并验证 `key` 参数
    /// 2. 检查安全策略是否允许该操作
    /// 3. 调用记忆后端执行删除
    /// 4. 根据结果构造返回值
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 提取必需的 key 参数，若缺失则返回错误
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'key' parameter"))?;

        // 安全策略检查：验证是否允许执行 memory_forget 操作
        if let Err(error) =
            self.security.enforce_tool_operation(ToolOperation::Act, "memory_forget")
        {
            return Ok(ToolResult { success: false, output: String::new(), error: Some(error) });
        }

        // 执行记忆删除操作并处理不同结果
        match self.memory.forget(key).await {
            // 成功删除记忆
            Ok(true) => Ok(ToolResult {
                success: true,
                output: format!("Forgot memory: {key}"),
                error: None,
            }),
            // 未找到指定键的记忆（非错误情况）
            Ok(false) => Ok(ToolResult {
                success: true,
                output: format!("No memory found with key: {key}"),
                error: None,
            }),
            // 删除操作失败
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to forget memory: {e}")),
            }),
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
