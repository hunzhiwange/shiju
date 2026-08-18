//! # 文件读取工具测试模块
//!
//! 本模块提供 [`FileReadTool`] 的完整测试套件，覆盖以下场景：
//!
//! ## 功能测试
//! - 工具基本属性验证（名称、参数 schema）
//! - 正常文件读取（普通文件、空文件、嵌套路径）
//! - 分页读取（offset/limit 参数）
//!
//! ## 安全边界测试
//! - 路径遍历攻击防护（`../` 序列）
//! - 绝对路径访问拦截
//! - 符号链接逃逸防护（Unix 平台）
//! - 空字节注入防护
//! - 工作区外访问控制
//!
//! ## 资源限制测试
//! - 速率限制（每小时操作数上限）
//! - 文件大小限制（10MB 上限）
//!
//! ## 特殊格式支持
//! - PDF 文本提取
//! - 非 UTF-8 二进制文件容错读取
//!
//! ## 端到端（E2E）测试
//! - 完整代理流水线集成测试
//! - 真实 Provider + 工具调用场景

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;
    use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
    use crate::app::agent::tools::{
        FileReadStateEntry, ToolUseContext, execute_tool_from_registry,
    };
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

    /// 创建基础测试安全策略
    ///
    /// 使用默认配置创建一个监督级别的安全策略，用于大多数基本测试。
    ///
    /// # 参数
    /// - `workspace`: 测试工作区目录路径
    ///
    /// # 返回
    /// 返回包装在 `Arc` 中的安全策略实例
    fn test_security(workspace: std::path::PathBuf) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            allowed_roots: vec![workspace.clone()],
            workspace_dir: workspace,
            ..SecurityPolicy::default()
        })
    }

    /// 创建可配置的测试安全策略
    ///
    /// 允许自定义自主级别和速率限制参数，用于需要特定权限配置的测试。
    ///
    /// # 参数
    /// - `workspace`: 测试工作区目录路径
    /// - `autonomy`: 自主级别（Supervised/ReadOnly/Autonomous 等）
    /// - `max_actions_per_hour`: 每小时允许的最大操作数
    ///
    /// # 返回
    /// 返回包装在 `Arc` 中的自定义安全策略实例
    fn test_security_with(
        workspace: std::path::PathBuf,
        autonomy: AutonomyLevel,
        max_actions_per_hour: u32,
    ) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy,
            allowed_roots: vec![workspace.clone()],
            workspace_dir: workspace,
            max_actions_per_hour,
            ..SecurityPolicy::default()
        })
    }

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

    fn tiny_png_bytes() -> Vec<u8> {
        BASE64_STANDARD
            .decode(
                "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aU+0AAAAASUVORK5CYII=",
            )
            .expect("decode tiny png")
    }

    fn read_state_entry(
        context: &Arc<ToolUseContext>,
        root: &std::path::Path,
        path: &str,
    ) -> Option<FileReadStateEntry> {
        let handle = context.read_state_handle();
        let mut state = handle.lock().unwrap_or_else(|error| error.into_inner());
        state.get(Some(root), path)
    }

    /// 测试工具名称是否正确
    ///
    /// 验证 `FileReadTool` 返回的工具名称为 `"file_read"`，
    /// 这是工具注册和调用的关键标识符。
    #[test]
    fn file_read_name() {
        let tool = FileReadTool::new(test_security(std::env::temp_dir()));
        assert_eq!(tool.name(), "file_read");
    }

    /// 测试参数 schema 包含必要字段
    ///
    /// 验证工具的 JSON Schema 定义了以下参数：
    /// - `path`（必需）：文件路径
    /// - `offset`（可选）：起始行号
    /// - `limit`（可选）：最大行数
    #[test]
    fn file_read_schema_has_path() {
        let tool = FileReadTool::new(test_security(std::env::temp_dir()));
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["path"].is_object());
        assert!(schema["properties"]["offset"].is_object());
        assert!(schema["properties"]["limit"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("path")));
        assert_eq!(schema["additionalProperties"], json!(false));
        // offset 和 limit 是可选参数
        assert!(!schema["required"].as_array().unwrap().contains(&json!("offset")));
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

    /// 测试读取存在的文件
    ///
    /// 验证能够成功读取工作区内的普通文件，输出应包含：
    /// - 带行号前缀的内容（如 `1: hello world`）
    /// - 行数统计信息（如 `[1 lines total]`）
    #[tokio::test]
    async fn file_read_existing_file() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("test.txt"), "hello world").await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"path": "test.txt"})).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("<file_link>"));
        assert!(result.output.contains("00001| hello world"));
        assert!(result.output.contains("(End of file: 1 lines)"));
        assert!(result.error.is_none());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_read_call_returns_structured_text_result() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_structured_text");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("test.txt"), "alpha\nbeta").await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.call(json!({"path": "test.txt", "limit": 1})).await.unwrap();

        assert_eq!(result.data["kind"], json!("text"));
        assert_eq!(result.data["start_line"], json!(1));
        assert_eq!(result.data["end_line"], json!(1));
        assert_eq!(result.data["total_lines"], json!(2));
        assert_eq!(result.render_hint.as_ref().unwrap().metadata["kind"], json!("text"));
        assert_eq!(result.render_hint.as_ref().unwrap().summary.as_deref(), Some("lines 1-1 of 2"));

        let model = result.model_result.as_str().unwrap_or("");
        assert!(model.contains("00001| alpha"));
        assert!(model.contains("(File has more lines. Use 'offset' to continue after line 1)"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试读取不存在的文件
    ///
    /// 当尝试读取不存在的文件时，应返回失败结果，
    /// 错误信息应包含 "Failed to resolve" 提示。
    #[tokio::test]
    async fn file_read_nonexistent_file() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_missing");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"path": "nope.txt"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("File does not exist"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试阻止路径遍历攻击
    ///
    /// 验证工具拒绝包含 `../` 序列的路径，防止攻击者访问工作区外的文件。
    /// 这是核心安全边界之一。
    #[tokio::test]
    async fn file_read_blocks_path_traversal() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_traversal");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"path": "../../../etc/passwd"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试阻止绝对路径访问
    ///
    /// 验证工具拒绝以 `/` 开头的绝对路径，
    /// 强制所有文件访问必须相对于工作区目录。
    #[tokio::test]
    async fn file_read_blocks_absolute_path() {
        let tool = FileReadTool::new(test_security(std::env::temp_dir()));
        let result = tool.execute(json!({"path": "/etc/passwd"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));
    }

    /// 测试速率限制生效
    ///
    /// 当配置 `max_actions_per_hour` 为 0 时，
    /// 任何文件读取操作都应被速率限制器拦截。
    #[tokio::test]
    async fn file_read_blocks_when_rate_limited() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_rate_limited");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("test.txt"), "hello world").await.unwrap();

        let tool = FileReadTool::new(test_security_with(dir.clone(), AutonomyLevel::Supervised, 0));
        let result = tool.execute(json!({"path": "test.txt"})).await.unwrap();

        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or("").contains("Rate limit exceeded"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试只读模式允许读取操作
    ///
    /// 在 `ReadOnly` 自主级别下，文件读取是允许的操作，
    /// 因为它不会修改文件系统状态。
    #[tokio::test]
    async fn file_read_allows_readonly_mode() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_readonly");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("test.txt"), "readonly ok").await.unwrap();

        let tool = FileReadTool::new(test_security_with(dir.clone(), AutonomyLevel::ReadOnly, 20));
        let result = tool.execute(json!({"path": "test.txt"})).await.unwrap();

        assert!(result.success);
        assert!(result.output.contains("00001| readonly ok"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试缺少必需参数时返回错误
    ///
    /// 当请求中不包含必需的 `path` 参数时，
    /// 工具应返回错误而非静默失败。
    #[tokio::test]
    async fn file_read_missing_path_param() {
        let tool = FileReadTool::new(test_security(std::env::temp_dir()));
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn file_read_rejects_unknown_fields() {
        let tool = FileReadTool::new(test_security(std::env::temp_dir()));
        let result = tool.execute(json!({"path": "a.txt", "filePath": "b.txt"})).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown field `filePath`"));
    }

    /// 测试读取空文件
    ///
    /// 空文件应成功读取，输出为空字符串而非报错。
    #[tokio::test]
    async fn file_read_empty_file() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_empty");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("empty.txt"), "").await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"path": "empty.txt"})).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("<file_link>"));
        assert!(result.output.contains("(End of file: 0 lines)"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试读取嵌套路径文件
    ///
    /// 验证能够正确处理工作区内的深层嵌套路径（如 `sub/dir/deep.txt`）。
    #[tokio::test]
    async fn file_read_nested_path() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_nested");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(dir.join("sub/dir")).await.unwrap();
        tokio::fs::write(dir.join("sub/dir/deep.txt"), "deep content").await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"path": "sub/dir/deep.txt"})).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("00001| deep content"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试阻止符号链接逃逸（仅 Unix 平台）
    ///
    /// 攻击者可能在工作区内创建指向工作区外敏感文件的符号链接。
    /// 本测试验证工具能够检测并阻止此类逃逸尝试。
    #[cfg(unix)]
    #[tokio::test]
    async fn file_read_blocks_symlink_escape() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join("vibewindow_test_file_read_symlink_escape");
        let workspace = root.join("workspace");
        let outside = root.join("outside");

        let _ = tokio::fs::remove_dir_all(&root).await;
        tokio::fs::create_dir_all(&workspace).await.unwrap();
        tokio::fs::create_dir_all(&outside).await.unwrap();

        tokio::fs::write(outside.join("secret.txt"), "outside workspace").await.unwrap();

        symlink(outside.join("secret.txt"), workspace.join("escape.txt")).unwrap();

        let tool = FileReadTool::new(test_security(workspace.clone()));
        let result = tool.execute(json!({"path": "escape.txt"})).await.unwrap();

        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or("").contains("escapes workspace"));

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    /// 测试禁用 workspace_only 时允许读取工作区外文件
    ///
    /// 当安全策略中 `workspace_only` 设为 `false` 时，
    /// 允许读取工作区目录外的文件（受其他约束限制）。
    #[tokio::test]
    async fn file_read_outside_workspace_allowed_when_workspace_only_disabled() {
        let root = std::env::temp_dir().join("vibewindow_test_file_read_allowed_roots_hint");
        let workspace = root.join("workspace");
        let outside = root.join("outside");
        let outside_file = outside.join("notes.txt");

        let _ = tokio::fs::remove_dir_all(&root).await;
        tokio::fs::create_dir_all(&workspace).await.unwrap();
        tokio::fs::create_dir_all(&outside).await.unwrap();
        tokio::fs::write(&outside_file, "outside").await.unwrap();

        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: workspace,
            workspace_only: false,
            forbidden_paths: vec![],
            ..SecurityPolicy::default()
        });
        let tool = FileReadTool::new(security);

        let result = tool
            .execute(json!({"path": outside_file.to_string_lossy().to_string()}))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.error.is_none());
        assert!(result.output.contains("outside"));

        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    /// 测试失败的文件读取也会消耗速率配额
    ///
    /// 即使文件不存在，读取尝试仍会计入速率限制配额。
    /// 这可以防止攻击者通过探测不存在的文件来绕过速率限制。
    #[tokio::test]
    async fn file_read_nonexistent_consumes_rate_limit_budget() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_probe");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        // 仅允许总共 2 次操作
        let tool = FileReadTool::new(test_security_with(dir.clone(), AutonomyLevel::Supervised, 2));

        // 两次读取都失败（文件不存在）但会消耗配额
        let r1 = tool.execute(json!({"path": "nope1.txt"})).await.unwrap();
        assert!(!r1.success);
        assert!(r1.error.as_ref().unwrap().contains("File does not exist"));

        let r2 = tool.execute(json!({"path": "nope2.txt"})).await.unwrap();
        assert!(!r2.success);
        assert!(r2.error.as_ref().unwrap().contains("File does not exist"));

        // 第三次尝试应该被速率限制，即使文件不存在
        let r3 = tool.execute(json!({"path": "nope3.txt"})).await.unwrap();
        assert!(!r3.success);
        assert!(
            r3.error.as_ref().unwrap().contains("Rate limit"),
            "Expected rate limit error, got: {:?}",
            r3.error
        );

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试 offset 和 limit 参数的分页功能
    ///
    /// 验证以下分页场景：
    /// - 同时使用 offset 和 limit 读取指定行范围
    /// - 仅使用 offset 从指定行读取到文件末尾
    /// - 仅使用 limit 读取前 N 行
    /// - 不使用分页参数读取全部内容
    #[tokio::test]
    async fn file_read_with_offset_and_limit() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_offset");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("lines.txt"), "aaa\nbbb\nccc\nddd\neee").await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));

        // 读取第 2-3 行
        let result =
            tool.execute(json!({"path": "lines.txt", "offset": 2, "limit": 2})).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("00002| bbb"));
        assert!(result.output.contains("00003| ccc"));
        assert!(!result.output.contains("00001| aaa"));
        assert!(!result.output.contains("00004| ddd"));
        assert!(
            result.output.contains("(File has more lines. Use 'offset' to continue after line 3)")
        );

        // 从第 4 行读取到末尾
        let result = tool.execute(json!({"path": "lines.txt", "offset": 4})).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("00004| ddd"));
        assert!(result.output.contains("00005| eee"));
        assert!(result.output.contains("(End of file: 5 lines)"));

        // 仅限制行数（前 2 行）
        let result = tool.execute(json!({"path": "lines.txt", "limit": 2})).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("00001| aaa"));
        assert!(result.output.contains("00002| bbb"));
        assert!(!result.output.contains("00003| ccc"));
        assert!(
            result.output.contains("(File has more lines. Use 'offset' to continue after line 2)")
        );

        // 完整读取（无 offset/limit）显示所有行
        let result = tool.execute(json!({"path": "lines.txt"})).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("00001| aaa"));
        assert!(result.output.contains("00005| eee"));
        assert!(result.output.contains("(End of file: 5 lines)"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试 offset 超出文件总行数
    ///
    /// 当请求的起始行号大于文件总行数时，
    /// 应返回成功但带有提示信息，而非报错。
    #[tokio::test]
    async fn file_read_offset_beyond_end() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_offset_end");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("short.txt"), "one\ntwo").await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"path": "short.txt", "offset": 100})).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("(No lines in range, file has 2 lines)"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_read_returns_file_unchanged_for_duplicate_runtime_read() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_unchanged");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("repeat.txt"), "same content").await.unwrap();

        let tools: Vec<Box<dyn Tool>> =
            vec![Box::new(FileReadTool::new(test_security(dir.clone())))];
        let context = Arc::new(ToolUseContext::new(
            "file-read-unchanged",
            Some(dir.to_string_lossy().to_string()),
        ));

        let first = execute_tool_from_registry(
            &tools,
            "file_read",
            json!({"path": "repeat.txt"}),
            context.clone(),
        )
        .await
        .expect("first read succeeds");
        assert_eq!(first.result.data["kind"], json!("text"));

        let second = execute_tool_from_registry(
            &tools,
            "file_read",
            json!({"path": "repeat.txt"}),
            context.clone(),
        )
        .await
        .expect("duplicate read succeeds");

        assert_eq!(second.result.data["kind"], json!("file_unchanged"));
        assert_eq!(second.result.data["result_kind"], json!("text"));
        assert_eq!(second.result.data["partial_view"], json!(false));
        assert!(second.result.model_result.as_str().unwrap_or("").contains("<file_unchanged>"));
        assert_eq!(context.read_state_snapshot().len(), 1);

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_read_marks_default_truncation_as_partial_in_read_state() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_partial_default_limit");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let content = (1..=2105).map(|line| format!("line-{line}")).collect::<Vec<_>>().join("\n");
        tokio::fs::write(dir.join("large.txt"), content).await.unwrap();

        let tools: Vec<Box<dyn Tool>> =
            vec![Box::new(FileReadTool::new(test_security(dir.clone())))];
        let context = Arc::new(ToolUseContext::new(
            "file-read-partial-default-limit",
            Some(dir.to_string_lossy().to_string()),
        ));

        let first = execute_tool_from_registry(
            &tools,
            "file_read",
            json!({"path": "large.txt"}),
            context.clone(),
        )
        .await
        .expect("first read succeeds");

        assert_eq!(first.result.data["kind"], json!("text"));
        assert_eq!(first.result.data["has_more"], json!(true));

        let entry =
            read_state_entry(&context, dir.as_path(), "large.txt").expect("read_state entry");
        assert!(entry.partial_view, "default truncated read should be marked partial");
        assert_eq!(entry.offset, None);
        assert_eq!(entry.limit, None);

        let second = execute_tool_from_registry(
            &tools,
            "file_read",
            json!({"path": "large.txt"}),
            context.clone(),
        )
        .await
        .expect("duplicate read succeeds");

        assert_eq!(second.result.data["kind"], json!("file_unchanged"));
        assert_eq!(second.result.data["partial_view"], json!(true));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_read_marks_full_read_even_when_limit_is_provided() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_limit_full");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("short.txt"), "one\ntwo").await.unwrap();

        let tools: Vec<Box<dyn Tool>> =
            vec![Box::new(FileReadTool::new(test_security(dir.clone())))];
        let context = Arc::new(ToolUseContext::new(
            "file-read-limit-full",
            Some(dir.to_string_lossy().to_string()),
        ));

        let result = execute_tool_from_registry(
            &tools,
            "file_read",
            json!({"path": "short.txt", "limit": 10}),
            context.clone(),
        )
        .await
        .expect("read succeeds");

        assert_eq!(result.result.data["kind"], json!("text"));
        assert_eq!(result.result.data["has_more"], json!(false));

        let entry =
            read_state_entry(&context, dir.as_path(), "short.txt").expect("read_state entry");
        assert!(
            !entry.partial_view,
            "full read should not be marked partial just because limit was provided"
        );
        assert_eq!(entry.limit, Some(10));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试拒绝超大文件
    ///
    /// 文件大小超过 10MB 限制时，应返回错误而非尝试读取，
    /// 避免内存耗尽攻击。
    #[tokio::test]
    async fn file_read_rejects_oversized_file() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_large");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        // 创建一个刚好超过 10MB 的文件
        let big = vec![b'x'; 10 * 1024 * 1024 + 1];
        tokio::fs::write(dir.join("huge.bin"), &big).await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"path": "huge.bin"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("File too large"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试 PDF 文件文本提取
    ///
    /// PDF 文件应通过 pdf-extract 库提取文本内容，
    /// 而非返回原始二进制数据。
    #[tokio::test]
    async fn file_read_extracts_pdf_text() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_pdf");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("report.pdf"), minimal_pdf_bytes()).await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"path": "report.pdf"})).await.unwrap();

        assert!(result.success, "PDF read must succeed, error: {:?}", result.error);
        assert!(
            result.output.contains("Hello"),
            "extracted text must contain 'Hello', got: {}",
            result.output
        );

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_read_call_returns_structured_pdf_result() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_pdf_structured");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("report.pdf"), minimal_pdf_bytes()).await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.call(json!({"path": "report.pdf"})).await.unwrap();

        assert_eq!(result.data["kind"], json!("pdf"));
        assert_eq!(result.render_hint.as_ref().unwrap().metadata["kind"], json!("pdf"));
        assert!(result.model_result.as_str().unwrap_or("").contains("<pdf>"));
        assert!(result.model_result.as_str().unwrap_or("").contains("Hello"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// 测试非 UTF-8 二进制文件的容错读取
    ///
    /// 对于非 UTF-8 编码的二进制文件（非 PDF），
    /// 应使用 lossy 转换方式读取，无效字节替换为 Unicode 替换字符。
    #[tokio::test]
    async fn file_read_lossy_reads_binary_file() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_lossy");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        // 写入非有效 UTF-8 字节且非 PDF 格式的数据
        let binary_data: Vec<u8> = vec![0x00, 0x80, 0xFF, 0xFE, b'h', b'i', 0x80];
        tokio::fs::write(dir.join("data.bin"), &binary_data).await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"path": "data.bin"})).await.unwrap();

        assert!(result.success, "lossy read must succeed, error: {:?}", result.error);
        assert!(
            result.output.contains('\u{FFFD}'),
            "lossy output must contain replacement character, got: {:?}",
            result.output
        );
        assert!(
            result.output.contains("hi"),
            "lossy output must preserve valid ASCII, got: {:?}",
            result.output
        );

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_read_reads_image_as_structured_result() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_image");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("pixel.png"), tiny_png_bytes()).await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.call(json!({"path": "pixel.png"})).await.unwrap();

        assert_eq!(result.data["kind"], json!("image"));
        assert_eq!(result.data["mime_type"], json!("image/png"));
        assert_eq!(result.data["width"], json!(1));
        assert_eq!(result.data["height"], json!(1));
        assert_eq!(result.render_hint.as_ref().unwrap().metadata["kind"], json!("image"));

        let model = result.model_result.as_str().unwrap_or("");
        assert!(model.contains("<image>"));
        assert!(model.contains("[IMAGE:data:image/png;base64,"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_read_reads_notebook_as_structured_result() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_notebook");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(
            dir.join("demo.ipynb"),
            serde_json::to_vec(&json!({
                "cells": [
                    {
                        "cell_type": "markdown",
                        "metadata": {"language": "markdown"},
                        "source": ["# Title\n", "Body"]
                    },
                    {
                        "cell_type": "code",
                        "metadata": {"language": "python"},
                        "source": ["print('hello')\n"],
                        "outputs": [{"data": {"text/plain": ["hello"]}}]
                    }
                ]
            }))
            .unwrap(),
        )
        .await
        .unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.call(json!({"path": "demo.ipynb"})).await.unwrap();

        assert_eq!(result.data["kind"], json!("notebook"));
        assert_eq!(result.data["total_cells"], json!(2));
        assert_eq!(result.data["cells"][0]["cell_type"], json!("markdown"));
        assert_eq!(result.data["cells"][1]["language"], json!("python"));
        assert_eq!(result.data["cells"][1]["output_mime_types"][0], json!("text/plain"));
        assert_eq!(result.render_hint.as_ref().unwrap().metadata["kind"], json!("notebook"));

        let model = result.model_result.as_str().unwrap_or("");
        assert!(model.contains("<notebook>"));
        assert!(model.contains("[Cell 1] markdown (markdown)"));
        assert!(model.contains("[Cell 2] code (python)"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // E2E 测试：完整代理流水线集成测试（真实 FileReadTool + PDF 提取）
    // ═══════════════════════════════════════════════════════════════════════════════

    /// E2E 测试辅助工具模块
    ///
    /// 提供端到端测试所需的基础设施：
    /// - `RecordingProvider`: 记录所有请求的模拟 Provider
    /// - `make_memory`: 创建测试用内存后端
    /// - `make_observer`: 创建空操作观察器
    mod e2e_helpers {
        use crate::app::agent::config::MemoryConfig;
        use crate::app::agent::memory::{self, Memory};
        use crate::app::agent::observability::{NoopObserver, Observer};
        use crate::app::agent::providers::{ChatMessage, ChatRequest, ChatResponse, Provider};
        use std::sync::{Arc, Mutex};

        /// 共享请求记录类型
        ///
        /// 用于在测试中访问 Provider 接收到的所有聊天请求历史。
        pub type SharedRequests = Arc<Mutex<Vec<Vec<ChatMessage>>>>;

        /// 可录制请求的模拟 Provider
        ///
        /// 预设响应序列并记录所有接收到的请求，
        /// 用于验证代理与工具的交互流程。
        pub struct RecordingProvider {
            /// 预设的响应队列
            responses: Mutex<Vec<ChatResponse>>,
            /// 记录的所有请求历史
            pub requests: SharedRequests,
        }

        impl RecordingProvider {
            /// 创建新的录制 Provider
            ///
            /// # 参数
            /// - `responses`: 预设的响应序列，按先进先出顺序返回
            ///
            /// # 返回
            /// 返回 Provider 实例和共享请求记录器的元组
            pub fn new(responses: Vec<ChatResponse>) -> (Self, SharedRequests) {
                let requests: SharedRequests = Arc::new(Mutex::new(Vec::new()));
                let provider =
                    Self { responses: Mutex::new(responses), requests: requests.clone() };
                (provider, requests)
            }
        }

        #[async_trait::async_trait]
        impl Provider for RecordingProvider {
            async fn chat_with_system(
                &self,
                _system_prompt: Option<&str>,
                _message: &str,
                _model: &str,
                _temperature: f64,
            ) -> anyhow::Result<String> {
                Ok("fallback".into())
            }

            async fn chat(
                &self,
                request: ChatRequest<'_>,
                _model: &str,
                _temperature: f64,
            ) -> anyhow::Result<ChatResponse> {
                // 记录本次请求的消息历史
                self.requests.lock().unwrap().push(request.messages.to_vec());

                let mut guard = self.responses.lock().unwrap();
                if guard.is_empty() {
                    // 响应队列为空时返回默认完成响应
                    return Ok(ChatResponse {
                        text: Some("done".into()),
                        tool_calls: vec![],
                        usage: None,
                        reasoning_content: None,
                    });
                }
                // 返回队列中的下一个预设响应
                Ok(guard.remove(0))
            }
        }

        /// 创建测试用内存后端
        ///
        /// 使用 "none" 后端配置，适用于不需要持久化的测试场景。
        pub fn make_memory() -> Arc<dyn Memory> {
            let cfg = MemoryConfig { backend: "none".into(), ..MemoryConfig::default() };
            Arc::from(memory::create_memory(&cfg, &std::env::temp_dir(), None).unwrap())
        }

        /// 创建空操作观察器
        ///
        /// 用于不需要观测输出的测试场景。
        pub fn make_observer() -> Arc<dyn Observer> {
            Arc::from(NoopObserver {})
        }
    }

    /// E2E 测试：脚本化 Provider 调用 file_read 读取真实 PDF
    ///
    /// 测试完整代理流水线：
    /// 1. Provider 请求调用 `file_read` 工具
    /// 2. 工具通过 pdf-extract 提取 PDF 文本
    /// 3. 提取的内容出现在工具结果消息中
    /// 4. Provider 基于工具结果生成最终回复
    #[tokio::test]
    async fn e2e_agent_file_read_pdf_extraction() {
        use crate::app::agent::agent::agent::Agent;
        use crate::app::agent::providers::{ChatResponse, Provider, ToolCall};
        use e2e_helpers::*;

        // ── 设置包含 PDF 测试文件的工作区 ──
        let workspace = std::env::temp_dir().join("vibewindow_test_e2e_file_read_pdf");
        let _ = tokio::fs::remove_dir_all(&workspace).await;
        tokio::fs::create_dir_all(&workspace).await.unwrap();

        tokio::fs::write(workspace.join("report.pdf"), minimal_pdf_bytes()).await.unwrap();

        // ── 构建真实 FileReadTool ──
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            allowed_roots: vec![workspace.clone()],
            workspace_dir: workspace.clone(),
            ..SecurityPolicy::default()
        });
        let file_read_tool: Box<dyn Tool> = Box::new(FileReadTool::new(security));

        // ── 脚本化 Provider：调用 file_read → 然后回答 ──
        let (provider, recorded) = RecordingProvider::new(vec![
            // 第 1 轮响应：Provider 请求读取 PDF
            ChatResponse {
                text: Some(String::new()),
                tool_calls: vec![ToolCall {
                    id: "tc1".into(),
                    name: "file_read".into(),
                    arguments: r#"{"path": "report.pdf"}"#.into(),
                }],
                usage: None,
                reasoning_content: None,
            },
            // 第 1 轮继续：Provider 看到工具结果后回答
            ChatResponse {
                text: Some("The PDF contains a greeting: Hello PDF".into()),
                tool_calls: vec![],
                usage: None,
                reasoning_content: None,
            },
        ]);

        let mut agent = Agent::builder()
            .provider(Box::new(provider) as Box<dyn Provider>)
            .tools(vec![file_read_tool])
            .memory(make_memory())
            .observer(make_observer())
            .workspace_dir(workspace.clone())
            .build()
            .unwrap();

        // ── 执行代理 ──
        let response = agent.turn("Read report.pdf and tell me what it says").await.unwrap();

        // ── 验证最终响应 ──
        assert!(
            response.contains("Hello PDF"),
            "agent response must contain PDF content, got: {response}",
        );

        // ── 验证 Provider 在工具结果中收到提取的 PDF 文本 ──
        {
            let all_requests = recorded.lock().unwrap();
            assert!(
                all_requests.len() >= 2,
                "expected at least 2 provider requests (initial + after tool), got {}",
                all_requests.len(),
            );

            let second_request = &all_requests[1];
            let tool_result_msg = second_request
                .iter()
                .find(|m| m.role == "tool")
                .expect("second request must contain a tool result message");

            assert!(
                tool_result_msg.content.contains("Hello"),
                "tool result must contain extracted PDF text 'Hello', got: {}",
                tool_result_msg.content,
            );
        }

        let _ = tokio::fs::remove_dir_all(&workspace).await;
    }

    /// E2E 测试：代理读取二进制文件获得 lossy UTF-8 输出
    ///
    /// 验证工具结果中包含带替换字符的容错输出，
    /// 证明二进制文件能够被安全读取。
    #[tokio::test]
    async fn e2e_agent_file_read_lossy_binary() {
        use crate::app::agent::agent::agent::Agent;
        use crate::app::agent::providers::{ChatResponse, Provider, ToolCall};
        use e2e_helpers::*;

        // ── 设置包含二进制文件的工作区 ──
        let workspace = std::env::temp_dir().join("vibewindow_test_e2e_file_read_lossy");
        let _ = tokio::fs::remove_dir_all(&workspace).await;
        tokio::fs::create_dir_all(&workspace).await.unwrap();

        let binary_data: Vec<u8> = vec![0x00, 0x80, 0xFF, 0xFE, b'v', b'a', b'l', b'i', b'd', 0x80];
        tokio::fs::write(workspace.join("data.bin"), &binary_data).await.unwrap();

        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            allowed_roots: vec![workspace.clone()],
            workspace_dir: workspace.clone(),
            ..SecurityPolicy::default()
        });
        let file_read_tool: Box<dyn Tool> = Box::new(FileReadTool::new(security));

        let (provider, recorded) = RecordingProvider::new(vec![
            ChatResponse {
                text: Some(String::new()),
                tool_calls: vec![ToolCall {
                    id: "tc1".into(),
                    name: "file_read".into(),
                    arguments: r#"{"path": "data.bin"}"#.into(),
                }],
                usage: None,
                reasoning_content: None,
            },
            ChatResponse {
                text: Some("The file appears to be binary data.".into()),
                tool_calls: vec![],
                usage: None,
                reasoning_content: None,
            },
        ]);

        let mut agent = Agent::builder()
            .provider(Box::new(provider) as Box<dyn Provider>)
            .tools(vec![file_read_tool])
            .memory(make_memory())
            .observer(make_observer())
            .workspace_dir(workspace.clone())
            .build()
            .unwrap();

        let response = agent.turn("Read data.bin").await.unwrap();

        assert!(response.contains("binary"), "agent response must mention binary, got: {response}",);

        // 验证工具结果包含带替换字符的 lossy 输出
        {
            let all_requests = recorded.lock().unwrap();
            assert!(
                all_requests.len() >= 2,
                "expected at least 2 provider requests, got {}",
                all_requests.len(),
            );

            let tool_result_msg = all_requests[1]
                .iter()
                .find(|m| m.role == "tool")
                .expect("second request must contain a tool result message");

            assert!(
                tool_result_msg.content.contains("valid"),
                "tool result must preserve valid ASCII from binary file, got: {}",
                tool_result_msg.content,
            );
            assert!(
                tool_result_msg.content.contains('\u{FFFD}'),
                "tool result must contain replacement character for invalid bytes, got: {}",
                tool_result_msg.content,
            );
        }

        let _ = tokio::fs::remove_dir_all(&workspace).await;
    }

    /// 实时 E2E 测试：真实 OpenAI Codex Provider + 真实 FileReadTool + PDF 文件
    ///
    /// 验证模型能够接收提取的 PDF 文本并做出有意义的响应。
    /// 此测试需要有效的 OAuth 凭证配置在 `~/.vibewindow/` 目录。
    ///
    /// 运行命令：
    /// ```bash
    /// cargo test --lib -- crate::app::agent::tools::file_read::tests::e2e_live_file_read_pdf --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore = "requires valid OpenAI Codex OAuth credentials"]
    async fn e2e_live_file_read_pdf() {
        use crate::app::agent::agent::agent::Agent;
        use crate::app::agent::providers::{self, ProviderRuntimeOptions};
        use e2e_helpers::*;

        // ── 设置包含 PDF 测试文件的工作区 ──
        let workspace = std::env::temp_dir().join("vibewindow_test_e2e_live_file_read_pdf");
        let _ = tokio::fs::remove_dir_all(&workspace).await;
        tokio::fs::create_dir_all(&workspace).await.unwrap();

        tokio::fs::write(workspace.join("report.pdf"), minimal_pdf_bytes()).await.unwrap();

        // ── 构建真实 FileReadTool ──
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            allowed_roots: vec![workspace.clone()],
            workspace_dir: workspace.clone(),
            ..SecurityPolicy::default()
        });
        let file_read_tool: Box<dyn Tool> = Box::new(FileReadTool::new(security));

        // ── 真实 Provider（通过统一 providers 工厂解析）──
        let provider = providers::create_provider_with_options(
            "openai-codex",
            None,
            &ProviderRuntimeOptions::default(),
        )
        .expect("provider should initialize");

        let mut agent = Agent::builder()
            .provider(provider)
            .tools(vec![file_read_tool])
            .memory(make_memory())
            .observer(make_observer())
            .workspace_dir(workspace.clone())
            .model_name("gpt-5.3-codex".to_string())
            .build()
            .unwrap();

        // ── 执行代理 ──
        let response = agent
            .turn("Use the file_read tool to read report.pdf, then tell me what text it contains. Be concise.")
            .await
            .unwrap();

        eprintln!("=== Live e2e response ===\n{response}\n=========================");

        // ── 验证模型看到了实际的 PDF 内容（"Hello PDF"）──
        let lower = response.to_lowercase();
        assert!(
            lower.contains("hello"),
            "model response must reference extracted PDF text 'Hello PDF', got: {response}",
        );

        let _ = tokio::fs::remove_dir_all(&workspace).await;
    }

    /// 测试阻止路径中的空字节注入
    ///
    /// 空字节（`\0`）可能被用于截断路径字符串绕过安全检查。
    /// 验证工具拒绝包含空字节的路径。
    #[tokio::test]
    async fn file_read_blocks_null_byte_in_path() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_null_byte");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let tool = FileReadTool::new(test_security(dir.clone()));
        let result = tool.execute(json!({"path": "test\0evil.txt"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
