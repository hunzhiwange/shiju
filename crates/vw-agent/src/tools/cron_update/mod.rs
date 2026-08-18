//! Cron 任务更新工具
//!
//! 本模块提供更新现有定时任务的能力，是 Cron 工具集的核心组件之一。
//!
//! # 功能概述
//!
//! - 更新任务的调度表达式（cron schedule）
//! - 修改任务执行的命令或提示词
//! - 调整任务的启用/禁用状态
//! - 更新投递配置（如通知通道）
//! - 修改任务使用的模型配置
//!
//! # 安全机制
//!
//! 该工具实现了多层安全防护：
//! - 只读模式检查：禁止在只读模式下执行修改操作
//! - 频率限制：防止短时间内过多修改操作
//! - 命令验证：在更新任务命令时进行安全审查
//! - 监督模式：中/高风险命令需要显式批准
//!
//! # 使用示例
//!
//! ```ignore
//! use vibe_window::app::agent::tools::CronUpdateTool;
//!
//! let tool = CronUpdateTool::new(config, security);
//! let result = tool.execute(json!({
//!     "job_id": "task-123",
//!     "patch": {
//!         "schedule": "0 0 * * *",
//!         "enabled": true
//!     }
//! })).await?;
//! ```

use super::traits::{Tool, ToolResult};
use crate::app::agent::config::Config;
use crate::app::agent::cron::{self, CronJobPatch};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Cron 任务更新工具
///
/// 该结构体实现了 `Tool` trait，用于更新已存在的定时任务。
/// 它持有配置和安全策略的引用，以便在执行更新操作时进行必要的权限检查。
///
/// # 字段说明
///
/// - `config`: 应用配置的共享引用，包含 cron 相关的配置项
/// - `security`: 安全策略的共享引用，用于权限验证和频率限制
///
/// # 线程安全
///
/// 该结构体使用 `Arc` 包装其字段，因此可以安全地在多个线程间共享。
pub struct CronUpdateTool {
    config: Arc<Config>,
    security: Arc<SecurityPolicy>,
}

impl CronUpdateTool {
    /// 创建新的 CronUpdateTool 实例
    ///
    /// # 参数
    ///
    /// - `config`: 应用配置的共享引用
    /// - `security`: 安全策略的共享引用
    ///
    /// # 返回值
    ///
    /// 返回一个初始化好的 `CronUpdateTool` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let config = Arc::new(Config::default());
    /// let security = Arc::new(SecurityPolicy::default());
    /// let tool = CronUpdateTool::new(config, security);
    /// ```
    pub fn new(config: Arc<Config>, security: Arc<SecurityPolicy>) -> Self {
        Self { config, security }
    }

    /// 检查是否允许执行变更操作
    ///
    /// 该方法实现了一个三阶段的安全检查流程：
    ///
    /// 1. **只读模式检查**：验证当前是否处于只读模式
    ///    如果是只读模式，任何修改操作都将被拒绝
    ///
    /// 2. **频率限制检查**：检查是否已达到频率限制阈值
    ///    防止在短时间内执行过多操作
    ///
    /// 3. **操作预算检查**：尝试记录此次操作
    ///    如果操作预算已耗尽，操作将被拒绝
    ///
    /// # 参数
    ///
    /// - `action`: 操作名称，用于在错误消息中标识被拒绝的操作类型
    ///
    /// # 返回值
    ///
    /// - `Some(ToolResult)`: 如果操作被安全策略阻止，返回包含错误信息的 `ToolResult`
    /// - `None`: 如果操作被允许执行
    ///
    /// # 安全说明
    ///
    /// 该方法应该在任何实际的变更操作之前调用，以确保不会违反安全策略。
    /// 返回 `None` 表示可以安全地继续执行操作。
    fn enforce_mutation_allowed(&self, action: &str) -> Option<ToolResult> {
        // 阶段 1: 检查是否处于只读模式
        // 在只读模式下，所有修改操作都被禁止
        if !self.security.can_act() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Security policy: read-only mode, cannot perform '{action}'")),
            });
        }

        // 阶段 2: 检查是否触发了频率限制
        // 这可以防止短时间内的大量操作
        if self.security.is_rate_limited() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".to_string()),
            });
        }

        // 阶段 3: 尝试记录操作并检查操作预算
        // record_action 会递减可用操作次数，如果预算耗尽则返回 false
        if !self.security.record_action() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".to_string()),
            });
        }

        // 所有检查通过，允许执行操作
        None
    }
}

/// 实现 Tool trait
///
/// 为 CronUpdateTool 实现 Tool trait，使其可以作为通用工具被调用。
/// 该实现使用了条件编译属性，以支持 WASM 和 native 两种目标平台：
/// - 在 WASM 目标上，使用 `async_trait(?Send)` 因为 WASM 不支持 Send trait
/// - 在其他平台上，使用标准的 `async_trait`
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for CronUpdateTool {
    /// 返回工具名称
    ///
    /// 该名称用于在系统中标识和调用此工具。
    ///
    /// # 返回值
    ///
    /// 返回固定字符串 `"cron_update"`
    fn name(&self) -> &str {
        "cron_update"
    }

    /// 返回工具描述
    ///
    /// 提供工具功能的简要说明，用于向用户或其他系统展示工具用途。
    ///
    /// # 返回值
    ///
    /// 返回描述字符串，说明此工具可以更新的任务属性类型
    fn description(&self) -> &str {
        "Update an existing scheduled task, including its schedule, command, prompt, enabled state, delivery configuration, or model"
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// 定义了调用此工具时所需参数的结构和类型约束。
    ///
    /// # 参数结构
    ///
    /// - `job_id` (string, 必需): 要更新的任务唯一标识符
    /// - `patch` (object, 必需): 包含要更新的字段的补丁对象
    ///   - 可包含的字段: schedule, command, prompt, enabled, delivery, model 等
    /// - `approved` (boolean, 可选): 是否显式批准中/高风险 Shell 命令
    ///   - 默认值: false
    ///   - 在监督模式下，某些命令需要显式批准才能执行
    ///
    /// # 返回值
    ///
    /// 返回描述参数结构的 JSON Schema 对象
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "job_id": {
                    "type": "string",
                    "description": "Unique identifier of the scheduled task to update"
                },
                "patch": {
                    "type": "object",
                    "description": "Patch object containing only the fields to update"
                },
                "approved": {
                    "type": "boolean",
                    "description": "Set to true to explicitly approve medium/high-risk Shell commands in supervised mode",
                    "default": false
                }
            },
            "required": ["job_id", "patch"]
        })
    }

    /// 执行任务更新操作
    ///
    /// 该方法是工具的核心实现，负责执行完整的任务更新流程：
    ///
    /// 1. **配置检查**: 验证 cron 功能是否已启用
    /// 2. **参数解析**: 提取并验证 job_id 和 patch 参数
    /// 3. **命令验证**: 如果更新包含命令，进行安全验证
    /// 4. **权限检查**: 执行多层安全检查
    /// 5. **执行更新**: 调用 cron 模块执行实际更新
    ///
    /// # 参数
    ///
    /// - `args`: JSON 格式的参数对象，包含:
    ///   - `job_id`: 要更新的任务 ID
    ///   - `patch`: 包含更新字段的补丁对象
    ///   - `approved` (可选): 是否批准高风险命令
    ///
    /// # 返回值
    ///
    /// - `Ok(ToolResult)`: 返回操作结果
    ///   - `success: true`: 更新成功，output 包含更新后的任务信息
    ///   - `success: false`: 更新失败，error 包含错误原因
    /// - `Err(anyhow::Error)`: 序列化错误等系统级错误
    ///
    /// # 错误情况
    ///
    /// 以下情况会导致操作失败：
    /// - cron 功能未启用 (cron.enabled = false)
    /// - 缺少必需参数 (job_id 或 patch)
    /// - job_id 为空字符串
    /// - patch 格式无效
    /// - 命令未通过安全验证
    /// - 触发只读模式或频率限制
    /// - 任务不存在
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 更新任务的调度时间和启用状态
    /// let result = tool.execute(json!({
    ///     "job_id": "task-123",
    ///     "patch": {
    ///         "schedule": "0 0 * * *",
    ///         "enabled": true
    ///     }
    /// })).await?;
    ///
    /// // 更新命令并显式批准
    /// let result = tool.execute(json!({
    ///     "job_id": "task-456",
    ///     "patch": {
    ///         "command": "rm -rf /tmp/cache/*"
    ///     },
    ///     "approved": true
    /// })).await?;
    /// ```
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // ===== 阶段 1: 检查 cron 功能是否启用 =====
        if !self.config.cron.enabled {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("cron is disabled by config (cron.enabled=false)".to_string()),
            });
        }

        // ===== 阶段 2: 解析并验证 job_id 参数 =====
        // job_id 是必需参数，不能为空
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

        // ===== 阶段 3: 解析并验证 patch 参数 =====
        // patch 是必需参数，包含要更新的字段
        let patch_val = match args.get("patch") {
            Some(v) => v.clone(),
            None => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Missing 'patch' parameter".to_string()),
                });
            }
        };

        // 将 JSON 值反序列化为 CronJobPatch 结构体
        // 这会验证 patch 的格式和字段类型
        let patch = match serde_json::from_value::<CronJobPatch>(patch_val) {
            Ok(patch) => patch,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Invalid patch payload: {e}")),
                });
            }
        };

        // ===== 阶段 4: 解析可选的 approved 参数 =====
        // approved 用于在监督模式下显式批准中/高风险命令
        let approved = args.get("approved").and_then(serde_json::Value::as_bool).unwrap_or(false);

        // ===== 阶段 5: 命令安全验证 =====
        // 如果 patch 中包含命令更新，需要进行安全检查
        if let Some(command) = &patch.command {
            if let Err(reason) = self.security.validate_command_execution(command, approved) {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(reason),
                });
            }
        }

        // ===== 阶段 6: 执行权限检查 =====
        // 检查只读模式、频率限制和操作预算
        if let Some(blocked) = self.enforce_mutation_allowed("cron_update") {
            return Ok(blocked);
        }

        // ===== 阶段 7: 执行实际更新操作 =====
        // 所有检查通过后，调用 cron 模块执行更新
        match cron::update_job(&self.config, job_id, patch) {
            Ok(job) => Ok(ToolResult {
                success: true,
                output: serde_json::to_string_pretty(&job)?,
                error: None,
            }),
            Err(e) => {
                Ok(ToolResult { success: false, output: String::new(), error: Some(e.to_string()) })
            }
        }
    }
}

/// 测试模块
///
/// 测试代码位于 tests/cron_update.rs 文件中，保持测试与实现分离。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
