//! 复合 Shell 命令分析器，负责识别管道、串联、子 shell 等需要额外审批的结构。

use std::path::{Path, PathBuf};

use crate::shell::ast::{ParsedCommand, parse_command};
use crate::shell::permissions::{
    Permission, PermissionContext, PermissionResult, RuleEngine,
};
use crate::shell::security::SecurityFinding;

/// 声明 semantics 子模块，保持当前领域的职责拆分清晰。
pub mod semantics;

/// UnsafeStructure 枚举描述当前模块中一组明确的状态或分类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsafeStructure {
    Subshell,
    CommandGroup,
    Coprocess,
    Background,
    ProcessSubstitution,
    HereString,
}

/// SegmentResult 结构体保存当前模块对外暴露的数据。
#[derive(Debug, Clone)]
pub struct SegmentResult {
    /// segment 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub segment: ParsedCommand,
    /// permission 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub permission: PermissionResult,
}

/// CompoundAnalysisResult 枚举描述当前模块中一组明确的状态或分类。
#[derive(Debug, Clone)]
pub enum CompoundAnalysisResult {
    Allowed {
        segments: Vec<SegmentResult>,
        findings: Vec<SecurityFinding>,
    },
    NeedsApproval {
        segments: Vec<SegmentResult>,
        reasons: Vec<String>,
        warning: Option<String>,
        findings: Vec<SecurityFinding>,
    },
    Blocked {
        reason: String,
        findings: Vec<SecurityFinding>,
    },
}

impl CompoundAnalysisResult {
    /// 执行 into_permission_result 操作，并返回调用方需要的结果。
    pub fn into_permission_result(self) -> PermissionResult {
        match self {
            Self::Allowed { findings, .. } => PermissionResult::allow().with_findings(findings),
            Self::NeedsApproval { reasons, warning, findings, .. } => {
                let reason = reasons.join("; ");
                PermissionResult::ask(reason, warning).with_findings(findings)
            }
            Self::Blocked { reason, findings } => {
                PermissionResult::deny(reason).with_findings(findings)
            }
        }
    }
}

/// CompoundCommandAnalyzer 结构体保存当前模块对外暴露的数据。
#[derive(Debug, Default)]
pub struct CompoundCommandAnalyzer;

impl CompoundCommandAnalyzer {
    /// 执行 analyze 操作，并返回调用方需要的结果。
    pub fn analyze(
        cmd: &ParsedCommand,
        permission_engine: &RuleEngine,
        context: &PermissionContext,
    ) -> CompoundAnalysisResult {
        if let Err(structure) = Self::check_unsafe_structures(cmd) {
            return CompoundAnalysisResult::Blocked {
                reason: format!(
                    "Compound command uses unsupported unsafe structure: {}",
                    unsafe_structure_label(structure)
                ),
                findings: Vec::new(),
            };
        }

        let segments = Self::split_segments(cmd);
        let results = segments
            .iter()
            .cloned()
            .map(|segment| SegmentResult {
                permission: permission_engine.check(&segment, context),
                segment,
            })
            .collect::<Vec<_>>();

        if let Err(reason) = Self::check_cross_segment_patterns(&segments, &results, context) {
            return CompoundAnalysisResult::Blocked {
                reason,
                findings: collect_findings(&results),
            };
        }

        Self::aggregate_results(results)
    }

    fn split_segments(cmd: &ParsedCommand) -> Vec<ParsedCommand> {
        let raw = cmd.raw().trim();
        let segments = split_top_level_segments(raw);
        if segments.len() <= 1 {
            return vec![cmd.clone()];
        }

        segments.into_iter().map(|segment| parse_command(segment.as_str())).collect()
    }

    fn check_unsafe_structures(cmd: &ParsedCommand) -> Result<(), UnsafeStructure> {
        let trimmed = cmd.raw().trim();

        if is_wrapped_by(trimmed, '(', ')') {
            return Err(UnsafeStructure::Subshell);
        }
        if is_wrapped_command_group(trimmed) {
            return Err(UnsafeStructure::CommandGroup);
        }
        if trimmed.starts_with("coproc ") || trimmed == "coproc" {
            return Err(UnsafeStructure::Coprocess);
        }
        if contains_top_level_here_string(trimmed) {
            return Err(UnsafeStructure::HereString);
        }
        if contains_process_substitution(trimmed) {
            return Err(UnsafeStructure::ProcessSubstitution);
        }
        if contains_top_level_background(trimmed) {
            return Err(UnsafeStructure::Background);
        }

        Ok(())
    }

    fn check_cross_segment_patterns(
        segments: &[ParsedCommand],
        _results: &[SegmentResult],
        context: &PermissionContext,
    ) -> Result<(), String> {
        let cd_segments = segments
            .iter()
            .enumerate()
            .filter(|(_, segment)| segment.command_name() == Some("cd"))
            .collect::<Vec<_>>();
        if cd_segments.len() > 1 {
            return Err("Multiple cd commands in compound command are not supported".into());
        }

        let git_after_cd =
            segments.iter().enumerate().find(|(_, segment)| segment.command_name() == Some("git"));
        if let (Some((cd_index, cd_segment)), Some((git_index, _))) =
            (cd_segments.first().copied(), git_after_cd)
            && git_index > cd_index
        {
            let target = cd_segment
                .args()
                .first()
                .map(String::as_str)
                .filter(|value| !value.is_empty())
                .unwrap_or(".");
            let Some(resolved) = resolve_path(target, &context.workspace_dir) else {
                return Err(format!(
                    "cd to '{target}' followed by git command could not be resolved safely"
                ));
            };
            if !is_path_allowed(&resolved, &context.workspace_dir, &context.allowed_roots) {
                return Err(format!(
                    "cd to '{}' followed by git command may escape workspace",
                    resolved.display()
                ));
            }
        }

        Ok(())
    }

    fn aggregate_results(results: Vec<SegmentResult>) -> CompoundAnalysisResult {
        let findings = collect_findings(&results);

        if let Some(reason) =
            results.iter().find_map(|result| match &result.permission.permission {
                Some(Permission::Deny { reason }) => Some(reason.clone()),
                _ => None,
            })
        {
            return CompoundAnalysisResult::Blocked { reason, findings };
        }

        let reasons = results
            .iter()
            .filter_map(|result| match &result.permission.permission {
                Some(Permission::Ask { reason, .. }) => Some(reason.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        if !reasons.is_empty() {
            let warning = results.iter().find_map(|result| match &result.permission.permission {
                Some(Permission::Ask { warning, .. }) => warning.clone(),
                _ => None,
            });
            return CompoundAnalysisResult::NeedsApproval {
                segments: results,
                reasons,
                warning,
                findings,
            };
        }

        CompoundAnalysisResult::Allowed { segments: results, findings }
    }
}

fn collect_findings(results: &[SegmentResult]) -> Vec<SecurityFinding> {
    results.iter().flat_map(|result| result.permission.security_findings.clone()).collect()
}

fn unsafe_structure_label(structure: UnsafeStructure) -> &'static str {
    match structure {
        UnsafeStructure::Subshell => "subshell",
        UnsafeStructure::CommandGroup => "command group",
        UnsafeStructure::Coprocess => "coprocess",
        UnsafeStructure::Background => "background execution",
        UnsafeStructure::ProcessSubstitution => "process substitution",
        UnsafeStructure::HereString => "here-string",
    }
}

fn split_top_level_segments(command: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut single_quote = false;
    let mut double_quote = false;
    let mut backtick = false;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;

    while let Some(ch) = chars.next() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' && !single_quote {
            current.push(ch);
            escaped = true;
            continue;
        }

        match ch {
            '\'' if !double_quote && !backtick => {
                single_quote = !single_quote;
                current.push(ch);
                continue;
            }
            '"' if !single_quote && !backtick => {
                double_quote = !double_quote;
                current.push(ch);
                continue;
            }
            '`' if !single_quote => {
                backtick = !backtick;
                current.push(ch);
                continue;
            }
            _ => {}
        }

        if single_quote || double_quote || backtick {
            current.push(ch);
            continue;
        }

        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ => {}
        }

        if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
            let is_split = match ch {
                ';' | '|' => true,
                '&' => chars.peek().is_some_and(|next| *next == '&'),
                _ => false,
            };

            if is_split {
                match ch {
                    '&' => {
                        current.push(ch);
                        if chars.peek().is_some_and(|next| *next == '&') {
                            current.pop();
                            let _ = chars.next();
                            push_segment(&mut segments, &mut current);
                            continue;
                        }
                    }
                    '|' => {
                        if chars.peek().is_some_and(|next| *next == '|') {
                            let _ = chars.next();
                        }
                        push_segment(&mut segments, &mut current);
                        continue;
                    }
                    ';' => {
                        push_segment(&mut segments, &mut current);
                        continue;
                    }
                    _ => {}
                }
            }
        }

        current.push(ch);
    }

    push_segment(&mut segments, &mut current);
    if segments.is_empty() { vec![command.trim().to_string()] } else { segments }
}

fn push_segment(segments: &mut Vec<String>, current: &mut String) {
    let segment = current.trim();
    if !segment.is_empty() {
        segments.push(segment.to_string());
    }
    current.clear();
}

fn is_wrapped_by(command: &str, open: char, close: char) -> bool {
    let chars = command.chars().collect::<Vec<_>>();
    if chars.first() != Some(&open) || chars.last() != Some(&close) {
        return false;
    }

    let mut depth = 0usize;
    for (index, ch) in chars.iter().enumerate() {
        if *ch == open {
            depth += 1;
        } else if *ch == close {
            depth = depth.saturating_sub(1);
            if depth == 0 && index != chars.len() - 1 {
                return false;
            }
        }
    }

    depth == 0
}

fn is_wrapped_command_group(command: &str) -> bool {
    command.starts_with('{')
        && command.ends_with('}')
        && command.contains(';')
        && is_wrapped_by(command, '{', '}')
}

fn contains_top_level_here_string(command: &str) -> bool {
    contains_top_level_token(command, "<<<")
}

fn contains_process_substitution(command: &str) -> bool {
    contains_top_level_token(command, "<(") || contains_top_level_token(command, ">(")
}

fn contains_top_level_background(command: &str) -> bool {
    let chars = command.chars().collect::<Vec<_>>();
    let mut single_quote = false;
    let mut double_quote = false;
    let mut backtick = false;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;

    for (index, ch) in chars.iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if *ch == '\\' && !single_quote {
            escaped = true;
            continue;
        }
        match *ch {
            '\'' if !double_quote && !backtick => {
                single_quote = !single_quote;
                continue;
            }
            '"' if !single_quote && !backtick => {
                double_quote = !double_quote;
                continue;
            }
            '`' if !single_quote => {
                backtick = !backtick;
                continue;
            }
            _ => {}
        }
        if single_quote || double_quote || backtick {
            continue;
        }

        match *ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '&' if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                let prev = index.checked_sub(1).and_then(|pos| chars.get(pos)).copied();
                let next = chars.get(index + 1).copied();
                if prev != Some('>') && prev != Some('&') && next != Some('&') {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

fn contains_top_level_token(command: &str, needle: &str) -> bool {
    let chars = command.chars().collect::<Vec<_>>();
    let needle_chars = needle.chars().collect::<Vec<_>>();
    let mut single_quote = false;
    let mut double_quote = false;
    let mut backtick = false;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut index = 0usize;

    while index < chars.len() {
        let ch = chars[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if ch == '\\' && !single_quote {
            escaped = true;
            index += 1;
            continue;
        }
        match ch {
            '\'' if !double_quote && !backtick => {
                single_quote = !single_quote;
                index += 1;
                continue;
            }
            '"' if !single_quote && !backtick => {
                double_quote = !double_quote;
                index += 1;
                continue;
            }
            '`' if !single_quote => {
                backtick = !backtick;
                index += 1;
                continue;
            }
            _ => {}
        }
        if single_quote || double_quote || backtick {
            index += 1;
            continue;
        }

        if paren_depth == 0
            && brace_depth == 0
            && bracket_depth == 0
            && chars[index..].starts_with(&needle_chars)
        {
            return true;
        }

        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ => {}
        }

        index += 1;
    }

    false
}

fn resolve_path(value: &str, workspace_dir: &Path) -> Option<PathBuf> {
    if value.is_empty() {
        return None;
    }

    let path = if value == "~" || value.starts_with("~/") {
        let home = directories::BaseDirs::new()?.home_dir().to_path_buf();
        if value == "~" { home } else { home.join(value.trim_start_matches("~/")) }
    } else {
        PathBuf::from(value)
    };

    Some(if path.is_absolute() { path } else { workspace_dir.join(path) })
}

fn is_path_allowed(path: &Path, workspace_dir: &Path, allowed_roots: &[PathBuf]) -> bool {
    path.starts_with(workspace_dir) || allowed_roots.iter().any(|root| path.starts_with(root))
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
