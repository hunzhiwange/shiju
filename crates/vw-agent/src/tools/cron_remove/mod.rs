//! Cron 任务删除工具
//!
//! 本模块提供删除指定定时任务的功能实现。
//!
//! # 主要功能
//!
//! - 根据任务 ID 删除已存在的定时任务
//! - 执行安全策略检查（只读模式、速率限制、操作配额）
//! - 验证 cron 功能是否启用
//!
//! # 使用场景
//!
//! 当需要取消某个已调度的定时任务时，通过此工具传入任务 ID 进行删除。
//!
//! # 安全考虑
//!
//! 删除操作属于变更性操作，会经过严格的安全策略验证：
//! - 检查是否处于只读模式
//! - 检查是否触发速率限制
//! - 检查操作配额是否耗尽

use super::traits::{Tool, ToolResult};
use crate::app::agent::config::Config;
use crate::app::agent::cron;
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Cron 任务删除工具
///
/// 负责根据任务 ID 删除指定的定时任务。该工具会执行严格的安全策略检查，
/// 确保删除操作符合系统安全约束。
///
/// # 字段
///
/// - `config`: 应用配置的共享引用，用于检查 cron 功能是否启用
/// - `security`: 安全策略的共享引用，用于执行权限和速率限制检查
pub struct CronRemoveTool {
    config: Arc<Config>,
    security: Arc<SecurityPolicy>,
}

impl CronRemoveTool {
    /// 创建新的 CronRemoveTool 实例
    ///
    /// # 参数
    ///
    /// - `config`: 应用配置的共享引用
    /// - `security`: 安全策略的共享引用
    ///
    /// # 返回值
    ///
    /// 返回初始化后的 `CronRemoveTool` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// let config = Arc::new(Config::default());
    /// let security = Arc::new(SecurityPolicy::default());
    /// let tool = CronRemoveTool::new(config, security);
    /// ```
    pub fn new(config: Arc<Config>, security: Arc<SecurityPolicy>) -> Self {
        Self { config, security }
    }

    /// 强制执行变更操作的安全策略检查
    ///
    /// 在执行删除操作前，依次检查以下安全约束：
    /// 1. 是否处于只读模式（不允许任何变更操作）
    /// 2. 是否触发速率限制（小时内操作次数过多）
    /// 3. 是否成功记录本次操作（配额是否耗尽）
    ///
    /// # 参数
    ///
    /// - `action`: 操作名称，用于错误消息中标识具体的操作类型
    ///
    /// # 返回值
    ///
    /// - `Some(ToolResult)`: 如果安全检查失败，返回包含错误信息的 `ToolResult`
    /// - `None`: 如果所有安全检查通过，返回 `None`，允许操作继续执行
    fn enforce_mutation_allowed(&self, action: &str) -> Option<ToolResult> {
        // 检查是否处于只读模式
        if !self.security.can_act() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Security policy: read-only mode, cannot perform '{action}'")),
            });
        }

        // 尝试记录本次操作，检查配额是否耗尽
        if !self.security.record_action() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".to_string()),
            });
        }

        None
    }
}

/// 实现 Tool trait
///
/// 为 CronRemoveTool 实现 Tool trait，使其能够作为可执行工具集成到代理系统中。
/// 该实现根据目标架构（WASM 或原生）自动选择合适的 async_trait 配置。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for CronRemoveTool {
    /// 返回工具名称
    ///
    /// # 返回值
    ///
    /// 返回固定字符串 `"cron_remove"`，用于在工具注册表中标识此工具
    fn name(&self) -> &str {
        "cron_remove"
    }

    /// 返回工具描述
    ///
    /// # 返回值
    ///
    /// 返回中文描述 `"根据 ID 移除定时任务"`，用于向用户说明工具功能
    fn description(&self) -> &str {
        "根据 ID 移除定时任务"
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// 定义工具所需的参数结构：
    /// - `job_id`: 字符串类型，必需参数，指定要删除的定时任务 ID
    ///
    /// # 返回值
    ///
    /// 返回描述参数结构的 JSON 对象：
    /// ```json
    /// {
    ///     "type": "object",
    ///     "properties": {
    ///         "job_id": { "type": "string" }
    ///     },
    ///     "required": ["job_id"]
    /// }
    /// ```
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "job_id": { "type": "string" }
            },
            "required": ["job_id"]
        })
    }

    /// 执行删除定时任务操作
    ///
    /// 根据提供的参数删除指定的定时任务。执行流程：
    /// 1. 检查 cron 功能是否在配置中启用
    /// 2. 验证并提取 `job_id` 参数
    /// 3. 执行安全策略检查
    /// 4. 调用 cron 模块删除任务
    ///
    /// # 参数
    ///
    /// - `args`: JSON 格式的参数对象，必须包含 `job_id` 字段
    ///
    /// # 返回值
    ///
    /// 返回 `anyhow::Result<ToolResult>`，其中：
    /// - 成功时：`ToolResult.success = true`，`output` 包含成功消息
    /// - 失败时：`ToolResult.success = false`，`error` 包含错误信息
    ///
    /// # 错误情况
    ///
    /// - cron 功能未启用
    /// - 缺少 `job_id` 参数或参数为空
    /// - 安全策略检查失败（只读模式、速率限制、配额耗尽）
    /// - 任务删除失败（任务不存在或存储错误）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let args = json!({ "job_id": "task_12345" });
    /// let result = tool.execute(args).await?;
    /// if result.success {
    ///     println!("任务删除成功: {}", result.output);
    /// }
    /// ```
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 检查 cron 功能是否启用
        if !self.config.cron.enabled {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("cron is disabled by config (cron.enabled=false)".to_string()),
            });
        }

        // 提取并验证 job_id 参数
        let job_id = match args.get("job_id").and_then(serde_json::Value::as_str) {
            Some(v) if !v.trim().is_empty() => v,
            _ => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Missing 'job_id' parameter".to_string()),
                });
            }
        };

        // 执行安全策略检查
        if let Some(blocked) = self.enforce_mutation_allowed("cron_remove") {
            return Ok(blocked);
        }

        // 调用 cron 模块执行删除操作
        match cron::remove_job(&self.config, job_id) {
            Ok(()) => Ok(ToolResult {
                success: true,
                output: format!("Removed cron job {job_id}"),
                error: None,
            }),
            Err(e) => {
                Ok(ToolResult { success: false, output: String::new(), error: Some(e.to_string()) })
            }
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
