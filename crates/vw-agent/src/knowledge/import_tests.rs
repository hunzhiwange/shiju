use super::import::parse_document_import;
use vw_api_types::knowledge::KnowledgeDocumentImportRequest;

fn import_request(
    filename: &str,
    mime_type: Option<&str>,
    content: &[u8],
) -> KnowledgeDocumentImportRequest {
    KnowledgeDocumentImportRequest {
        filename: filename.to_string(),
        mime_type: mime_type.map(str::to_string),
        content_base64: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, content),
        metadata: serde_json::json!({ "source": "test" }),
        enabled: true,
    }
}

#[test]
fn knowledge_import_text_file() {
    let parsed = parse_document_import(import_request(
        "notes.txt",
        Some("text/plain"),
        b" first line\r\n\r\nsecond line ",
    ))
    .expect("text import should parse");

    assert_eq!(parsed.create.name, "notes.txt");
    assert_eq!(parsed.create.content, "first line\n\nsecond line");
    assert_eq!(parsed.filename, "notes.txt");
    assert_eq!(parsed.mime_type.as_deref(), Some("text/plain"));
    assert_eq!(parsed.source_bytes, 27);
    assert_eq!(parsed.extracted_text_bytes, 23);
    assert_eq!(parsed.create.metadata["source"], "test");
    assert_eq!(parsed.create.metadata["import"]["filename"], "notes.txt");
    assert_eq!(parsed.create.metadata["import"]["mime_type"], "text/plain");
}

#[test]
fn knowledge_import_markdown_file() {
    let parsed = parse_document_import(import_request(
        "guide.md",
        Some("text/markdown"),
        b"# Title\r\n\r\n- Keep **Markdown** readable.\n",
    ))
    .expect("markdown import should parse");

    assert_eq!(parsed.create.content, "# Title\n\n- Keep **Markdown** readable.");
}

#[test]
fn knowledge_import_mdx_file_by_extension() {
    let parsed = parse_document_import(import_request(
        "guide.mdx",
        None,
        b"# Title\n\nimport Note from './Note'\n\n<Note>Body</Note>\n",
    ))
    .expect("mdx import should parse as markdown");

    assert_eq!(parsed.create.metadata["import"]["kind"], "markdown");
}

#[test]
fn knowledge_import_text_file_by_extension_without_mime_allows_null_metadata() {
    let mut request = import_request("notes.TXT", None, b"extension content");
    request.metadata = serde_json::Value::Null;

    let parsed = parse_document_import(request).expect("txt extension should parse as text");

    assert_eq!(parsed.mime_type.as_deref(), Some("text/plain"));
    assert_eq!(parsed.create.content, "extension content");
    assert_eq!(parsed.create.metadata["import"]["kind"], "text");
}

#[cfg(feature = "rag-pdf")]
#[test]
fn knowledge_import_pdf_file() {
    let parsed = parse_document_import(import_request(
        "guide.pdf",
        Some("application/pdf"),
        &minimal_pdf_bytes(),
    ))
    .expect("pdf import should parse");

    assert!(parsed.create.content.contains("Hello PDF"));
    assert_eq!(parsed.create.metadata["import"]["kind"], "pdf");
}

#[test]
fn knowledge_import_rejects_unsupported_file() {
    let error = parse_document_import(import_request(
        "report.docx",
        Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        b"PK\x03\x04",
    ))
    .expect_err("docx import should be unsupported");

    assert!(error.to_string().contains("unsupported knowledge import file type"));
    assert!(error.to_string().contains("DOCX"));
}

#[test]
fn knowledge_import_rejects_unknown_extension_without_declared_mime() {
    let error = parse_document_import(import_request("archive.custom", None, b"content"))
        .expect_err("unknown extension without MIME should fail");

    assert!(error.to_string().contains(".custom files"), "{error}");
}

#[test]
fn knowledge_import_rejects_image_mime_without_extension() {
    let error = parse_document_import(import_request("upload", Some("image/png"), b"content"))
        .expect_err("image MIME should fail");

    assert!(error.to_string().contains("image files are not supported"), "{error}");
}

#[test]
fn knowledge_import_rejects_empty_content() {
    let error = parse_document_import(import_request("empty.txt", Some("text/plain"), b" \n\t "))
        .expect_err("empty import should fail");

    assert!(error.to_string().contains("extracted text is empty"));
}

#[test]
fn knowledge_import_rejects_oversized_payload_before_parsing() {
    let request = KnowledgeDocumentImportRequest {
        filename: "large.txt".to_string(),
        mime_type: Some("text/plain".to_string()),
        content_base64: "A".repeat(super::import::MAX_IMPORT_FILE_BYTES * 2),
        metadata: serde_json::json!({}),
        enabled: true,
    };

    let error = parse_document_import(request).expect_err("oversized payload should fail");

    assert!(error.to_string().contains("knowledge import file is too large"));
}

#[test]
fn knowledge_import_cleans_path_and_normalizes_declared_mime() {
    let parsed = parse_document_import(import_request(
        r" C:\exports\NOTES.TXT ",
        Some(" Text/Plain; charset=utf-8 "),
        b"content",
    ))
    .expect("path-like filename and parameterized MIME should parse");

    assert_eq!(parsed.filename, "NOTES.TXT");
    assert_eq!(parsed.mime_type.as_deref(), Some("text/plain"));
    assert_eq!(parsed.create.metadata["import"]["kind"], "text");
    assert!(parsed.warnings.is_empty());
}

#[test]
fn knowledge_import_rejects_invalid_base64_and_utf8() {
    let mut invalid_base64 = import_request("notes.txt", Some("text/plain"), b"content");
    invalid_base64.content_base64 = "%%%".to_string();
    let base64_error =
        parse_document_import(invalid_base64).expect_err("invalid base64 should fail");
    assert!(base64_error.to_string().contains("content_base64 is invalid"));

    let utf8_error =
        parse_document_import(import_request("notes.txt", Some("text/plain"), &[0xff, 0xfe]))
            .expect_err("invalid UTF-8 should fail");
    assert!(utf8_error.to_string().contains("text is not valid UTF-8"));
}

#[test]
fn knowledge_import_rejects_non_object_metadata() {
    let mut request = import_request("notes.txt", Some("text/plain"), b"content");
    request.metadata = serde_json::json!(["not", "an", "object"]);

    let error = parse_document_import(request).expect_err("array metadata should fail");

    assert!(error.to_string().contains("metadata must be an object"));
}

#[test]
fn knowledge_import_rejects_missing_filename_and_unknown_type() {
    let filename_error =
        parse_document_import(import_request(" / ", Some("text/plain"), b"content"))
            .expect_err("missing basename should fail");
    assert!(filename_error.to_string().contains("filename is required"));

    let type_error = parse_document_import(import_request(
        "archive.custom",
        Some("application/x-custom"),
        b"content",
    ))
    .expect_err("unknown file type should fail");
    assert!(type_error.to_string().contains("MIME type application/x-custom"));
}

#[cfg(feature = "rag-pdf")]
#[test]
fn knowledge_import_rejects_invalid_pdf_header() {
    let error =
        parse_document_import(import_request("broken.pdf", Some("application/pdf"), b"not a PDF"))
            .expect_err("invalid PDF header should fail");

    assert!(error.to_string().contains("invalid PDF header"));
}

#[cfg(feature = "rag-pdf")]
fn minimal_pdf_bytes() -> Vec<u8> {
    let stream = "BT\n/F1 12 Tf\n72 720 Td\n(Hello PDF) Tj\nET";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", stream.len(), stream),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len() + 1);
    offsets.push(0usize);

    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }

    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_offset
        )
        .as_bytes(),
    );
    pdf
}
