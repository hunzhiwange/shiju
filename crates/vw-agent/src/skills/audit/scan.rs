//! Skill 静态审计的路径扫描入口。
//!
//! 本模块负责以确定顺序遍历 skill 目录，并把每个文件交给
//! Markdown/TOML 专用审计器。这里集中处理跨文件类型的安全边界，
//! 例如符号链接、脚本文件和超大文本文件，避免下游解析器在不可信
//! 输入上承担额外风险。

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use vw_config_types::skills::SkillSecurityConfig;

use super::MAX_TEXT_FILE_BYTES;
use super::manifest::audit_manifest_file;
use super::markdown::{audit_markdown_file, audit_markdown_resource_file};
use super::report::SkillAuditReport;
use super::support::{
    is_markdown_file, is_toml_file, is_unsupported_script_file, relative_display,
};

/// 按深度优先顺序收集 `root` 下的所有路径。
///
/// # 参数
///
/// - `root`: 要遍历的 skill 根目录。
///
/// # 返回值
///
/// 返回包含根路径自身、子目录与文件的路径列表。子项先排序再压栈，
/// 因此结果稳定，方便审计报告和测试保持可重复。
///
/// # 错误
///
/// 当目录读取失败时返回错误，并在错误上下文中带上失败目录。
pub(super) fn collect_paths_depth_first(root: &Path) -> Result<Vec<PathBuf>> {
    let mut stack = vec![root.to_path_buf()];
    let mut out = Vec::new();

    while let Some(current) = stack.pop() {
        out.push(current.clone());

        if !current.is_dir() {
            continue;
        }

        let mut children = Vec::new();
        for entry in fs::read_dir(&current)
            .with_context(|| format!("failed to read directory {}", current.display()))?
        {
            let entry = entry?;
            children.push(entry.path());
        }

        children.sort();
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }

    Ok(out)
}

/// 审计单个路径，并把发现写入 `report`。
///
/// # 参数
///
/// - `root`: skill 根目录，用于生成报告中的相对路径。
/// - `path`: 当前要审计的文件、目录或链接。
/// - `report`: 聚合审计发现的报告对象。
///
/// # 返回值
///
/// 审计成功完成时返回 `Ok(())`。发现安全问题会记录到报告中，不会把
/// 可预期的不合规输入当作函数错误。
///
/// # 错误
///
/// 元数据读取失败、Markdown/TOML 解析审计失败时返回错误。
pub(super) fn audit_path(
    root: &Path,
    path: &Path,
    security: &SkillSecurityConfig,
    report: &mut SkillAuditReport,
) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("failed to read metadata for {}", path.display()))?;
    let rel = relative_display(root, path);

    if metadata.file_type().is_symlink() {
        if security.reject_symlinks {
            report.findings.push(format!("{rel}: symlinks are not allowed in installed skills."));
        }
        return Ok(());
    }

    if metadata.is_dir() {
        return Ok(());
    }

    if security.reject_script_files && is_unsupported_script_file(path) {
        report
            .findings
            .push(format!("{rel}: script-like files are blocked by skill security policy."));
    }

    if security.reject_oversized_text_files
        && metadata.len() > MAX_TEXT_FILE_BYTES
        && (is_markdown_file(path) || is_toml_file(path))
    {
        // 只对需要静态解析的文本清单限制大小，避免恶意 skill 用超大文件
        // 消耗内存或拖慢审计；其他文件类型此处不读取内容。
        report.findings.push(format!(
            "{rel}: file is too large for static audit (>{MAX_TEXT_FILE_BYTES} bytes)."
        ));
        return Ok(());
    }

    if is_skill_entry_markdown(path) {
        audit_markdown_file(root, path, security, report)?;
    } else if is_markdown_file(path) {
        audit_markdown_resource_file(root, path, security, report)?;
    } else if is_toml_file(path) {
        audit_manifest_file(root, path, security, report)?;
    }

    Ok(())
}

fn is_skill_entry_markdown(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
}
#[cfg(test)]
#[path = "scan_tests.rs"]
mod scan_tests;
