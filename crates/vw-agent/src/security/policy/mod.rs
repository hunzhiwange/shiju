//! 代理运行时安全策略实现。
//!
//! 本模块负责把自治级别、命令 allowlist、路径边界、风险分类和行动预算组合成
//! 一次明确的授权判断。策略默认收紧：先通过命令规则和只读约束，再检查路径
//! 是否逃逸工作区，最后根据风险等级决定允许、拒绝或要求批准。

mod action_tracker;
pub(crate) mod allowlist;
mod path_utils;
mod risk;
mod shell_lexer;
mod shell_redirect;
mod types;

pub use action_tracker::ActionTracker;
pub use types::{AutonomyLevel, CommandRiskLevel, QuoteState, ShellRedirectPolicy, ToolOperation};

use std::path::{Path, PathBuf};

use vw_config_types::security::{BashPolicyChecksConfig, BashSecurityConfig};

use crate::tools::shell::ast::parse_command;
use crate::tools::shell::compound::CompoundCommandAnalyzer;
use crate::tools::shell::path::{PathCheckResult, check_path_constraints};
use crate::tools::shell::permissions::{
    Permission, PermissionContext, PermissionMode, PermissionResult, PermissionRule, RuleAction,
    RuleCondition, RuleEngine, RulePattern,
};
use crate::tools::shell::readonly::check_readonly_constraints;
use allowlist::is_allowlist_entry_match;
use path_utils::expand_user_path;
use risk::classify_command_risk;
use shell_lexer::{
    contains_unquoted_char, contains_unquoted_shell_variable_expansion,
    contains_unquoted_single_ampersand, skip_env_assignments, split_unquoted_segments,
    strip_wrapping_quotes,
};
use shell_redirect::strip_supported_redirects;
/// 运行时安全策略。
///
/// 该结构体承载 shell 执行、路径访问和行动预算相关的所有策略输入。字段保持
/// 显式，便于配置层、测试和调用方清楚看到每个安全边界。
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// 是否启用 Bash 命令执行前安全检查。
    pub bash_security_enabled: bool,
    /// Bash 命令执行前安全检查的完整分层配置。
    pub bash_security: BashSecurityConfig,
    /// 当前自治级别，决定是否允许主动行动以及是否需要人工批准。
    pub autonomy: AutonomyLevel,
    /// 当前工作区根目录。
    pub workspace_dir: PathBuf,
    /// 是否默认只允许访问工作区和额外允许根目录。
    pub workspace_only: bool,
    /// 旧版命令 allowlist，支持命令名、路径和 `*`。
    pub allowed_commands: Vec<String>,
    /// 明确禁止访问的路径前缀。
    pub forbidden_paths: Vec<String>,
    /// 工作区之外仍允许访问的额外根目录。
    pub allowed_roots: Vec<PathBuf>,
    /// 每小时允许的行动操作数量。
    pub max_actions_per_hour: u32,
    /// 每日成本预算，供上层成本控制使用。
    pub max_cost_per_day_cents: u32,
    /// 中风险命令在监督模式下是否必须批准。
    pub require_approval_for_medium_risk: bool,
    /// 是否直接阻断高风险命令。
    pub block_high_risk_commands: bool,
    /// shell 重定向处理策略。
    pub shell_redirect_policy: ShellRedirectPolicy,
    /// 允许传递给 shell 的环境变量名。
    pub shell_env_passthrough: Vec<String>,
    /// 是否允许高风险 shell 语法模式。
    pub allow_unsafe_shell_patterns: bool,
    /// 行动计数器，用于本地速率限制。
    pub tracker: ActionTracker,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        let bash_security = BashSecurityConfig { enabled: true, ..BashSecurityConfig::default() };
        Self {
            bash_security_enabled: true,
            bash_security,
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: PathBuf::from("."),
            workspace_only: true,
            allowed_commands: vec![
                "git".into(),
                "npm".into(),
                "cargo".into(),
                "ls".into(),
                "cat".into(),
                "grep".into(),
                "find".into(),
                "echo".into(),
                "pwd".into(),
                "wc".into(),
                "head".into(),
                "tail".into(),
                "date".into(),
            ],
            forbidden_paths: vec![
                "/etc".into(),
                "/root".into(),
                "/home".into(),
                "/usr".into(),
                "/bin".into(),
                "/sbin".into(),
                "/lib".into(),
                "/opt".into(),
                "/boot".into(),
                "/dev".into(),
                "/proc".into(),
                "/sys".into(),
                "/var".into(),
                "/tmp".into(),
                "~/.ssh".into(),
                "~/.gnupg".into(),
                "~/.aws".into(),
                "~/.config".into(),
            ],
            allowed_roots: Vec::new(),
            max_actions_per_hour: 20,
            max_cost_per_day_cents: 500,
            require_approval_for_medium_risk: true,
            block_high_risk_commands: true,
            shell_redirect_policy: ShellRedirectPolicy::Block,
            shell_env_passthrough: vec![],
            allow_unsafe_shell_patterns: false,
            tracker: ActionTracker::new(),
        }
    }
}

impl SecurityPolicy {
    fn bash_checks_enabled(&self) -> bool {
        self.bash_security_enabled && self.bash_security.enabled
    }

    fn policy_check_enabled(&self, check: impl FnOnce(&BashPolicyChecksConfig) -> bool) -> bool {
        self.bash_checks_enabled()
            && self.bash_security.policy.enabled
            && check(&self.bash_security.policy)
    }

    /// 是否启用 Bash 禁止路径检查。
    pub fn bash_forbidden_paths_enabled(&self) -> bool {
        self.policy_check_enabled(|policy| policy.forbidden_paths)
    }

    /// 是否启用 Bash 重定向检查。
    pub fn bash_redirects_enabled(&self) -> bool {
        self.policy_check_enabled(|policy| policy.redirects)
    }

    /// 按策略处理 shell 重定向。
    ///
    /// 参数 `command` 是原始命令。返回值可能是原命令，也可能是移除受支持重定向
    /// 后的命令；本函数不返回错误，不支持的危险重定向仍会交给后续权限检查拒绝。
    pub fn apply_shell_redirect_policy(&self, command: &str) -> String {
        if !self.bash_redirects_enabled() {
            return command.to_string();
        }
        match self.shell_redirect_policy {
            ShellRedirectPolicy::Allow => command.to_string(),
            ShellRedirectPolicy::Block => command.to_string(),
            ShellRedirectPolicy::Strip => strip_supported_redirects(command),
        }
    }

    /// 计算命令的风险等级。
    ///
    /// 参数 `command` 是待执行命令。返回值用于决定是否需要人工批准或直接拒绝；
    /// 风险分类本身不产生授权结果。
    pub fn command_risk_level(&self, command: &str) -> CommandRiskLevel {
        classify_command_risk(command)
    }

    /// 验证命令是否可以执行。
    ///
    /// `command` 是原始 shell 命令，`approved` 表示调用方是否已经获得人工批准。
    /// 成功时返回命令风险等级；失败时返回面向调用方的拒绝原因。该函数会先应用
    /// 重定向策略，再委托 `check_shell_permission` 做完整权限判断。
    pub fn validate_command_execution(
        &self,
        command: &str,
        approved: bool,
    ) -> Result<CommandRiskLevel, String> {
        let effective_command = self.apply_shell_redirect_policy(command);
        let context = PermissionContext {
            autonomy: self.autonomy,
            in_sandbox: false,
            mode: PermissionMode::Normal,
            approved,
            workspace_dir: self.workspace_dir.clone(),
            allowed_roots: self.allowed_roots.clone(),
        };

        match self.check_shell_permission(&effective_command, &context).permission {
            Some(Permission::Allow) => Ok(self.command_risk_level(&effective_command)),
            Some(Permission::Deny { reason }) => {
                if reason.contains("not allowed") || reason.contains("blocked") {
                    Err(reason)
                } else {
                    Err(format!("Command not allowed: {reason}"))
                }
            }
            Some(Permission::Ask { reason, warning }) => Err(match warning {
                Some(warning) => format!("{reason}. {warning}"),
                None => reason,
            }),
            None => Err("Shell permission check returned no decision".into()),
        }
    }

    /// 执行完整 shell 权限检查。
    ///
    /// `command` 是待检查命令，`context` 提供当前批准状态、工作区和允许根目录。
    /// 返回 `PermissionResult`，其中可能是允许、拒绝或要求批准。安全检查按收紧
    /// 顺序进行：只读约束、规则引擎、路径边界、风险等级。
    pub fn check_shell_permission(
        &self,
        command: &str,
        context: &PermissionContext,
    ) -> PermissionResult {
        if !self.bash_checks_enabled() {
            return PermissionResult::allow();
        }
        let parsed = parse_command(command);
        if self.policy_check_enabled(|policy| policy.read_only)
            && self.autonomy == AutonomyLevel::ReadOnly
            && !check_readonly_constraints(&parsed).is_readonly()
        {
            return PermissionResult::deny(format!(
                "Command not allowed by security policy: {command}"
            ));
        }
        if self.policy_check_enabled(|policy| policy.unsafe_shell_patterns)
            && !self.allow_unsafe_shell_patterns
            && contains_unquoted_shell_variable_expansion(command)
        {
            return PermissionResult::deny(format!(
                "Command not allowed by security policy: {command}"
            ));
        }

        let engine = self.build_rule_engine();
        let result =
            CompoundCommandAnalyzer::analyze(&parsed, &engine, context).into_permission_result();
        match result.permission {
            Some(Permission::Allow) => {}
            _ => return result,
        }

        if self.policy_check_enabled(|policy| policy.workspace_paths) && self.workspace_only {
            if let PathCheckResult::Blocked { path, reason } =
                check_path_constraints(&parsed, &context.workspace_dir, &context.allowed_roots)
            {
                return PermissionResult::deny(format!(
                    "Path blocked by security policy: {} ({reason})",
                    path.display()
                ));
            }
        }

        // 风险检查放在 allowlist 和路径检查之后，确保“已允许的命令”仍会因为破坏性
        // 行为被二次拦截。
        let risk = self.command_risk_level(command);
        if self.policy_check_enabled(|policy| policy.risk) && risk == CommandRiskLevel::High {
            if self.block_high_risk_commands {
                return PermissionResult::deny(
                    "Command blocked: high-risk command is disallowed by policy",
                );
            }
            if context.autonomy == AutonomyLevel::Supervised && !context.approved {
                return PermissionResult::ask(
                    "Command requires explicit approval (approved=true): high-risk operation",
                    crate::tools::shell::permissions::warning::get_destructive_warning(&parsed),
                );
            }
        }

        if self.policy_check_enabled(|policy| policy.risk)
            && risk == CommandRiskLevel::Medium
            && context.autonomy == AutonomyLevel::Supervised
            && self.require_approval_for_medium_risk
            && !context.approved
        {
            return PermissionResult::ask(
                "Command requires explicit approval (approved=true): medium-risk operation",
                crate::tools::shell::permissions::warning::get_destructive_warning(&parsed),
            );
        }

        PermissionResult::allow().with_findings(result.security_findings)
    }

    /// 构造 shell 权限规则引擎。
    ///
    /// 返回值包含旧 allowlist 兼容规则，以及自治级别派生出的额外拒绝/询问规则。
    /// 不支持的不安全 shell 模式默认以严格模式处理，只有完全自治且显式允许时才放宽。
    fn build_rule_engine(&self) -> RuleEngine {
        let strict = !self.allow_unsafe_shell_patterns || self.autonomy != AutonomyLevel::Full;
        let mut engine = RuleEngine::new_with_config(
            strict,
            self.policy_check_enabled(|policy| policy.command_rules),
            &self.bash_security.validators,
        )
        .with_legacy_allowlist(self.allowed_commands.clone());

        for entry in &self.allowed_commands {
            let entry = strip_wrapping_quotes(entry).trim();
            if entry.is_empty() {
                continue;
            }
            engine.push_rule(PermissionRule {
                action: RuleAction::Allow,
                pattern: RulePattern::Exact { command: entry.to_string() },
                condition: None,
                reason: format!("legacy allowlist entry: {entry}"),
            });
            engine.push_rule(PermissionRule {
                action: RuleAction::Allow,
                pattern: RulePattern::Prefix { command: entry.to_string() },
                condition: None,
                reason: format!("legacy allowlist prefix: {entry}"),
            });
        }

        if self.policy_check_enabled(|policy| policy.risk)
            && self.autonomy == AutonomyLevel::Supervised
            && self.require_approval_for_medium_risk
        {
            for command in ["touch", "mkdir", "cp", "mv", "chmod", "chown"] {
                engine.push_rule(PermissionRule {
                    action: RuleAction::Ask,
                    pattern: RulePattern::Prefix { command: command.into() },
                    condition: Some(RuleCondition::NotHasArgument { arg: "--help".into() }),
                    reason:
                        "Command requires explicit approval (approved=true): medium-risk operation"
                            .into(),
                });
            }
        }

        engine
    }

    /// 检查命令是否满足旧版 allowlist 与基础安全规则。
    ///
    /// 参数 `command` 是原始 shell 命令。返回 `true` 表示命令在旧策略层面可执行；
    /// 返回 `false` 表示命令被只读约束、危险 shell 语法、allowlist 或参数安全检查拒绝。
    pub fn is_command_allowed(&self, command: &str) -> bool {
        if !self.bash_checks_enabled() {
            return true;
        }
        tracing::info!(
            command = %command,
            allow_unsafe_shell_patterns = self.allow_unsafe_shell_patterns,
            autonomy = ?self.autonomy,
            "is_command_allowed: checking"
        );

        if self.policy_check_enabled(|policy| policy.read_only)
            && self.autonomy == AutonomyLevel::ReadOnly
            && !check_readonly_constraints(&parse_command(command)).is_readonly()
        {
            tracing::warn!(
                command = %command,
                autonomy = ?self.autonomy,
                "is_command_allowed: blocked by read-only autonomy"
            );
            return false;
        }

        if self.policy_check_enabled(|policy| policy.unsafe_shell_patterns)
            && !self.allow_unsafe_shell_patterns
        {
            // 未显式启用危险模式时，先拒绝容易绕过静态解析的 shell 结构，防止
            // 子命令、变量展开或重定向把实际执行面扩大到 allowlist 之外。
            if command.contains('`')
                || contains_unquoted_shell_variable_expansion(command)
                || command.contains("<(")
                || command.contains(">(")
            {
                tracing::warn!(
                    command = %command,
                    has_backtick = command.contains('`'),
                    has_var_expansion = contains_unquoted_shell_variable_expansion(command),
                    has_process_sub_in = command.contains("<("),
                    has_process_sub_out = command.contains(">("),
                    "is_command_allowed: blocked by subshell/variable expansion"
                );
                return false;
            }

            if command.split_whitespace().any(|w| w == "tee" || w.ends_with("/tee")) {
                tracing::warn!(command = %command, "is_command_allowed: blocked by tee command");
                return false;
            }

            if contains_unquoted_single_ampersand(command) {
                tracing::warn!(command = %command, "is_command_allowed: blocked by single ampersand");
                return false;
            }
        } else if self.policy_check_enabled(|policy| policy.unsafe_shell_patterns) {
            tracing::debug!(command = %command, "is_command_allowed: unsafe patterns allowed, skipping hardcoded checks");
        }

        if self.policy_check_enabled(|policy| policy.redirects)
            && !self.allow_unsafe_shell_patterns
            && (contains_unquoted_char(command, '>') || contains_unquoted_char(command, '<'))
        {
            tracing::warn!(
                command = %command,
                has_gt = contains_unquoted_char(command, '>'),
                has_lt = contains_unquoted_char(command, '<'),
                "is_command_allowed: blocked by redirect operators"
            );
            return false;
        }

        if !self.policy_check_enabled(|policy| policy.command_rules) {
            return true;
        }

        let segments = split_unquoted_segments(command);
        for segment in &segments {
            let cmd_part = skip_env_assignments(segment);

            let mut words = cmd_part.split_whitespace();
            let executable = strip_wrapping_quotes(words.next().unwrap_or("")).trim();
            let base_cmd = executable.rsplit('/').next().unwrap_or("");

            if base_cmd.is_empty() {
                continue;
            }

            if !self
                .allowed_commands
                .iter()
                .any(|allowed| is_allowlist_entry_match(allowed, executable, base_cmd))
            {
                tracing::warn!(
                    command = %command,
                    segment = %segment,
                    executable = %executable,
                    base_cmd = %base_cmd,
                    allowed_commands = ?self.allowed_commands,
                    "is_command_allowed: blocked by allowlist"
                );
                return false;
            }

            let args: Vec<String> = words.map(|w| w.to_ascii_lowercase()).collect();
            if !self.is_args_safe(base_cmd, &args) {
                tracing::warn!(
                    command = %command,
                    segment = %segment,
                    base_cmd = %base_cmd,
                    args = ?args,
                    "is_command_allowed: blocked by args safety check"
                );
                return false;
            }
        }

        let has_cmd = segments.iter().any(|s| {
            let s = skip_env_assignments(s.trim());
            s.split_whitespace().next().is_some_and(|w| !w.is_empty())
        });

        if !has_cmd {
            tracing::warn!(command = %command, "is_command_allowed: no valid command found");
        }

        has_cmd
    }

    /// 检查特定命令参数是否落在安全子集内。
    ///
    /// `base` 是命令 basename，`args` 是已小写化的参数列表。返回 `false` 表示
    /// 参数可能引入二次执行或配置劫持风险。
    fn is_args_safe(&self, base: &str, args: &[String]) -> bool {
        if self.allow_unsafe_shell_patterns {
            return true;
        }
        let base = base.to_ascii_lowercase();
        match base.as_str() {
            "find" => !args.iter().any(|arg| arg == "-exec" || arg == "-ok"),
            "git" => !args.iter().any(|arg| {
                arg == "config"
                    || arg.starts_with("config.")
                    || arg == "alias"
                    || arg.starts_with("alias.")
                    || arg == "-c"
            }),
            _ => true,
        }
    }

    /// 返回命令中第一个违反路径约束的参数。
    ///
    /// 参数 `command` 是待检查命令。返回 `Some(path)` 表示解析后的命令访问了工作区
    /// 或允许根目录之外的路径；返回 `None` 表示未发现路径违规。
    pub fn forbidden_path_argument(&self, command: &str) -> Option<String> {
        let workspace_paths_enabled =
            self.policy_check_enabled(|policy| policy.workspace_paths) && self.workspace_only;
        if !workspace_paths_enabled && !self.bash_forbidden_paths_enabled() {
            return None;
        }
        if workspace_paths_enabled {
            let parsed = parse_command(command);
            if let PathCheckResult::Blocked { path, .. } =
                check_path_constraints(&parsed, &self.workspace_dir, &self.allowed_roots)
            {
                return Some(path.display().to_string());
            }
        }
        if !self.bash_forbidden_paths_enabled() {
            return None;
        }
        allowlist::find_forbidden_path_argument(command, |path| {
            let expanded = normalize_path(&expand_user_path(path));
            !self
                .forbidden_paths
                .iter()
                .map(|forbidden| normalize_path(&expand_user_path(forbidden)))
                .any(|forbidden| expanded.starts_with(forbidden))
        })
    }

    /// 判断未解析路径字符串是否允许访问。
    ///
    /// 参数 `path` 可以是相对路径、绝对路径或 `~` 路径。返回 `false` 表示路径包含
    /// 空字节、父目录逃逸、编码逃逸、非法 home 写法或命中禁止路径。绝对路径会
    /// 进一步交给 `is_resolved_path_allowed` 检查。
    pub fn is_path_allowed(&self, path: &str) -> bool {
        // 空字节和父目录组件常用于绕过下游文件 API 或目录前缀检查，必须在任何
        // 路径展开前拒绝。
        if path.contains('\0') {
            return false;
        }

        if Path::new(path).components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return false;
        }

        let lower = path.to_lowercase();
        if lower.contains("..%2f") || lower.contains("%2f..") {
            return false;
        }

        if path.starts_with('~') && path != "~" && !path.starts_with("~/") {
            return false;
        }

        let expanded_path = expand_user_path(path);

        if expanded_path.is_absolute() {
            return self.is_resolved_path_allowed(&normalize_path(&expanded_path));
        }

        for forbidden in &self.forbidden_paths {
            let forbidden_path = expand_user_path(forbidden);
            if expanded_path.starts_with(forbidden_path) {
                return false;
            }
        }

        true
    }

    /// 判断已规范化/解析后的路径是否允许访问。
    ///
    /// `resolved` 应为调用方已经解析过的路径。返回 `true` 表示路径位于工作区或
    /// 额外允许根目录中；如果 `workspace_only` 为 `false`，未命中禁止路径的外部
    /// 路径也会被允许。
    pub fn is_resolved_path_allowed(&self, resolved: &Path) -> bool {
        let normalized_resolved = normalize_path(resolved);
        let normalized_workspace = normalize_path(&self.workspace_dir);
        if normalized_resolved.starts_with(&normalized_workspace) {
            return true;
        }

        let workspace_root =
            self.workspace_dir.canonicalize().unwrap_or_else(|_| self.workspace_dir.clone());
        if resolved.starts_with(&workspace_root) {
            return true;
        }

        for root in &self.allowed_roots {
            let normalized_root = normalize_path(root);
            if normalized_resolved.starts_with(&normalized_root) {
                return true;
            }

            let canonical = root.canonicalize().unwrap_or_else(|_| root.clone());
            if resolved.starts_with(&canonical) {
                return true;
            }
        }

        for forbidden in &self.forbidden_paths {
            let forbidden_path = expand_user_path(forbidden);
            if resolved.starts_with(&forbidden_path) {
                return false;
            }
        }

        if !self.workspace_only {
            return true;
        }

        false
    }

    /// 构造路径逃逸提示信息。
    ///
    /// 参数 `resolved` 是被拒绝的解析后路径。返回值包含具体路径和如何配置
    /// `allowed_roots` 的修复建议。
    pub fn resolved_path_violation_message(&self, resolved: &Path) -> String {
        let guidance = if self.allowed_roots.is_empty() {
            "Add the directory to [autonomy].allowed_roots (for example: allowed_roots = [\"/absolute/path\"]), or move the file into the workspace."
        } else {
            "Add a matching parent directory to [autonomy].allowed_roots, or move the file into the workspace."
        };

        format!("Resolved path escapes workspace allowlist: {}. {}", resolved.display(), guidance)
    }

    /// 判断当前策略是否允许产生外部副作用。
    pub fn can_act(&self) -> bool {
        self.autonomy != AutonomyLevel::ReadOnly
    }

    /// 对工具操作执行通用权限检查。
    ///
    /// `operation` 指定读或行动类别，`operation_name` 用于错误消息。读操作总是允许；
    /// 行动操作会受只读自治和每小时行动预算约束。
    pub fn enforce_tool_operation(
        &self,
        operation: ToolOperation,
        operation_name: &str,
    ) -> Result<(), String> {
        match operation {
            ToolOperation::Read => Ok(()),
            ToolOperation::Act => {
                if !self.can_act() {
                    return Err(format!(
                        "Security policy: read-only mode, cannot perform '{operation_name}'"
                    ));
                }

                if !self.record_action() {
                    return Err("Rate limit exceeded: action budget exhausted".to_string());
                }

                Ok(())
            }
        }
    }

    /// 记录一次行动并返回是否仍在预算内。
    ///
    /// 返回 `false` 表示当前小时行动次数已经超过 `max_actions_per_hour`。
    pub fn record_action(&self) -> bool {
        let count = self.tracker.record();
        count <= self.max_actions_per_hour as usize
    }

    /// 判断行动预算是否已经耗尽。
    pub fn is_rate_limited(&self) -> bool {
        self.tracker.count() >= self.max_actions_per_hour as usize
    }

    /// 从配置构造安全策略。
    ///
    /// `autonomy_config` 是配置层自治策略，`workspace_dir` 是当前工作区根目录。返回值
    /// 会把相对 `allowed_roots` 解析到工作区下，并创建新的行动计数器。
    pub fn from_config(
        autonomy_config: &crate::app::agent::config::AutonomyConfig,
        workspace_dir: &Path,
    ) -> Self {
        let bash_security = BashSecurityConfig { enabled: true, ..BashSecurityConfig::default() };
        Self::from_autonomy_config(autonomy_config, workspace_dir, bash_security)
    }

    /// 从完整安全配置构造策略。
    ///
    /// Bash 检查默认遵循 `security.bash.enabled`；调用方如果提供完全访问上下文，
    /// 应在会话策略组装阶段显式关闭 `bash_security_enabled`。
    pub fn from_configs(
        autonomy_config: &crate::app::agent::config::AutonomyConfig,
        security_config: &vw_config_types::security::SecurityConfig,
        workspace_dir: &Path,
    ) -> Self {
        Self::from_autonomy_config(autonomy_config, workspace_dir, security_config.bash.clone())
    }

    fn from_autonomy_config(
        autonomy_config: &crate::app::agent::config::AutonomyConfig,
        workspace_dir: &Path,
        bash_security: BashSecurityConfig,
    ) -> Self {
        Self {
            bash_security_enabled: bash_security.enabled,
            bash_security,
            autonomy: autonomy_config.level,
            workspace_dir: workspace_dir.to_path_buf(),
            workspace_only: autonomy_config.workspace_only,
            allowed_commands: autonomy_config.allowed_commands.clone(),
            forbidden_paths: autonomy_config.forbidden_paths.clone(),
            allowed_roots: autonomy_config
                .allowed_roots
                .iter()
                .map(|root| {
                    let expanded = expand_user_path(root);
                    if expanded.is_absolute() { expanded } else { workspace_dir.join(expanded) }
                })
                .collect(),
            max_actions_per_hour: autonomy_config.max_actions_per_hour,
            max_cost_per_day_cents: autonomy_config.max_cost_per_day_cents,
            require_approval_for_medium_risk: autonomy_config.require_approval_for_medium_risk,
            block_high_risk_commands: autonomy_config.block_high_risk_commands,
            shell_redirect_policy: autonomy_config.shell_redirect_policy,
            shell_env_passthrough: autonomy_config.shell_env_passthrough.clone(),
            allow_unsafe_shell_patterns: autonomy_config.allow_unsafe_shell_patterns,
            tracker: ActionTracker::new(),
        }
    }
}

/// 对路径做纯语法级规范化。
///
/// 该函数折叠 `.` 与 `..` 组件，但不访问文件系统；用于在不存在的绝对路径上也能
/// 保持可预测的前缀判断。
fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = if path.is_absolute() {
        PathBuf::from(std::path::Component::RootDir.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::RootDir => {
                normalized = PathBuf::from(std::path::Component::RootDir.as_os_str());
            }
            std::path::Component::Normal(part) => normalized.push(part),
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
        }
    }

    normalized
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
