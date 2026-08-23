//! MCP 调用完成后保留的、与宽度无关的展示内容。
//!
//! 线路结果可能包含数 MB 的图像、音频或资源正文；每个块只由 MCP 模型验证一次，
//! 随后只保留历史渲染真正需要的内容，避免长期历史持有大载荷。
//! Width-independent display content retained after an MCP call completes.
//!
//! The wire result can contain multi-megabyte image, audio, or resource bodies. Validate each
//! block once with the MCP model, then retain only what history rendering actually displays.

use crate::exec_cell::TOOL_CALL_MAX_LINES;
use crate::text_formatting::format_and_truncate_tool_result;
use base64::Engine;
use codex_protocol::mcp::CallToolResult;
use image::DynamicImage;
use image::ImageReader;
use rmcp::model::ContentBlock;
use rmcp::model::ResourceContents;
use serde::Deserialize;
use std::borrow::Cow;
use std::io::Cursor;
use tracing::error;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum McpResultKind {
    Standard,
    NodeRepl,
}

#[derive(Debug)]
pub(super) struct McpToolResult {
    pub(super) content: Vec<McpContentBlock>,
    pub(super) is_error: bool,
    pub(super) has_image: bool,
}

#[derive(Debug)]
pub(super) struct McpContentBlock {
    display: McpContentDisplay,
    /// 即使块格式错误或并非文本，代码模式也使用顶层 `text` 字段。
    /// Code mode uses a top-level `text` field even on malformed or non-text blocks.
    original_text: Option<String>,
}

#[derive(Debug)]
enum McpContentDisplay {
    Text(String),
    Summary(Cow<'static, str>),
    Json(String),
}

impl McpToolResult {
    /// 消费线路结果，丢弃仅由媒体或资源摘要代表的正文。
    ///
    /// 标准 MCP 反序列化保留完整文本及错误或未知块的精确 JSON 回退。
    /// 所有块都会投影为展示内容，但图像解码在首个完全有效的图像处停止。
    /// Consumes a wire result, dropping bodies represented only by media or resource summaries.
    ///
    /// Canonical MCP deserialization preserves full text and the exact JSON fallback for malformed
    /// or unknown blocks. Every block is projected, but image decoding stops at the first fully
    /// valid image.
    pub(super) fn new(result: CallToolResult, kind: McpResultKind) -> Self {
        let mut has_image = false;
        let content = result
            .content
            .into_iter()
            .map(|block| {
                // 按引用反序列化，使格式错误的块仍可用于精确 JSON 回退；成功块不再保留线路表示。
                // Deserialize by reference so malformed blocks remain available for the exact
                // JSON fallback. Successful blocks no longer retain their wire representation.
                let parsed = ContentBlock::deserialize(&block);
                let original_text = match (&parsed, kind) {
                    (Ok(ContentBlock::Text(_)), _) | (_, McpResultKind::Standard) => None,
                    (_, McpResultKind::NodeRepl) => block
                        .get("text")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned),
                };
                let display = match parsed {
                    Ok(ContentBlock::Text(text)) => McpContentDisplay::Text(text.text),
                    Ok(ContentBlock::Image(image)) => {
                        // 保留标记原有的完整解码有效性检查，并在首个有效图像后停止解码。
                        // Keep the marker's existing full-decode validity check, and stop
                        // decoding after the first valid image, just as the old search did.
                        if !has_image {
                            has_image = decode_mcp_image(&image.data).is_some();
                        }
                        McpContentDisplay::Summary("<image content>".into())
                    }
                    Ok(ContentBlock::Audio(_)) => {
                        McpContentDisplay::Summary("<audio content>".into())
                    }
                    Ok(ContentBlock::Resource(resource)) => {
                        let summary = match resource.resource {
                            ResourceContents::TextResourceContents { uri, .. }
                            | ResourceContents::BlobResourceContents { uri, .. } => {
                                format!("embedded resource: {uri}").into()
                            }
                            _ => "<unknown embedded resource>".into(),
                        };
                        McpContentDisplay::Summary(summary)
                    }
                    Ok(ContentBlock::ResourceLink(link)) => {
                        McpContentDisplay::Summary(format!("link: {}", link.uri).into())
                    }
                    Ok(_) | Err(_) => McpContentDisplay::Json(block.to_string()),
                };
                McpContentBlock {
                    display,
                    original_text,
                }
            })
            .collect();

        Self {
            content,
            is_error: result.is_error.unwrap_or(false),
            has_image,
        }
    }
}

impl McpContentBlock {
    /// 返回 node_repl 紧凑视图与转录视图使用的未截断顶层文本。
    ///
    /// 有效文本块复用展示存储；只有 node_repl 会为错误或非文本块保留额外字段，
    /// 且该字段优先于通常的展示内容。
    /// Returns the untruncated top-level text used by node_repl's compact and transcript views.
    ///
    /// Valid text blocks reuse their display storage. Only node_repl retains this extra field on
    /// malformed or non-text blocks, where it takes precedence over the usual display content.
    pub(super) fn text(&self) -> Option<&str> {
        match &self.display {
            McpContentDisplay::Text(text) => Some(text),
            McpContentDisplay::Summary(_) | McpContentDisplay::Json(_) => {
                self.original_text.as_deref()
            }
        }
    }

    /// 对完整文本或回退 JSON 应用随宽度变化的格式，而不重新解析 MCP 块。
    /// 媒体与资源摘要保持已有的未截断展示形式。
    /// Applies width-dependent formatting to full text or fallback JSON without reparsing the MCP
    /// block. Media and resource summaries retain their existing untruncated display form.
    pub(super) fn render(&self, width: usize) -> String {
        match &self.display {
            McpContentDisplay::Text(text) | McpContentDisplay::Json(text) => {
                format_and_truncate_tool_result(text, TOOL_CALL_MAX_LINES, width)
            }
            McpContentDisplay::Summary(summary) => summary.to_string(),
        }
    }
}

/// 完整解码 MCP 图像后才暴露独立的图像输出标记。
///
/// 仅检查头部会接受像素数据无法解码的图像；这里保留对无效 base64、未知格式、
/// 损坏图像和数据 URL 的既有拒绝行为。
/// Fully decodes an MCP image before exposing the separate image-output marker.
///
/// A header-only check would accept images whose decoder rejects their pixel data. Preserve the
/// existing behavior for invalid base64, unknown formats, corrupt images, and data URLs.
fn decode_mcp_image(data: &str) -> Option<DynamicImage> {
    let base64_data = if let Some(data_url) = data.strip_prefix("data:") {
        data_url.split_once(',')?.1
    } else {
        data
    };
    let raw_data = base64::engine::general_purpose::STANDARD
        .decode(base64_data)
        .map_err(|e| {
            error!("Failed to decode image data: {e}");
            e
        })
        .ok()?;
    let reader = ImageReader::new(Cursor::new(raw_data))
        .with_guessed_format()
        .map_err(|e| {
            error!("Failed to guess image format: {e}");
            e
        })
        .ok()?;

    reader
        .decode()
        .map_err(|e| {
            error!("Image decoding failed: {e}");
            e
        })
        .ok()
}
