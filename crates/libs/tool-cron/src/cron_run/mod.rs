//! Cron 任务运行工具
//!
//! 强制立即运行指定的定时任务并记录运行历史。

use agent_config::runtime::Config;
use agent_cron::{self as cron, JobType};
use agent_security::{SecurityPolicy, policy::ActionTracker};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tool_types::{Tool, ToolResult};

/// 宿主立即执行入口；返回的 future 随工具调用取消，不派生后台任务。
#[cfg(not(target_arch = "wasm32"))]
pub type JobExecutionFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = (bool, String)> + Send + 'a>>;
#[cfg(target_arch = "wasm32")]
pub type JobExecutionFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = (bool, String)> + 'a>>;
pub type ExecuteJobNow = for<'a> fn(&'a Config, &'a cron::CronJob) -> JobExecutionFuture<'a>;

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
    execute_job: ExecuteJobNow,
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
    /// let tool = CronRunTool::new(config, security, execute_job_now);
    /// ```
    pub fn new(
        config: Arc<Config>,
        security: Arc<SecurityPolicy>,
        execute_job: ExecuteJobNow,
    ) -> Self {
        Self { config, security, execute_job }
    }

    fn authorized_job(
        &self,
        job_id: &str,
        tracker: &ActionTracker,
        full_access_allowed: bool,
    ) -> Result<cron::CronJob, String> {
        if !self.config.cron.enabled {
            return Err("cron is disabled by config (cron.enabled=false)".into());
        }
        if !self.security.can_act() {
            return Err("Security policy: read-only mode, cannot perform 'cron_run'".into());
        }
        let limit = self.security.max_actions_per_hour as usize;
        if tracker.count() >= limit {
            return Err("Rate limit exceeded: too many actions in the last hour".into());
        }
        let job = cron::get_job(&self.config, job_id).map_err(|error| error.to_string())?;
        if job.full_access && !full_access_allowed {
            return Err("A scheduled task cannot exceed the session's access level".into());
        }
        if matches!(job.job_type, JobType::Shell) {
            self.security.validate_command_execution(&job.command, false)?;
        }
        if tracker.record() > limit {
            return Err("Rate limit exceeded: action budget exhausted".into());
        }
        Ok(job)
    }

    /// Authorize an authenticated manual run before returning its background work.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn prepare_background_run(
        self,
        job_id: &str,
        tracker: &ActionTracker,
    ) -> Result<impl std::future::Future<Output = ()> + Send + 'static + use<>, String> {
        let job = self.authorized_job(job_id, tracker, true)?;
        let (started_at, active) = self.start_run(&job)?;
        Ok(async move {
            let _active = active;
            self.run_job(&job, started_at).await;
        })
    }

    fn start_run(
        &self,
        job: &cron::CronJob,
    ) -> Result<(chrono::DateTime<Utc>, cron::active_runs::ActiveRunGuard), String> {
        let started_at = Utc::now();
        if let Some(error) = job.window_error(started_at) {
            return Err(error.into());
        }
        let active = cron::active_runs::register_if_idle(&self.config, job, started_at)
            .ok_or_else(|| "This scheduled task is already running".to_string())?;
        Ok((started_at, active))
    }

    async fn run_job(
        &self,
        job: &cron::CronJob,
        started_at: chrono::DateTime<Utc>,
    ) -> (bool, String, i64) {
        let (success, output) = (self.execute_job)(&self.config, job).await;
        let finished_at = Utc::now();
        let duration_ms = (finished_at - started_at).num_milliseconds();
        let status = if success { "ok" } else { "error" };
        let _ = cron::record_run(
            &self.config,
            &job.id,
            started_at,
            finished_at,
            status,
            Some(&output),
            duration_ms,
        );
        let _ = cron::record_last_run(&self.config, &job.id, finished_at, success, &output);
        (success, output, duration_ms)
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
        "Run a scheduled task immediately within its execution window and record the result without advancing the next planned run"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "job_id": { "type": "string" }
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
        let Some(job_id) = args
            .get("job_id")
            .and_then(serde_json::Value::as_str)
            .filter(|id| !id.trim().is_empty())
        else {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(
                    if self.config.cron.enabled {
                        "Missing 'job_id' parameter"
                    } else {
                        "cron is disabled by config (cron.enabled=false)"
                    }
                    .into(),
                ),
            });
        };
        let full_access_allowed =
            self.security.autonomy == agent_security::AutonomyLevel::Full
                && !self.security.workspace_only;
        let job = match self.authorized_job(job_id, &self.security.tracker, full_access_allowed) {
            Ok(job) => job,
            Err(error) => {
                return Ok(ToolResult { success: false, output: String::new(), error: Some(error) });
            }
        };
        let (started_at, _active_run) = match self.start_run(&job) {
            Ok(run) => run,
            Err(error) => return Ok(super::failure(error)),
        };
        let (success, output, duration_ms) = self.run_job(&job, started_at).await;
        let status = if success { "ok" } else { "error" };

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
