//! Cron 任务运行工具
//!
//! 强制立即运行指定的定时任务并记录运行历史。

use super::traits::{Tool, ToolResult};
use crate::app::agent::config::Config;
use crate::app::agent::cron::{self, JobType};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;

/// Cron 任务手动执行工具
///
/// 该工具允许通过工具调用接口强制立即运行指定的定时任务。
/// 它会执行完整的安全检查、速率限制验证，并记录运行历史。
///
/// # 功能特性
///
/// - 强制立即执行指定的 cron 任务
/// - 支持安全策略检查（只读模式、速率限制）
/// - 对 Shell 类型任务进行命令执行验证
/// - 自动记录运行历史和执行时长
///
/// # 使用场景
///
/// - 手动触发定时任务进行测试
/// - 紧急执行需要立即运行的任务
/// - 调试和验证定时任务配置
pub struct CronRunTool {
    /// 应用配置，包含 cron 设置和存储路径
    config: Arc<Config>,
    /// 安全策略，用于权限检查和速率限制
    security: Arc<SecurityPolicy>,
}

impl CronRunTool {
    /// 创建新的 CronRunTool 实例
    ///
    /// # 参数
    ///
    /// * `config` - 应用配置的原子引用，包含 cron 启用状态和存储配置
    /// * `security` - 安全策略的原子引用，用于权限验证和速率限制
    ///
    /// # 返回值
    ///
    /// 返回初始化后的 `CronRunTool` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// let config = Arc::new(Config::default());
    /// let security = Arc::new(SecurityPolicy::default());
    /// let tool = CronRunTool::new(config, security);
    /// ```
    pub fn new(config: Arc<Config>, security: Arc<SecurityPolicy>) -> Self {
        Self { config, security }
    }
}

/// 实现 Tool trait
///
/// 为 CronRunTool 实现 Tool trait，使其可以作为代理工具被调用。
/// 该实现支持 WASM 和非 WASM 目标平台的异步执行。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for CronRunTool {
    /// 返回工具名称
    ///
    /// # 返回值
    ///
    /// 返回固定字符串 `"cron_run"`，用于工具注册和调用识别
    fn name(&self) -> &str {
        "cron_run"
    }

    /// 返回工具描述
    ///
    /// # 返回值
    ///
    /// 返回工具的功能描述字符串，说明该工具用于立即强制运行定时任务并记录运行历史
    fn description(&self) -> &str {
        "Run a scheduled task immediately and record its run history"
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// # 返回值
    ///
    /// 返回描述工具参数结构的 JSON 对象，包含：
    /// - `job_id`: 必需的字符串参数，指定要运行的任务 ID
    /// - `approved`: 可选的布尔参数，用于在监督模式下批准中/高风险 Shell 命令
    ///
    /// # 示例
    ///
    /// ```json
    /// {
    ///     "type": "object",
    ///     "properties": {
    ///         "job_id": { "type": "string" },
    ///         "approved": { "type": "boolean", "default": false }
    ///     },
    ///     "required": ["job_id"]
    /// }
    /// ```
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "job_id": { "type": "string" },
                "approved": {
                    "type": "boolean",
                    "description": "Set to true to explicitly approve medium/high-risk Shell commands in supervised mode",
                    "default": false
                }
            },
            "required": ["job_id"]
        })
    }

    /// 执行 cron 任务
    ///
    /// 该方法执行完整的任务运行流程，包括配置检查、安全验证、任务执行和结果记录。
    ///
    /// # 参数
    ///
    /// * `args` - JSON 格式的参数对象，必须包含：
    ///   - `job_id`: 要执行的任务 ID（必需）
    ///   - `approved`: 是否批准执行中/高风险命令（可选，默认 false）
    ///
    /// # 返回值
    ///
    /// 返回 `anyhow::Result<ToolResult>`，其中：
    /// - `success`: 任务是否执行成功
    /// - `output`: JSON 格式的执行结果，包含 job_id、status、duration_ms 和 output
    /// - `error`: 错误信息（仅在失败时）
    ///
    /// # 执行流程
    ///
    /// 1. 检查 cron 功能是否启用
    /// 2. 验证并提取 job_id 参数
    /// 3. 检查安全策略（只读模式）
    /// 4. 检查速率限制
    /// 5. 获取任务配置
    /// 6. 对 Shell 类型任务进行命令验证
    /// 7. 记录操作并执行任务
    /// 8. 记录运行历史和最后运行时间
    /// 9. 返回执行结果
    ///
    /// # 错误情况
    ///
    /// - cron 功能被禁用
    /// - 缺少或无效的 job_id 参数
    /// - 安全策略处于只读模式
    /// - 超过速率限制
    /// - 任务不存在或配置错误
    /// - Shell 命令验证失败
    /// - 操作预算耗尽
    /// - 任务执行失败
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 检查 cron 功能是否在配置中启用
        if !self.config.cron.enabled {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("cron is disabled by config (cron.enabled=false)".to_string()),
            });
        }

        // 提取并验证 job_id 参数，确保非空
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

        // 提取 approved 参数，默认为 false
        let approved = args.get("approved").and_then(serde_json::Value::as_bool).unwrap_or(false);

        // 检查安全策略是否允许执行操作（非只读模式）
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Security policy: read-only mode, cannot perform 'cron_run'".into()),
            });
        }

        // 检查是否超过速率限制
        if self.security.is_rate_limited() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
            });
        }

        // 根据任务 ID 获取任务配置
        let job = match cron::get_job(&self.config, job_id) {
            Ok(job) => job,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                });
            }
        };

        // 对 Shell 类型任务进行命令执行安全验证
        if matches!(job.job_type, JobType::Shell) {
            if let Err(reason) = self.security.validate_command_execution(&job.command, approved) {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(reason),
                });
            }
        }

        // 记录本次操作，检查操作预算
        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
            });
        }

        // 记录开始时间
        let started_at = Utc::now();

        // 立即执行任务
        let (success, output) = cron::scheduler::execute_job_now(&self.config, &job).await;

        // 记录结束时间并计算执行时长
        let finished_at = Utc::now();
        let duration_ms = (finished_at - started_at).num_milliseconds();

        // 根据执行结果确定状态字符串
        let status = if success { "ok" } else { "error" };

        // 记录本次运行到历史记录
        let _ = cron::record_run(
            &self.config,
            &job.id,
            started_at,
            finished_at,
            status,
            Some(&output),
            duration_ms,
        );

        // 更新任务的最后运行时间
        let _ = cron::record_last_run(&self.config, &job.id, finished_at, success, &output);

        // 构建并返回执行结果
        Ok(ToolResult {
            success,
            output: serde_json::to_string_pretty(&json!({
                "job_id": job.id,
                "status": status,
                "duration_ms": duration_ms,
                "output": output
            }))?,
            error: if success { None } else { Some("cron job execution failed".to_string()) },
        })
    }
}

/// 单元测试模块
///
/// 包含 CronRunTool 的测试用例，测试文件位于 `tests/cron_run.rs`
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
