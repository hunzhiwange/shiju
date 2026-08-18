//! 文件读取工具
//!
//! 读取文件内容并返回 Claude Tools V2 风格的结构化结果。
//! 支持文本分页、图片预览、PDF 按页提取、notebook 单元预览，以及重复读取去重。

use super::context::current_read_state_for_path;
use super::external_directory;
use super::traits::{Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult};
use crate::app::agent::file::time;
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use image::{GenericImageView, ImageFormat};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 文件最大允许大小（字节）
const MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024;

/// 默认读取行数限制
const DEFAULT_LIMIT: usize = 2000;

/// 单行最大字符数，超过此长度的行将被截断
const MAX_LINE_LENGTH: usize = 2000;

/// 单次读取的最大字节数限制（50KB）
const MAX_BYTES: usize = 50 * 1024;

/// 允许内联到模型结果中的原始图片最大字节数。
const MAX_INLINE_IMAGE_BYTES: usize = 24 * 1024;

#[derive(Debug, Clone)]
struct Args {
    path: String,
    offset: Option<usize>,
    limit: Option<usize>,
}

fn parse_args(input: Value) -> anyhow::Result<Args> {
    let object = input
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid parameters: expected object"))?;

    for key in object.keys() {
        if !matches!(key.as_str(), "path" | "offset" | "limit") {
            anyhow::bail!("Missing or invalid parameters: unknown field `{key}`");
        }
    }

    let path = object
        .get("path")
        .ok_or_else(|| {
            anyhow::anyhow!("Missing or invalid parameters: missing required field `path`")
        })?
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing or invalid parameters: `path` must be a string"))?
        .to_string();
    let offset = parse_optional_usize(object, "offset")?;
    let limit = parse_optional_usize(object, "limit")?;

    Ok(Args { path, offset, limit })
}

fn parse_optional_usize(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> anyhow::Result<Option<usize>> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    let Some(raw) = value.as_u64() else {
        anyhow::bail!("Missing or invalid parameters: `{key}` must be a non-negative integer");
    };
    let parsed = usize::try_from(raw)
        .map_err(|_| anyhow::anyhow!("Missing or invalid parameters: `{key}` is too large"))?;
    Ok(Some(parsed))
}

#[derive(Debug, Clone, Serialize)]
struct FileDescriptor {
    path: String,
    absolute_path: String,
    open: String,
    size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
struct LineSnippet {
    line: usize,
    text: String,
}

#[derive(Debug, Clone, Serialize)]
struct PdfPageSnippet {
    page_number: usize,
    text: String,
}

#[derive(Debug, Clone, Serialize)]
struct NotebookCellSnippet {
    cell_number: usize,
    cell_type: String,
    language: Option<String>,
    source: String,
    output_mime_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum StructuredReadResult {
    Text {
        file: FileDescriptor,
        offset: usize,
        limit: usize,
        total_lines: usize,
        start_line: usize,
        end_line: usize,
        has_more: bool,
        truncated_by_bytes: bool,
        notice: String,
        lines: Vec<LineSnippet>,
    },
    Pdf {
        file: FileDescriptor,
        offset: usize,
        limit: usize,
        total_pages: usize,
        start_page: usize,
        end_page: usize,
        has_more: bool,
        truncated_by_bytes: bool,
        notice: String,
        pages: Vec<PdfPageSnippet>,
    },
    Image {
        file: FileDescriptor,
        format: String,
        mime_type: String,
        width: Option<u32>,
        height: Option<u32>,
        inline_data_url: Option<String>,
        inline_omitted: bool,
        notice: String,
    },
    Notebook {
        file: FileDescriptor,
        offset: usize,
        limit: usize,
        total_cells: usize,
        start_cell: usize,
        end_cell: usize,
        has_more: bool,
        truncated_by_bytes: bool,
        notice: String,
        cells: Vec<NotebookCellSnippet>,
    },
    FileUnchanged {
        file: FileDescriptor,
        result_kind: String,
        message: String,
        partial_view: bool,
        offset: Option<usize>,
        limit: Option<usize>,
    },
}

#[derive(Debug, Clone)]
struct ReadResponse {
    title: String,
    summary: String,
    metadata: Value,
    model_text: String,
    payload: StructuredReadResult,
}

#[derive(Debug, Clone)]
struct TextSlice {
    lines: Vec<LineSnippet>,
    total_lines: usize,
    start_line: usize,
    end_line: usize,
    has_more: bool,
    truncated_by_bytes: bool,
    notice: String,
}

#[derive(Debug, Clone)]
struct PdfSlice {
    pages: Vec<PdfPageSnippet>,
    total_pages: usize,
    start_page: usize,
    end_page: usize,
    has_more: bool,
    truncated_by_bytes: bool,
    notice: String,
}

#[derive(Debug, Clone)]
struct NotebookSlice {
    cells: Vec<NotebookCellSnippet>,
    total_cells: usize,
    start_cell: usize,
    end_cell: usize,
    has_more: bool,
    truncated_by_bytes: bool,
    notice: String,
}

#[derive(Debug, Clone)]
struct ImageReadInfo {
    format: String,
    mime_type: String,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, Clone)]
enum DetectedReadKind {
    Text,
    Pdf,
    Notebook,
    Image(ImageReadInfo),
}

#[derive(Debug, Clone, Deserialize)]
struct NotebookDocument {
    #[serde(default)]
    cells: Vec<NotebookCellRaw>,
}

#[derive(Debug, Clone, Deserialize)]
struct NotebookCellRaw {
    #[serde(default)]
    cell_type: String,
    #[serde(default)]
    metadata: Value,
    #[serde(default)]
    source: Value,
    #[serde(default)]
    outputs: Vec<Value>,
}

impl ReadResponse {
    fn into_tool_call_result(self) -> ToolCallResult {
        ToolCallResult {
            data: serde_json::to_value(&self.payload).unwrap_or(Value::Null),
            model_result: Value::String(self.model_text),
            render_hint: Some(ToolRenderHint {
                title: Some(self.title),
                kind: Some("file_read".to_string()),
                summary: Some(self.summary),
                metadata: self.metadata,
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        }
    }
}

/// 文件读取工具
pub struct FileReadTool {
    /// 安全策略引用，用于路径验证和速率限制
    security: Arc<SecurityPolicy>,

    /// 当前会话标识符，用于记录文件访问时间线
    session_id: Option<String>,
}

impl FileReadTool {
    /// 创建新的文件读取工具实例
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security, session_id: None }
    }

    pub fn with_session(security: Arc<SecurityPolicy>, session_id: impl Into<String>) -> Self {
        Self { security, session_id: Some(session_id.into()) }
    }

    fn normalize_slashes(s: String) -> String {
        s.replace('\\', "/")
    }

    fn display_path(&self, full: &Path) -> String {
        let workspace_root = self
            .security
            .workspace_dir
            .canonicalize()
            .unwrap_or_else(|_| self.security.workspace_dir.clone());
        if let Ok(rel) = full.strip_prefix(&workspace_root) {
            return Self::normalize_slashes(rel.to_string_lossy().to_string());
        }
        Self::normalize_slashes(full.to_string_lossy().to_string())
    }

    fn build_file_descriptor(&self, full: &Path, size_bytes: u64) -> FileDescriptor {
        FileDescriptor {
            path: self.display_path(full),
            absolute_path: Self::normalize_slashes(full.to_string_lossy().to_string()),
            open: format!("file:///{}", full.to_string_lossy()),
            size_bytes,
        }
    }

    fn file_link_block(&self, file: &FileDescriptor) -> String {
        format!(
            "<file_link>\npath: {}\nopen: {}\nsize_bytes: {}\n</file_link>",
            file.path, file.open, file.size_bytes
        )
    }

    fn resolve_full_path(&self, path: &str) -> PathBuf {
        if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.security
                .workspace_dir
                .canonicalize()
                .unwrap_or_else(|_| self.security.workspace_dir.clone())
                .join(path)
        }
    }

    fn truncate_line(line: &str) -> String {
        if line.chars().count() <= MAX_LINE_LENGTH {
            return line.to_string();
        }
        line.chars().take(MAX_LINE_LENGTH).collect()
    }

    fn truncate_to_bytes(text: &str, max_bytes: usize) -> String {
        if text.len() <= max_bytes {
            return text.to_string();
        }

        let mut output = String::new();
        let mut used = 0usize;
        for ch in text.chars() {
            let ch_bytes = ch.len_utf8();
            if used + ch_bytes > max_bytes {
                break;
            }
            output.push(ch);
            used += ch_bytes;
        }
        output
    }

    fn truncate_multiline_text(text: &str, max_bytes: usize) -> (String, bool) {
        if max_bytes == 0 {
            return (String::new(), !text.is_empty());
        }

        let mut rendered = String::new();
        let mut used = 0usize;
        let mut truncated = false;
        let lines: Vec<&str> =
            if text.is_empty() { Vec::new() } else { text.split('\n').collect() };

        for raw_line in lines {
            let line = Self::truncate_line(raw_line);
            let separator = if rendered.is_empty() { 0 } else { 1 };
            let line_bytes = line.len();

            if used + separator + line_bytes > max_bytes {
                let remaining = max_bytes.saturating_sub(used + separator);
                if remaining > 0 {
                    if !rendered.is_empty() {
                        rendered.push('\n');
                    }
                    rendered.push_str(&Self::truncate_to_bytes(&line, remaining));
                }
                truncated = true;
                break;
            }

            if !rendered.is_empty() {
                rendered.push('\n');
            }
            rendered.push_str(&line);
            used += separator + line_bytes;
        }

        (rendered, truncated)
    }

    fn read_kind_hint(path: &Path) -> &'static str {
        match path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()) {
            Some(ext) if ext == "pdf" => "pdf",
            Some(ext) if ext == "ipynb" => "notebook",
            Some(ext)
                if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp") =>
            {
                "image"
            }
            _ => "text",
        }
    }

    fn is_duplicate_request(&self, resolved_path: &Path, args: &Args) -> bool {
        current_read_state_for_path(resolved_path)
            .is_some_and(|entry| entry.offset == args.offset && entry.limit == args.limit)
    }

    fn missing_file_error(path: &Path) -> String {
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let base = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();

        let mut suggestions = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let lowered = name.to_lowercase();
                if !base.is_empty() && (lowered.contains(&base) || base.contains(&lowered)) {
                    suggestions.push(dir.join(name).display().to_string());
                    if suggestions.len() >= 3 {
                        break;
                    }
                }
            }
        }

        if suggestions.is_empty() {
            format!("File does not exist: {}", path.display())
        } else {
            format!(
                "File does not exist: {}\n\nDid you mean one of:\n{}",
                path.display(),
                suggestions.join("\n")
            )
        }
    }

    fn collect_text_slice(text: &str, offset: usize, limit: usize) -> TextSlice {
        let lines: Vec<&str> =
            if text.is_empty() { Vec::new() } else { text.split('\n').collect() };
        let total_lines = lines.len();
        let start = if offset == 0 { 0 } else { offset.saturating_sub(1) }.min(total_lines);

        let mut rendered_lines = Vec::new();
        let mut bytes = 0usize;
        let mut truncated_by_bytes = false;

        for (index, line) in lines.iter().enumerate().skip(start).take(limit) {
            let line = Self::truncate_line(line);
            let size = line.len() + if rendered_lines.is_empty() { 0 } else { 1 };
            if bytes + size > MAX_BYTES {
                truncated_by_bytes = true;
                break;
            }
            bytes += size;
            rendered_lines.push(LineSnippet { line: index + 1, text: line });
        }

        let start_line = rendered_lines.first().map(|line| line.line).unwrap_or(0);
        let end_line = rendered_lines.last().map(|line| line.line).unwrap_or(start);
        let has_more_lines = total_lines > end_line;

        let notice = if total_lines == 0 {
            "(End of file: 0 lines)".to_string()
        } else if start >= total_lines {
            format!("(No lines in range, file has {} lines)", total_lines)
        } else if truncated_by_bytes {
            format!(
                "(Output truncated at {} bytes. Use 'offset' to continue after line {})",
                MAX_BYTES, end_line
            )
        } else if has_more_lines {
            format!("(File has more lines. Use 'offset' to continue after line {})", end_line)
        } else {
            format!("(End of file: {} lines)", total_lines)
        };

        TextSlice {
            lines: rendered_lines,
            total_lines,
            start_line,
            end_line,
            has_more: has_more_lines,
            truncated_by_bytes,
            notice,
        }
    }

    fn collect_pdf_slice(pages: &[String], offset: usize, limit: usize) -> PdfSlice {
        let total_pages = pages.len();
        let start = if offset == 0 { 0 } else { offset.saturating_sub(1) }.min(total_pages);
        let mut rendered_pages = Vec::new();
        let mut bytes = 0usize;
        let mut truncated_by_bytes = false;

        for (index, page) in pages.iter().enumerate().skip(start).take(limit) {
            let header = format!("Page {}", index + 1);
            let separator = if rendered_pages.is_empty() { 0 } else { 2 };
            let available = MAX_BYTES.saturating_sub(bytes + separator + header.len() + 1);
            if available == 0 {
                truncated_by_bytes = true;
                break;
            }

            let (text, page_truncated) = Self::truncate_multiline_text(page, available);
            bytes += separator + header.len() + 1 + text.len();
            rendered_pages.push(PdfPageSnippet { page_number: index + 1, text });
            if page_truncated {
                truncated_by_bytes = true;
                break;
            }
        }

        let start_page = rendered_pages.first().map(|page| page.page_number).unwrap_or(0);
        let end_page = rendered_pages.last().map(|page| page.page_number).unwrap_or(start);
        let has_more = total_pages > end_page;

        let notice = if total_pages == 0 {
            "(PDF contains no extractable pages)".to_string()
        } else if start >= total_pages {
            format!("(No pages in range, PDF has {} pages)", total_pages)
        } else if truncated_by_bytes {
            format!(
                "(Output truncated at {} bytes. Use 'offset' to continue after page {})",
                MAX_BYTES, end_page
            )
        } else if has_more {
            format!("(PDF has more pages. Use 'offset' to continue after page {})", end_page)
        } else {
            format!("(End of PDF: {} pages)", total_pages)
        };

        PdfSlice {
            pages: rendered_pages,
            total_pages,
            start_page,
            end_page,
            has_more,
            truncated_by_bytes,
            notice,
        }
    }

    fn collect_notebook_slice(
        cells: &[NotebookCellRaw],
        offset: usize,
        limit: usize,
    ) -> NotebookSlice {
        let total_cells = cells.len();
        let start = if offset == 0 { 0 } else { offset.saturating_sub(1) }.min(total_cells);
        let mut rendered_cells = Vec::new();
        let mut bytes = 0usize;
        let mut truncated_by_bytes = false;

        for (index, cell) in cells.iter().enumerate().skip(start).take(limit) {
            let language = Self::notebook_cell_language(&cell.metadata);
            let mut label = format!("[Cell {}] {}", index + 1, cell.cell_type);
            if let Some(language) = language.as_deref() {
                label.push_str(&format!(" ({language})"));
            }

            let source = Self::notebook_source_to_string(&cell.source);
            let source = if source.is_empty() { "(empty cell)".to_string() } else { source };
            let separator = if rendered_cells.is_empty() { 0 } else { 2 };
            let available = MAX_BYTES.saturating_sub(bytes + separator + label.len() + 1);
            if available == 0 {
                truncated_by_bytes = true;
                break;
            }

            let (source, cell_truncated) = Self::truncate_multiline_text(&source, available);
            bytes += separator + label.len() + 1 + source.len();
            rendered_cells.push(NotebookCellSnippet {
                cell_number: index + 1,
                cell_type: cell.cell_type.clone(),
                language,
                source,
                output_mime_types: Self::notebook_output_mime_types(&cell.outputs),
            });
            if cell_truncated {
                truncated_by_bytes = true;
                break;
            }
        }

        let start_cell = rendered_cells.first().map(|cell| cell.cell_number).unwrap_or(0);
        let end_cell = rendered_cells.last().map(|cell| cell.cell_number).unwrap_or(start);
        let has_more = total_cells > end_cell;

        let notice = if total_cells == 0 {
            "(Notebook has 0 cells)".to_string()
        } else if start >= total_cells {
            format!("(No cells in range, notebook has {} cells)", total_cells)
        } else if truncated_by_bytes {
            format!(
                "(Output truncated at {} bytes. Use 'offset' to continue after cell {})",
                MAX_BYTES, end_cell
            )
        } else if has_more {
            format!("(Notebook has more cells. Use 'offset' to continue after cell {})", end_cell)
        } else {
            format!("(End of notebook: {} cells)", total_cells)
        };

        NotebookSlice {
            cells: rendered_cells,
            total_cells,
            start_cell,
            end_cell,
            has_more,
            truncated_by_bytes,
            notice,
        }
    }

    fn read_text_from_bytes(bytes: &[u8]) -> String {
        match String::from_utf8(bytes.to_vec()) {
            Ok(text) => text,
            Err(error) => String::from_utf8_lossy(&error.into_bytes()).into_owned(),
        }
    }

    fn notebook_source_to_string(source: &Value) -> String {
        match source {
            Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
            Value::String(text) => text.clone(),
            _ => String::new(),
        }
    }

    fn notebook_cell_language(metadata: &Value) -> Option<String> {
        metadata
            .get("language")
            .and_then(Value::as_str)
            .or_else(|| {
                metadata
                    .get("language_info")
                    .and_then(|value| value.get("name"))
                    .and_then(Value::as_str)
            })
            .or_else(|| {
                metadata
                    .get("vscode")
                    .and_then(|value| value.get("languageId"))
                    .and_then(Value::as_str)
            })
            .map(ToOwned::to_owned)
    }

    fn notebook_output_mime_types(outputs: &[Value]) -> Vec<String> {
        let mut mime_types = Vec::new();
        for output in outputs {
            if let Some(data) = output.get("data").and_then(Value::as_object) {
                for mime_type in data.keys() {
                    if !mime_types.iter().any(|existing| existing == mime_type) {
                        mime_types.push(mime_type.clone());
                    }
                }
            }
        }
        mime_types
    }

    fn is_notebook_path(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ipynb"))
    }

    fn is_pdf_bytes(bytes: &[u8]) -> bool {
        bytes.len() >= 5 && &bytes[..5] == b"%PDF-"
    }

    fn extract_pdf_pages(bytes: &[u8]) -> Vec<String> {
        let Some(text) = try_extract_pdf_text(bytes) else {
            return Vec::new();
        };

        let mut pages = text
            .split('\u{000C}')
            .map(|page| page.trim_matches('\n').to_string())
            .collect::<Vec<_>>();

        if pages.len() > 1 {
            pages.retain(|page| !page.trim().is_empty());
        }

        if pages.is_empty() { vec![text] } else { pages }
    }

    fn image_format_from_path(path: &Path) -> Option<ImageFormat> {
        match path.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()) {
            Some(ext) if ext == "png" => Some(ImageFormat::Png),
            Some(ext) if matches!(ext.as_str(), "jpg" | "jpeg") => Some(ImageFormat::Jpeg),
            Some(ext) if ext == "gif" => Some(ImageFormat::Gif),
            Some(ext) if ext == "webp" => Some(ImageFormat::WebP),
            Some(ext) if ext == "bmp" => Some(ImageFormat::Bmp),
            _ => None,
        }
    }

    fn image_format_label(format: ImageFormat) -> Option<(&'static str, &'static str)> {
        match format {
            ImageFormat::Png => Some(("png", "image/png")),
            ImageFormat::Jpeg => Some(("jpeg", "image/jpeg")),
            ImageFormat::Gif => Some(("gif", "image/gif")),
            ImageFormat::WebP => Some(("webp", "image/webp")),
            ImageFormat::Bmp => Some(("bmp", "image/bmp")),
            _ => None,
        }
    }

    fn image_dimensions_from_header(bytes: &[u8], format: &str) -> Option<(u32, u32)> {
        match format {
            "png" if bytes.len() >= 24 => Some((
                u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
                u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
            )),
            "gif" if bytes.len() >= 10 => Some((
                u32::from(u16::from_le_bytes([bytes[6], bytes[7]])),
                u32::from(u16::from_le_bytes([bytes[8], bytes[9]])),
            )),
            "bmp" if bytes.len() >= 26 => Some((
                u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]),
                i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]).unsigned_abs(),
            )),
            _ => None,
        }
    }

    fn inspect_image(bytes: &[u8], path: &Path) -> Option<ImageReadInfo> {
        let format =
            image::guess_format(bytes).ok().or_else(|| Self::image_format_from_path(path))?;
        let (format_name, mime_type) = Self::image_format_label(format)?;
        let dimensions = image::load_from_memory_with_format(bytes, format)
            .ok()
            .map(|image| image.dimensions())
            .or_else(|| Self::image_dimensions_from_header(bytes, format_name));

        Some(ImageReadInfo {
            format: format_name.to_string(),
            mime_type: mime_type.to_string(),
            width: dimensions.map(|(width, _)| width),
            height: dimensions.map(|(_, height)| height),
        })
    }

    fn detect_read_kind(path: &Path, bytes: &[u8]) -> DetectedReadKind {
        if Self::is_notebook_path(path) {
            return DetectedReadKind::Notebook;
        }

        if Self::is_pdf_bytes(bytes) {
            return DetectedReadKind::Pdf;
        }

        if let Some(info) = Self::inspect_image(bytes, path) {
            return DetectedReadKind::Image(info);
        }

        DetectedReadKind::Text
    }

    fn build_text_response(&self, file: FileDescriptor, args: &Args, text: String) -> ReadResponse {
        let offset = args.offset.unwrap_or(1);
        let limit = args.limit.unwrap_or(DEFAULT_LIMIT);
        let slice = Self::collect_text_slice(&text, offset, limit);

        let mut output = self.file_link_block(&file);
        output.push_str("\n<file>\n");
        let content = slice
            .lines
            .iter()
            .map(|line| format!("{:0>5}| {}", line.line, line.text))
            .collect::<Vec<_>>()
            .join("\n");
        output.push_str(&content);
        output.push_str("\n\n");
        output.push_str(&slice.notice);
        output.push_str("\n</file>");

        let title = file.path.clone();
        let summary = if slice.total_lines == 0 {
            "empty file".to_string()
        } else if slice.start_line == 0 {
            format!("0 of {} lines", slice.total_lines)
        } else {
            format!("lines {}-{} of {}", slice.start_line, slice.end_line, slice.total_lines)
        };
        let metadata = json!({
            "kind": "text",
            "path": file.path,
            "sizeBytes": file.size_bytes,
            "offset": offset,
            "limit": limit,
            "startLine": slice.start_line,
            "endLine": slice.end_line,
            "totalLines": slice.total_lines,
            "hasMore": slice.has_more,
            "truncatedByBytes": slice.truncated_by_bytes,
        });

        ReadResponse {
            title,
            summary,
            metadata,
            model_text: output,
            payload: StructuredReadResult::Text {
                file,
                offset,
                limit,
                total_lines: slice.total_lines,
                start_line: slice.start_line,
                end_line: slice.end_line,
                has_more: slice.has_more,
                truncated_by_bytes: slice.truncated_by_bytes,
                notice: slice.notice,
                lines: slice.lines,
            },
        }
    }

    fn build_pdf_response(&self, file: FileDescriptor, args: &Args, bytes: &[u8]) -> ReadResponse {
        let offset = args.offset.unwrap_or(1);
        let limit = args.limit.unwrap_or(DEFAULT_LIMIT);
        let pages = Self::extract_pdf_pages(bytes);
        let slice = Self::collect_pdf_slice(&pages, offset, limit);

        let mut output = self.file_link_block(&file);
        output.push_str("\n<pdf>\n");
        let content = slice
            .pages
            .iter()
            .map(|page| {
                if page.text.is_empty() {
                    format!("Page {}", page.page_number)
                } else {
                    format!("Page {}\n{}", page.page_number, page.text)
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        output.push_str(&content);
        output.push_str("\n\n");
        output.push_str(&slice.notice);
        output.push_str("\n</pdf>");

        let title = file.path.clone();
        let summary = if slice.total_pages == 0 {
            "pdf with no extractable pages".to_string()
        } else if slice.start_page == 0 {
            format!("0 of {} pages", slice.total_pages)
        } else {
            format!("pages {}-{} of {}", slice.start_page, slice.end_page, slice.total_pages)
        };
        let metadata = json!({
            "kind": "pdf",
            "path": file.path,
            "sizeBytes": file.size_bytes,
            "offset": offset,
            "limit": limit,
            "startPage": slice.start_page,
            "endPage": slice.end_page,
            "totalPages": slice.total_pages,
            "hasMore": slice.has_more,
            "truncatedByBytes": slice.truncated_by_bytes,
        });

        ReadResponse {
            title,
            summary,
            metadata,
            model_text: output,
            payload: StructuredReadResult::Pdf {
                file,
                offset,
                limit,
                total_pages: slice.total_pages,
                start_page: slice.start_page,
                end_page: slice.end_page,
                has_more: slice.has_more,
                truncated_by_bytes: slice.truncated_by_bytes,
                notice: slice.notice,
                pages: slice.pages,
            },
        }
    }

    fn build_image_response(
        &self,
        file: FileDescriptor,
        bytes: &[u8],
        info: ImageReadInfo,
    ) -> ReadResponse {
        let inline_data_url = if bytes.len() <= MAX_INLINE_IMAGE_BYTES {
            Some(format!("data:{};base64,{}", info.mime_type, BASE64_STANDARD.encode(bytes)))
        } else {
            None
        };
        let inline_omitted = inline_data_url.is_none();
        let notice = if inline_omitted {
            format!(
                "Inline image omitted because the payload exceeds {} bytes.",
                MAX_INLINE_IMAGE_BYTES
            )
        } else {
            "Inline image preview included.".to_string()
        };

        let mut output = self.file_link_block(&file);
        output.push_str("\n<image>\n");
        output.push_str(&format!("format: {}\nmime_type: {}", info.format, info.mime_type));
        if let (Some(width), Some(height)) = (info.width, info.height) {
            output.push_str(&format!("\ndimensions: {width}x{height}"));
        }
        output.push_str(&format!("\nsize_bytes: {}", file.size_bytes));
        if let Some(data_url) = inline_data_url.as_deref() {
            output.push_str(&format!("\ndata_url: {data_url}\n[IMAGE:{data_url}]"));
        } else {
            output.push_str("\n(inline preview omitted)");
        }
        output.push_str(&format!("\n\n{}", notice));
        output.push_str("\n</image>");

        let title = file.path.clone();
        let dimensions = match (info.width, info.height) {
            (Some(width), Some(height)) => format!(" {width}x{height}"),
            _ => String::new(),
        };
        let summary = format!("{}{}", info.format, dimensions).trim().to_string();
        let metadata = json!({
            "kind": "image",
            "path": file.path,
            "sizeBytes": file.size_bytes,
            "format": info.format,
            "mimeType": info.mime_type,
            "width": info.width,
            "height": info.height,
            "inlineOmitted": inline_omitted,
        });

        ReadResponse {
            title,
            summary,
            metadata,
            model_text: output,
            payload: StructuredReadResult::Image {
                file,
                format: info.format,
                mime_type: info.mime_type,
                width: info.width,
                height: info.height,
                inline_data_url,
                inline_omitted,
                notice,
            },
        }
    }

    fn build_notebook_response(
        &self,
        file: FileDescriptor,
        args: &Args,
        bytes: &[u8],
    ) -> anyhow::Result<ReadResponse> {
        let notebook: NotebookDocument = serde_json::from_slice(bytes)
            .map_err(|error| anyhow::anyhow!("Failed to parse notebook JSON: {error}"))?;
        let offset = args.offset.unwrap_or(1);
        let limit = args.limit.unwrap_or(DEFAULT_LIMIT);
        let slice = Self::collect_notebook_slice(&notebook.cells, offset, limit);

        let mut output = self.file_link_block(&file);
        output.push_str("\n<notebook>\n");
        let content = slice
            .cells
            .iter()
            .map(|cell| {
                let mut label = format!("[Cell {}] {}", cell.cell_number, cell.cell_type);
                if let Some(language) = cell.language.as_deref() {
                    label.push_str(&format!(" ({language})"));
                }
                if cell.source.is_empty() { label } else { format!("{label}\n{}", cell.source) }
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        output.push_str(&content);
        output.push_str("\n\n");
        output.push_str(&slice.notice);
        output.push_str("\n</notebook>");

        let title = file.path.clone();
        let summary = if slice.total_cells == 0 {
            "empty notebook".to_string()
        } else if slice.start_cell == 0 {
            format!("0 of {} cells", slice.total_cells)
        } else {
            format!("cells {}-{} of {}", slice.start_cell, slice.end_cell, slice.total_cells)
        };
        let metadata = json!({
            "kind": "notebook",
            "path": file.path,
            "sizeBytes": file.size_bytes,
            "offset": offset,
            "limit": limit,
            "startCell": slice.start_cell,
            "endCell": slice.end_cell,
            "totalCells": slice.total_cells,
            "hasMore": slice.has_more,
            "truncatedByBytes": slice.truncated_by_bytes,
        });

        Ok(ReadResponse {
            title,
            summary,
            metadata,
            model_text: output,
            payload: StructuredReadResult::Notebook {
                file,
                offset,
                limit,
                total_cells: slice.total_cells,
                start_cell: slice.start_cell,
                end_cell: slice.end_cell,
                has_more: slice.has_more,
                truncated_by_bytes: slice.truncated_by_bytes,
                notice: slice.notice,
                cells: slice.cells,
            },
        })
    }

    fn build_unchanged_response(
        &self,
        file: FileDescriptor,
        args: &Args,
        result_kind: &str,
        partial_view: bool,
    ) -> ReadResponse {
        let message =
            "Identical read request already exists in the current tool context. Reuse the previously returned content.".to_string();
        let mut output = self.file_link_block(&file);
        output.push_str("\n<file_unchanged>\n");
        output.push_str(&message);
        output.push_str(&format!("\nkind: {result_kind}"));
        if let Some(offset) = args.offset {
            output.push_str(&format!("\noffset: {offset}"));
        }
        if let Some(limit) = args.limit {
            output.push_str(&format!("\nlimit: {limit}"));
        }
        output.push_str("\n</file_unchanged>");

        let title = file.path.clone();
        let summary = "unchanged view reused".to_string();
        let metadata = json!({
            "kind": "file_unchanged",
            "path": file.path,
            "sizeBytes": file.size_bytes,
            "resultKind": result_kind,
            "offset": args.offset,
            "limit": args.limit,
        });

        ReadResponse {
            title,
            summary,
            metadata,
            model_text: output,
            payload: StructuredReadResult::FileUnchanged {
                file,
                result_kind: result_kind.to_string(),
                message,
                partial_view,
                offset: args.offset,
                limit: args.limit,
            },
        }
    }

    async fn execute_internal(
        &self,
        args: Args,
    ) -> anyhow::Result<Result<ReadResponse, ToolResult>> {
        let path = args.path.trim();

        if path.is_empty() {
            return Ok(Err(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Missing path".to_string()),
            }));
        }

        if !self.security.is_path_allowed(path) {
            return Ok(Err(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Path not allowed by security policy: {path}")),
            }));
        }

        if self.security.is_rate_limited() {
            return Ok(Err(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
            }));
        }

        let full_path = self.resolve_full_path(path);
        let full_str = full_path.to_string_lossy().to_string();
        if let Err(error) = external_directory::assert_external_directory(
            &self.security,
            Some(&full_str),
            Some(external_directory::Options {
                bypass: false,
                kind: external_directory::Kind::File,
            }),
        )
        .await
        {
            return Ok(Err(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error),
            }));
        }

        if !self.security.record_action() {
            return Ok(Err(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
            }));
        }

        let resolved_path = match tokio::fs::canonicalize(&full_path).await {
            Ok(path) => path,
            Err(_) => full_path.clone(),
        };

        if !resolved_path.exists() {
            return Ok(Err(ToolResult {
                success: false,
                output: String::new(),
                error: Some(Self::missing_file_error(&resolved_path)),
            }));
        }

        let resolved_str = resolved_path.to_string_lossy().to_string();
        if let Err(error) = external_directory::assert_external_directory(
            &self.security,
            Some(&resolved_str),
            Some(external_directory::Options {
                bypass: false,
                kind: external_directory::Kind::File,
            }),
        )
        .await
        {
            return Ok(Err(ToolResult {
                success: false,
                output: String::new(),
                error: Some(error),
            }));
        }

        let metadata = match tokio::fs::metadata(&resolved_path).await {
            Ok(meta) => meta,
            Err(error) => {
                return Ok(Err(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to read file metadata: {error}")),
                }));
            }
        };

        if !metadata.is_file() {
            return Ok(Err(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Path is not a file: {}", resolved_path.display())),
            }));
        }

        if metadata.len() > MAX_FILE_SIZE_BYTES {
            return Ok(Err(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "File too large: {} bytes (limit: {MAX_FILE_SIZE_BYTES} bytes)",
                    metadata.len()
                )),
            }));
        }

        if let Some(session_id) = &self.session_id {
            time::read(session_id, &resolved_str);
        }

        let file = self.build_file_descriptor(&resolved_path, metadata.len());
        if self.is_duplicate_request(&resolved_path, &args) {
            let partial_view =
                current_read_state_for_path(&resolved_path).is_some_and(|entry| entry.partial_view);
            return Ok(Ok(self.build_unchanged_response(
                file,
                &args,
                Self::read_kind_hint(&resolved_path),
                partial_view,
            )));
        }

        let bytes = tokio::fs::read(&resolved_path)
            .await
            .map_err(|error| anyhow::anyhow!("Failed to read file: {error}"))?;

        let response = match Self::detect_read_kind(&resolved_path, &bytes) {
            DetectedReadKind::Text => {
                self.build_text_response(file, &args, Self::read_text_from_bytes(&bytes))
            }
            DetectedReadKind::Pdf => self.build_pdf_response(file, &args, &bytes),
            DetectedReadKind::Notebook => self.build_notebook_response(file, &args, &bytes)?,
            DetectedReadKind::Image(info) => self.build_image_response(file, &bytes, info),
        };

        Ok(Ok(response))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "读取文件内容并返回结构化 Read 结果。文本按行分页，PDF 按页分页，notebook 按 cell 分页；图片会返回元数据与可选内联预览；重复请求会返回 file_unchanged。"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "文件路径。相对路径从工作区解析；外部路径需要策略白名单。"
                },
                "offset": {
                    "type": "integer",
                    "description": "起始位置（文本按行、PDF 按页、notebook 按 cell；从 1 开始，0 表示开头）"
                },
                "limit": {
                    "type": "integer",
                    "description": "返回的最大数量（文本按行、PDF 按页、notebook 按 cell；默认：2000）"
                }
            },
            "required": ["path"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let args = parse_args(args)?;
        match self.execute_internal(args).await? {
            Ok(response) => {
                Ok(ToolResult { success: true, output: response.model_text, error: None })
            }
            Err(result) => Ok(result),
        }
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let args = parse_args(input)?;

        match self.execute_internal(args).await? {
            Ok(response) => Ok(response.into_tool_call_result()),
            Err(result) => Ok(ToolCallResult::from_legacy_result(result)),
        }
    }
}

#[cfg(feature = "rag-pdf")]
fn try_extract_pdf_text(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 5 || &bytes[..5] != b"%PDF-" {
        return None;
    }
    let text = pdf_extract::extract_text_from_mem(bytes).ok()?;
    if text.trim().is_empty() {
        return None;
    }
    Some(text)
}

#[cfg(not(feature = "rag-pdf"))]
fn try_extract_pdf_text(_bytes: &[u8]) -> Option<String> {
    None
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
