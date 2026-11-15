use super::*;
type FileReadTool = super::FileReadTool<()>;
fn test_security(workspace: PathBuf) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy { workspace_dir: workspace, ..SecurityPolicy::default() })
}
#[test]
fn file_read_rejects_non_integer_ranges() {
    let negative = parse_args(json!({"path": "notes.txt", "offset": -1})).unwrap_err();
    assert!(negative.to_string().contains("non-negative integer"));

    let fractional = parse_args(json!({"path": "notes.txt", "limit": 1.5})).unwrap_err();
    assert!(fractional.to_string().contains("non-negative integer"));
}

#[test]
fn file_read_truncates_unicode_without_splitting_characters() {
    let long_line = "界".repeat(MAX_LINE_LENGTH + 1);
    let slice = FileReadTool::collect_text_slice(&long_line, 0, 1);

    assert_eq!(slice.lines.len(), 1);
    assert_eq!(slice.lines[0].text.chars().count(), MAX_LINE_LENGTH);
    assert_eq!(slice.lines[0].text, "界".repeat(MAX_LINE_LENGTH));
    assert_eq!(FileReadTool::truncate_to_bytes("界界", 4), "界");
}

#[test]
fn file_read_extracts_notebook_language_fallbacks_and_unique_mime_types() {
    assert_eq!(
        FileReadTool::notebook_cell_language(&json!({
            "language_info": {"name": "rust"}
        })),
        Some("rust".to_string())
    );
    assert_eq!(
        FileReadTool::notebook_cell_language(&json!({
            "vscode": {"languageId": "typescript"}
        })),
        Some("typescript".to_string())
    );

    let mime_types = FileReadTool::notebook_output_mime_types(&[
        json!({"data": {"text/plain": ["first"], "image/png": "data"}}),
        json!({"data": {"text/plain": ["second"]}}),
    ]);
    assert_eq!(mime_types.len(), 2);
    assert!(mime_types.iter().any(|mime| mime == "text/plain"));
    assert!(mime_types.iter().any(|mime| mime == "image/png"));
}

#[test]
fn file_read_uses_gif_header_dimensions_when_decoding_fails() {
    let mut bytes = vec![0u8; 10];
    bytes[6..8].copy_from_slice(&3u16.to_le_bytes());
    bytes[8..10].copy_from_slice(&2u16.to_le_bytes());

    let info = FileReadTool::inspect_image(&bytes, std::path::Path::new("broken.gif"))
        .expect("extension and header should identify GIF");

    assert_eq!(info.format, "gif");
    assert_eq!(info.mime_type, "image/gif");
    assert_eq!(info.width, Some(3));
    assert_eq!(info.height, Some(2));
}

#[test]
fn file_read_parse_args_rejects_non_object_unknown_fields_and_non_string_path() {
    let error = parse_args(json!(["notes.txt"])).unwrap_err();
    assert!(error.to_string().contains("expected object"));

    let error = parse_args(json!({"filePath": 7})).unwrap_err();
    assert!(error.to_string().contains("unknown field `filePath`"));

    let error = parse_args(json!({"file_path": "notes.txt"})).unwrap_err();
    assert!(error.to_string().contains("unknown field `file_path`"));

    let error = parse_args(json!({"path": 7})).unwrap_err();
    assert!(error.to_string().contains("`path` must be a string"));
}

#[test]
fn file_read_multiline_and_text_slices_report_byte_truncation_edges() {
    assert_eq!(FileReadTool::truncate_multiline_text("alpha", 0), ("".to_string(), true));
    assert_eq!(
        FileReadTool::truncate_multiline_text("alpha\nbeta", 8),
        ("alpha\nbe".to_string(), true)
    );

    let text =
        std::iter::repeat_n("x".repeat(MAX_LINE_LENGTH), 40).collect::<Vec<_>>().join("\n");
    let slice = FileReadTool::collect_text_slice(&text, 1, 40);
    assert!(slice.truncated_by_bytes);
    assert!(slice.notice.contains("Output truncated"));
}

#[test]
fn file_read_pdf_slice_covers_empty_out_of_range_more_and_truncated_notices() {
    let empty = FileReadTool::collect_pdf_slice(&[], 1, 10);
    assert_eq!(empty.notice, "(PDF contains no extractable pages)");

    let pages = vec!["one".to_string(), "two".to_string()];
    let out_of_range = FileReadTool::collect_pdf_slice(&pages, 9, 10);
    assert!(out_of_range.notice.contains("No pages in range"));

    let has_more = FileReadTool::collect_pdf_slice(&pages, 1, 1);
    assert!(has_more.has_more);
    assert!(has_more.notice.contains("PDF has more pages"));

    let huge = vec![
        std::iter::repeat_n("x".repeat(MAX_LINE_LENGTH), 40).collect::<Vec<_>>().join("\n"),
    ];
    let truncated = FileReadTool::collect_pdf_slice(&huge, 1, 1);
    assert!(truncated.truncated_by_bytes);
    assert!(truncated.notice.contains("Output truncated"));
}

#[test]
fn file_read_notebook_slice_covers_empty_sources_ranges_and_truncation() {
    let empty = FileReadTool::collect_notebook_slice(&[], 1, 10);
    assert_eq!(empty.notice, "(Notebook has 0 cells)");

    let cells = vec![
        NotebookCellRaw {
            cell_type: "markdown".to_string(),
            metadata: json!({"language": "markdown"}),
            source: json!([]),
            outputs: Vec::new(),
        },
        NotebookCellRaw {
            cell_type: "code".to_string(),
            metadata: json!({}),
            source: json!(
                std::iter::repeat_n("x".repeat(MAX_LINE_LENGTH), 40)
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            outputs: vec![json!({"data": {"application/json": "{}"}})],
        },
    ];

    let out_of_range = FileReadTool::collect_notebook_slice(&cells, 9, 10);
    assert!(out_of_range.notice.contains("No cells in range"));

    let has_more = FileReadTool::collect_notebook_slice(&cells, 1, 1);
    assert!(has_more.has_more);
    assert_eq!(has_more.cells[0].source, "(empty cell)");
    assert_eq!(has_more.cells[0].language.as_deref(), Some("markdown"));

    let truncated = FileReadTool::collect_notebook_slice(&cells, 2, 1);
    assert!(truncated.truncated_by_bytes);
    assert!(truncated.notice.contains("Output truncated"));
    assert_eq!(truncated.cells[0].output_mime_types, vec!["application/json"]);
}

#[test]
fn file_read_image_helpers_cover_extension_labels_headers_and_large_omission() {
    assert_eq!(
        FileReadTool::image_format_from_path(std::path::Path::new("photo.JPG")),
        Some(ImageFormat::Jpeg)
    );
    assert_eq!(
        FileReadTool::image_format_from_path(std::path::Path::new("scan.webp")),
        Some(ImageFormat::WebP)
    );
    assert_eq!(FileReadTool::image_format_label(ImageFormat::Bmp), Some(("bmp", "image/bmp")));

    let mut png = vec![0u8; 24];
    png[16..20].copy_from_slice(&5u32.to_be_bytes());
    png[20..24].copy_from_slice(&7u32.to_be_bytes());
    assert_eq!(FileReadTool::image_dimensions_from_header(&png, "png"), Some((5, 7)));

    let mut bmp = vec![0u8; 26];
    bmp[18..22].copy_from_slice(&11u32.to_le_bytes());
    bmp[22..26].copy_from_slice(&(-13i32).to_le_bytes());
    assert_eq!(FileReadTool::image_dimensions_from_header(&bmp, "bmp"), Some((11, 13)));

    let tool = FileReadTool::new(test_security(std::env::temp_dir()));
    let file = FileDescriptor {
        path: "large.bmp".to_string(),
        absolute_path: "/tmp/large.bmp".to_string(),
        open: "file:///tmp/large.bmp".to_string(),
        size_bytes: (MAX_INLINE_IMAGE_BYTES + 1) as u64,
    };
    let response = tool.build_image_response(
        file,
        &vec![0u8; MAX_INLINE_IMAGE_BYTES + 1],
        ImageReadInfo {
            format: "bmp".to_string(),
            mime_type: "image/bmp".to_string(),
            width: None,
            height: None,
        },
    );

    assert_eq!(response.summary, "bmp");
    assert!(response.model_text.contains("inline preview omitted"));
    assert_eq!(response.metadata["inlineOmitted"], json!(true));
}
