//! 文件模式匹配搜索工具
//!
//! 使用 glob 模式快速搜索匹配的文件路径。支持在指定目录下递归搜索，
//! 结果按修改时间排序并限制返回数量。

use super::external_directory;
use super::traits::{Tool, ToolResult};
use crate::app::agent::file::ripgrep;
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 最大返回结果数量
///
/// 为避免返回过多结果导致性能问题，限制单次搜索最多返回 100 个匹配项。
const MAX_RESULTS: usize = 100;

/// glob 工具的输入参数
///
/// # 字段说明
///
/// * `pattern` - glob 匹配模式（如 `**/*.rs`）
/// * `path` - 可选的搜索起始目录，未指定时使用工作空间根目录
#[derive(Debug, Clone, Deserialize)]
struct Args {
    pattern: String,
    #[serde(alias = "cwd", alias = "root")]
    path: Option<String>,
}

/// 文件 glob 模式搜索工具
///
/// 提供基于 glob 模式的文件搜索能力，支持递归搜索和结果排序。
/// 所有搜索操作均受安全策略约束，确保只能访问允许的目录。
///
/// # 安全性
///
/// - 所有路径访问均经过安全策略验证
/// - 支持速率限制以防止滥用
/// - 自动规范化路径以防止路径遍历攻击
///
/// # 示例
///
/// ```ignore
/// use std::sync::Arc;
/// use crate::app::agent::security::SecurityPolicy;
/// use crate::app::agent::tools::glob::GlobTool;
///
/// let security = Arc::new(SecurityPolicy::default());
/// let tool = GlobTool::new(security);
/// ```
pub struct GlobTool {
    /// 安全策略引用，用于路径访问控制和速率限制
    security: Arc<SecurityPolicy>,
}

impl GlobTool {
    /// 创建新的 GlobTool 实例
    ///
    /// # 参数
    ///
    /// * `security` - 安全策略的 Arc 引用，用于控制文件访问权限和速率限制
    ///
    /// # 返回值
    ///
    /// 返回配置好安全策略的 GlobTool 实例
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }

    /// 生成工具参数的 JSON Schema
    ///
    /// 定义工具接受的参数结构，包括参数类型、描述和必填性。
    ///
    /// # 返回值
    ///
    /// 返回符合 JSON Schema 规范的参数定义
    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern used to match files"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search. Omit this field to use the current working directory; do not pass \"undefined\" or \"null\". When provided, it must be a valid directory path."
                },
                "cwd": {
                    "type": "string",
                    "description": "Compatibility alias for path."
                },
                "root": {
                    "type": "string",
                    "description": "Compatibility alias for path."
                }
            },
            "required": ["pattern"]
        })
    }

    /// 解析并验证搜索路径
    ///
    /// 处理用户提供的路径参数，执行以下验证步骤：
    /// 1. 如果未提供路径，使用工作空间根目录
    /// 2. 验证相对路径是否符合安全策略
    /// 3. 将相对路径转换为绝对路径
    /// 4. 验证路径是否存在且为目录
    /// 5. 规范化路径（解析符号链接）
    /// 6. 检查外部目录访问权限
    ///
    /// # 参数
    ///
    /// * `path` - 可选的路径字符串，可以是绝对路径或相对路径
    ///
    /// # 返回值
    ///
    /// 成功时返回规范化后的绝对路径，失败时返回错误信息
    ///
    /// # 错误
    ///
    /// - 路径被安全策略拒绝
    /// - 路径不存在
    /// - 路径不是目录
    /// - 外部目录访问未授权
    async fn resolve_search_path(&self, path: Option<&str>) -> anyhow::Result<PathBuf> {
        // 如果未提供路径或路径为空，使用工作空间根目录
        let Some(raw) = path.map(str::trim).filter(|v| !v.is_empty()) else {
            return Ok(self.security.workspace_dir.clone());
        };

        // 对于相对路径，检查是否被安全策略允许
        if !Path::new(raw).is_absolute() && !self.security.is_path_allowed(raw) {
            anyhow::bail!("Path not allowed by security policy: {raw}");
        }

        // 将路径转换为绝对路径
        let requested = if Path::new(raw).is_absolute() {
            PathBuf::from(raw)
        } else {
            self.security.workspace_dir.join(raw)
        };

        // 验证路径是否存在
        if !requested.exists() {
            anyhow::bail!("Search path does not exist: {}", requested.display());
        }

        // 规范化路径（解析符号链接、移除 `.` 和 `..`）
        let resolved = std::fs::canonicalize(&requested)
            .map_err(|e| anyhow::anyhow!("Failed to resolve search path: {e}"))?;

        // 确保路径是目录而非文件
        if !resolved.is_dir() {
            anyhow::bail!("Search path is not a directory: {}", resolved.display());
        }

        // 检查外部目录访问权限
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

/// 实现 Tool trait，提供文件 glob 搜索能力
///
/// 该实现支持 WASM 和非 WASM 目标平台，通过条件编译自动选择合适的异步 trait 绑定。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for GlobTool {
    /// 返回工具名称
    ///
    /// # 返回值
    ///
    /// 返回固定字符串 "glob"，用于工具注册和调用识别
    fn name(&self) -> &str {
        "glob"
    }

    /// 返回工具描述
    ///
    /// 从外部文件加载工具的详细说明文本，便于维护和国际化。
    ///
    /// # 返回值
    ///
    /// 返回工具的使用说明文档
    fn description(&self) -> &str {
        include_str!("glob.txt")
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// # 返回值
    ///
    /// 返回参数结构的 JSON Schema 定义，供调用方验证参数格式
    fn parameters_schema(&self) -> serde_json::Value {
        Self::schema()
    }

    /// 执行 glob 文件搜索
    ///
    /// 完整的搜索流程包括：
    /// 1. 解析并验证输入参数
    /// 2. 检查速率限制状态
    /// 3. 解析并验证搜索路径
    /// 4. 记录操作到速率限制器
    /// 5. 使用 ripgrep 执行 glob 匹配
    /// 6. 收集文件元数据（修改时间）
    /// 7. 按修改时间降序排序
    /// 8. 限制结果数量并格式化输出
    ///
    /// # 参数
    ///
    /// * `args` - JSON 格式的参数对象，包含：
    ///   - `pattern`: glob 匹配模式（必填）
    ///   - `path`: 搜索目录（可选）
    ///
    /// # 返回值
    ///
    /// 返回 ToolResult，其中：
    /// - `success`: 操作是否成功
    /// - `output`: 匹配的文件路径列表（每行一个），按修改时间降序排列
    /// - `error`: 错误信息（如有）
    ///
    /// # 错误处理
    ///
    /// 以下情况返回失败结果：
    /// - 参数缺失或格式错误
    /// - 模式字符串为空
    /// - 超过速率限制
    /// - 路径解析失败或权限不足
    /// - ripgrep 执行失败
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 解析输入参数
        let args: Args = serde_json::from_value(args)
            .map_err(|e| anyhow::anyhow!("Missing or invalid parameters: {e}"))?;

        // 验证模式字符串非空
        if args.pattern.trim().is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Missing pattern".to_string()),
            });
        }

        // 检查是否超过速率限制（前置检查）
        if self.security.is_rate_limited() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
            });
        }

        // 解析并验证搜索路径
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

        // 记录本次操作到速率限制器
        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
            });
        }

        // 准备收集结果：文件路径和修改时间（毫秒时间戳）
        let mut files: Vec<(String, i64)> = Vec::new();
        let mut truncated = false;

        // 使用 ripgrep 执行 glob 匹配搜索
        // 配置：包含隐藏文件、不跟随符号链接、无深度限制
        let matches = match ripgrep::files(ripgrep::FilesInput {
            cwd: search_path.clone(),
            glob: Some(vec![args.pattern.clone()]),
            hidden: Some(true),
            follow: Some(false),
            max_depth: None,
        }) {
            Ok(entries) => entries,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to glob files: {e}")),
                });
            }
        };

        // 遍历匹配结果，收集文件路径和修改时间
        for rel in matches {
            // 达到最大结果数时停止收集
            if files.len() >= MAX_RESULTS {
                truncated = true;
                break;
            }

            // 构建完整文件路径
            let full = search_path.join(&rel);
            // 统一路径分隔符为正斜杠，确保跨平台一致性
            let full_str = full.to_string_lossy().to_string().replace('\\', "/");

            // 获取文件修改时间（Unix 时间戳，毫秒）
            let mtime_ms = full
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);

            files.push((full_str, mtime_ms));
        }

        // 按修改时间降序排序，相同时按路径升序排序
        files.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        // 处理空结果情况
        if files.is_empty() {
            return Ok(ToolResult {
                success: true, output: "未找到文件".to_string(), error: None
            });
        }

        // 提取排序后的文件路径
        let mut out: Vec<String> = files.into_iter().map(|(path, _)| path).collect();

        // 如果结果被截断，添加提示信息
        if truncated {
            out.push(String::new());
            out.push("（结果已截断。请考虑使用更具体的 path 或 pattern。）".to_string());
        }

        // 返回格式化结果（每行一个文件路径）
        Ok(ToolResult { success: true, output: out.join("\n"), error: None })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
