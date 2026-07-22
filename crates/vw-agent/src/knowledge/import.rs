use crate::app::agent::gateway::ApiError;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Map, Value, json};
use std::path::Path;
use vw_api_types::knowledge::{KnowledgeDocumentCreateRequest, KnowledgeDocumentImportRequest};

pub(crate) const MAX_IMPORT_FILE_BYTES: usize = 20 * 1024 * 1024;
pub(crate) const MAX_EXTRACTED_TEXT_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct ParsedDocumentImport {
    pub(crate) create: KnowledgeDocumentCreateRequest,
    pub(crate) filename: String,
    pub(crate) mime_type: Option<String>,
    pub(crate) source_bytes: u64,
    pub(crate) extracted_text_bytes: u64,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportKind {
    Text,
    Markdown,
    Pdf,
}

pub(crate) fn parse_document_import(
    body: KnowledgeDocumentImportRequest,
) -> Result<ParsedDocumentImport, ApiError> {
    let filename = clean_filename(&body.filename)?;
    let declared_mime = normalize_mime(body.mime_type.as_deref());
    let resolved_mime = declared_mime.clone().or_else(|| guess_mime_from_filename(&filename));
    let kind = detect_import_kind(&filename, resolved_mime.as_deref())?;
    reject_oversized_base64(&body.content_base64)?;

    let bytes = STANDARD.decode(body.content_base64.trim()).map_err(|error| {
        ApiError::bad_request(format!("knowledge import content_base64 is invalid: {error}"))
    })?;
    if bytes.len() > MAX_IMPORT_FILE_BYTES {
        return Err(ApiError::bad_request(format!(
            "knowledge import file is too large: max {} bytes",
            MAX_IMPORT_FILE_BYTES
        )));
    }

    let extracted = extract_text(kind, &bytes)?;
    let content = normalize_text(&extracted);
    if content.is_empty() {
        return Err(ApiError::bad_request(
            "knowledge import extracted text is empty; OCR-only PDFs and blank files are not supported",
        ));
    }
    if content.len() > MAX_EXTRACTED_TEXT_BYTES {
        return Err(ApiError::bad_request(format!(
            "knowledge import extracted text is too large: max {} bytes",
            MAX_EXTRACTED_TEXT_BYTES
        )));
    }

    let source_bytes = usize_to_u64(bytes.len());
    let extracted_text_bytes = usize_to_u64(content.len());
    let metadata = import_metadata(
        body.metadata,
        &filename,
        resolved_mime.as_deref(),
        kind,
        source_bytes,
        extracted_text_bytes,
    )?;

    Ok(ParsedDocumentImport {
        create: KnowledgeDocumentCreateRequest {
            name: filename.clone(),
            content,
            metadata,
            enabled: body.enabled,
        },
        filename,
        mime_type: resolved_mime,
        source_bytes,
        extracted_text_bytes,
        warnings: Vec::new(),
    })
}

fn reject_oversized_base64(content_base64: &str) -> Result<(), ApiError> {
    let encoded = content_base64.trim().len();
    let max_encoded = ((MAX_IMPORT_FILE_BYTES + 2) / 3) * 4;
    if encoded > max_encoded {
        return Err(ApiError::bad_request(format!(
            "knowledge import file is too large: max {} bytes",
            MAX_IMPORT_FILE_BYTES
        )));
    }
    Ok(())
}

fn clean_filename(filename: &str) -> Result<String, ApiError> {
    let normalized = filename.trim().replace('\\', "/");
    let name = normalized.rsplit('/').find(|part| !part.trim().is_empty()).unwrap_or("").trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("knowledge import filename is required"));
    }
    Ok(name.to_string())
}

fn normalize_mime(mime_type: Option<&str>) -> Option<String> {
    mime_type
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
}

fn guess_mime_from_filename(filename: &str) -> Option<String> {
    mime_guess::from_path(filename).first_raw().map(str::to_string)
}

fn detect_import_kind(filename: &str, mime_type: Option<&str>) -> Result<ImportKind, ApiError> {
    let extension = file_extension(filename);
    if let Some(mime) = mime_type {
        match mime {
            "text/plain" => return Ok(ImportKind::Text),
            "text/markdown" | "text/x-markdown" | "text/md" | "application/x-markdown" => {
                return Ok(ImportKind::Markdown);
            }
            "application/pdf" => return Ok(ImportKind::Pdf),
            _ => {}
        }
    }

    match extension.as_deref() {
        Some("txt") => Ok(ImportKind::Text),
        Some("md" | "markdown" | "mdx") => Ok(ImportKind::Markdown),
        Some("pdf") => Ok(ImportKind::Pdf),
        Some(ext) => Err(unsupported_file_error(Some(ext), mime_type)),
        None => Err(unsupported_file_error(None, mime_type)),
    }
}

fn file_extension(filename: &str) -> Option<String> {
    Path::new(filename)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.trim_start_matches('.').to_ascii_lowercase())
        .filter(|value| !value.is_empty())
}

fn unsupported_file_error(extension: Option<&str>, mime_type: Option<&str>) -> ApiError {
    let label = unsupported_label(extension, mime_type);
    ApiError::bad_request(format!(
        "unsupported knowledge import file type: {label}; only TXT, Markdown, and PDF files are supported"
    ))
}

fn unsupported_label(extension: Option<&str>, mime_type: Option<&str>) -> String {
    if let Some(ext) = extension.and_then(known_unsupported_extension_label) {
        return ext.to_string();
    }
    if let Some(mime) = mime_type {
        if let Some(label) = known_unsupported_mime_label(mime) {
            return label.to_string();
        }
        return format!("MIME type {mime}");
    }
    extension.map(|ext| format!(".{ext} files")).unwrap_or_else(|| "unknown file type".to_string())
}

fn known_unsupported_extension_label(extension: &str) -> Option<&'static str> {
    match extension {
        "docx" => Some("DOCX files are not supported"),
        "xlsx" => Some("XLSX files are not supported"),
        "pptx" => Some("PPTX files are not supported"),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "heic" | "svg" => {
            Some("image files are not supported")
        }
        "mp3" | "wav" | "m4a" | "aac" | "flac" | "ogg" => Some("audio files are not supported"),
        "mp4" | "mov" | "avi" | "mkv" | "webm" => Some("video files are not supported"),
        _ => None,
    }
}

fn known_unsupported_mime_label(mime_type: &str) -> Option<&'static str> {
    match mime_type {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
            Some("DOCX files are not supported")
        }
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
            Some("XLSX files are not supported")
        }
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
            Some("PPTX files are not supported")
        }
        value if value.starts_with("image/") => Some("image files are not supported"),
        value if value.starts_with("audio/") => Some("audio files are not supported"),
        value if value.starts_with("video/") => Some("video files are not supported"),
        _ => None,
    }
}

fn extract_text(kind: ImportKind, bytes: &[u8]) -> Result<String, ApiError> {
    match kind {
        ImportKind::Text | ImportKind::Markdown => {
            String::from_utf8(bytes.to_vec()).map_err(|error| {
                ApiError::bad_request(format!("knowledge import text is not valid UTF-8: {error}"))
            })
        }
        ImportKind::Pdf => extract_pdf_text(bytes),
    }
}

#[cfg(feature = "rag-pdf")]
fn extract_pdf_text(bytes: &[u8]) -> Result<String, ApiError> {
    if bytes.len() < 5 || &bytes[..5] != b"%PDF-" {
        return Err(ApiError::bad_request(
            "knowledge import PDF text is unreadable: invalid PDF header",
        ));
    }
    pdf_extract::extract_text_from_mem(bytes).map_err(|error| {
        ApiError::bad_request(format!("knowledge import PDF text is unreadable: {error}"))
    })
}

#[cfg(not(feature = "rag-pdf"))]
fn extract_pdf_text(_bytes: &[u8]) -> Result<String, ApiError> {
    Err(ApiError::not_implemented("knowledge import PDF extraction requires the rag-pdf feature"))
}

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n").trim().to_string()
}

fn import_metadata(
    metadata: Value,
    filename: &str,
    mime_type: Option<&str>,
    kind: ImportKind,
    source_bytes: u64,
    extracted_text_bytes: u64,
) -> Result<Value, ApiError> {
    let mut object = match metadata {
        Value::Null => Map::new(),
        Value::Object(object) => object,
        _ => {
            return Err(ApiError::bad_request("knowledge import metadata must be an object"));
        }
    };
    object.insert(
        "import".to_string(),
        json!({
            "filename": filename,
            "mime_type": mime_type,
            "kind": import_kind_as_str(kind),
            "source_bytes": source_bytes,
            "extracted_text_bytes": extracted_text_bytes,
        }),
    );
    Ok(Value::Object(object))
}

fn import_kind_as_str(kind: ImportKind) -> &'static str {
    match kind {
        ImportKind::Text => "text",
        ImportKind::Markdown => "markdown",
        ImportKind::Pdf => "pdf",
    }
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
