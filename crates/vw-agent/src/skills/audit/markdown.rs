//! 技能 Markdown 审计逻辑，负责检查文档链接、脚本引用和提示注入风险。

use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::{Component, Path};
use std::sync::OnceLock;
use vw_config_types::skills::SkillSecurityConfig;

use super::report::SkillAuditReport;
use super::risk::detect_high_risk_snippet;
use super::support::{
    has_markdown_suffix, has_script_suffix, looks_like_absolute_path, relative_display,
    strip_query_and_fragment, url_scheme,
};

/// 执行 audit_markdown_file 操作，并返回调用方需要的结果。
pub(super) fn audit_markdown_file(
    root: &Path,
    path: &Path,
    security: &SkillSecurityConfig,
    report: &mut SkillAuditReport,
) -> Result<()> {
    audit_markdown_content(root, path, security, report, true)
}

/// 审计附加 Markdown 资源。
///
/// 附加资料常包含示例链接、上游文档路径或占位文件名。运行时不会自动展开这些链接，
/// 因此这里保留高风险文本检测，但不把链接完整性作为技能加载的阻断条件。
pub(super) fn audit_markdown_resource_file(
    root: &Path,
    path: &Path,
    security: &SkillSecurityConfig,
    report: &mut SkillAuditReport,
) -> Result<()> {
    audit_markdown_content(root, path, security, report, false)
}

fn audit_markdown_content(
    root: &Path,
    path: &Path,
    security: &SkillSecurityConfig,
    report: &mut SkillAuditReport,
    audit_links: bool,
) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read markdown file {}", path.display()))?;
    let rel = relative_display(root, path);

    if security.detect_high_risk_content {
        if let Some(pattern) = detect_high_risk_snippet(&content) {
            report.findings.push(format!("{rel}: detected high-risk command pattern ({pattern})."));
        }
    }

    if audit_links && (security.reject_script_files || security.validate_markdown_links) {
        for raw_target in extract_markdown_links(&content) {
            audit_markdown_link_target(root, path, &raw_target, security, report);
        }
    }

    Ok(())
}

fn audit_markdown_link_target(
    root: &Path,
    source: &Path,
    raw: &str,
    security: &SkillSecurityConfig,
    report: &mut SkillAuditReport,
) {
    let normalized = normalize_markdown_target(raw);
    if normalized.is_empty() || normalized.starts_with('#') {
        return;
    }

    let rel = relative_display(root, source);

    if let Some(scheme) = url_scheme(normalized) {
        if matches!(scheme, "http" | "https" | "mailto") {
            if security.validate_markdown_links && has_markdown_suffix(normalized) {
                report.findings.push(format!(
                    "{rel}: remote markdown links are blocked by skill security audit ({normalized})."
                ));
            }
            return;
        }

        if security.validate_markdown_links {
            report
                .findings
                .push(format!("{rel}: unsupported URL scheme in markdown link ({normalized})."));
        }
        return;
    }

    let stripped = strip_query_and_fragment(normalized);
    if stripped.is_empty() {
        return;
    }

    if looks_like_absolute_path(stripped) {
        if security.validate_markdown_links {
            report.findings.push(format!(
                "{rel}: absolute markdown link paths are not allowed ({normalized})."
            ));
        }
        return;
    }

    if security.reject_script_files && has_script_suffix(stripped) {
        report
            .findings
            .push(format!("{rel}: markdown links to script files are blocked ({normalized})."));
    }

    if !security.validate_markdown_links || !has_markdown_suffix(stripped) {
        return;
    }

    let Some(base_dir) = source.parent() else {
        report.findings.push(format!(
            "{rel}: failed to resolve parent directory for markdown link ({normalized})."
        ));
        return;
    };
    let linked_path = base_dir.join(stripped);

    match linked_path.canonicalize() {
        Ok(canonical_target) => {
            if !canonical_target.starts_with(root) {
                report
                    .findings
                    .push(format!("{rel}: markdown link escapes skill root ({normalized})."));
                return;
            }
            if !canonical_target.is_file() {
                report
                    .findings
                    .push(format!("{rel}: markdown link must point to a file ({normalized})."));
            }
        }
        Err(_) => {
            if is_cross_skill_reference(stripped) {
                return;
            }
            report
                .findings
                .push(format!("{rel}: markdown link points to a missing file ({normalized})."));
        }
    }
}

/// 执行 is_cross_skill_reference 操作，并返回调用方需要的结果。
pub(super) fn is_cross_skill_reference(target: &str) -> bool {
    let path = Path::new(target);

    if path.components().any(|component| component == Component::ParentDir) {
        return true;
    }

    let stripped = target.strip_prefix("./").unwrap_or(target);
    !stripped.contains('/') && !stripped.contains('\\') && has_markdown_suffix(stripped)
}

fn extract_markdown_links(content: &str) -> Vec<String> {
    static MARKDOWN_LINK_RE: OnceLock<Regex> = OnceLock::new();
    let regex = MARKDOWN_LINK_RE.get_or_init(|| {
        Regex::new(r#"\[[^\]]*\]\(([^)]+)\)"#).expect("markdown link regex must compile")
    });

    regex
        .captures_iter(content)
        .filter_map(|capture| capture.get(1))
        .map(|target| target.as_str().trim().to_string())
        .collect()
}

fn normalize_markdown_target(raw_target: &str) -> &str {
    let trimmed = raw_target.trim();
    let trimmed = trimmed.strip_prefix('<').unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix('>').unwrap_or(trimmed);
    trimmed.split_whitespace().next().unwrap_or_default()
}
#[cfg(test)]
#[path = "markdown_tests.rs"]
mod markdown_tests;
