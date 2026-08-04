//! 技能清单审计逻辑，负责检查 TOML manifest 中可能扩大能力或触发高风险行为的配置。

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use vw_config_types::skills::SkillSecurityConfig;

use super::report::SkillAuditReport;
use super::risk::{contains_shell_chaining, detect_high_risk_snippet};
use super::support::relative_display;

/// 执行 audit_manifest_file 操作，并返回调用方需要的结果。
pub(super) fn audit_manifest_file(
    root: &Path,
    path: &Path,
    security: &SkillSecurityConfig,
    report: &mut SkillAuditReport,
) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read TOML manifest {}", path.display()))?;
    let rel = relative_display(root, path);

    let parsed: toml::Value = match toml::from_str(&content) {
        Ok(value) => value,
        Err(err) => {
            if security.validate_manifest {
                report.findings.push(format!("{rel}: invalid TOML manifest ({err})."));
            }
            return Ok(());
        }
    };

    if let Some(tools) = parsed.get("tools").and_then(toml::Value::as_array) {
        for (idx, tool) in tools.iter().enumerate() {
            let command = tool.get("command").and_then(toml::Value::as_str);
            let kind = tool.get("kind").and_then(toml::Value::as_str).unwrap_or("unknown");

            if let Some(command) = command {
                if security.detect_shell_chaining && contains_shell_chaining(command) {
                    report.findings.push(format!(
                        "{rel}: tools[{idx}].command uses shell chaining operators, which are blocked."
                    ));
                }
                if security.detect_high_risk_content {
                    if let Some(pattern) = detect_high_risk_snippet(command) {
                        report.findings.push(format!(
                            "{rel}: tools[{idx}].command matches high-risk pattern ({pattern})."
                        ));
                    }
                }
            } else if security.validate_manifest {
                report.findings.push(format!("{rel}: tools[{idx}] is missing a command field."));
            }

            if security.validate_manifest
                && (kind.eq_ignore_ascii_case("script") || kind.eq_ignore_ascii_case("shell"))
                && command.is_some_and(|value| value.trim().is_empty())
            {
                report.findings.push(format!("{rel}: tools[{idx}] has an empty {kind} command."));
            }
        }
    }

    if let Some(prompts) = parsed.get("prompts").and_then(toml::Value::as_array) {
        for (idx, prompt) in prompts.iter().enumerate() {
            if security.detect_high_risk_content {
                if let Some(prompt) = prompt.as_str() {
                    if let Some(pattern) = detect_high_risk_snippet(prompt) {
                        report.findings.push(format!(
                            "{rel}: prompts[{idx}] contains high-risk pattern ({pattern})."
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}
#[cfg(test)]
#[path = "manifest_tests.rs"]
mod manifest_tests;
