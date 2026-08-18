//! 文件内容搜索工具
//!
//! 使用正则表达式在文件内容中搜索匹配项。当前支持：
//! - `content`
//! - `files_with_matches`
//! - `count`
//!
//! 结果按文件修改时间排序，自动跳过二进制文件。

use super::external_directory;
use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use crate::app::agent::file::ripgrep;
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use regex::RegexBuilder;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vw_api_types::tools::ToolResultContentDto;

const MAX_LINE_LENGTH: usize = 2000;
const LIMIT: usize = 100;
#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
enum OutputMode {
    Content,
    #[default]
    FilesWithMatches,
    Count,
}

#[derive(Debug, Clone, Deserialize)]
struct Args {
    pattern: String,
    path: Option<String>,
    #[serde(alias = "glob")]
    include: Option<String>,
    #[serde(default)]
    output_mode: OutputMode,
    #[serde(rename = "-n", default)]
    line_numbers: bool,
    #[serde(rename = "-i", default)]
    case_insensitive: bool,
    #[serde(default = "default_head_limit")]
    head_limit: usize,
    #[serde(default)]
    offset: usize,
    #[serde(rename = "-A", default)]
    after_context: usize,
    #[serde(rename = "-B", default)]
    before_context: usize,
    #[serde(rename = "-C", default)]
    context: usize,
    #[serde(rename = "type", default)]
    file_type: Option<String>,
    #[serde(default)]
    multiline: bool,
}

#[derive(Clone)]
struct MatchRow {
    path: String,
    mod_time_ms: i64,
    line_num: usize,
    line_text: String,
}

struct GrepExecution {
    output: String,
    data: Value,
    summary: String,
}

fn default_head_limit() -> usize {
    LIMIT
}

pub struct GrepTool {
    security: Arc<SecurityPolicy>,
}

impl GrepTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }

    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regular expression to search for in file contents"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search. Defaults to the current working directory."
                },
                "include": {
                    "type": "string",
                    "description": "File pattern to include, such as \"*.js\" or \"*.{ts,tsx}\""
                },
                "glob": {
                    "type": "string",
                    "description": "Compatibility alias for include."
                },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "Output mode. Defaults to files_with_matches."
                },
                "-n": {
                    "type": "boolean",
                    "description": "Show line numbers in content mode."
                },
                "-i": {
                    "type": "boolean",
                    "description": "Perform a case-insensitive search."
                },
                "head_limit": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Maximum number of entries to return. Defaults to 100."
                },
                "offset": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Skip the first N results before returning entries. Defaults to 0."
                },
                "-A": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Not currently supported."
                },
                "-B": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Not currently supported."
                },
                "-C": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Not currently supported."
                },
                "type": {
                    "type": "string",
                    "description": "File type filter; not currently supported."
                },
                "multiline": {
                    "type": "boolean",
                    "description": "Multiline matching switch; not currently supported."
                }
            },
            "required": ["pattern"]
        })
    }

    async fn resolve_search_path(&self, path: Option<&str>) -> anyhow::Result<PathBuf> {
        let Some(raw) = path.map(str::trim).filter(|v| !v.is_empty()) else {
            return Ok(self.security.workspace_dir.clone());
        };

        if !Path::new(raw).is_absolute() && !self.security.is_path_allowed(raw) {
            anyhow::bail!("Path not allowed by security policy: {raw}");
        }

        let requested = if Path::new(raw).is_absolute() {
            PathBuf::from(raw)
        } else {
            self.security.workspace_dir.join(raw)
        };

        if !requested.exists() {
            anyhow::bail!("Search path does not exist: {}", requested.display());
        }

        let resolved = std::fs::canonicalize(&requested)
            .map_err(|e| anyhow::anyhow!("Failed to resolve search path: {e}"))?;

        if !resolved.is_dir() {
            anyhow::bail!("Search path is not a directory: {}", resolved.display());
        }

        let resolved_string = resolved.to_string_lossy().to_string();
        external_directory::assert_external_directory(
            &self.security,
            Some(&resolved_string),
            Some(external_directory::Options {
                bypass: false,
                kind: external_directory::Kind::Directory,
            }),
        )
        .await
        .map_err(anyhow::Error::msg)?;

        Ok(resolved)
    }

    fn validate_capabilities(args: &Args) -> anyhow::Result<()> {
        if args.after_context > 0 || args.before_context > 0 || args.context > 0 {
            anyhow::bail!("grep 当前暂不支持上下文参数 -A/-B/-C");
        }
        if args.file_type.is_some() {
            anyhow::bail!("grep 当前暂不支持 type 文件类型过滤，请改用 include 或 glob");
        }
        if args.multiline {
            anyhow::bail!("grep 当前暂不支持 multiline 跨行匹配");
        }
        Ok(())
    }

    async fn execute_args(&self, args: Args) -> anyhow::Result<GrepExecution> {
        Self::validate_capabilities(&args)?;

        if args.pattern.trim().is_empty() {
            anyhow::bail!("Missing pattern");
        }

        if self.security.is_rate_limited() {
            anyhow::bail!("Rate limit exceeded: too many actions in the last hour");
        }

        let search_path = self.resolve_search_path(args.path.as_deref()).await?;

        if !self.security.record_action() {
            anyhow::bail!("Rate limit exceeded: action budget exhausted");
        }

        let re = RegexBuilder::new(&args.pattern)
            .case_insensitive(args.case_insensitive)
            .build()
            .map_err(|e| anyhow::anyhow!("正则表达式无效：{e}"))?;

        let mut include_globs = Vec::new();
        if let Some(include) = args.include.as_deref() {
            for pattern in expand_braces(include) {
                let glob_pattern =
                    glob::Pattern::new(&pattern).map_err(|e| anyhow::anyhow!("glob 无效：{e}"))?;
                include_globs.push(glob_pattern);
            }
        }

        let include_is_basename_only =
            args.include.as_deref().map(|p| !p.contains('/') && !p.contains('\\')).unwrap_or(false);

        let rel_files = ripgrep::files(ripgrep::FilesInput {
            cwd: search_path.clone(),
            glob: None,
            hidden: Some(true),
            follow: Some(false),
            max_depth: None,
        })
        .map_err(|e| anyhow::anyhow!("列出文件失败：{e}"))?;

        let mut mod_time_cache: HashMap<String, i64> = HashMap::new();
        let mut matches: Vec<MatchRow> = Vec::new();
        let mut has_errors = false;

        for rel in rel_files {
            if !include_globs.is_empty() {
                let hay = if include_is_basename_only {
                    Path::new(&rel).file_name().unwrap_or_default().to_string_lossy().to_string()
                } else {
                    rel.clone()
                };
                if !include_globs.iter().any(|g| g.matches(&hay)) {
                    continue;
                }
            }

            let full = search_path.join(&rel);
            if is_binary(&full) {
                continue;
            }

            let content = match std::fs::read_to_string(&full) {
                Ok(c) => c,
                Err(_) => {
                    has_errors = true;
                    continue;
                }
            };

            let abs_path = full.to_string_lossy().to_string().replace('\\', "/");
            let mod_time_ms = if let Some(ms) = mod_time_cache.get(&abs_path) {
                *ms
            } else {
                let ms = match full.metadata().and_then(|m| m.modified()) {
                    Ok(t) => {
                        t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
                            as i64
                    }
                    Err(_) => {
                        has_errors = true;
                        0
                    }
                };
                mod_time_cache.insert(abs_path.clone(), ms);
                ms
            };

            for (i, line) in content.lines().enumerate() {
                if !re.is_match(line) {
                    continue;
                }

                matches.push(MatchRow {
                    path: abs_path.clone(),
                    mod_time_ms,
                    line_num: i + 1,
                    line_text: line.to_string(),
                });
            }
        }

        if matches.is_empty() {
            return Ok(GrepExecution {
                output: "未找到匹配".to_string(),
                data: json!({
                    "success": true,
                    "output_mode": format_output_mode(args.output_mode),
                    "files": [],
                    "matches": [],
                    "counts": [],
                    "truncated": false,
                    "has_errors": has_errors,
                    "returned_count": 0,
                    "total_count": 0,
                }),
                summary: "未找到匹配".to_string(),
            });
        }

        matches.sort_by(|a, b| {
            b.mod_time_ms
                .cmp(&a.mod_time_ms)
                .then_with(|| a.path.cmp(&b.path))
                .then_with(|| a.line_num.cmp(&b.line_num))
        });

        let total_match_count = matches.len();
        let offset = args.offset.min(total_match_count);
        let head_limit = args.head_limit.max(1);

        let (output, data, summary) = match args.output_mode {
            OutputMode::FilesWithMatches => {
                let files = unique_paths(&matches);
                let sliced_files = files
                    .iter()
                    .skip(offset.min(files.len()))
                    .take(head_limit)
                    .cloned()
                    .collect::<Vec<_>>();
                let truncated = offset + sliced_files.len() < files.len();
                let mut output_lines = if sliced_files.is_empty() {
                    vec!["未找到匹配".to_string()]
                } else {
                    sliced_files.clone()
                };
                if truncated {
                    output_lines.push(String::new());
                    output_lines
                        .push("（结果已截断。请考虑使用更具体的 path 或 pattern。）".to_string());
                }
                if has_errors {
                    output_lines.push(String::new());
                    output_lines.push("（部分路径无法访问，已跳过）".to_string());
                }
                let summary = format!("找到 {} 个匹配文件", files.len());
                (
                    output_lines.join("\n"),
                    json!({
                        "success": true,
                        "output_mode": "files_with_matches",
                        "files": sliced_files,
                        "truncated": truncated,
                        "has_errors": has_errors,
                        "returned_count": sliced_files.len(),
                        "total_count": files.len(),
                    }),
                    summary,
                )
            }
            OutputMode::Count => {
                let counts = counts_by_path(&matches);
                let sliced_counts = counts
                    .iter()
                    .skip(offset.min(counts.len()))
                    .take(head_limit)
                    .map(|(path, count)| json!({ "path": path, "count": count }))
                    .collect::<Vec<_>>();
                let truncated = offset + sliced_counts.len() < counts.len();
                let mut output_lines = if sliced_counts.is_empty() {
                    vec!["未找到匹配".to_string()]
                } else {
                    sliced_counts
                        .iter()
                        .map(|entry| {
                            format!(
                                "{}: {}",
                                entry["path"].as_str().unwrap_or_default(),
                                entry["count"].as_u64().unwrap_or_default()
                            )
                        })
                        .collect::<Vec<_>>()
                };
                if truncated {
                    output_lines.push(String::new());
                    output_lines
                        .push("（结果已截断。请考虑使用更具体的 path 或 pattern。）".to_string());
                }
                if has_errors {
                    output_lines.push(String::new());
                    output_lines.push("（部分路径无法访问，已跳过）".to_string());
                }
                let summary =
                    format!("找到 {} 个匹配文件，共 {} 处匹配", counts.len(), total_match_count);
                (
                    output_lines.join("\n"),
                    json!({
                        "success": true,
                        "output_mode": "count",
                        "counts": sliced_counts,
                        "truncated": truncated,
                        "has_errors": has_errors,
                        "returned_count": sliced_counts.len(),
                        "total_count": counts.len(),
                        "match_count": total_match_count,
                    }),
                    summary,
                )
            }
            OutputMode::Content => {
                let sliced_matches =
                    matches.iter().skip(offset).take(head_limit).cloned().collect::<Vec<_>>();
                let truncated = offset + sliced_matches.len() < total_match_count;
                let mut output_lines = vec![format!("找到 {} 处匹配", total_match_count)];

                let mut current_file = String::new();
                for m in &sliced_matches {
                    if current_file != m.path {
                        if !current_file.is_empty() {
                            output_lines.push(String::new());
                        }
                        current_file = m.path.clone();
                        output_lines.push(format!("{}:", m.path));
                    }

                    let truncated_line_text = if m.line_text.len() > MAX_LINE_LENGTH {
                        format!("{}...", &m.line_text[..MAX_LINE_LENGTH])
                    } else {
                        m.line_text.clone()
                    };

                    if args.line_numbers {
                        output_lines.push(format!("  {}: {}", m.line_num, truncated_line_text));
                    } else {
                        output_lines.push(format!("  {}", truncated_line_text));
                    }
                }

                if truncated {
                    output_lines.push(String::new());
                    output_lines
                        .push("（结果已截断。请考虑使用更具体的 path 或 pattern。）".to_string());
                }
                if has_errors {
                    output_lines.push(String::new());
                    output_lines.push("（部分路径无法访问，已跳过）".to_string());
                }

                let result_matches = sliced_matches
                    .iter()
                    .map(|m| {
                        json!({
                            "path": m.path,
                            "line_number": m.line_num,
                            "line_text": m.line_text,
                        })
                    })
                    .collect::<Vec<_>>();
                let summary = format!("找到 {} 处匹配", total_match_count);
                (
                    output_lines.join("\n"),
                    json!({
                        "success": true,
                        "output_mode": "content",
                        "matches": result_matches,
                        "truncated": truncated,
                        "has_errors": has_errors,
                        "returned_count": sliced_matches.len(),
                        "total_count": total_match_count,
                    }),
                    summary,
                )
            }
        };

        Ok(GrepExecution { output, data, summary })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        include_str!("./grep.txt")
    }

    fn parameters_schema(&self) -> serde_json::Value {
        Self::schema()
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new("grep", self.description(), self.parameters_schema())
            .with_display_name("grep")
            .with_read_only(true)
            .with_destructive(false)
            .with_concurrency_safe(true)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let args: Args = serde_json::from_value(args)
            .map_err(|e| anyhow::anyhow!("Missing or invalid parameters: {e}"))?;

        match self.execute_args(args).await {
            Ok(exec) => Ok(ToolResult { success: true, output: exec.output, error: None }),
            Err(error) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error.to_string()),
            }),
        }
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let args: Args = serde_json::from_value(input)
            .map_err(|e| anyhow::anyhow!("Missing or invalid parameters: {e}"))?;

        match self.execute_args(args).await {
            Ok(exec) => Ok(ToolCallResult {
                data: exec.data.clone(),
                model_result: Value::String(exec.output.clone()),
                content_blocks: vec![
                    ToolResultContentDto::Text { text: exec.output.clone() },
                    ToolResultContentDto::Json { value: exec.data.clone() },
                ],
                render_hint: Some(ToolRenderHint {
                    title: Some("grep".to_string()),
                    kind: Some("grep".to_string()),
                    summary: Some(exec.summary),
                    metadata: json!({
                        "tool_id": "grep",
                        "output_mode": exec.data.get("output_mode").cloned().unwrap_or(Value::Null),
                        "truncated": exec.data.get("truncated").cloned().unwrap_or(Value::Bool(false)),
                    }),
                }),
                telemetry: Some(ToolCallTelemetry {
                    success: true,
                    ..ToolCallTelemetry::default()
                }),
                ..ToolCallResult::default()
            }),
            Err(error) => Ok(ToolCallResult {
                data: json!({
                    "success": false,
                    "error": error.to_string(),
                }),
                model_result: Value::String(error.to_string()),
                content_blocks: vec![ToolResultContentDto::Text { text: error.to_string() }],
                render_hint: Some(ToolRenderHint {
                    title: Some("grep".to_string()),
                    kind: Some("grep".to_string()),
                    summary: Some("搜索失败".to_string()),
                    metadata: json!({
                        "tool_id": "grep",
                    }),
                }),
                telemetry: Some(ToolCallTelemetry {
                    success: false,
                    ..ToolCallTelemetry::default()
                }),
                ..ToolCallResult::default()
            }),
        }
    }
}

fn format_output_mode(output_mode: OutputMode) -> &'static str {
    match output_mode {
        OutputMode::Content => "content",
        OutputMode::FilesWithMatches => "files_with_matches",
        OutputMode::Count => "count",
    }
}

fn unique_paths(matches: &[MatchRow]) -> Vec<String> {
    let mut out = Vec::new();
    for row in matches {
        if out.last() != Some(&row.path) {
            out.push(row.path.clone());
        }
    }
    out
}

fn counts_by_path(matches: &[MatchRow]) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for row in matches {
        if let Some((_, count)) = out.iter_mut().find(|(path, _)| path == &row.path) {
            *count += 1;
        } else {
            out.push((row.path.clone(), 1));
        }
    }
    out
}

fn expand_braces(pattern: &str) -> Vec<String> {
    let mut out = vec![pattern.to_string()];
    loop {
        let mut changed = false;
        let mut next = Vec::new();
        for p in out {
            let Some(open) = p.find('{') else {
                next.push(p);
                continue;
            };
            let Some(close_rel) = p[open + 1..].find('}') else {
                next.push(p);
                continue;
            };
            let close = open + 1 + close_rel;
            let prefix = &p[..open];
            let suffix = &p[close + 1..];
            let inner = &p[open + 1..close];
            let parts = inner.split(',').collect::<Vec<_>>();
            if parts.len() <= 1 {
                next.push(p);
                continue;
            }
            changed = true;
            for part in parts {
                let mut s = String::new();
                s.push_str(prefix);
                s.push_str(part);
                s.push_str(suffix);
                next.push(s);
            }
        }
        out = next;
        if !changed {
            break;
        }
    }
    out
}

fn is_binary(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "zip" | "tar" | "gz" | "exe" | "dll" | "so" | "class" | "jar" | "war" | "7z" | "doc"
        | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" | "bin" | "dat"
        | "obj" | "o" | "a" | "lib" | "wasm" | "pyc" | "pyo" => return true,
        _ => {}
    }

    use std::io::Read;

    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut bytes = [0u8; 4096];
    let Ok(n) = file.read(&mut bytes) else {
        return false;
    };
    if n == 0 {
        return false;
    }

    let buf = &bytes[..n];
    let mut bad = 0usize;
    for b in buf {
        if *b == 0 {
            return true;
        }
        if *b < 9 || (*b > 13 && *b < 32) {
            bad += 1;
        }
    }

    (bad as f32) / (buf.len() as f32) > 0.3
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
