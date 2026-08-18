//! # Cron 任务添加工具模块
//!
//! 本模块提供了创建和添加定时任务（cron job）的功能，是代理系统调度能力的核心组件。
//!
//! ## 主要功能
//!
//! - **多种调度类型支持**：
//!   - `cron`：基于标准 cron 表达式的周期性任务
//!   - `at`：一次性定时任务，在指定时间执行
//!   - `every`：固定间隔的周期性任务（毫秒级精度）
//!
//! - **双模式任务执行**：
//!   - `shell` 模式：执行系统 shell 命令
//!   - `agent` 模式：启动 AI 智能体执行复杂任务
//!
//! - **灵活的输出投递**：
//!   - 支持将任务执行结果推送到多种通讯渠道
//!   - 包括 Discord、Telegram、Slack、Mattermost、QQ、Email 等
//!
//! - **安全控制**：
//!   - 集成安全策略，支持命令执行验证
//!   - 支持速率限制，防止滥用
//!   - 支持只读模式，在敏感环境下禁用变更
//!
//! ## 使用场景
//!
//! - 定期执行系统维护脚本
//! - 按计划运行 AI 智能体进行数据收集或分析
//! - 定时向用户推送消息或通知
//! - 实现延迟消息投递功能

use super::traits::{Tool, ToolResult};
use crate::app::agent::config::Config;
use crate::app::agent::cron::{self, DeliveryConfig, JobType, Schedule, SessionTarget};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Cron 任务添加工具
///
/// 该结构体实现了 `Tool` trait，提供了创建和管理定时任务的能力。
/// 它是代理工具系统的一部分，允许通过统一的工具接口来添加定时任务。
///
/// # 架构设计
///
/// - **配置引用**：持有全局配置的 Arc 引用，用于读取 cron 相关设置
/// - **安全策略**：持有安全策略的 Arc 引用，确保所有操作符合安全约束
/// - **无状态执行**：每次执行都是独立的，不维护内部状态
///
/// # 线程安全
///
/// 由于使用了 `Arc` 包装，该工具可以在多线程环境中安全共享。
/// 内部所有操作都是只读或通过线程安全的数据结构进行的。
///
/// # 示例
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use crate::app::agent::tools::CronAddTool;
/// use crate::app::agent::config::Config;
/// use crate::app::agent::security::SecurityPolicy;
///
/// let config = Arc::new(Config::default());
/// let security = Arc::new(SecurityPolicy::default());
/// let tool = CronAddTool::new(config, security);
///
/// // 通过 Tool trait 执行
/// let result = tool.execute(args).await?;
/// ```
pub struct CronAddTool {
    /// 应用全局配置，包含 cron 系统的启用状态和其他设置
    config: Arc<Config>,

    /// 安全策略实例，用于验证操作权限和限制执行频率
    security: Arc<SecurityPolicy>,

    /// 当前工具调用所在的项目目录；通过通用工具注册表创建时自动注入。
    default_project_path: Option<PathBuf>,

    /// 当前工具调用是否具有完全访问权限；通过通用工具注册表创建时自动注入。
    default_full_access: Option<bool>,
}

impl CronAddTool {
    /// 创建新的 CronAddTool 实例
    ///
    /// # 参数
    ///
    /// - `config`：全局配置的共享引用，用于读取 cron 启用状态等配置
    /// - `security`：安全策略的共享引用，用于权限验证和速率限制
    ///
    /// # 返回值
    ///
    /// 返回一个配置好的 `CronAddTool` 实例，可立即用于执行任务添加操作
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let config = Arc::new(Config::load()?);
    /// let security = Arc::new(SecurityPolicy::strict());
    /// let tool = CronAddTool::new(config, security);
    /// ```
    pub fn new(config: Arc<Config>, security: Arc<SecurityPolicy>) -> Self {
        Self { config, security, default_project_path: None, default_full_access: None }
    }

    /// 设置新任务默认继承的当前执行上下文。
    ///
    /// 显式工具参数仍优先于这些默认值。
    pub fn with_execution_defaults(
        mut self,
        project_path: impl AsRef<Path>,
        full_access: bool,
    ) -> Self {
        self.default_project_path = Some(project_path.as_ref().to_path_buf());
        self.default_full_access = Some(full_access);
        self
    }

    /// 强制执行变更权限检查
    ///
    /// 该方法实现了多层安全检查，确保当前的变更操作（如添加 cron 任务）
    /// 是被允许的。这是防御性编程的关键部分，防止在不安全的上下文中
    /// 执行危险操作。
    ///
    /// # 检查层次
    ///
    /// 1. **只读模式检查**：如果安全策略处于只读模式，拒绝所有变更操作
    /// 2. **速率限制检查**：检查是否超过时间窗口内的操作次数限制
    /// 3. **操作配额检查**：尝试记录本次操作，如果配额已用尽则拒绝
    ///
    /// # 参数
    ///
    /// - `action`：操作的名称，用于生成更有意义的错误消息
    ///
    /// # 返回值
    ///
    /// - `Some(ToolResult)`：如果操作被拒绝，返回包含错误信息的 ToolResult
    /// - `None`：如果操作被允许，返回 None，调用者可继续执行
    ///
    /// # 安全考虑
    ///
    /// - 所有检查失败都会立即返回，遵循"快速失败"原则
    /// - 错误消息清晰说明拒绝原因，便于调试和审计
    /// - 速率限制防止恶意用户通过大量 cron 任务消耗系统资源
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // 在执行变更操作前调用
    /// if let Some(blocked) = self.enforce_mutation_allowed("cron_add") {
    ///     return Ok(blocked); // 操作被拒绝
    /// }
    /// // 继续执行变更操作...
    /// ```
    fn enforce_mutation_allowed(&self, action: &str) -> Option<ToolResult> {
        // 第一层检查：验证是否处于只读模式
        // 在只读模式下，任何变更操作都被禁止，确保系统状态不可变
        if !self.security.can_act() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Security policy: read-only mode, cannot perform '{action}'")),
            });
        }

        // 第二层检查：验证是否触发速率限制
        // 速率限制基于滑动时间窗口（通常是最近一小时），
        // 防止在短时间内执行过多操作
        if self.security.is_rate_limited() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".to_string()),
            });
        }

        // 第三层检查：记录本次操作并验证配额
        // 即使速率限制未触发，也可能因为总操作配额用尽而被拒绝
        // record_action() 会尝试从配额中扣除一次操作，返回是否成功
        if !self.security.record_action() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".to_string()),
            });
        }

        // 所有检查通过，允许操作继续
        None
    }
}

/// Tool trait 实现 - 为 CronAddTool 提供标准的工具接口
///
/// 该实现使用 `async_trait` 宏来支持异步操作，并根据目标平台
/// 自动调整 Send 约束：
/// - 在 WASM 目标上使用 `?Send` 以支持单线程异步运行时
/// - 在其他平台上使用标准 Send trait
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for CronAddTool {
    /// 返回工具名称
    ///
    /// 工具名称是工具的唯一标识符，用于在工具注册表中查找和调用。
    /// 该名称也是用户在请求中使用的标识符。
    ///
    /// # 返回值
    ///
    /// 固定返回 `"cron_add"` 字符串
    fn name(&self) -> &str {
        "cron_add"
    }

    /// 返回工具的详细描述
    ///
    /// 该描述会被 AI 模型使用，用于理解工具的功能和用法。
    /// 描述应该清晰、准确，包含关键参数和使用示例。
    ///
    /// # 返回值
    ///
    /// 返回包含以下信息的描述字符串：
    /// - 工具的主要功能
    /// - 支持的任务类型和调度方式
    /// - 投递配置的使用方法
    /// - 推荐的使用场景
    fn description(&self) -> &str {
        "Create a scheduled task (Shell or agent) using cron, at, or every scheduling.\
         Set job_type='agent' with prompt to run an AI agent on schedule.\
         To send output to a channel (Discord, Telegram, Slack, Mattermost, QQ, or Email), set \
         delivery={\"mode\":\"announce\",\"channel\":\"discord\",\"to\":\"<channel-or-chat-id>\"}.\
         This is the preferred tool for sending scheduled or delayed messages to users through a channel."
    }

    /// 返回工具参数的 JSON Schema 定义
    ///
    /// 该 Schema 定义了工具接受的所有参数及其类型、约束和描述。
    /// AI 模型会使用该 Schema 来验证和构建参数。
    ///
    /// # 返回值
    ///
    /// 返回一个 JSON Schema 对象，包含：
    /// - `name`：任务的可选名称
    /// - `schedule`：必需的调度配置对象
    /// - `job_type`：任务类型（shell 或 agent）
    /// - `command`/`prompt`：根据任务类型的执行内容
    /// - `session_target`：智能体会话目标类型
    /// - `model`：可选的模型指定
    /// - `delivery`：输出投递配置
    /// - `delete_after_run`：执行后是否自动删除
    /// - `approved`：是否预先批准高风险操作
    ///
    /// # Schema 设计原则
    ///
    /// - 必需参数最小化（只有 schedule 是必需的）
    /// - 提供合理的默认值推断逻辑
    /// - 枚举值明确列出，便于模型选择
    /// - 嵌套对象结构清晰，描述详细
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                // 任务名称 - 用于标识和日志记录
                "name": { "type": "string" },

                // 调度配置 - 核心参数，定义任务的执行时间
                "schedule": {
                    "type": "object",
                    "description": "Schedule object: {kind:'cron',expr,tz?} | {kind:'at',at} | {kind:'every',every_ms}"
                },

                // 任务类型 - 决定执行引擎
                "job_type": { "type": "string", "enum": ["shell", "agent"] },

                // Shell 命令 - 用于 shell 类型任务
                "command": { "type": "string" },

                // 智能体提示 - 用于 agent 类型任务
                "prompt": { "type": "string" },

                // 会话目标 - 决定智能体的会话上下文
                "session_target": { "type": "string", "enum": ["isolated", "main"] },

                // 模型指定 - 可选的模型覆盖
                "model": { "type": "string" },

                // 项目目录 - 省略时继承当前工具调用所在的工作区
                "project_path": {
                    "type": "string",
                    "description": "Project directory in which to run the task; inherits the current workspace when omitted"
                },

                // 完全访问权限 - 省略时继承当前工具调用权限
                "full_access": {
                    "type": "boolean",
                    "description": "Whether the task runs with full access; inherits current session permissions when omitted"
                },

                // 投递配置 - 控制输出如何发送到外部渠道
                "delivery": {
                    "type": "object",
                    "description": "Delivery configuration for sending task output to a channel. Example: {\"mode\":\"announce\",\"channel\":\"discord\",\"to\":\"<channel-id>\"}",
                    "properties": {
                        "mode": {
                            "type": "string",
                            "enum": ["none", "announce"],
                            "description": "Set to 'announce' to send output to a channel"
                        },
                        "channel": {
                            "type": "string",
                            "enum": ["telegram", "discord", "slack", "mattermost", "qq", "email"],
                            "description": "Type of channel to deliver to"
                        },
                        "to": {
                            "type": "string",
                            "description": "Destination, such as a Discord channel ID, Telegram chat ID, or Slack channel"
                        },
                        "best_effort": {
                            "type": "boolean",
                            "description": "When true, a delivery failure does not fail the task"
                        }
                    }
                },

                // 执行后删除标志 - 对于一次性任务特别有用
                "delete_after_run": { "type": "boolean" },

                // 预先批准标志 - 用于监督模式下的高风险命令
                "approved": {
                    "type": "boolean",
                    "description": "Set to true to explicitly approve medium/high-risk Shell commands in supervised mode",
                    "default": false
                }
            },
            "required": ["schedule"]
        })
    }

    /// 执行工具逻辑 - 添加定时任务
    ///
    /// 这是工具的核心方法，负责解析参数、验证权限、创建任务并返回结果。
    /// 该方法的实现遵循"快速失败"原则，在遇到错误时立即返回，避免继续执行无效操作。
    ///
    /// # 参数
    ///
    /// - `args`：JSON 格式的参数对象，必须符合 parameters_schema() 定义的结构
    ///
    /// # 返回值
    ///
    /// 返回 `anyhow::Result<ToolResult>`，其中：
    /// - 成功时，ToolResult 包含新创建任务的详细信息（ID、名称、下次运行时间等）
    /// - 失败时，ToolResult 包含详细的错误信息
    ///
    /// # 执行流程
    ///
    /// 1. **配置检查**：验证 cron 功能是否已启用
    /// 2. **参数解析**：解析并验证 schedule、job_type 等核心参数
    /// 3. **任务类型分支**：
    ///    - Shell 任务：验证命令安全性，检查变更权限，创建 shell job
    ///    - Agent 任务：解析智能体配置，检查变更权限，创建 agent job
    /// 4. **结果处理**：将创建的 job 信息序列化为 JSON 返回
    ///
    /// # 错误处理
    ///
    /// - 配置禁用：返回明确的错误信息
    /// - 参数缺失/无效：返回详细的验证错误
    /// - 权限不足：返回安全策略相关的错误
    /// - 任务创建失败：返回底层错误信息
    ///
    /// # 安全考虑
    ///
    /// - Shell 命令会经过安全策略的严格验证
    /// - 变更操作需要通过 enforce_mutation_allowed() 的所有检查
    /// - 敏感操作有明确的批准机制
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // 创建一个简单的 cron 任务
    /// let args = json!({
    ///     "name": "daily_backup",
    ///     "schedule": {"kind": "cron", "expr": "0 2 * * *"},
    ///     "job_type": "shell",
    ///     "command": "/usr/local/bin/backup.sh"
    /// });
    /// let result = tool.execute(args).await?;
    /// ```
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 步骤 1: 检查 cron 功能是否全局启用
        // 如果配置中禁用了 cron，直接返回错误，避免执行任何后续操作
        if !self.config.cron.enabled {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("cron is disabled by config (cron.enabled=false)".to_string()),
            });
        }

        // 步骤 2: 解析并验证 schedule 参数
        // schedule 是必需参数，定义任务的执行时间和频率
        let schedule = match args.get("schedule") {
            Some(v) => match serde_json::from_value::<Schedule>(v.clone()) {
                Ok(schedule) => schedule,
                Err(e) => {
                    // schedule 格式错误，返回详细的错误信息帮助用户修正
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Invalid schedule: {e}")),
                    });
                }
            },
            None => {
                // schedule 缺失，这是必需参数
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Missing 'schedule' parameter".to_string()),
                });
            }
        };

        // 步骤 3: 解析可选的 name 参数
        // name 用于任务标识和日志记录，如果没有提供将使用默认名称
        let name = args.get("name").and_then(serde_json::Value::as_str).map(str::to_string);

        // 步骤 4: 确定任务类型（shell 或 agent）
        // job_type 可以显式指定，也可以通过参数推断：
        // - 如果提供了 prompt 参数，推断为 agent 类型
        // - 否则默认为 shell 类型
        let job_type = match args.get("job_type").and_then(serde_json::Value::as_str) {
            Some("agent") => JobType::Agent,
            Some("shell") => JobType::Shell,
            Some(other) => {
                // 无效的 job_type 值，返回错误
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Invalid job_type: {other}")),
                });
            }
            None => {
                // 未显式指定 job_type，根据其他参数推断
                if args.get("prompt").is_some() { JobType::Agent } else { JobType::Shell }
            }
        };

        // 步骤 5: 确定 delete_after_run 标志
        // 对于一次性任务（Schedule::At），默认在执行后删除
        // 对于周期性任务，默认保留
        let default_delete_after_run = matches!(schedule, Schedule::At { .. });
        let delete_after_run = args
            .get("delete_after_run")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(default_delete_after_run);

        // 解析 approved 标志，用于预先批准高风险命令
        let approved = args.get("approved").and_then(serde_json::Value::as_bool).unwrap_or(false);
        let project_path = args
            .get("project_path")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                self.default_project_path.as_ref().map(|path| path.to_string_lossy().to_string())
            });
        let full_access = args
            .get("full_access")
            .and_then(serde_json::Value::as_bool)
            .or(self.default_full_access);

        // 步骤 6: 根据任务类型执行不同的创建逻辑
        let result = match job_type {
            JobType::Shell => {
                // ===== Shell 任务创建分支 =====

                // 6a-1: 验证 command 参数是否存在且非空
                let command = match args.get("command").and_then(serde_json::Value::as_str) {
                    Some(command) if !command.trim().is_empty() => command,
                    _ => {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some("Missing 'command' for shell job".to_string()),
                        });
                    }
                };

                // 6a-2: 安全策略验证 - 检查命令是否被允许执行
                // 这是关键的安全检查点，会根据安全策略验证命令的风险等级
                if let Err(reason) = self.security.validate_command_execution(command, approved) {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(reason),
                    });
                }

                // 6a-3: 变更权限检查 - 验证是否有权限执行此变更操作
                if let Some(blocked) = self.enforce_mutation_allowed("cron_add") {
                    return Ok(blocked);
                }

                // 6a-4: 调用 cron 模块创建 shell 任务
                cron::add_shell_job(&self.config, name, schedule, command)
            }
            JobType::Agent => {
                // ===== Agent 任务创建分支 =====

                // 6b-1: 验证 prompt 参数是否存在且非空
                // prompt 是 agent 任务的核心，定义了智能体要执行的任务
                let prompt = match args.get("prompt").and_then(serde_json::Value::as_str) {
                    Some(prompt) if !prompt.trim().is_empty() => prompt,
                    _ => {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some("Missing 'prompt' for agent job".to_string()),
                        });
                    }
                };

                // 6b-2: 解析 session_target 参数
                // - isolated: 创建新的独立会话，适合独立任务
                // - main: 使用主会话，适合需要保持上下文的任务
                let session_target = match args.get("session_target") {
                    Some(v) => match serde_json::from_value::<SessionTarget>(v.clone()) {
                        Ok(target) => target,
                        Err(e) => {
                            return Ok(ToolResult {
                                success: false,
                                output: String::new(),
                                error: Some(format!("Invalid session_target: {e}")),
                            });
                        }
                    },
                    None => SessionTarget::Isolated, // 默认使用独立会话
                };

                // 6b-3: 解析可选的 model 参数
                // 如果未指定，将使用配置中的默认模型
                let model =
                    args.get("model").and_then(serde_json::Value::as_str).map(str::to_string);

                // 6b-4: 解析可选的 delivery 配置
                // delivery 定义了如何将任务输出发送到外部渠道
                let delivery = match args.get("delivery") {
                    Some(v) => match serde_json::from_value::<DeliveryConfig>(v.clone()) {
                        Ok(cfg) => Some(cfg),
                        Err(e) => {
                            return Ok(ToolResult {
                                success: false,
                                output: String::new(),
                                error: Some(format!("Invalid delivery config: {e}")),
                            });
                        }
                    },
                    None => None, // 无 delivery 配置时，输出不会被推送
                };

                // 6b-5: 变更权限检查 - 验证是否有权限执行此变更操作
                if let Some(blocked) = self.enforce_mutation_allowed("cron_add") {
                    return Ok(blocked);
                }

                // 6b-6: 调用 cron 模块创建 agent 任务
                cron::add_agent_job(
                    &self.config,
                    name,
                    schedule,
                    prompt,
                    session_target,
                    model,
                    delivery,
                    delete_after_run,
                )
            }
        }
        .and_then(|job| {
            if project_path.is_none() && full_access.is_none() {
                return Ok(job);
            }
            cron::update_job(
                &self.config,
                &job.id,
                cron::CronJobPatch {
                    project_path: project_path.clone(),
                    full_access,
                    ..cron::CronJobPatch::default()
                },
            )
        });

        // 步骤 7: 处理任务创建结果
        // 将结果转换为 ToolResult 格式返回给调用者
        match result {
            Ok(job) => {
                // 任务创建成功，构建包含任务详细信息的成功响应
                // 返回的信息包括：任务 ID、名称、类型、调度规则、下次运行时间、启用状态
                Ok(ToolResult {
                    success: true,
                    output: serde_json::to_string_pretty(&json!({
                        "id": job.id,
                        "name": job.name,
                        "job_type": job.job_type,
                        "schedule": job.schedule,
                        "next_run": job.next_run,
                        "enabled": job.enabled
                    }))?,
                    error: None,
                })
            }
            Err(e) => {
                // 任务创建失败，返回底层错误信息
                Ok(ToolResult { success: false, output: String::new(), error: Some(e.to_string()) })
            }
        }
    }
}

/// 测试模块
///
/// 测试代码位于独立的测试文件中，保持主代码的整洁性。
/// 测试覆盖了参数验证、安全检查、任务创建等核心功能。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
