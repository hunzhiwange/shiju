//! Shell 命令执行工具模块
//!
//! 本模块提供了在受控沙箱环境中执行 Shell 命令的能力，是代理系统的核心工具之一。
//!
//! # 主要功能
//!
//! - **命令执行**：在工作区目录中安全地执行 Shell 命令
//! - **沙箱隔离**：通过环境变量过滤、命令白名单等机制实现安全隔离
//! - **重定向策略**：支持自定义命令重定向策略，防止危险操作
//! - **系统调用检测**：集成异常系统调用检测器，监控可疑行为
//! - **速率限制**：防止命令滥用和资源耗尽
//! - **输出截断**：自动截断过大的输出，防止内存溢出
//!
//! # 安全特性
//!
//! - 环境变量白名单机制，只传递功能性变量，绝不泄露 API 密钥或敏感信息
//! - 命令执行前的安全验证和风险评级
//! - 禁止访问特定路径，防止敏感文件被访问
//! - 超时机制，防止命令长时间挂起
//! - 动作预算管理，限制单位时间内的操作次数
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use crate::app::agent::tools::shell::ShellTool;
//! use crate::app::agent::runtime::RuntimeAdapter;
//! use crate::app::agent::security::SecurityPolicy;
//!
//! // 创建 Shell 工具实例
//! let shell_tool = ShellTool::new(security_policy, runtime_adapter);
//!
//! // 执行命令
//! let result = shell_tool.execute(json!({"command": "ls -la"})).await?;
//! println!("输出: {}", result.output);
//! ```

pub mod ast;
pub mod compound;
pub mod path;
pub mod permissions;
pub mod readonly;
pub mod sandbox;
pub mod security;
pub mod sed;

use super::traits::{Tool, ToolResult, ToolSpec};
use crate::app::agent::runtime::RuntimeAdapter;
use crate::app::agent::security::SyscallAnomalyDetector;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy, ShellRedirectPolicy};
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::compound::semantics::{ExitInterpretation, ExitSemantics};
use crate::tools::shell::permissions::{Permission, PermissionContext, PermissionMode};
use crate::tools::shell::sandbox::{
    SandboxConfig, SandboxDecision, SandboxExecutor, should_use_sandbox,
};
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Shell 命令默认超时时间（毫秒）。
const DEFAULT_TIMEOUT_MS: u64 = 120_000;

/// 最大输出字节数（1MB）
///
/// 限制命令输出的大小，防止超大输出导致内存溢出（OOM）。
/// 当输出超过此限制时，将被自动截断并在末尾添加截断标记。
const MAX_OUTPUT_BYTES: usize = 1_048_576;

/// 安全的环境变量列表
///
/// 这些环境变量被允许传递给 Shell 命令。只包含功能性变量，
/// 绝不包含 API 密钥、令牌或其他敏感信息。
///
/// # 包含的变量
///
/// - `PATH`: 可执行文件搜索路径
/// - `HOME`: 用户主目录
/// - `TERM`: 终端类型
/// - `LANG`, `LC_ALL`, `LC_CTYPE`: 语言和字符集设置
/// - `USER`: 当前用户名
/// - `SHELL`: 当前 Shell 路径
/// - `TMPDIR`: 临时目录路径
const SAFE_ENV_VARS: &[&str] =
    &["PATH", "HOME", "TERM", "LANG", "LC_ALL", "LC_CTYPE", "USER", "SHELL", "TMPDIR"];

/// Shell 命令执行工具
///
/// 提供在沙箱环境中安全执行 Shell 命令的能力。该工具实现了 [`Tool`] trait，
/// 可以作为代理系统的工具被调用。
///
/// # 安全机制
///
/// - **环境隔离**：清除所有环境变量，只重新注入白名单中的安全变量
/// - **命令验证**：执行前对命令进行安全性和风险评估
/// - **路径过滤**：阻止访问被禁止的路径
/// - **速率限制**：防止单位时间内执行过多命令
/// - **超时保护**：命令执行超时后自动终止
/// - **输出限制**：防止超大输出导致内存问题
///
/// # 字段说明
///
/// - `security`: 安全策略引用，包含沙箱规则和限制配置
/// - `runtime`: 运行时适配器，负责实际的命令执行
/// - `syscall_detector`: 可选的系统调用异常检测器，用于监控可疑行为
pub struct ShellTool {
    security: Arc<SecurityPolicy>,
    runtime: Arc<dyn RuntimeAdapter>,
    syscall_detector: Option<Arc<SyscallAnomalyDetector>>,
    sandbox_config: SandboxConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ShellArgs {
    command: String,
    description: String,
    timeout_ms: u64,
    workdir: PathBuf,
    approved: bool,
}

struct ShellExecutionPlan {
    effective_command: String,
    sandbox_config: SandboxConfig,
    sandbox_decision: SandboxDecision,
}

impl ShellTool {
    fn sandbox_config_for_security(security: &SecurityPolicy) -> SandboxConfig {
        let mut sandbox_config = SandboxConfig::for_workspace(security.workspace_dir.clone());
        if security.autonomy == AutonomyLevel::Full && !security.workspace_only {
            sandbox_config.enabled = false;
        }
        sandbox_config
    }

    /// 创建新的 Shell 工具实例
    ///
    /// 使用默认配置（不启用系统调用检测）创建 Shell 工具实例。
    ///
    /// # 参数
    ///
    /// - `security`: 安全策略的共享引用，定义了沙箱规则和限制
    /// - `runtime`: 运行时适配器的共享引用，负责实际的命令构建和执行
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `ShellTool` 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let shell_tool = ShellTool::new(security_policy, runtime_adapter);
    /// ```
    pub fn new(security: Arc<SecurityPolicy>, runtime: Arc<dyn RuntimeAdapter>) -> Self {
        let sandbox_config = Self::sandbox_config_for_security(&security);
        Self::new_with_options(security, runtime, None, sandbox_config)
    }

    /// 创建带系统调用检测器的 Shell 工具实例
    ///
    /// 创建 Shell 工具实例，可选择启用系统调用异常检测功能。
    ///
    /// # 参数
    ///
    /// - `security`: 安全策略的共享引用，定义了沙箱规则和限制
    /// - `runtime`: 运行时适配器的共享引用，负责实际的命令构建和执行
    /// - `syscall_detector`: 可选的系统调用异常检测器，启用后会监控命令执行过程中的可疑行为
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `ShellTool` 实例
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // 不启用系统调用检测
    /// let shell_tool = ShellTool::new_with_syscall_detector(
    ///     security_policy,
    ///     runtime_adapter,
    ///     None
    /// );
    ///
    /// // 启用系统调用检测
    /// let shell_tool = ShellTool::new_with_syscall_detector(
    ///     security_policy,
    ///     runtime_adapter,
    ///     Some(syscall_detector)
    /// );
    /// ```
    pub fn new_with_syscall_detector(
        security: Arc<SecurityPolicy>,
        runtime: Arc<dyn RuntimeAdapter>,
        syscall_detector: Option<Arc<SyscallAnomalyDetector>>,
    ) -> Self {
        let sandbox_config = Self::sandbox_config_for_security(&security);
        Self::new_with_options(security, runtime, syscall_detector, sandbox_config)
    }

    pub fn new_with_sandbox_config(
        security: Arc<SecurityPolicy>,
        runtime: Arc<dyn RuntimeAdapter>,
        sandbox_config: SandboxConfig,
    ) -> Self {
        Self::new_with_options(security, runtime, None, sandbox_config)
    }

    fn new_with_options(
        security: Arc<SecurityPolicy>,
        runtime: Arc<dyn RuntimeAdapter>,
        syscall_detector: Option<Arc<SyscallAnomalyDetector>>,
        sandbox_config: SandboxConfig,
    ) -> Self {
        Self { security, runtime, syscall_detector, sandbox_config }
    }

    fn parse_args(&self, args: &serde_json::Value) -> anyhow::Result<ShellArgs> {
        let command = extract_command_argument(args)
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter"))?;
        let description =
            extract_description_argument(args).unwrap_or_else(|| format!("run: {command}"));
        let timeout_ms = parse_timeout_ms(args)?;
        let approved = args.get("approved").and_then(|v| v.as_bool()).unwrap_or(false);
        let workdir =
            resolve_workdir(args.get("workdir").and_then(|value| value.as_str()), &self.security)?;

        Ok(ShellArgs { command, description, timeout_ms, workdir, approved })
    }

    fn sandbox_config_for_workdir(&self, workdir: &Path) -> SandboxConfig {
        let mut config = self.sandbox_config.clone();
        config.filesystem.workspace_dir = workdir.to_path_buf();

        let mut read_paths = vec![workdir.to_path_buf()];
        read_paths.extend(self.security.allowed_roots.iter().cloned());
        config.filesystem.read_paths = dedupe_paths(read_paths);

        let mut write_paths = vec![workdir.to_path_buf()];
        write_paths.extend(self.security.allowed_roots.iter().cloned());
        config.filesystem.write_paths = dedupe_paths(write_paths);

        config
    }

    fn permission_context(
        &self,
        shell_args: &ShellArgs,
        sandbox_decision: &SandboxDecision,
    ) -> PermissionContext {
        PermissionContext {
            autonomy: self.security.autonomy,
            in_sandbox: sandbox_decision.use_sandbox,
            mode: PermissionMode::Normal,
            approved: shell_args.approved,
            workspace_dir: shell_args.workdir.clone(),
            allowed_roots: self.security.allowed_roots.clone(),
        }
    }

    fn validate_execution_plan(
        &self,
        shell_args: &ShellArgs,
    ) -> anyhow::Result<ShellExecutionPlan> {
        let effective_command = self.security.apply_shell_redirect_policy(&shell_args.command);
        let sandbox_config = self.sandbox_config_for_workdir(&shell_args.workdir);
        let parsed = parse_command(&effective_command);
        let sandbox_decision = should_use_sandbox(&parsed, &sandbox_config);
        let permission_context = self.permission_context(shell_args, &sandbox_decision);

        match self
            .security
            .check_shell_permission(&effective_command, &permission_context)
            .permission
        {
            Some(Permission::Allow) => {}
            Some(Permission::Deny { reason }) => anyhow::bail!("Command not allowed: {reason}"),
            Some(Permission::Ask { reason, warning }) => {
                let message =
                    warning.map_or(reason.clone(), |warning| format!("{reason}. {warning}"));
                anyhow::bail!(message);
            }
            None => anyhow::bail!("Shell permission check returned no decision"),
        }

        if self.security.bash_forbidden_paths_enabled()
            && let Some(path) = self.security.forbidden_path_argument(&effective_command)
        {
            anyhow::bail!("Path blocked by security policy: {path}");
        }

        if self.security.bash_redirects_enabled()
            && self.security.shell_redirect_policy == ShellRedirectPolicy::Strip
            && parsed.info().is_some_and(|info| {
                info.redirects
                    .iter()
                    .any(|redirect| !redirect.is_fd_duplicate && redirect.target != "/dev/null")
            })
        {
            anyhow::bail!("Command not allowed: unsupported redirection target");
        }

        Ok(ShellExecutionPlan { effective_command, sandbox_config, sandbox_decision })
    }
}

/// 验证环境变量名称是否有效
///
/// 检查给定的字符串是否符合环境变量名称的命名规范。
/// 有效的环境变量名称必须以字母或下划线开头，后续字符可以是字母、数字或下划线。
///
/// # 参数
///
/// - `name`: 待验证的环境变量名称
///
/// # 返回值
///
/// 如果名称有效返回 `true`，否则返回 `false`
///
/// # 示例
///
/// ```
/// # use crate::app::agent::tools::shell::is_valid_env_var_name;
/// assert!(is_valid_env_var_name("PATH"));
/// assert!(is_valid_env_var_name("_HOME"));
/// assert!(is_valid_env_var_name("MY_VAR_123"));
/// assert!(!is_valid_env_var_name("123_VAR"));  // 不能以数字开头
/// assert!(!is_valid_env_var_name("MY-VAR"));   // 不能包含连字符
/// ```
fn is_valid_env_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    // 检查第一个字符：必须是字母或下划线
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    // 检查剩余字符：必须是字母、数字或下划线
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// 收集允许传递给 Shell 命令的环境变量列表
///
/// 根据安全策略，收集所有允许传递给 Shell 命令的环境变量名称。
/// 这些变量来自两个来源：
/// 1. 内置的安全环境变量白名单（[`SAFE_ENV_VARS`]
/// 2. 安全策略中配置的额外允许传递的变量（`shell_env_passthrough`）
///
/// # 参数
///
/// - `security`: 安全策略引用，包含额外的环境变量传递配置
///
/// # 返回值
///
/// 返回去重后的环境变量名称列表
///
/// # 处理逻辑
///
/// 1. 合并内置白名单和策略配置的变量列表
/// 2. 去除空白字符
/// 3. 验证变量名称的有效性
/// 4. 去除重复项
///
/// # 示例
///
/// ```rust,ignore
/// let allowed_vars = collect_allowed_shell_env_vars(&security_policy);
/// for var in allowed_vars {
///     if let Ok(val) = std::env::var(&var) {
///         cmd.env(&var, val);
///     }
/// }
/// ```
pub(super) fn collect_allowed_shell_env_vars(security: &SecurityPolicy) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    // 遍历内置白名单和策略配置的变量
    for key in SAFE_ENV_VARS
        .iter()
        .copied()
        .chain(security.shell_env_passthrough.iter().map(|s| s.as_str()))
    {
        let candidate = key.trim();
        // 跳过空字符串和无效的变量名
        if candidate.is_empty() || !is_valid_env_var_name(candidate) {
            continue;
        }
        // 去重：只添加未见过的变量
        if seen.insert(candidate.to_string()) {
            out.push(candidate.to_string());
        }
    }
    out
}

pub(super) fn allowed_shell_env_value(var: &str) -> Option<String> {
    if var == "PATH" {
        crate::app::agent::shell::effective_path_env()
    } else {
        crate::app::agent::shell::shell_profile_env_var(var)
    }
}

/// 将白名单环境变量注入到 Shell 命令中。
///
/// 执行前会先清空环境，再重新注入允许透传的变量；其中 `PATH` 使用补全后的
/// 路径值，尽量覆盖 GUI 启动场景下缺少 Homebrew 或用户自定义 bin 目录的问题。
pub(super) fn apply_allowed_shell_environment(
    cmd: &mut tokio::process::Command,
    security: &SecurityPolicy,
) {
    cmd.env_clear();

    for var in collect_allowed_shell_env_vars(security) {
        if let Some(val) = allowed_shell_env_value(&var) {
            cmd.env(&var, val);
        }
    }
}

/// 从参数中提取命令字符串
///
/// 从 JSON 参数中提取要执行的 Shell 命令。支持多种参数名称和格式，
/// 以提供更好的兼容性和用户体验。
///
/// # 参数
///
/// - `args`: JSON 格式的参数对象
///
/// # 返回值
///
/// 如果找到有效命令，返回 `Some(String)`，否则返回 `None`
///
/// # 支持的参数名称（按优先级排序）
///
/// 1. `command` - 首选参数名
/// 2. `cmd` - 简写形式
/// 3. `script` - 脚本形式
/// 4. `shell_command` - 完整描述形式
/// 5. `command_line` - 命令行形式
/// 6. `bash` - Bash 特定
/// 7. `sh` - Shell 特定
/// 8. `input` - 输入形式
/// 9. 如果参数本身是字符串，直接使用
///
/// # 示例
///
/// ```json
/// {"command": "ls -la"}
/// {"cmd": "echo hello"}
/// {"script": "#!/bin/bash\necho test"}
/// "ls -la"
/// ```
fn extract_command_argument(args: &serde_json::Value) -> Option<String> {
    // 首先尝试 "command" 参数（首选）
    if let Some(command) =
        args.get("command").and_then(|v| v.as_str()).map(str::trim).filter(|cmd| !cmd.is_empty())
    {
        return Some(command.to_string());
    }

    // 尝试其他常见的命令参数别名
    for alias in ["cmd", "script", "shell_command", "command_line", "bash", "sh", "input"] {
        if let Some(command) =
            args.get(alias).and_then(|v| v.as_str()).map(str::trim).filter(|cmd| !cmd.is_empty())
        {
            return Some(command.to_string());
        }
    }

    // 如果参数本身就是字符串，直接使用
    args.as_str().map(str::trim).filter(|cmd| !cmd.is_empty()).map(ToString::to_string)
}

fn extract_description_argument(args: &serde_json::Value) -> Option<String> {
    args.get("description")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn parse_timeout_ms(args: &serde_json::Value) -> anyhow::Result<u64> {
    match args.get("timeout") {
        None => Ok(DEFAULT_TIMEOUT_MS),
        Some(value) => {
            let timeout = value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|value| u64::try_from(value).ok()))
                .ok_or_else(|| {
                    anyhow::anyhow!("'timeout' must be a positive integer in milliseconds")
                })?;

            if timeout == 0 {
                anyhow::bail!("'timeout' must be greater than 0");
            }

            Ok(timeout)
        }
    }
}

fn resolve_workdir(raw: Option<&str>, security: &SecurityPolicy) -> anyhow::Result<PathBuf> {
    let requested = raw.map(str::trim).filter(|value| !value.is_empty());
    let candidate = match requested {
        Some(path) => {
            let path = PathBuf::from(path);
            if path.is_absolute() { path } else { security.workspace_dir.join(path) }
        }
        None => security.workspace_dir.clone(),
    };

    let resolved = candidate.canonicalize().unwrap_or(candidate);
    if !security.bash_security_enabled || security.is_resolved_path_allowed(&resolved) {
        Ok(resolved)
    } else {
        anyhow::bail!(security.resolved_path_violation_message(&resolved));
    }
}

fn normalize_line_endings(output: String) -> String {
    output.replace("\r\n", "\n").replace('\r', "\n")
}

fn merge_shell_output(stdout: &str, stderr: &str) -> String {
    match (stdout.trim_end(), stderr.trim_end()) {
        ("", "") => String::new(),
        ("", _) => stderr.to_string(),
        (_, "") => stdout.to_string(),
        _ => format!("{stdout}\n{stderr}"),
    }
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::new();
    for path in paths {
        if !unique.iter().any(|existing| existing == &path) {
            unique.push(path);
        }
    }
    unique
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ShellTool {
    /// 获取工具名称
    ///
    /// 返回工具的唯一标识符 "shell"。
    ///
    /// # 返回值
    ///
    /// 工具名称字符串 "shell"
    fn name(&self) -> &str {
        "shell"
    }

    /// 获取工具描述
    ///
    /// 返回工具的功能描述，用于向用户或代理系统说明工具的用途。
    ///
    /// # 返回值
    ///
    /// 工具描述字符串
    fn description(&self) -> &str {
        include_str!("./shell.txt")
    }

    /// 获取工具参数的 JSON Schema
    ///
    /// 返回描述工具接受参数结构的 JSON Schema，用于参数验证和自动补全。
    ///
    /// # 返回值
    ///
    /// JSON Schema 对象，包含以下字段：
    /// - `command`: (必填) 要执行的 Shell 命令字符串
    /// - `description`: (必填) 5-10 个词描述命令作用
    /// - `timeout`: (可选) 超时毫秒数，默认 120000
    /// - `workdir`: (可选) 命令工作目录
    /// - `approved`: (可选) 布尔值，设为 true 可在监督模式下显式批准中/高风险命令
    ///
    /// # 示例
    ///
    /// ```json
    /// {
    ///     "type": "object",
    ///     "properties": {
    ///         "command": {
    ///             "type": "string",
    ///             "description": "要执行的 Shell 命令"
    ///         },
    ///         "approved": {
    ///             "type": "boolean",
    ///             "description": "设为 true 以在监督模式下显式批准中/高风险命令",
    ///             "default": false
    ///         }
    ///     },
    ///     "required": ["command", "description"]
    /// }
    /// ```
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "description": {
                    "type": "string",
                    "description": "Describe the command's purpose in 5-10 words"
                },
                "timeout": {
                    "type": "number",
                    "description": "Timeout in milliseconds. Defaults to 120000."
                },
                "workdir": {
                    "type": "string",
                    "description": "Working directory for the command. Prefer this argument over cd."
                },
                "approved": {
                    "type": "boolean",
                    "description": "Set to true to explicitly approve medium/high-risk commands in supervised mode",
                    "default": false
                }
            },
            "required": ["command", "description"]
        })
    }

    async fn check_permissions(&self, input: &serde_json::Value) -> anyhow::Result<()> {
        let shell_args = self.parse_args(input)?;

        if self.security.is_rate_limited() {
            anyhow::bail!("Rate limit exceeded: too many actions in the last hour");
        }

        let _ = self.validate_execution_plan(&shell_args)?;
        Ok(())
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new("bash", self.description(), self.parameters_schema())
            .with_display_name("bash")
            .with_aliases(["shell"])
            .with_read_only(false)
            .with_destructive(true)
            .with_concurrency_safe(false)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    /// 执行 Shell 命令
    ///
    /// 在沙箱环境中安全地执行给定的 Shell 命令。执行过程包括多层安全检查、
    /// 环境隔离、超时控制和输出处理。
    ///
    /// # 参数
    ///
    /// - `args`: JSON 格式的参数对象，必须包含 `command` 字段
    ///
    /// # 返回值
    ///
    /// 返回 `anyhow::Result<ToolResult>`，其中 `ToolResult` 包含：
    /// - `success`: 命令是否成功执行（退出码为 0）
    /// - `output`: 命令的标准输出（stdout）
    /// - `error`: 命令的标准错误输出（stderr），如果有的话
    ///
    /// # 执行流程
    ///
    /// 1. **参数提取**：从参数中提取命令字符串
    /// 2. **命令处理**：应用重定向策略修改命令
    /// 3. **速率限制检查**：检查是否超过小时级速率限制
    /// 4. **命令验证**：验证命令的安全性和风险等级
    /// 5. **路径检查**：检查命令是否试图访问禁止的路径
    /// 6. **动作预算**：检查并消耗动作预算
    /// 7. **环境准备**：清除环境变量，只注入安全的变量
    /// 8. **命令执行**：在超时限制内执行命令
    /// 9. **输出处理**：截断过大的输出
    /// 10. **系统调用检测**：如果启用，检查输出中的可疑模式
    ///
    /// # 安全检查
    ///
    /// - 小时级速率限制：防止单位时间内执行过多命令
    /// - 命令安全验证：检查命令是否在白名单中，评估风险等级
    /// - 路径访问控制：阻止访问安全策略禁止的路径
    /// - 动作预算管理：限制总操作次数
    /// - 环境变量过滤：只传递白名单中的安全变量
    /// - 超时保护：60 秒后强制终止命令
    /// - 输出截断：限制输出大小为 1MB
    ///
    /// # 错误处理
    ///
    /// - 缺少命令参数：返回错误消息
    /// - 速率限制：返回 "Rate limit exceeded" 错误
    /// - 命令被阻止：返回阻止原因
    /// - 路径被禁止：返回被禁止的路径信息
    /// - 动作预算耗尽：返回预算耗尽消息
    /// - 命令执行失败：返回错误详情
    /// - 命令超时：返回超时消息
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // 执行简单命令
    /// let result = shell_tool.execute(json!({"command": "ls -la"})).await?;
    /// if result.success {
    ///     println!("输出: {}", result.output);
    /// }
    ///
    /// // 执行需要批准的高风险命令
    /// let result = shell_tool.execute(json!({
    ///     "command": "rm -rf /tmp/test",
    ///     "approved": true
    /// })).await?;
    /// ```
    #[allow(clippy::incompatible_msrv)]
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let shell_args = self.parse_args(&args)?;
        let command = shell_args.command.clone();

        // 检查小时级速率限制
        if self.security.is_rate_limited() {
            let msg = "Rate limit exceeded: too many actions in the last hour";
            tracing::warn!(command = %command, "shell tool: rate limited");
            return Ok(ToolResult {
                success: false,
                output: msg.to_string(),
                error: Some(msg.into()),
            });
        }

        let execution_plan = match self.validate_execution_plan(&shell_args) {
            Ok(plan) => plan,
            Err(error) => {
                let message = error.to_string();
                tracing::warn!(command = %command, reason = %message, "shell tool: command blocked");
                return Ok(ToolResult {
                    success: false,
                    output: format!("Command blocked: {message}"),
                    error: Some(message),
                });
            }
        };

        tracing::debug!(
            command = %command,
            effective_command = %execution_plan.effective_command,
            description = %shell_args.description,
            approved = shell_args.approved,
            timeout_ms = shell_args.timeout_ms,
            workdir = %shell_args.workdir.display(),
            allow_unsafe_shell_patterns = self.security.allow_unsafe_shell_patterns,
            autonomy = ?self.security.autonomy,
            "shell tool: executing command"
        );

        let risk = self.security.command_risk_level(&execution_plan.effective_command);
        tracing::debug!(
            command = %command,
            risk = ?risk,
            sandboxed = execution_plan.sandbox_decision.use_sandbox,
            sandbox_reason = ?execution_plan.sandbox_decision.reason,
            "shell tool: command validated"
        );

        // 消耗动作预算（检查是否还有剩余预算）
        if !self.security.record_action() {
            let msg = "Rate limit exceeded: action budget exhausted";
            tracing::warn!(command = %command, "shell tool: action budget exhausted");
            return Ok(ToolResult {
                success: false,
                output: msg.to_string(),
                error: Some(msg.into()),
            });
        }

        tracing::debug!(command = %execution_plan.effective_command, "shell tool: executing");

        // 构建命令执行器，使用超时机制防止命令挂起
        // 清除环境变量以防止泄露 API 密钥等敏感信息（CWE-200）
        // 然后只重新添加安全的、功能性的环境变量
        let mut cmd = if execution_plan.sandbox_decision.use_sandbox {
            match SandboxExecutor::new(execution_plan.sandbox_config.clone()).build_command(
                self.runtime.as_ref(),
                &execution_plan.effective_command,
                &shell_args.workdir,
            ) {
                Ok(cmd) => cmd,
                Err(e) => {
                    let msg = format!("Failed to build sandboxed command: {e}");
                    return Ok(ToolResult {
                        success: false,
                        output: msg.clone(),
                        error: Some(msg),
                    });
                }
            }
        } else {
            match self
                .runtime
                .build_shell_command(&execution_plan.effective_command, &shell_args.workdir)
            {
                Ok(cmd) => cmd,
                Err(e) => {
                    let msg = format!("Failed to build runtime command: {e}");
                    return Ok(ToolResult {
                        success: false,
                        output: msg.clone(),
                        error: Some(msg),
                    });
                }
            }
        };

        // 只注入白名单中的安全环境变量
        apply_allowed_shell_environment(&mut cmd, &self.security);
        cmd.kill_on_drop(true);

        // 在超时限制内执行命令
        let result =
            tokio::time::timeout(Duration::from_millis(shell_args.timeout_ms), cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                // 命令执行成功，处理输出
                let mut stdout =
                    normalize_line_endings(String::from_utf8_lossy(&output.stdout).to_string());
                let mut stderr =
                    normalize_line_endings(String::from_utf8_lossy(&output.stderr).to_string());

                // 截断过大的输出以防止内存溢出（OOM）
                if stdout.len() > MAX_OUTPUT_BYTES {
                    stdout.truncate(crate::app::agent::util::floor_utf8_char_boundary(
                        &stdout,
                        MAX_OUTPUT_BYTES,
                    ));
                    stdout.push_str("\n... [output truncated at 1MB]");
                }
                if stderr.len() > MAX_OUTPUT_BYTES {
                    stderr.truncate(crate::app::agent::util::floor_utf8_char_boundary(
                        &stderr,
                        MAX_OUTPUT_BYTES,
                    ));
                    stderr.push_str("\n... [stderr truncated at 1MB]");
                }

                // 如果启用了系统调用检测器，检查输出中的可疑模式
                if let Some(detector) = &self.syscall_detector {
                    let _ = detector.inspect_command_output(
                        &execution_plan.effective_command,
                        &stdout,
                        &stderr,
                        output.status.code(),
                    );
                }

                let parsed = parse_command(&execution_plan.effective_command);
                let semantics = ExitSemantics::for_parsed_command(&parsed);
                let interpretation = semantics.interpret(output.status.code());

                Ok(tool_result_from_interpretation(interpretation, stdout, stderr))
            }
            Ok(Err(e)) => {
                // 命令执行失败（例如：命令不存在）
                let msg = format!("Failed to execute command: {e}");
                Ok(ToolResult { success: false, output: msg.clone(), error: Some(msg) })
            }
            Err(_) => {
                // 命令执行超时
                let msg =
                    format!("Command timed out after {}ms and was killed", shell_args.timeout_ms);
                Ok(ToolResult { success: false, output: msg.clone(), error: Some(msg) })
            }
        }
    }
}

fn tool_result_from_interpretation(
    interpretation: ExitInterpretation,
    stdout: String,
    stderr: String,
) -> ToolResult {
    let merged_output = merge_shell_output(&stdout, &stderr);
    match interpretation {
        ExitInterpretation::Success
        | ExitInterpretation::ConditionTrue
        | ExitInterpretation::DifferencesFound => {
            ToolResult { success: true, output: merged_output, error: None }
        }
        ExitInterpretation::NoMatches => ToolResult {
            success: true,
            output: append_note(merged_output, "[No matches found]"),
            error: None,
        },
        ExitInterpretation::PartialSuccess => ToolResult {
            success: true,
            output: append_note(merged_output, "[Command completed with partial success]"),
            error: None,
        },
        ExitInterpretation::ConditionFalse => ToolResult {
            success: true,
            output: append_note(merged_output, "[Condition evaluated to false]"),
            error: None,
        },
        ExitInterpretation::Error { message } => ToolResult {
            success: false,
            output: merged_output,
            error: Some(if stderr.is_empty() { message } else { format!("{message}: {stderr}") }),
        },
    }
}

fn append_note(mut output: String, note: &str) -> String {
    if output.trim().is_empty() {
        note.to_string()
    } else {
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(note);
        output
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
