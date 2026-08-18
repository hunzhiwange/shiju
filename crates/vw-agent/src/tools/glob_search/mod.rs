//! 文件模式搜索工具
//!
//! 在工作区内使用 glob 模式搜索文件路径。
//! 返回相对于工作区根目录的匹配文件路径列表。

use super::external_directory;
use super::traits::{Tool, ToolResult};
use crate::app::agent::file::ripgrep;
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 文件最大返回结果数量限制
///
/// 为避免性能问题和响应过大，限制单次搜索最多返回的文件数量。
/// 超过此数量的结果将被截断，并在输出中提示用户。
const MAX_RESULTS: usize = 1000;

#[derive(Debug, Clone, Deserialize)]
struct Args {
    pattern: String,
    path: Option<String>,
}

/// Glob 模式文件搜索工具
///
/// 在工作区内使用 glob 模式搜索文件路径，返回相对于工作区根目录的匹配文件路径列表。
/// 该工具实现了严格的安全策略，防止路径遍历攻击和符号链接逃逸。
///
/// # 安全特性
///
/// - 拒绝绝对路径（以 `/` 或 `\` 开头）
/// - 拒绝包含路径遍历（`..`）的模式
/// - 解析符号链接并验证目标路径仍位于工作区内
/// - 仅返回文件，不包括目录
/// - 受安全策略的速率限制约束
///
/// # 示例模式
///
/// - `**/*.rs` - 匹配所有 Rust 源文件
/// - `src/**/mod.rs` - 匹配 src 目录下所有的 mod.rs 文件
/// - `docs/**/*.md` - 匹配 docs 目录下所有的 Markdown 文件
pub struct GlobSearchTool {
    /// 安全策略引用，用于工作区边界验证和速率限制
    security: Arc<SecurityPolicy>,
}

impl GlobSearchTool {
    /// 创建新的 GlobSearchTool 实例
    ///
    /// # 参数
    ///
    /// - `security`: 安全策略的共享引用，用于工作区边界验证和速率限制检查
    ///
    /// # 返回值
    ///
    /// 返回配置了指定安全策略的 GlobSearchTool 实例
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }

    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern for matching files, such as '**/*.rs' or 'src/**/mod.rs'"
                },
                "path": {
                    "type": "string",
                    "description": "Optional directory from which to start the search. Defaults to the workspace root."
                }
            },
            "required": ["pattern"]
        })
    }

    async fn resolve_search_path(&self, path: Option<&str>) -> anyhow::Result<PathBuf> {
        let Some(raw) = path.map(str::trim).filter(|value| !value.is_empty()) else {
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
}

/// 为 GlobSearchTool 实现 Tool trait
///
/// 实现 Tool trait 定义的工具接口，使其能够被代理系统调用执行文件搜索任务。
/// 该实现支持 WASM 和原生两种运行时环境。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for GlobSearchTool {
    /// 获取工具名称
    ///
    /// # 返回值
    ///
    /// 返回工具的唯一标识符 "glob_search"
    fn name(&self) -> &str {
        "glob_search"
    }

    /// 获取工具描述
    ///
    /// # 返回值
    ///
    /// 返回工具的功能描述字符串，包括使用说明和示例
    fn description(&self) -> &str {
        "Search the workspace for files matching a glob pattern.\
         Returns a sorted list of matching paths relative to the workspace root.\
         Examples: '**/*.rs' (all Rust files) and 'src/**/mod.rs' (all mod.rs files under src)."
    }

    /// 获取工具参数的 JSON Schema
    ///
    /// # 返回值
    ///
    /// 返回描述工具输入参数结构的 JSON Schema 对象：
    /// - `pattern` (必需): 匹配文件的 glob 模式字符串
    /// - `path` (可选): 搜索起始目录
    fn parameters_schema(&self) -> serde_json::Value {
        Self::schema()
    }

    /// 执行文件搜索
    ///
    /// 根据提供的 glob 模式在工作区内搜索匹配的文件路径。
    /// 执行严格的安全验证，包括速率限制、路径遍历检查和符号链接解析。
    ///
    /// # 参数
    ///
    /// - `args`: 包含搜索参数的 JSON 对象，必须包含 `pattern` 字段
    ///
    /// # 返回值
    ///
    /// 返回 ToolResult 结构：
    /// - 成功时：`success` 为 true，`output` 包含匹配文件路径列表（每行一个）
    /// - 失败时：`success` 为 false，`error` 包含错误描述
    ///
    /// # 安全检查
    ///
    /// 1. 速率限制检查（快速路径）
    /// 2. 拒绝绝对路径
    /// 3. 拒绝路径遍历（`..`）
    /// 4. 解析符号链接并验证仍在工作区内
    ///
    /// # 错误情况
    ///
    /// - 缺少 `pattern` 参数
    /// - 速率限制超出
    /// - 使用绝对路径
    /// - 使用路径遍历
    /// - glob 模式语法无效
    /// - 工作区目录无法解析
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let args: Args = serde_json::from_value(args)
            .map_err(|e| anyhow::anyhow!("Missing or invalid parameters: {e}"))?;
        let pattern = args.pattern.trim();

        if pattern.is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Missing pattern".to_string()),
            });
        }

        // 速率限制检查（快速路径）
        if self.security.is_rate_limited() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
            });
        }

        // 安全检查：拒绝绝对路径
        if pattern.starts_with('/') || pattern.starts_with('\\') {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Absolute paths are not allowed. Use a relative glob pattern.".into()),
            });
        }

        // 安全检查：拒绝路径遍历
        if pattern.contains("../") || pattern.contains("..\\") || pattern == ".." {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Path traversal ('..') is not allowed in glob patterns.".into()),
            });
        }

        if let Err(error) = glob::Pattern::new(pattern) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Invalid glob pattern: {error}")),
            });
        }

        let search_path = match self.resolve_search_path(args.path.as_deref()).await {
            Ok(path) => path,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                });
            }
        };

        // 记录操作以消耗速率限制配额
        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
            });
        }

        let entries = match ripgrep::files(ripgrep::FilesInput {
            cwd: search_path.clone(),
            glob: Some(vec![pattern.to_string()]),
            hidden: Some(true),
            follow: Some(false),
            max_depth: None,
        }) {
            Ok(paths) => paths,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to glob files: {e}")),
                });
            }
        };

        let workspace_canon = match std::fs::canonicalize(&self.security.workspace_dir) {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Cannot resolve workspace directory: {e}")),
                });
            }
        };

        // 收集匹配结果
        let mut results = Vec::new();
        let mut truncated = false;

        for entry in entries {
            let path = search_path.join(&entry);
            // 规范化路径以解析符号链接，然后验证是否仍在工作区内
            let resolved = match std::fs::canonicalize(&path) {
                Ok(p) => p,
                Err(_) => continue, // 跳过损坏的符号链接或无法解析的路径
            };

            // 验证解析后的路径是否在允许范围内
            if !self.security.is_resolved_path_allowed(&resolved) {
                continue; // 静默过滤符号链接逃逸
            }

            // 仅包含文件，不包括目录
            if resolved.is_dir() {
                continue;
            }

            // 转换为相对于工作区的路径
            if let Ok(rel) = resolved.strip_prefix(&workspace_canon) {
                let rel = rel.to_string_lossy().to_string().replace('\\', "/");
                let mtime_ms = resolved
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .ok()
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|duration| duration.as_millis() as i64)
                    .unwrap_or(0);
                results.push((rel, mtime_ms));
            }

            // 检查是否达到结果数量上限
            if results.len() >= MAX_RESULTS {
                truncated = true;
                break;
            }
        }

        // 按修改时间降序排序，相同时按路径升序排序
        results.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        // 格式化输出结果
        let output = if results.is_empty() {
            // 无匹配结果时的提示信息
            format!("No files matching pattern '{pattern}' found in workspace.")
        } else {
            // 构建匹配文件列表
            use std::fmt::Write;
            let mut buf =
                results.iter().map(|(path, _)| path.as_str()).collect::<Vec<_>>().join("\n");

            // 如果结果被截断，添加截断提示
            if truncated {
                let _ = write!(
                    buf,
                    "\n\n[Results truncated: showing first {MAX_RESULTS} of more matches]"
                );
            }

            // 添加结果总数
            let _ = write!(buf, "\n\nTotal: {} files", results.len());
            buf
        };

        Ok(ToolResult { success: true, output, error: None })
    }
}

/// 测试模块
///
/// 测试代码位于 tests/glob_search.rs 文件中
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
