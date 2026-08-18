//! 目录列表工具
//!
//! 本模块提供了目录结构列表功能，用于浏览和分析文件系统层级。
//!
//! # 主要功能
//!
//! - 列出指定目录下的所有文件和子目录
//! - 支持通过 glob 模式过滤需要忽略的目录和文件
//! - 内置常见构建产物和缓存目录的忽略规则
//! - 支持安全策略校验，防止访问未授权路径
//!
//! # 默认忽略目录
//!
//! 工具默认忽略以下常见目录，以减少输出噪音：
//! - 构建产物：`target/`、`build/`、`dist/`、`bin/`、`obj/`
//! - 依赖目录：`node_modules/`、`vendor/`
//! - 缓存目录：`.cache/`、`cache/`、`.git/`、`__pycache__/`
//! - IDE 配置：`.idea/`、`.vscode/`
//! - 虚拟环境：`.venv/`、`venv/`、`env/`
//!
//! # 使用示例
//!
//! ```ignore
//! use std::sync::Arc;
//! use crate::app::agent::security::SecurityPolicy;
//! use crate::app::agent::tools::ls::LsTool;
//!
//! let security = Arc::new(SecurityPolicy::default());
//! let tool = LsTool::new(security);
//!
//! // 列出当前工作目录
//! let result = tool.execute(json!({"path": "."})).await?;
//! println!("{}", result.output);
//! ```

use super::external_directory;
use super::traits::{Tool, ToolResult};
use crate::app::agent::file::ripgrep;
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use glob::Pattern;
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 目录列表结果的最大文件数量限制
///
/// 防止在大型项目中返回过多结果，影响性能和可读性。
/// 当文件数量达到此限制时，将停止收集更多文件。
const LIMIT: usize = 100;

/// 默认忽略的目录模式列表
///
/// 这些 glob 模式用于过滤常见的构建产物、缓存目录、依赖目录等，
/// 以减少输出噪音并提升列表性能。斜杠后缀表示目录匹配。
///
/// # 包含的目录类型
///
/// - **构建产物**：`target/`、`build/`、`dist/`、`bin/`、`obj/`、`zig-out`
/// - **依赖目录**：`node_modules/`、`vendor/`
/// - **版本控制**：`.git/`
/// - **缓存**：`.cache/`、`cache/`、`__pycache__/`、`.zig-cache/`、`.coverage`、`coverage/`
/// - **临时文件**：`tmp/`、`temp/`、`logs/`
/// - **IDE 配置**：`.idea/`、`.vscode/`
/// - **Python 环境**：`.venv/`、`venv/`、`env/`
pub const IGNORE_PATTERNS: &[&str] = &[
    "node_modules/",
    "__pycache__/",
    ".git/",
    "dist/",
    "build/",
    "target/",
    "vendor/",
    "bin/",
    "obj/",
    ".idea/",
    ".vscode/",
    ".zig-cache/",
    "zig-out",
    ".coverage",
    "coverage/",
    "vendor/",
    "tmp/",
    "temp/",
    ".cache/",
    "cache/",
    "logs/",
    ".venv/",
    "venv/",
    "env/",
];

/// 目录列表工具的输入参数
///
/// 用于配置 `LsTool` 的执行行为，包括目标路径和忽略模式。
///
/// # 字段说明
///
/// - `path`：可选的目标目录路径。若未提供，默认使用安全策略中配置的工作目录。
///           支持相对路径（相对于工作区）和绝对路径。
/// - `ignore`：可选的自定义忽略模式列表。这些模式会追加到默认忽略列表之上。
///
/// # 示例
///
/// ```json
/// {
///     "path": "./src",
///     "ignore": ["*.log", "temp_*"]
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
struct Args {
    path: Option<String>,
    ignore: Option<Vec<String>>,
}

/// 目录列表工具
///
/// 实现 `Tool` trait，提供目录结构浏览功能。该工具会递归列出指定目录下的
/// 所有文件和子目录，并根据配置的忽略模式进行过滤。
///
/// # 安全特性
///
/// - 路径访问受安全策略控制，防止访问工作区外的敏感目录
/// - 支持速率限制，防止单位时间内执行过多操作
/// - 自动规范化路径，防止目录遍历攻击
///
/// # 输出格式
///
/// 输出为树形结构，使用缩进表示层级关系：
///
/// ```text
/// /path/to/dir/
///   src/
///     main.rs
///     lib.rs
///   Cargo.toml
/// ```
pub struct LsTool {
    /// 安全策略引用，用于路径校验和速率限制
    security: Arc<SecurityPolicy>,
}

impl LsTool {
    /// 创建新的目录列表工具实例
    ///
    /// # 参数
    ///
    /// - `security`：安全策略的共享引用，用于：
    ///   - 校验请求的路径是否在允许范围内
    ///   - 实施速率限制以防止滥用
    ///   - 提供工作区根目录信息
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use crate::app::agent::security::SecurityPolicy;
    /// use crate::app::agent::tools::ls::LsTool;
    ///
    /// let security = Arc::new(SecurityPolicy::default());
    /// let tool = LsTool::new(security);
    /// ```
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }

    /// 生成工具参数的 JSON Schema
    ///
    /// 返回符合 JSON Schema 规范的参数定义，用于：
    /// - 向 LLM 提供工具调用接口说明
    /// - 验证输入参数的格式和类型
    ///
    /// # 返回值
    ///
    /// 包含 `path` 和 `ignore` 两个可选参数的 schema 定义
    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list. Relative paths are resolved from the workspace; paths outside it must be allowed by the security policy."
                },
                "ignore": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of glob patterns to ignore"
                }
            }
        })
    }

    /// 解析并校验搜索路径
    ///
    /// 将用户提供的路径参数转换为绝对路径，并进行安全校验。
    ///
    /// # 参数
    ///
    /// - `path`：可选的路径字符串，可以是相对路径或绝对路径
    ///
    /// # 返回值
    ///
    /// - 成功：返回解析后的规范化的绝对路径
    /// - 失败：返回错误信息，包括：
    ///   - 路径不在安全策略允许范围内
    ///   - 路径不存在
    ///   - 路径不是目录
    ///   - 外部目录访问被拒绝
    ///
    /// # 处理逻辑
    ///
    /// 1. 若路径为空或未提供，返回工作区目录
    /// 2. 相对路径会相对于工作区解析
    /// 3. 检查路径是否被安全策略允许
    /// 4. 规范化路径并验证其存在性和类型
    /// 5. 对外部目录执行额外的访问控制检查
    async fn resolve_search_path(&self, path: Option<&str>) -> anyhow::Result<PathBuf> {
        // 处理空路径或未提供路径的情况，默认使用工作区目录
        let Some(raw) = path.map(str::trim).filter(|v| !v.is_empty()) else {
            return Ok(self.security.workspace_dir.clone());
        };

        // 校验相对路径是否在安全策略允许范围内
        // 绝对路径的校验在后续规范化后进行
        if !Path::new(raw).is_absolute() && !self.security.is_path_allowed(raw) {
            anyhow::bail!("Path not allowed by security policy: {raw}");
        }

        // 解析路径：绝对路径直接使用，相对路径基于工作区解析
        let requested = if Path::new(raw).is_absolute() {
            PathBuf::from(raw)
        } else {
            self.security.workspace_dir.join(raw)
        };

        // 验证路径是否存在
        if !requested.exists() {
            anyhow::bail!("Search path does not exist: {}", requested.display());
        }

        // 规范化路径，解析符号链接和相对路径组件
        let resolved = std::fs::canonicalize(&requested)
            .map_err(|e| anyhow::anyhow!("Failed to resolve search path: {e}"))?;

        // 确保路径是目录而非文件
        if !resolved.is_dir() {
            anyhow::bail!("Search path is not a directory: {}", resolved.display());
        }

        // 对外部目录执行访问控制检查
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

/// 递归渲染目录结构为树形文本
///
/// 将目录和文件信息格式化为缩进的树形结构字符串。
///
/// # 参数
///
/// - `dir_path`：当前要渲染的目录路径（相对路径）
/// - `depth`：当前递归深度，用于计算缩进级别
/// - `dirs`：所有目录路径的集合
/// - `files_by_dir`：目录路径到其包含文件列表的映射
///
/// # 返回值
///
/// 格式化后的树形结构字符串，使用两级空格缩进表示层级关系
///
/// # 示例输出
///
/// ```text
/// src/
///   components/
///     button.rs
///     input.rs
///   main.rs
/// ```
fn render_dir(
    dir_path: &str,
    depth: usize,
    dirs: &HashSet<String>,
    files_by_dir: &HashMap<String, Vec<String>>,
) -> String {
    // 计算当前层级的缩进（每级两个空格）
    let indent = "  ".repeat(depth);
    let mut out = String::new();

    // 根目录（depth == 0）不输出目录名，由调用者处理
    if depth > 0 {
        out.push_str(&format!(
            "{}{}/\n",
            indent,
            Path::new(dir_path).file_name().unwrap_or_default().to_string_lossy()
        ));
    }

    // 子项使用更深一级的缩进
    let child_indent = "  ".repeat(depth + 1);

    // 筛选出当前目录的直接子目录（排除自身和孙目录）
    let mut children: Vec<&String> = dirs
        .iter()
        .filter(|d| {
            let parent = Path::new(d.as_str()).parent().unwrap_or(Path::new("."));
            let parent = parent.to_string_lossy();
            // 空父路径视为当前目录 "."
            let parent = if parent.is_empty() { "." } else { parent.as_ref() };
            // 只保留直接子目录：父目录是当前目录且不是当前目录自身
            parent == dir_path && d.as_str() != dir_path
        })
        .collect();

    // 按字母顺序排序子目录以保证输出稳定性
    children.sort();

    // 递归渲染每个子目录
    for child in children {
        out.push_str(&render_dir(child, depth + 1, dirs, files_by_dir));
    }

    // 获取并排序当前目录下的文件
    let mut files = files_by_dir.get(dir_path).cloned().unwrap_or_default();
    files.sort();

    // 渲染文件列表
    for file in files {
        out.push_str(&format!("{}{}\n", child_indent, file));
    }

    out
}

/// 为 LsTool 实现 Tool trait
///
/// 提供目录列表功能的标准工具接口实现。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for LsTool {
    /// 返回工具名称标识
    ///
    /// 该名称用于工具注册和调用时的唯一标识
    fn name(&self) -> &str {
        "ls"
    }

    /// 返回工具的详细描述
    ///
    /// 描述内容从外部文件加载，便于多语言支持和独立维护
    fn description(&self) -> &str {
        include_str!("./ls.txt")
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// 用于参数验证和 LLM 工具调用接口定义
    fn parameters_schema(&self) -> serde_json::Value {
        Self::schema()
    }

    /// 执行目录列表操作
    ///
    /// # 参数
    ///
    /// - `args`：JSON 格式的输入参数，包含：
    ///   - `path`（可选）：目标目录路径
    ///   - `ignore`（可选）：自定义忽略模式列表
    ///
    /// # 返回值
    ///
    /// 返回 `ToolResult`，其中：
    /// - `success`：操作是否成功
    /// - `output`：树形格式的目录结构（成功时）
    /// - `error`：错误信息（失败时）
    ///
    /// # 执行流程
    ///
    /// 1. 解析并验证输入参数
    /// 2. 检查速率限制
    /// 3. 解析并校验目标路径
    /// 4. 记录操作以更新速率限制计数
    /// 5. 构建忽略模式列表（默认 + 自定义）
    /// 6. 使用 ripgrep 获取文件列表
    /// 7. 应用忽略模式过滤
    /// 8. 构建目录树结构并格式化输出
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 解析输入参数
        let args: Args = serde_json::from_value(args)
            .map_err(|e| anyhow::anyhow!("Missing or invalid parameters: {e}"))?;

        // 检查速率限制，防止短时间内执行过多操作
        if self.security.is_rate_limited() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
            });
        }

        // 解析并校验搜索路径
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

        // 记录本次操作以更新速率限制计数
        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
            });
        }

        // 构建排除模式列表
        // 首先添加默认的忽略模式
        let mut exclude_globs: Vec<Pattern> = Vec::new();
        for p in IGNORE_PATTERNS {
            // 为目录模式添加通配符以匹配目录下的所有内容
            let glob = format!("{}*", p);
            match Pattern::new(&glob) {
                Ok(pattern) => exclude_globs.push(pattern),
                Err(e) => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Invalid ignore pattern '{glob}': {e}")),
                    });
                }
            }
        }

        // 追加用户自定义的忽略模式
        if let Some(extra) = args.ignore {
            for p in extra {
                match Pattern::new(&p) {
                    Ok(pattern) => exclude_globs.push(pattern),
                    Err(e) => {
                        return Ok(ToolResult {
                            success: false,
                            output: String::new(),
                            error: Some(format!("Invalid ignore pattern '{p}': {e}")),
                        });
                    }
                }
            }
        }

        // 使用 ripgrep 高效获取目录中的所有文件
        let matches = match ripgrep::files(ripgrep::FilesInput {
            cwd: search_path.clone(),
            glob: None,
            hidden: None,
            follow: None,
            max_depth: None,
        }) {
            Ok(entries) => entries,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to list files: {e}")),
                });
            }
        };

        // 应用忽略模式过滤文件列表，并限制结果数量
        let mut files: Vec<String> = Vec::new();
        for rel in matches {
            // 跳过匹配任何忽略模式的文件
            if exclude_globs.iter().any(|g| g.matches(&rel)) {
                continue;
            }
            files.push(rel);
            // 达到限制后停止收集
            if files.len() >= LIMIT {
                break;
            }
        }

        // 构建目录层次结构数据
        let mut dirs: HashSet<String> = HashSet::new();
        let mut files_by_dir: HashMap<String, Vec<String>> = HashMap::new();

        // 遍历文件列表，提取目录层次和文件归属关系
        for file in &files {
            // 获取文件所在的目录路径
            let dir =
                Path::new(file).parent().unwrap_or(Path::new(".")).to_string_lossy().to_string();

            // 分解目录路径，确保所有父目录都被添加到集合中
            let parts: Vec<&str> = if dir == "." { vec![] } else { dir.split('/').collect() };

            // 逐级添加目录路径到集合，构建完整的目录树
            for i in 0..=parts.len() {
                let dir_path = if i == 0 { ".".to_string() } else { parts[0..i].join("/") };
                dirs.insert(dir_path);
            }

            // 将文件名添加到其所属目录的文件列表中
            files_by_dir.entry(dir).or_default().push(
                Path::new(file).file_name().unwrap_or_default().to_string_lossy().to_string(),
            );
        }

        // 生成格式化的树形输出
        Ok(ToolResult {
            success: true,
            output: format!(
                "{}/\n{}",
                search_path.display(),
                render_dir(".", 0, &dirs, &files_by_dir)
            ),
            error: None,
        })
    }
}

/// 单元测试模块
///
/// 包含目录列表工具的功能测试用例，验证：
/// - 路径解析和安全校验
/// - 忽略模式过滤
/// - 输出格式正确性
/// - 边界条件处理
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
