//! AGENTS `@include` 展开。
//!
//! 只接受已知的文本文件，并在读取每个被包含文件前复用运行时的路径策略。包含内容
//! 总在包含者之前输出，防止父级指令覆盖其依赖的细化约束。

use crate::app::agent::security::SecurityPolicy;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 展开结果中单个文件的来源关系。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Source {
    pub(crate) filepath: String,
    pub(crate) parent: Option<String>,
}

/// 一次安全包含展开的结果。
#[derive(Debug, Default)]
pub(crate) struct Expanded {
    pub(crate) content: String,
    pub(crate) sources: Vec<Source>,
    pub(crate) diagnostics: Vec<String>,
}

/// 安全地展开一个指令文件中的 `@path` 引用。
pub(crate) fn expand(filepath: &Path, policy: &SecurityPolicy) -> Expanded {
    let mut expanded = Expanded::default();
    let mut visiting = HashSet::new();
    expanded.content =
        expand_file(filepath, None, policy, &mut visiting, &mut expanded, true).unwrap_or_default();
    expanded
}

fn expand_file(
    filepath: &Path,
    parent: Option<&Path>,
    policy: &SecurityPolicy,
    visiting: &mut HashSet<PathBuf>,
    expanded: &mut Expanded,
    is_root: bool,
) -> Option<String> {
    if !is_safe_text_path(filepath) {
        expanded.diagnostics.push(format!(
            "Rejected AGENTS include with unsupported text type: {}",
            filepath.display()
        ));
        return None;
    }
    let canonical = match std::fs::canonicalize(filepath) {
        Ok(filepath) if filepath.is_file() => filepath,
        _ => {
            expanded.diagnostics.push(format!(
                "AGENTS include file is missing or not a file: {}",
                filepath.display()
            ));
            return None;
        }
    };
    if !is_root && !policy.is_resolved_path_allowed(&canonical) {
        expanded.diagnostics.push(policy.resolved_path_violation_message(&canonical));
        return None;
    }
    if !visiting.insert(canonical.clone()) {
        expanded
            .diagnostics
            .push(format!("Rejected AGENTS include cycle: {}", canonical.display()));
        return None;
    }

    let content = match std::fs::read_to_string(&canonical) {
        Ok(content) if !content.contains('\0') => content,
        Ok(_) => {
            expanded.diagnostics.push(format!(
                "Rejected AGENTS include with binary content: {}",
                canonical.display()
            ));
            visiting.remove(&canonical);
            return None;
        }
        Err(error) => {
            expanded
                .diagnostics
                .push(format!("Failed to read AGENTS include {}: {error}", canonical.display()));
            visiting.remove(&canonical);
            return None;
        }
    };

    expanded.sources.push(Source {
        filepath: canonical.to_string_lossy().to_string(),
        parent: parent.map(|parent| parent.to_string_lossy().to_string()),
    });
    let mut children = Vec::new();
    let mut parent_content = String::new();
    let mut fenced = false;
    for segment in content.split_inclusive('\n') {
        let line = segment.trim_end_matches(['\r', '\n']);
        if is_fence(line) {
            fenced = !fenced;
            parent_content.push_str(segment);
            continue;
        }
        if fenced {
            parent_content.push_str(segment);
            continue;
        }

        let references = include_references(line);
        if references.is_empty() {
            parent_content.push_str(segment);
            continue;
        }
        let child_count = children.len();
        for reference in references {
            let path = if reference.is_absolute() {
                reference
            } else {
                canonical.parent().unwrap_or_else(|| Path::new(".")).join(reference)
            };
            if let Some(child) =
                expand_file(&path, Some(&canonical), policy, visiting, expanded, false)
            {
                children.push(child);
            }
        }
        if children.len() == child_count {
            // 保留未展开的引用，连同诊断一起让调用方明确看到拒绝原因。
            parent_content.push_str(segment);
        }
    }
    visiting.remove(&canonical);

    children.push(parent_content);
    Some(children.join("\n"))
}

fn is_safe_text_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("md" | "mdx" | "txt" | "rst" | "adoc" | "toml" | "yaml" | "yml" | "json")
    )
}

fn is_fence(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

fn include_references(line: &str) -> Vec<PathBuf> {
    line.split_whitespace()
        .filter_map(|token| token.strip_prefix('@'))
        .map(|path| {
            path.trim_end_matches(|character: char| {
                matches!(character, ',' | '.' | ':' | ';' | '!' | '?' | ')' | ']' | '}')
            })
        })
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .collect()
}
