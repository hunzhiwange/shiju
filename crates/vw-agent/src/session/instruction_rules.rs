//! `.vibewindow/rules` 的路径条件解析。

use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Paths {
    One(String),
    Many(Vec<String>),
}

impl Paths {
    fn into_vec(self) -> Vec<String> {
        match self {
            Self::One(path) => vec![path],
            Self::Many(paths) => paths,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct Frontmatter {
    #[serde(default)]
    paths: Option<Paths>,
}

/// 已激活规则的正文和原始路径条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveRule {
    pub(crate) content: String,
    pub(crate) conditions: Vec<String>,
}

/// 仅在目标文件符合规则 frontmatter 的 `paths` glob 时读取规则正文。
pub(crate) fn load_if_matches(rule: &Path, workspace: &Path, target: &Path) -> Option<ActiveRule> {
    let content = std::fs::read_to_string(rule).ok()?;
    let (frontmatter, body) = split_frontmatter(&content);
    let conditions = frontmatter
        .and_then(|frontmatter| serde_yaml::from_str::<Frontmatter>(frontmatter).ok())
        .and_then(|frontmatter| frontmatter.paths)
        .map(Paths::into_vec)
        .unwrap_or_default();
    if !conditions.is_empty() && !matches_any(&conditions, workspace, target) {
        return None;
    }
    Some(ActiveRule { content: body.to_string(), conditions })
}

fn split_frontmatter(content: &str) -> (Option<&str>, &str) {
    let Some(rest) = content.strip_prefix("---\n") else { return (None, content) };
    let mut consumed = 4;
    for line in rest.split_inclusive('\n') {
        consumed += line.len();
        if line.trim() == "---" {
            return (Some(&content[4..consumed - line.len()]), &content[consumed..]);
        }
    }
    (None, content)
}

fn matches_any(patterns: &[String], workspace: &Path, target: &Path) -> bool {
    let target = normalize_relative(target, workspace);
    patterns.iter().any(|pattern| {
        glob::Pattern::new(pattern.trim()).is_ok_and(|pattern| pattern.matches_path(&target))
    })
}

fn normalize_relative(target: &Path, workspace: &Path) -> PathBuf {
    target.strip_prefix(workspace).unwrap_or(target).components().collect()
}
