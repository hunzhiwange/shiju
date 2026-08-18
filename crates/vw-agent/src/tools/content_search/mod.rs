//! 内容搜索工具
//!
//! 在工作区内使用正则表达式搜索文件内容。
//! 优先使用 ripgrep (rg)，不可用时回退到 grep -rn -E。

use super::traits::{Tool, ToolResult};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::json;
use std::process::Stdio;
use std::sync::{Arc, OnceLock};

/// 最大返回结果数量
const MAX_RESULTS: usize = 1000;
/// 最大输出字节数限制 (1 MB)
const MAX_OUTPUT_BYTES: usize = 1_048_576;
/// 命令执行超时时间(秒)
const TIMEOUT_SECS: u64 = 30;

/// 内容搜索工具
///
/// 在工作区内按正则表达式模式搜索文件内容。
/// 优先使用 ripgrep (`rg`)，不可用时回退到 `grep -rn -E`。
/// 所有搜索都通过安全策略限制在工作区目录内。
///
/// # 功能特性
///
/// - 支持正则表达式搜索模式
/// - 支持多种输出格式 (content/files_with_matches/count)
/// - 支持 glob 模式过滤文件
/// - 支持大小写敏感/不敏感搜索
/// - 支持上下文行显示
/// - 支持多行匹配(仅 ripgrep)
/// - 自动路径安全检查
/// - 自动速率限制检查
pub struct ContentSearchTool {
    /// 安全策略引用
    security: Arc<SecurityPolicy>,
    /// 系统是否安装了 ripgrep
    has_rg: bool,
}

impl ContentSearchTool {
    /// 创建新的内容搜索工具实例
    ///
    /// # 参数
    ///
    /// - `security`: 安全策略的 Arc 引用
    ///
    /// # 返回值
    ///
    /// 返回新创建的 ContentSearchTool 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let security = Arc::new(SecurityPolicy::default());
    /// let tool = ContentSearchTool::new(security);
    /// ```
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let has_rg = which::which("rg").is_ok();
        #[cfg(target_arch = "wasm32")]
        let has_rg = false;

        Self { security, has_rg }
    }

    /// 创建指定后端的工具实例(仅测试用)
    ///
    /// # 参数
    ///
    /// - `security`: 安全策略的 Arc 引用
    /// - `has_rg`: 是否模拟有 ripgrep 可用
    ///
    /// # 返回值
    ///
    /// 返回新创建的 ContentSearchTool 实例
    #[cfg(test)]
    fn new_with_backend(security: Arc<SecurityPolicy>, has_rg: bool) -> Self {
        Self { security, has_rg }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ContentSearchTool {
    /// 获取工具名称
    ///
    /// # 返回值
    ///
    /// 返回工具标识符 "content_search"
    fn name(&self) -> &str {
        "content_search"
    }

    /// 获取工具描述
    ///
    /// # 返回值
    ///
    /// 返回工具的详细描述字符串,说明工具的功能、支持的参数和使用示例
    fn description(&self) -> &str {
        "Search file contents in the workspace using a regular expression.\
         Uses ripgrep (rg) with a grep fallback.\
         Output modes: 'content' (matching lines with context),\
         'files_with_matches' (file paths only), and 'count' (match count per file).\
         Example: pattern='fn main', include='*.rs', output_mode='content'."
    }

    /// 获取参数 JSON Schema
    ///
    /// # 返回值
    ///
    /// 返回描述工具参数的 JSON Schema 对象,包含所有参数的定义、类型、默认值和说明
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regular expression to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search, relative to the workspace root. Defaults to '.'.",
                    "default": "."
                },
                "output_mode": {
                    "type": "string",
                    "description": "Output format: 'content' (matching lines), 'files_with_matches' (paths only), or 'count' (match counts)",
                    "enum": ["content", "files_with_matches", "count"],
                    "default": "content"
                },
                "include": {
                    "type": "string",
                    "description": "File glob filter, such as '*.rs' or '*.{ts,tsx}'"
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Whether matching is case-sensitive. Defaults to true.",
                    "default": true
                },
                "context_before": {
                    "type": "integer",
                    "description": "Lines of context before each match (content mode only)",
                    "default": 0
                },
                "context_after": {
                    "type": "integer",
                    "description": "Lines of context after each match (content mode only)",
                    "default": 0
                },
                "multiline": {
                    "type": "boolean",
                    "description": "Enable multiline matching (ripgrep only, errors on grep fallback)",
                    "default": false
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return. Defaults to 1000",
                    "default": 1000
                }
            },
            "required": ["pattern"]
        })
    }

    /// 执行内容搜索
    ///
    /// # 参数
    ///
    /// - `args`: 包含搜索参数的 JSON 对象,支持以下字段:
    ///   - `pattern` (必需): 正则表达式搜索模式
    ///   - `path`: 搜索路径,默认为 "."
    ///   - `output_mode`: 输出模式,默认为 "content"
    ///   - `include`: 文件 glob 过滤模式
    ///   - `case_sensitive`: 是否区分大小写,默认为 true
    ///   - `context_before`: 匹配前显示的上下文行数
    ///   - `context_after`: 匹配后显示的上下文行数
    ///   - `multiline`: 是否启用多行匹配(仅 ripgrep)
    ///   - `max_results`: 最大返回结果数
    ///
    /// # 返回值
    ///
    /// 返回 ToolResult,包含:
    /// - `success`: 执行是否成功
    /// - `output`: 格式化的搜索结果
    /// - `error`: 错误信息(如果有)
    ///
    /// # 安全检查
    ///
    /// 1. 速率限制检查
    /// 2. 路径遍历防护
    /// 3. 工作区边界检查
    /// 4. 符号链接解析检查
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' parameter"))?;

        if pattern.is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Empty pattern is not allowed.".into()),
            });
        }

        let search_path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let output_mode = args.get("output_mode").and_then(|v| v.as_str()).unwrap_or("content");

        if !matches!(output_mode, "content" | "files_with_matches" | "count") {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Invalid output_mode '{output_mode}'. Allowed values: content, files_with_matches, count."
                )),
            });
        }

        let include = args.get("include").and_then(|v| v.as_str());

        let case_sensitive = args.get("case_sensitive").and_then(|v| v.as_bool()).unwrap_or(true);

        #[allow(clippy::cast_possible_truncation)]
        let context_before =
            args.get("context_before").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        #[allow(clippy::cast_possible_truncation)]
        let context_after =
            args.get("context_after").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let multiline = args.get("multiline").and_then(|v| v.as_bool()).unwrap_or(false);

        #[allow(clippy::cast_possible_truncation)]
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(MAX_RESULTS)
            .min(MAX_RESULTS);

        if self.security.is_rate_limited() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
            });
        }

        if std::path::Path::new(search_path).is_absolute() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Absolute paths are not allowed. Use a relative path.".into()),
            });
        }

        if search_path.contains("../") || search_path.contains("..\\") || search_path == ".." {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Path traversal ('..') is not allowed.".into()),
            });
        }

        if !self.security.is_path_allowed(search_path) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Path '{search_path}' is not allowed by security policy.")),
            });
        }

        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
            });
        }

        let workspace = &self.security.workspace_dir;
        let resolved_path = workspace.join(search_path);

        let resolved_canon = match std::fs::canonicalize(&resolved_path) {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Cannot resolve path '{search_path}': {e}")),
                });
            }
        };

        if !self.security.is_resolved_path_allowed(&resolved_canon) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Resolved path for '{search_path}' is outside the allowed workspace."
                )),
            });
        }

        if multiline && !self.has_rg {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(
                    "Multiline matching requires ripgrep (rg), which is not available.".into(),
                ),
            });
        }

        let mut cmd = if self.has_rg {
            build_rg_command(
                pattern,
                &resolved_canon,
                output_mode,
                include,
                case_sensitive,
                context_before,
                context_after,
                multiline,
            )
        } else {
            build_grep_command(
                pattern,
                &resolved_canon,
                output_mode,
                include,
                case_sensitive,
                context_before,
                context_after,
            )
        };

        for key in &["PATH", "HOME", "LANG", "LC_ALL", "LC_CTYPE"] {
            if let Ok(val) = std::env::var(key) {
                cmd.env(key, val);
            }
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = match tokio::time::timeout(
            std::time::Duration::from_secs(TIMEOUT_SECS),
            tokio::process::Command::from(cmd).output(),
        )
        .await
        {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to execute search command: {e}")),
                });
            }
            Err(_) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Search timed out after {TIMEOUT_SECS} seconds.")),
                });
            }
        };

        let exit_code = output.status.code().unwrap_or(-1);
        if exit_code >= 2 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Search error: {}", stderr.trim())),
            });
        }

        let raw_stdout = String::from_utf8_lossy(&output.stdout);

        let workspace_canon =
            std::fs::canonicalize(workspace).unwrap_or_else(|_| workspace.clone());

        let formatted = if self.has_rg {
            format_rg_output(&raw_stdout, &workspace_canon, output_mode, max_results)
        } else {
            format_grep_output(&raw_stdout, &workspace_canon, output_mode, max_results)
        };

        let final_output = if formatted.len() > MAX_OUTPUT_BYTES {
            let mut truncated = truncate_utf8(&formatted, MAX_OUTPUT_BYTES).to_string();
            truncated.push_str("\n\n[Output truncated: exceeded 1 MB limit]");
            truncated
        } else {
            formatted
        };

        Ok(ToolResult { success: true, output: final_output, error: None })
    }
}

/// 构建 ripgrep (rg) 搜索命令
///
/// # 参数
///
/// - `pattern`: 正则表达式搜索模式
/// - `search_path`: 要搜索的路径
/// - `output_mode`: 输出模式 (content/files_with_matches/count)
/// - `include`: 文件 glob 过滤模式(可选)
/// - `case_sensitive`: 是否区分大小写
/// - `context_before`: 匹配前显示的上下文行数
/// - `context_after`: 匹配后显示的上下文行数
/// - `multiline`: 是否启用多行匹配
///
/// # 返回值
///
/// 返回配置好的 Command 对象
fn build_rg_command(
    pattern: &str,
    search_path: &std::path::Path,
    output_mode: &str,
    include: Option<&str>,
    case_sensitive: bool,
    context_before: usize,
    context_after: usize,
    multiline: bool,
) -> std::process::Command {
    let mut cmd = std_command("rg");

    cmd.arg("--no-heading");
    cmd.arg("--line-number");
    cmd.arg("--with-filename");

    match output_mode {
        "files_with_matches" => {
            cmd.arg("--files-with-matches");
        }
        "count" => {
            cmd.arg("--count");
        }
        _ => {
            if context_before > 0 {
                cmd.arg("-B").arg(context_before.to_string());
            }
            if context_after > 0 {
                cmd.arg("-A").arg(context_after.to_string());
            }
        }
    }

    if !case_sensitive {
        cmd.arg("-i");
    }

    if multiline {
        cmd.arg("-U");
        cmd.arg("--multiline-dotall");
    }

    if let Some(glob) = include {
        cmd.arg("--glob").arg(glob);
    }

    cmd.arg("--");
    cmd.arg(pattern);
    cmd.arg(search_path);

    cmd
}

/// 构建 grep 搜索命令
///
/// # 参数
///
/// - `pattern`: 正则表达式搜索模式
/// - `search_path`: 要搜索的路径
/// - `output_mode`: 输出模式 (content/files_with_matches/count)
/// - `include`: 文件 glob 过滤模式(可选)
/// - `case_sensitive`: 是否区分大小写
/// - `context_before`: 匹配前显示的上下文行数
/// - `context_after`: 匹配后显示的上下文行数
///
/// # 返回值
///
/// 返回配置好的 Command 对象
fn build_grep_command(
    pattern: &str,
    search_path: &std::path::Path,
    output_mode: &str,
    include: Option<&str>,
    case_sensitive: bool,
    context_before: usize,
    context_after: usize,
) -> std::process::Command {
    let mut cmd = std_command("grep");

    cmd.arg("-r");
    cmd.arg("-n");
    cmd.arg("-E");
    cmd.arg("--binary-files=without-match");

    match output_mode {
        "files_with_matches" => {
            cmd.arg("-l");
        }
        "count" => {
            cmd.arg("-c");
        }
        _ => {
            if context_before > 0 {
                cmd.arg("-B").arg(context_before.to_string());
            }
            if context_after > 0 {
                cmd.arg("-A").arg(context_after.to_string());
            }
        }
    }

    if !case_sensitive {
        cmd.arg("-i");
    }

    if let Some(glob) = include {
        cmd.arg("--include").arg(glob);
    }

    cmd.arg("--");
    cmd.arg(pattern);
    cmd.arg(search_path);

    cmd
}

/// 格式化 ripgrep 输出结果
///
/// # 参数
///
/// - `raw`: ripgrep 原始输出字符串
/// - `workspace_canon`: 工作区规范路径
/// - `output_mode`: 输出模式 (content/files_with_matches/count)
/// - `max_results`: 最大返回结果数
///
/// # 返回值
///
/// 返回格式化后的输出字符串
fn format_rg_output(
    raw: &str,
    workspace_canon: &std::path::Path,
    output_mode: &str,
    max_results: usize,
) -> String {
    format_line_output(raw, workspace_canon, output_mode, max_results)
}

/// 格式化 grep 输出结果
///
/// # 参数
///
/// - `raw`: grep 原始输出字符串
/// - `workspace_canon`: 工作区规范路径
/// - `output_mode`: 输出模式 (content/files_with_matches/count)
/// - `max_results`: 最大返回结果数
///
/// # 返回值
///
/// 返回格式化后的输出字符串
fn format_grep_output(
    raw: &str,
    workspace_canon: &std::path::Path,
    output_mode: &str,
    max_results: usize,
) -> String {
    format_line_output(raw, workspace_canon, output_mode, max_results)
}

/// ripgrep 和 grep 的通用行格式化函数
///
/// 两个工具在我们的配置下产生相似的行输出:
/// - content 模式: `path:line:content` 或 `path-line-content` (上下文行)
/// - files_with_matches 模式: `path`
/// - count 模式: `path:count`
///
/// # 参数
///
/// - `raw`: 原始输出字符串
/// - `workspace_canon`: 工作区规范路径
/// - `output_mode`: 输出模式 (content/files_with_matches/count)
/// - `max_results`: 最大返回结果数
///
/// # 返回值
///
/// 返回格式化后的输出字符串,包含:
/// - 结果行(路径已相对化)
/// - 截断提示(如果结果被截断)
/// - 汇总统计(文件数、匹配数等)
pub(crate) fn format_line_output(
    raw: &str,
    workspace_canon: &std::path::Path,
    output_mode: &str,
    max_results: usize,
) -> String {
    if raw.trim().is_empty() {
        return "No matches found.".to_string();
    }

    let workspace_prefix = workspace_canon.to_string_lossy();

    let mut lines: Vec<String> = Vec::new();
    let mut truncated = false;
    let mut file_set = std::collections::HashSet::new();
    let mut total_matches: usize = 0;

    for line in raw.lines() {
        if line.is_empty() {
            continue;
        }

        let relativized = relativize_path(line, &workspace_prefix);

        match output_mode {
            "files_with_matches" => {
                let path = relativized.trim();
                if !path.is_empty() && file_set.insert(path.to_string()) {
                    lines.push(path.to_string());
                    if lines.len() >= max_results {
                        truncated = true;
                        break;
                    }
                }
            }
            "count" => {
                if let Some((path, count)) = parse_count_line(&relativized) {
                    if count > 0 {
                        file_set.insert(path.to_string());
                        total_matches += count;
                        lines.push(format!("{path}:{count}"));
                        if lines.len() >= max_results {
                            truncated = true;
                            break;
                        }
                    }
                }
            }
            _ => {
                if relativized == "--" {
                    lines.push(relativized);
                    if lines.len() >= max_results {
                        truncated = true;
                        break;
                    }
                    continue;
                }
                if let Some((path, is_match)) = parse_content_line(&relativized) {
                    file_set.insert(path.to_string());
                    if is_match {
                        total_matches += 1;
                    }
                } else {
                    total_matches += 1;
                }
                lines.push(relativized);
                if lines.len() >= max_results {
                    truncated = true;
                    break;
                }
            }
        }
    }

    if lines.is_empty() {
        return "No matches found.".to_string();
    }

    use std::fmt::Write;
    let mut buf = lines.join("\n");

    if truncated {
        let _ = write!(buf, "\n\n[Results truncated: showing first {max_results} results]");
    }

    match output_mode {
        "files_with_matches" => {
            let _ = write!(buf, "\n\nTotal: {} files", file_set.len());
        }
        "count" => {
            let _ = write!(buf, "\n\nTotal: {} matches in {} files", total_matches, file_set.len());
        }
        _ => {
            let _ = write!(
                buf,
                "\n\nTotal: {} matching lines in {} files",
                total_matches,
                file_set.len()
            );
        }
    }

    buf
}

/// 将绝对路径转换为相对路径
///
/// 从输出行中移除工作区前缀,将绝对路径转换为相对于工作区的相对路径
///
/// # 参数
///
/// - `line`: 包含路径的输出行
/// - `workspace_prefix`: 工作区路径前缀
///
/// # 返回值
///
/// 返回相对化后的路径字符串
pub(crate) fn relativize_path(line: &str, workspace_prefix: &str) -> String {
    if let Some(rest) = line.strip_prefix(workspace_prefix) {
        let trimmed = rest.strip_prefix('/').or_else(|| rest.strip_prefix('\\')).unwrap_or(rest);
        return trimmed.to_string();
    }
    line.to_string()
}

/// 解析内容输出行并判断是否为真正的匹配行
///
/// 支持的格式:
/// - 匹配行: `path:line:content`
/// - 上下文行: `path-line-content`
///
/// # 参数
///
/// - `line`: 要解析的输出行
///
/// # 返回值
///
/// 返回元组 (路径, 是否为匹配行),如果无法解析则返回 None
fn parse_content_line(line: &str) -> Option<(&str, bool)> {
    static MATCH_RE: OnceLock<regex::Regex> = OnceLock::new();
    static CONTEXT_RE: OnceLock<regex::Regex> = OnceLock::new();

    let match_re = MATCH_RE.get_or_init(|| {
        regex::Regex::new(r"^(?P<path>.+?):\d+:").expect("match line regex must be valid")
    });
    if let Some(caps) = match_re.captures(line) {
        return caps.name("path").map(|m| (m.as_str(), true));
    }

    let context_re = CONTEXT_RE.get_or_init(|| {
        regex::Regex::new(r"^(?P<path>.+?)-\d+-").expect("context line regex must be valid")
    });
    if let Some(caps) = context_re.captures(line) {
        return caps.name("path").map(|m| (m.as_str(), false));
    }

    None
}

/// 解析计数输出行
///
/// 解析格式为 `path:count` 的输出行
///
/// # 参数
///
/// - `line`: 要解析的输出行
///
/// # 返回值
///
/// 返回元组 (路径, 计数),如果无法解析则返回 None
pub(crate) fn parse_count_line(line: &str) -> Option<(&str, usize)> {
    static COUNT_RE: OnceLock<regex::Regex> = OnceLock::new();
    let count_re = COUNT_RE.get_or_init(|| {
        regex::Regex::new(r"^(?P<path>.+?):(?P<count>\d+)\s*$").expect("count line regex valid")
    });

    let caps = count_re.captures(line)?;
    let path = caps.name("path")?.as_str();
    let count = caps.name("count")?.as_str().parse::<usize>().ok()?;
    Some((path, count))
}

/// 安全截断 UTF-8 字符串
///
/// 在指定的最大字节数处截断字符串,确保不会在 UTF-8 字符中间截断
///
/// # 参数
///
/// - `input`: 要截断的输入字符串
/// - `max_bytes`: 最大字节数限制
///
/// # 返回值
///
/// 返回截断后的字符串切片
pub(crate) fn truncate_utf8(input: &str, max_bytes: usize) -> &str {
    if input.len() <= max_bytes {
        return input;
    }
    let mut end = max_bytes;
    while end > 0 && !input.is_char_boundary(end) {
        end -= 1;
    }
    &input[..end]
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
use crate::app::agent::shell::std_command;
