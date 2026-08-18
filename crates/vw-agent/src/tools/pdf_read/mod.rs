//! PDF 读取工具
//!
//! 从 PDF 文件中提取纯文本内容。需要启用 `rag-pdf` 功能标志编译。
//!
//! # 功能特性
//!
//! - 从工作区 PDF 文件中提取可读文本
//! - 支持自定义输出字符数限制
//! - 完整的安全检查（路径访问控制、速率限制、文件大小限制）
//! - 防止路径遍历攻击
//!
//! # 使用方法
//!
//! ```bash
//! # 编译时需要启用 rag-pdf 功能
//! cargo build --features rag-pdf
//! ```
//!
//! # 工具参数
//!
//! - `path`（必需）：PDF 文件路径，相对路径从工作区解析
//! - `max_chars`（可选）：返回的最大字符数，默认 50000，最大 200000
//!
//! # 安全限制
//!
//! - 文件大小上限：50 MB
//! - 输出字符数上限：200000
//! - 路径访问受安全策略白名单限制
//! - 受速率限制保护
//!
//! # 注意事项
//!
//! - 纯图像 PDF（需要 OCR）将返回空结果提示
//! - 加密的 PDF 无法提取文本
//! - 未启用 `rag-pdf` 功能时，工具会返回明确的错误提示

use super::traits::{Tool, ToolResult};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// PDF 文件最大字节数限制（50 MB）。
/// 超过此大小的 PDF 文件将被拒绝处理，防止内存耗尽或处理超时。
const MAX_PDF_BYTES: u64 = 50 * 1024 * 1024;

/// 返回给 LLM 的默认字符数限制。
/// 当调用者未指定 max_chars 参数时使用此值。
const DEFAULT_MAX_CHARS: usize = 50_000;

/// 输出字符数的硬性上限。
/// 无论调用者请求多少字符，都不会超过此限制，防止内存过度使用。
const MAX_OUTPUT_CHARS: usize = 200_000;

/// PDF 文件读取工具。
///
/// 从工作区中的 PDF 文件提取纯文本内容。该工具需要启用 `rag-pdf` 功能标志才能正常工作：
/// ```bash
/// cargo build --features rag-pdf
/// ```
///
/// 即使未启用该功能，工具仍会被注册，这样 LLM 会收到明确的错误提示，
/// 而不是遇到"工具不存在"的困惑。
///
/// # 安全特性
///
/// - 路径访问受安全策略限制
/// - 文件大小有上限限制
/// - 输出字符数有上限限制
/// - 支持速率限制以防止滥用
pub struct PdfReadTool {
    /// 安全策略引用，用于路径访问控制和速率限制检查
    security: Arc<SecurityPolicy>,
}

impl PdfReadTool {
    /// 创建新的 PDF 读取工具实例。
    ///
    /// # 参数
    ///
    /// * `security` - 安全策略的共享引用，用于控制文件访问权限和速率限制
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `PdfReadTool` 实例
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

/// 实现 Tool trait，使 PdfReadTool 可以被 Agent 调用。
///
/// 该实现支持 WASM 和原生两种目标架构，通过条件编译自动适配：
/// - WASM 目标：使用 `?Send` 标记，因为 WASM 是单线程环境
/// - 原生目标：使用标准 Send trait
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for PdfReadTool {
    /// 返回工具名称，用于在 Agent 系统中标识此工具。
    ///
    /// # 返回值
    ///
    /// 返回固定字符串 `"pdf_read"`
    fn name(&self) -> &str {
        "pdf_read"
    }

    /// 返回工具的功能描述，供 LLM 理解工具用途。
    ///
    /// # 返回值
    ///
    /// 返回描述此工具功能的中文说明字符串
    fn description(&self) -> &str {
        "Extract plain text from a PDF file in the workspace.\
         Returns all readable text. Image-only or encrypted PDFs return an empty result.\
         Requires the 'rag-pdf' build feature."
    }

    /// 返回工具参数的 JSON Schema 定义。
    ///
    /// 定义了两个参数：
    /// - `path`（必需）：PDF 文件路径
    /// - `max_chars`（可选）：返回的最大字符数
    ///
    /// # 返回值
    ///
    /// 返回符合 JSON Schema 规范的参数定义
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "PDF file path. Relative paths are resolved from the workspace; external paths require a policy allowlist entry."
                },
                "max_chars": {
                    "type": "integer",
                    "description": "Maximum number of characters to return (default: 50000; maximum: 200000)",
                    "minimum": 1,
                    "maximum": 200_000
                }
            },
            "required": ["path"]
        })
    }

    /// 执行 PDF 文本提取操作。
    ///
    /// # 参数
    ///
    /// * `args` - JSON 格式的参数对象，包含：
    ///   - `path`（必需）：PDF 文件的相对或绝对路径
    ///   - `max_chars`（可选）：返回文本的最大字符数，默认 50000，最大 200000
    ///
    /// # 返回值
    ///
    /// 返回 `anyhow::Result<ToolResult>`，其中：
    /// - `success: true` 且 `output` 包含提取的文本（成功）
    /// - `success: false` 且 `error` 包含错误信息（失败）
    ///
    /// # 安全检查流程
    ///
    /// 1. 速率限制检查 - 防止操作过于频繁
    /// 2. 路径白名单检查 - 验证路径是否被允许访问
    /// 3. 记录操作次数 - 消耗操作配额
    /// 4. 路径规范化 - 防止路径遍历攻击
    /// 5. 文件大小检查 - 限制处理大文件
    ///
    /// # 错误情况
    ///
    /// - 路径不在白名单中
    /// - 速率限制或操作配额耗尽
    /// - 文件不存在或无法读取
    /// - PDF 文件过大
    /// - PDF 提取失败（损坏的文件等）
    /// - 功能未启用（需要 `rag-pdf` 特性）
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 解析必需的 path 参数
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

        // 解析可选的 max_chars 参数，确保在有效范围内
        // 如果提供的值超出限制，将被截断到 MAX_OUTPUT_CHARS
        let max_chars = args
            .get("max_chars")
            .and_then(|v| v.as_u64())
            .map(|n| usize::try_from(n).unwrap_or(MAX_OUTPUT_CHARS).min(MAX_OUTPUT_CHARS))
            .unwrap_or(DEFAULT_MAX_CHARS);

        // 检查速率限制 - 如果短时间内操作过于频繁则拒绝
        if self.security.is_rate_limited() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
            });
        }

        // 检查路径是否在安全策略允许的范围内
        if !self.security.is_path_allowed(path) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Path not allowed by security policy: {path}")),
            });
        }

        // 在路径规范化之前记录操作，这样即使路径探测失败也会消耗操作配额
        // 这是防止攻击者通过尝试不同路径来绕过限制的安全措施
        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
            });
        }

        // 将相对路径转换为完整的工作区路径
        let full_path = self.security.workspace_dir.join(path);

        // 规范化路径（解析符号链接、. 和 .. 等）
        // 这可以防止路径遍历攻击
        let resolved_path = match tokio::fs::canonicalize(&full_path).await {
            Ok(p) => p,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to resolve file path: {e}")),
                });
            }
        };

        // 对规范化后的路径进行二次安全检查
        // 这可以防止通过符号链接等方式绕过路径限制
        if !self.security.is_resolved_path_allowed(&resolved_path) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(self.security.resolved_path_violation_message(&resolved_path)),
            });
        }

        // 记录调试信息，帮助追踪 PDF 读取操作
        tracing::debug!("Reading PDF: {}", resolved_path.display());

        // 获取文件元数据以检查文件大小
        match tokio::fs::metadata(&resolved_path).await {
            Ok(meta) => {
                // 检查文件大小是否超过限制
                // 大文件可能导致内存耗尽或处理超时
                if meta.len() > MAX_PDF_BYTES {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!(
                            "PDF too large: {} bytes (limit: {MAX_PDF_BYTES} bytes)",
                            meta.len()
                        )),
                    });
                }
            }
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to read file metadata: {e}")),
                });
            }
        }

        // 将整个 PDF 文件读入内存
        let bytes = match tokio::fs::read(&resolved_path).await {
            Ok(b) => b,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to read PDF file: {e}")),
                });
            }
        };

        // 当启用 rag-pdf 功能时的处理逻辑
        // pdf_extract 是一个 CPU 密集型阻塞操作，使用 spawn_blocking 将其移出异步执行器
        // 以避免阻塞整个异步运行时
        #[cfg(feature = "rag-pdf")]
        {
            // 在阻塞线程中执行 PDF 文本提取
            let text = match tokio::task::spawn_blocking(move || {
                pdf_extract::extract_text_from_mem(&bytes)
            })
            .await
            {
                Ok(Ok(t)) => t,
                Ok(Err(e)) => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("PDF extraction failed: {e}")),
                    });
                }
                Err(e) => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("PDF extraction task panicked: {e}")),
                    });
                }
            };

            // 处理提取结果为空的情况
            // 这通常意味着 PDF 是纯图像（需要 OCR）或被加密
            if text.trim().is_empty() {
                return Ok(ToolResult {
                    success: true,
                    // 当前 Agent 调度器只在 success=false 时转发 error 字段
                    // 因此将警告信息放在 output 中，同时标记为成功执行
                    output: "PDF contains no extractable text (may be image-only or encrypted)"
                        .into(),
                    error: None,
                });
            }

            // 如果文本超过 max_chars 限制，进行截断处理
            // 截断时会保留完整的字符（不会在多字节字符中间截断）
            let output = if text.chars().count() > max_chars {
                let mut truncated: String = text.chars().take(max_chars).collect();
                use std::fmt::Write as _;
                // 添加截断标记，告知用户文本已被截断
                let _ = write!(truncated, "\n\n... [truncated at {max_chars} chars]");
                truncated
            } else {
                text
            };

            return Ok(ToolResult { success: true, output, error: None });
        }

        // 当未启用 rag-pdf 功能时的处理逻辑
        // 返回明确的错误信息，指导用户如何启用该功能
        #[cfg(not(feature = "rag-pdf"))]
        {
            // 显式忽略未使用的变量，避免编译器警告
            let _ = bytes;
            let _ = max_chars;
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(
                    "PDF extraction is not enabled. \
                     Rebuild with: cargo build --features rag-pdf"
                        .into(),
                ),
            })
        }
    }
}

/// 单元测试模块
///
/// 测试代码位于 tests/pdf_read.rs 文件中，通过 path 属性引用。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
