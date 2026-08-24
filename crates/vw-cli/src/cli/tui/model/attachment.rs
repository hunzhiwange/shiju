//! 消息附件视图模型。

use std::path::{Path, PathBuf};

use vw_tui::{V3Attachment, V3AttachmentKind};

use crate::cli::tui::paste::{PromptAttachment, PromptAttachmentKind};

use super::UiMessageBase;

const MAX_ATTACHMENT_BYTES: u64 = 5 * 1024 * 1024;

/// 在请求边界内解析并读取附件；返回值不保留未校验路径。
pub(crate) fn prepare_prompt_attachments(
    workspace_root: Option<&Path>,
    attachments: &[V3Attachment],
) -> Result<Vec<PromptAttachment>, String> {
    attachments
        .iter()
        .enumerate()
        .map(|(id, attachment)| prepare_prompt_attachment(workspace_root, id, attachment))
        .collect()
}

fn prepare_prompt_attachment(
    workspace_root: Option<&Path>,
    id: usize,
    attachment: &V3Attachment,
) -> Result<PromptAttachment, String> {
    let label = attachment.label.trim();
    if label.is_empty() {
        return Err("attachment label must not be empty".to_string());
    }

    if let Some(bytes) = attachment.bytes.as_ref() {
        if attachment.kind != V3AttachmentKind::Image {
            return Err(format!("attachment '{label}' has unexpected binary content"));
        }
        if bytes.is_empty() {
            return Err(format!("image attachment '{label}' is empty"));
        }
        if attachment.path.is_some() {
            return Err(format!("image attachment '{label}' has both path and binary content"));
        }
        reject_oversize(label, bytes.len() as u64)?;
        let media_type = attachment.media_type.as_deref().unwrap_or("image/png");
        if !matches!(
            media_type,
            "image/png" | "image/jpeg" | "image/webp" | "image/gif" | "image/bmp"
        ) {
            return Err(format!("unsupported image attachment type: {media_type}"));
        }
        return Ok(PromptAttachment {
            id,
            kind: PromptAttachmentKind::Image,
            label: label.to_string(),
            media_type: Some(media_type.to_string()),
            uri: None,
            text: None,
            bytes: bytes.clone(),
        });
    }

    if attachment.kind == V3AttachmentKind::PastedText {
        let text = attachment.text.as_deref().unwrap_or_default();
        if text.is_empty() {
            return Err(format!("pasted text attachment '{label}' is empty"));
        }
        reject_oversize(label, text.len() as u64)?;
        return Ok(PromptAttachment {
            id,
            kind: PromptAttachmentKind::PastedText,
            label: label.to_string(),
            media_type: Some("text/plain".to_string()),
            uri: None,
            text: Some(text.to_string()),
            bytes: Vec::new(),
        });
    }

    let root = workspace_root
        .ok_or_else(|| "workspace root is required for local attachments".to_string())?
        .canonicalize()
        .map_err(|error| format!("workspace root is unavailable: {error}"))?;
    let raw_path = attachment
        .path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| format!("attachment '{label}' has no path"))?;
    let candidate = PathBuf::from(raw_path);
    let candidate = if candidate.is_absolute() { candidate } else { root.join(candidate) };
    let path = candidate
        .canonicalize()
        .map_err(|_| format!("attachment path does not exist: {raw_path}"))?;
    if !path.starts_with(&root) {
        return Err(format!("attachment path escapes workspace: {raw_path}"));
    }
    let metadata = path
        .metadata()
        .map_err(|_| format!("attachment path is unreadable: {raw_path}"))?;
    if !metadata.is_file() {
        return Err(format!("attachment path is not a file: {raw_path}"));
    }
    reject_oversize(label, metadata.len())?;

    let media_type = attachment
        .media_type
        .clone()
        .unwrap_or_else(|| media_type_for_path(&path).to_string());
    let uri = Some(file_uri(&path));
    match attachment.kind {
        V3AttachmentKind::Image => {
            if !matches!(media_type.as_str(), "image/png" | "image/jpeg" | "image/webp" | "image/gif" | "image/bmp") {
                return Err(format!("unsupported image attachment type: {media_type}"));
            }
            let bytes = std::fs::read(&path)
                .map_err(|_| format!("attachment path is unreadable: {raw_path}"))?;
            Ok(PromptAttachment {
                id,
                kind: PromptAttachmentKind::Image,
                label: label.to_string(),
                media_type: Some(media_type),
                uri,
                text: None,
                bytes,
            })
        }
        V3AttachmentKind::File | V3AttachmentKind::Resource => {
            if !media_type.starts_with("text/") && media_type != "application/json" {
                return Err(format!("unsupported text attachment type: {media_type}"));
            }
            let text = std::fs::read_to_string(&path)
                .map_err(|_| format!("attachment is not valid UTF-8 text: {raw_path}"))?;
            Ok(PromptAttachment {
                id,
                kind: if attachment.kind == V3AttachmentKind::Resource {
                    PromptAttachmentKind::Resource
                } else {
                    PromptAttachmentKind::File
                },
                label: label.to_string(),
                media_type: Some(media_type),
                uri,
                text: Some(text),
                bytes: Vec::new(),
            })
        }
        V3AttachmentKind::PastedText => unreachable!("pasted text returned above"),
    }
}

fn reject_oversize(label: &str, size: u64) -> Result<(), String> {
    if size > MAX_ATTACHMENT_BYTES {
        Err(format!(
            "attachment '{label}' exceeds the 5 MiB Gateway input limit"
        ))
    } else {
        Ok(())
    }
}

fn media_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()).unwrap_or_default().to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "md" | "markdown" => "text/markdown",
        "json" => "application/json",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "xml" => "text/xml",
        "txt" | "rs" | "toml" | "yaml" | "yml" | "js" | "ts" | "tsx" | "jsx" | "py" | "sh" | "sql" => "text/plain",
        _ => "application/octet-stream",
    }
}

fn file_uri(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let mut encoded = String::with_capacity(normalized.len());
    for byte in normalized.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~' | b':') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    format!("file://{encoded}")
}

/// 终端中可见的附件类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiAttachmentKind {
    Image,
    File,
    Link,
    PastedText,
    Resource,
}

/// 一条独立附件消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UiAttachment {
    pub(crate) base: UiMessageBase,
    pub(crate) kind: UiAttachmentKind,
    pub(crate) name: String,
    pub(crate) media_type: Option<String>,
    pub(crate) uri: String,
    pub(crate) alt_text: Option<String>,
    pub(crate) width_px: Option<u32>,
    pub(crate) height_px: Option<u32>,
    pub(crate) size_bytes: Option<u64>,
    pub(crate) source: Option<String>,
    pub(crate) preview: Option<String>,
}

impl UiAttachment {
    pub(crate) fn new(
        base: UiMessageBase,
        kind: UiAttachmentKind,
        name: impl Into<String>,
        uri: impl Into<String>,
    ) -> Self {
        Self {
            base,
            kind,
            name: name.into(),
            media_type: None,
            uri: uri.into(),
            alt_text: None,
            width_px: None,
            height_px: None,
            size_bytes: None,
            source: None,
            preview: None,
        }
    }

    pub(crate) fn with_media_type(mut self, media_type: impl Into<String>) -> Self {
        self.media_type = Some(media_type.into());
        self
    }

    pub(crate) fn with_dimensions(mut self, width_px: u32, height_px: u32) -> Self {
        self.width_px = Some(width_px);
        self.height_px = Some(height_px);
        self
    }

    pub(crate) fn with_size_bytes(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }

    pub(crate) fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub(crate) fn with_preview(mut self, preview: impl Into<String>) -> Self {
        self.preview = Some(preview.into());
        self
    }
}
