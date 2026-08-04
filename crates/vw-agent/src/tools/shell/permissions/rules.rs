//! Shell 权限规则引擎，负责按命令模式、上下文和安全发现产出审批结果。

use std::path::PathBuf;

use glob::Pattern;
use regex::Regex;

use crate::security::policy::allowlist::is_allowlist_entry_match;
use crate::tools::shell::ast::ParsedCommand;
use crate::tools::shell::security::{SecurityPipeline, Severity};
use vw_config_types::security::BashValidatorChecksConfig;

use super::mode::PermissionMode;
use super::sandbox_allow::SandboxAutoAllow;
use super::warning::get_destructive_warning;
use super::{PermissionContext, PermissionResult};

/// RuleAction 枚举描述当前模块中一组明确的状态或分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleAction {
    Allow,
    Deny,
    Ask,
}

/// RulePattern 枚举描述当前模块中一组明确的状态或分类。
#[derive(Debug, Clone)]
pub enum RulePattern {
    Exact { command: String },
    Prefix { command: String },
    Glob { pattern: String },
    Regex { pattern: Regex },
}

impl RulePattern {
    /// 执行 matches 操作，并返回调用方需要的结果。
    pub fn matches(&self, cmd: &ParsedCommand) -> bool {
        self.matches_raw(cmd.raw().trim())
    }

    /// 执行 matches_raw 操作，并返回调用方需要的结果。
    pub fn matches_raw(&self, raw: &str) -> bool {
        let raw = raw.trim();
        match self {
            Self::Exact { command } => raw == command,
            Self::Prefix { command } => {
                if command.chars().last().is_some_and(char::is_whitespace) {
                    return raw.starts_with(command);
                }
                raw == command || raw.strip_prefix(command).is_some_and(is_boundary)
            }
            Self::Glob { pattern } => Pattern::new(pattern).is_ok_and(|glob| glob.matches(raw)),
            Self::Regex { pattern } => pattern.is_match(raw),
        }
    }
}

/// RuleCondition 枚举描述当前模块中一组明确的状态或分类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleCondition {
    InSandbox,
    HasArgument { arg: String },
    NotHasArgument { arg: String },
    WorkdirMatches { path: PathBuf },
}

/// PermissionRule 结构体保存当前模块对外暴露的数据。
#[derive(Debug, Clone)]
pub struct PermissionRule {
    /// action 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub action: RuleAction,
    /// pattern 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub pattern: RulePattern,
    /// condition 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub condition: Option<RuleCondition>,
    /// reason 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub reason: String,
}

impl PermissionRule {
    /// 执行 matches 操作，并返回调用方需要的结果。
    pub fn matches(&self, cmd: &ParsedCommand, context: &PermissionContext) -> bool {
        self.pattern.matches(cmd)
            && self
                .condition
                .as_ref()
                .is_none_or(|condition| condition_matches(condition, cmd, context))
    }
}

/// RuleEngine 结构体保存当前模块对外暴露的数据。
pub struct RuleEngine {
    deny_rules: Vec<PermissionRule>,
    ask_rules: Vec<PermissionRule>,
    allow_rules: Vec<PermissionRule>,
    legacy_allowlist: Vec<String>,
    rules_enabled: bool,
    security_pipeline: SecurityPipeline,
    sandbox_auto_allow: SandboxAutoAllow,
}

impl RuleEngine {
    /// 执行 new 操作，并返回调用方需要的结果。
    pub fn new(strict: bool) -> Self {
        Self::new_with_quoted_newline_blocking(strict, false)
    }

    /// 创建规则引擎，并显式控制是否阻断 Shell 引号内的换行符。
    pub fn new_with_quoted_newline_blocking(
        strict: bool,
        block_quoted_shell_newlines: bool,
    ) -> Self {
        let mut validators = BashValidatorChecksConfig::default();
        validators.quoted_newline = block_quoted_shell_newlines;
        Self::new_with_config(strict, true, &validators)
    }

    /// 创建规则引擎，并分别控制命令规则与 Shell validator。
    pub fn new_with_config(
        strict: bool,
        rules_enabled: bool,
        validators: &BashValidatorChecksConfig,
    ) -> Self {
        Self {
            deny_rules: Vec::new(),
            ask_rules: Vec::new(),
            allow_rules: Vec::new(),
            legacy_allowlist: Vec::new(),
            rules_enabled,
            security_pipeline: SecurityPipeline::new_with_config(strict, validators),
            sandbox_auto_allow: SandboxAutoAllow::default(),
        }
    }

    /// 执行 with_legacy_allowlist 操作，并返回调用方需要的结果。
    pub fn with_legacy_allowlist(mut self, allowlist: Vec<String>) -> Self {
        self.legacy_allowlist = allowlist;
        self
    }

    /// 执行 with_sandbox_auto_allow 操作，并返回调用方需要的结果。
    pub fn with_sandbox_auto_allow(mut self, sandbox_auto_allow: SandboxAutoAllow) -> Self {
        self.sandbox_auto_allow = sandbox_auto_allow;
        self
    }

    /// 执行 push_rule 操作，并返回调用方需要的结果。
    pub fn push_rule(&mut self, rule: PermissionRule) {
        match rule.action {
            RuleAction::Allow => self.allow_rules.push(rule),
            RuleAction::Deny => self.deny_rules.push(rule),
            RuleAction::Ask => self.ask_rules.push(rule),
        }
    }

    /// 执行 check 操作，并返回调用方需要的结果。
    pub fn check(&self, cmd: &ParsedCommand, context: &PermissionContext) -> PermissionResult {
        if context.mode == PermissionMode::AutoAccept {
            return PermissionResult::allow();
        }

        if self.rules_enabled {
            for rule in &self.deny_rules {
                if rule.matches(cmd, context) {
                    return PermissionResult::deny(rule.reason.clone());
                }
            }
        }

        let security_report = self.security_pipeline.validate(cmd);
        if security_report.blocked {
            let reason = security_report
                .findings
                .iter()
                .filter(|finding| finding.severity == Severity::Block)
                .map(|finding| finding.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            return PermissionResult::deny(reason).with_findings(security_report.findings);
        }

        if self.sandbox_auto_allow.should_auto_allow(cmd, context.in_sandbox) {
            return PermissionResult::allow().with_findings(security_report.findings);
        }

        if context.mode.auto_allows_command(command_name(cmd).unwrap_or_default()) {
            return PermissionResult::allow().with_findings(security_report.findings);
        }

        if !self.rules_enabled {
            return PermissionResult::allow().with_findings(security_report.findings);
        }

        for rule in &self.ask_rules {
            if rule.matches(cmd, context) {
                if context.approved {
                    return PermissionResult::allow().with_findings(security_report.findings);
                }
                let warning = get_destructive_warning(cmd);
                return PermissionResult::ask(rule.reason.clone(), warning)
                    .with_findings(security_report.findings);
            }
        }

        for rule in &self.allow_rules {
            if rule.matches(cmd, context) {
                return PermissionResult::allow().with_findings(security_report.findings);
            }
        }

        if self.matches_legacy_allowlist(cmd) {
            return PermissionResult::allow().with_findings(security_report.findings);
        }

        PermissionResult::deny("No matching allow rule").with_findings(security_report.findings)
    }

    fn matches_legacy_allowlist(&self, cmd: &ParsedCommand) -> bool {
        let raw = cmd.raw();
        for segment in raw.split([';', '\n']).map(str::trim).filter(|segment| !segment.is_empty()) {
            let tokens = shell_words::split(segment).unwrap_or_default();
            let Some(executable) = tokens.first() else {
                continue;
            };
            let base = executable.rsplit('/').next().unwrap_or_default();
            if !self
                .legacy_allowlist
                .iter()
                .any(|allowed| is_allowlist_entry_match(allowed, executable, base))
            {
                return false;
            }
        }

        !self.legacy_allowlist.is_empty()
    }
}

fn condition_matches(
    condition: &RuleCondition,
    cmd: &ParsedCommand,
    context: &PermissionContext,
) -> bool {
    match condition {
        RuleCondition::InSandbox => context.in_sandbox,
        RuleCondition::HasArgument { arg } => command_tokens(cmd).iter().any(|token| token == arg),
        RuleCondition::NotHasArgument { arg } => {
            !command_tokens(cmd).iter().any(|token| token == arg)
        }
        RuleCondition::WorkdirMatches { path } => &context.workspace_dir == path,
    }
}

fn command_name(cmd: &ParsedCommand) -> Option<&str> {
    match cmd {
        ParsedCommand::Ast(_, info) => Some(info.name.as_str()),
        ParsedCommand::Fallback { tokens, .. } => tokens.first().map(String::as_str),
    }
}

fn command_tokens(cmd: &ParsedCommand) -> Vec<&str> {
    match cmd {
        ParsedCommand::Ast(_, info) => std::iter::once(info.name.as_str())
            .chain(info.args.iter().map(String::as_str))
            .collect(),
        ParsedCommand::Fallback { tokens, .. } => tokens.iter().map(String::as_str).collect(),
    }
}

fn is_boundary(rest: &str) -> bool {
    rest.chars().next().is_some_and(|ch| ch.is_whitespace() || matches!(ch, ';' | '|' | '&'))
}
