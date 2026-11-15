//! 轻量文件列表、目录树和正则搜索能力。
//!
//! 本模块提供不依赖外部 `rg` 进程的文件遍历与搜索实现，供 agent 工具在受控
//! 工作区内列文件、构建目录摘要、执行文本匹配。遍历会复用默认忽略规则，降低
//! 依赖目录和构建产物带来的噪声。

use crate::ignore;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[cfg(test)]
#[path = "ripgrep_tests.rs"]
mod ripgrep_tests;

#[derive(Debug, Clone)]
/// 文件列表查询输入。
pub struct FilesInput {
    /// 遍历根目录。
    pub cwd: PathBuf,
    /// 可选包含 glob；为空时包含所有未忽略文件。
    pub glob: Option<Vec<String>>,
    /// 是否包含隐藏路径；默认包含。
    pub hidden: Option<bool>,
    /// 是否跟随符号链接；默认不跟随。
    pub follow: Option<bool>,
    /// 可选最大遍历深度。
    pub max_depth: Option<usize>,
}

/// 判断相对路径中是否包含隐藏片段。
///
/// 参数：
/// - `rel`：使用 `/` 分隔的相对路径。
///
/// 返回值：
/// 任一片段以 `.` 开头且不只是 `.` 时返回 `true`。
fn is_hidden_path(rel: &str) -> bool {
    rel.split('/').any(|p| p.starts_with('.') && p.len() > 1)
}

/// 判断路径是否命中任一 glob。
///
/// 参数：
/// - `globs`：已编译的 glob 集合。
/// - `rel`：使用 `/` 分隔的相对路径。
///
/// 返回值：
/// 命中任一 glob 时返回 `true`。
fn match_globs(globs: &[glob::Pattern], rel: &str) -> bool {
    globs.iter().any(|g| g.matches(rel))
}

/// 列出工作区内符合条件的文件。
///
/// 参数：
/// - `input`：遍历根目录、glob、隐藏文件、符号链接和深度配置。
///
/// 返回值：
/// 返回排序并去重后的相对路径列表。
///
/// 错误处理：
/// `walkdir` 单个条目的读取错误会被跳过；函数签名保留 `std::io::Error`，与同模块
/// 其他文件 API 保持一致。
pub fn files(input: FilesInput) -> Result<Vec<String>, std::io::Error> {
    let include_hidden = input.hidden.unwrap_or(true);
    let follow = input.follow.unwrap_or(false);

    let globs = input
        .glob
        .unwrap_or_default()
        .into_iter()
        .filter_map(|g| glob::Pattern::new(&g).ok())
        .collect::<Vec<_>>();

    let mut out = Vec::new();
    let mut walker = WalkDir::new(&input.cwd).follow_links(follow);
    if let Some(depth) = input.max_depth {
        walker = walker.max_depth(depth);
    }

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() {
            let name = entry.file_name().to_string_lossy();
            if name == ".git" {
                continue;
            }
        }

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let rel = path.strip_prefix(&input.cwd).unwrap_or(path);
        let rel = rel.to_string_lossy().to_string().replace('\\', "/");

        if !include_hidden && is_hidden_path(&rel) {
            continue;
        }

        if rel.starts_with(".git/") {
            continue;
        }

        if ignore::matches(&rel, None, None) {
            continue;
        }

        if !globs.is_empty() && !match_globs(&globs, &rel) {
            continue;
        }

        out.push(rel);
    }

    out.sort();
    out.dedup();
    Ok(out)
}

/// 构建目录树摘要。
///
/// 参数：
/// - `cwd`：遍历根目录。
/// - `limit`：最多输出的目录数量；为空时输出全部目录。
///
/// 返回值：
/// 返回以换行分隔的目录列表，超过限制时追加截断提示。
///
/// 错误处理：
/// 文件列表阶段的错误会向调用者返回。
pub fn tree(cwd: impl AsRef<Path>, limit: Option<usize>) -> Result<String, std::io::Error> {
    let cwd = cwd.as_ref().to_path_buf();
    let files = files(FilesInput {
        cwd: cwd.clone(),
        glob: None,
        hidden: Some(true),
        follow: Some(false),
        max_depth: None,
    })?;

    let mut dirs: BTreeMap<String, BTreeMap<String, ()>> = BTreeMap::new();
    for file in files {
        if file.contains(".vibewindow") {
            continue;
        }
        let parts = file.split('/').collect::<Vec<_>>();
        if parts.len() < 2 {
            continue;
        }
        let mut cur = String::new();
        let end = parts.len().saturating_sub(1);
        for part in parts.into_iter().take(end) {
            if !cur.is_empty() {
                cur.push('/');
            }
            cur.push_str(part);
            dirs.entry(cur.clone()).or_default();
        }
    }

    let total = dirs.len();
    let limit = limit.unwrap_or(total);

    let mut lines = Vec::new();
    let mut used = 0usize;
    for (p, _) in dirs.iter() {
        if used >= limit {
            break;
        }
        lines.push(p.clone());
        used += 1;
    }
    if total > used {
        lines.push(format!("[{} truncated]", total - used));
    }
    Ok(lines.join("\n"))
}

#[derive(Debug, Clone)]
/// 正则搜索输入。
pub struct SearchInput {
    /// 搜索根目录。
    pub cwd: PathBuf,
    /// Rust regex 语法的搜索模式。
    pub pattern: String,
    /// 可选文件包含 glob。
    pub glob: Option<Vec<String>>,
    /// 最大匹配数量；默认 100。
    pub limit: Option<usize>,
    /// 是否跟随符号链接。
    pub follow: Option<bool>,
}

#[derive(Debug, Clone)]
/// 单条正则匹配结果。
pub struct MatchData {
    /// 命中文件的相对路径。
    pub path: String,
    /// 1 基行号。
    pub line_number: usize,
    /// 命中所在完整行。
    pub line: String,
    /// 匹配在行内的起始字节偏移。
    pub start: usize,
    /// 匹配在行内的结束字节偏移。
    pub end: usize,
}

/// 在文件集合中执行正则搜索。
///
/// 参数：
/// - `input`：搜索根目录、模式、glob、限制和符号链接配置。
///
/// 返回值：
/// 返回按文件遍历顺序收集的匹配结果，最多 `limit` 条。
///
/// 错误处理：
/// 正则编译失败会转换为 `std::io::Error`；无法以 UTF-8 读取的文件会被跳过，避免
/// 二进制文件或权限问题中断整个搜索。
pub fn search(input: SearchInput) -> Result<Vec<MatchData>, std::io::Error> {
    let re = Regex::new(&input.pattern)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let files = files(FilesInput {
        cwd: input.cwd.clone(),
        glob: input.glob.clone(),
        hidden: Some(true),
        follow: input.follow,
        max_depth: None,
    })?;

    let mut out = Vec::new();
    let limit = input.limit.unwrap_or(100);
    for rel in files {
        if out.len() >= limit {
            break;
        }
        let full = input.cwd.join(&rel);
        let Ok(content) = std::fs::read_to_string(&full) else {
            continue;
        };
        for (i, line) in content.lines().enumerate() {
            if let Some(m) = re.find(line) {
                out.push(MatchData {
                    path: rel.clone(),
                    line_number: i + 1,
                    line: line.to_string(),
                    start: m.start(),
                    end: m.end(),
                });
                if out.len() >= limit {
                    break;
                }
            }
        }
    }
    Ok(out)
}
