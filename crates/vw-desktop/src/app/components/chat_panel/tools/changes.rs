//! 变更文件解析模块
//!
//! 该模块负责从文本输出中提取和解析代码变更信息。
//! 主要用于解析 AI 助手生成的带有 `<changes>` 标签的响应内容，
//! 将其中的文件变更信息提取为结构化的 `ChangeFile` 对象列表。
//!
//! # 核心功能
//!
//! - 从文本中提取 `<changes>...</changes>` 标签内的 JSON 内容
//! - 将 JSON 解析为 `ChangeFile` 结构体列表
//! - 支持新增行数、删除行数、修改前内容、修改后内容等字段的解析

use super::types::{ChangeFile, ChangeFileSummary};

/// 从文本中提取 `<changes>` 标签块的内容
///
/// 该函数在输入字符串中查找 `<changes>` 开始标签和 `</changes>` 结束标签，
/// 并提取两者之间的内容。
///
/// # 参数
///
/// * `s` - 待解析的输入字符串
///
/// # 返回值
///
/// - `Some(String)` - 成功提取到标签内容，返回去除首尾换行符的内容
/// - `None` - 以下情况返回 None：
///   - 未找到 `<changes>` 标签
///   - 未找到 `</changes>` 标签
///   - 结束标签位于开始标签之前（无效的标签顺序）
///
/// # 示例
///
/// ```ignore
/// let text = "前置内容<changes>\n{\"files\": []}\n</changes>后置内容";
/// let result = extract_changes_block(text);
/// assert_eq!(result, Some("{\"files\": []}".to_string()));
/// ```
fn extract_changes_block(s: &str) -> Option<String> {
    // 查找开始标签位置，并移动到标签内容起始处
    let start = s.find("<changes>")? + "<changes>".len();
    // 仅在开始标签之后查找结束标签，避免误匹配正文中的同名字符串
    let end = start + s[start..].find("</changes>")?;
    // 验证标签顺序的合法性
    if end <= start {
        return None;
    }
    // 提取内容并去除首尾换行符
    Some(s[start..end].trim_matches('\n').to_string())
}

/// 从输出文本中解析变更文件列表
///
/// 该函数首先从输入文本中提取 `<changes>` 标签块，然后将标签内的 JSON 内容
/// 解析为 `ChangeFile` 结构体列表。JSON 格式应为：
///
/// ```json
/// {
///   "files": [
///     {
///       "path": "src/main.rs",
///       "additions": 10,
///       "deletions": 5,
///       "before": "旧内容",
///       "after": "新内容"
///     }
///   ]
/// }
/// ```
///
/// # 参数
///
/// * `output` - 包含 `<changes>` 标签的输出文本
///
/// # 返回值
///
/// 返回解析成功的 `ChangeFile` 向量。在以下情况返回空向量：
/// - 未找到 `<changes>` 标签块
/// - JSON 解析失败
/// - JSON 中不存在 "files" 字段或该字段不是数组
///
/// # 字段解析规则
///
/// - `path`: 必需字段，文件路径（字符串）
/// - `additions`: 可选，新增行数（默认为 0）
/// - `deletions`: 可选，删除行数（默认为 0）
/// - `before`: 可选，修改前内容（默认为空字符串）
/// - `after`: 可选，修改后内容（默认为空字符串）
///
/// # 示例
///
/// ```ignore
/// let output = r#"
/// <changes>
/// {"files": [{"path": "test.rs", "additions": 5, "deletions": 2}]}
/// </changes>
/// "#;
/// let files = parse_changes_files(output);
/// assert_eq!(files.len(), 1);
/// assert_eq!(files[0].path, "test.rs");
/// ```
pub fn parse_changes_files(output: &str) -> Vec<ChangeFile> {
    // 步骤 1: 提取 <changes> 标签块
    let Some(block) = extract_changes_block(output) else {
        return Vec::new();
    };

    // 步骤 2: 解析 JSON 内容
    let Ok(v) = serde_json::from_str::<serde_json::Value>(block.trim()) else {
        return Vec::new();
    };

    // 步骤 3: 获取 "files" 数组字段
    let Some(files) = v.get("files").and_then(|x| x.as_array()) else {
        return Vec::new();
    };

    // 步骤 4: 遍历文件数组并构建 ChangeFile 对象
    let mut out = Vec::new();
    for f in files {
        // 提取文件路径（必需字段）
        let Some(path) = f.get("path").and_then(|x| x.as_str()).map(|s| s.to_string()) else {
            // 跳过缺少路径字段的条目
            continue;
        };

        // 提取可选字段，使用默认值
        let additions = f.get("additions").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
        let deletions = f.get("deletions").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
        let before = f.get("before").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let after = f.get("after").and_then(|x| x.as_str()).unwrap_or("").to_string();

        // 构建 ChangeFile 对象并添加到结果列表
        out.push(ChangeFile { path, additions, deletions, before, after });
    }

    out
}

pub fn parse_changes_file_summaries(output: &str) -> Vec<ChangeFileSummary> {
    let Some(block) = extract_changes_block(output) else {
        return Vec::new();
    };

    let Ok(v) = serde_json::from_str::<serde_json::Value>(block.trim()) else {
        return Vec::new();
    };

    let Some(files) = v.get("files").and_then(|x| x.as_array()) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for f in files {
        let Some(path) = f.get("path").and_then(|x| x.as_str()).map(|s| s.to_string()) else {
            continue;
        };

        let additions = f.get("additions").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
        let deletions = f.get("deletions").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
        let before = f.get("before").and_then(|x| x.as_str()).unwrap_or("");
        let after = f.get("after").and_then(|x| x.as_str()).unwrap_or("");
        let kind = if before.is_empty() && !after.is_empty() {
            'A'
        } else if !before.is_empty() && after.is_empty() {
            'D'
        } else {
            'M'
        };

        out.push(ChangeFileSummary { kind, path, additions, deletions });
    }

    out
}

#[cfg(test)]
#[path = "tests/changes.rs"]
mod tests;

#[cfg(test)]
#[path = "changes_tests.rs"]
mod coverage_tests;
