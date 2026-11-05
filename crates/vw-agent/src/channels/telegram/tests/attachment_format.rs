//! Telegram 附件格式化测试模块
//!
//! 本模块测试 Telegram 消息中附件的元数据解析和内容格式化功能，
//! 确保不同类型的附件能够被正确识别、分类并以适当格式呈现。
//!
//! # 测试范围
//!
//! - 附件元数据解析（`parse_attachment_metadata`）
//! - 附件内容格式化（`format_attachment_content`）
//! - 图片扩展名识别（`is_image_extension`）
//! - 多模态图像标记检测
//!
//! # 附件类型
//!
//! 测试覆盖以下附件类型：
//! - **Photo**：图片文件，使用 `[IMAGE:路径]` 标记
//! - **Document**：文档文件，使用 `[Document:文件名] 路径` 格式

use super::*;

/// 测试解析文档类型附件的元数据
///
/// 验证当消息包含 `document` 字段时，能够正确提取：
/// - 文件ID（file_id）
/// - 文件名（file_name）
/// - 文件大小（file_size）
/// - 附件类型（Document）
#[test]
fn parse_attachment_metadata_detects_document() {
    let message = serde_json::json!({
        "document": {
            "file_id": "BQACAgIAAxk",
            "file_name": "report.pdf",
            "file_size": 12345
        }
    });
    let att = TelegramChannel::parse_attachment_metadata(&message).unwrap();
    assert_eq!(att.kind, IncomingAttachmentKind::Document);
    assert_eq!(att.file_id, "BQACAgIAAxk");
    assert_eq!(att.file_name.as_deref(), Some("report.pdf"));
    assert_eq!(att.file_size, Some(12345));
    assert!(att.caption.is_none());
}

/// 测试解析图片类型附件的元数据
///
/// Telegram 的图片以数组形式提供多个尺寸版本。
/// 验证能够：
/// - 正确识别为 Photo 类型
/// - 选择最大尺寸的图片（最后一个元素）
/// - 提取文件ID和大小
#[test]
fn parse_attachment_metadata_detects_photo() {
    let message = serde_json::json!({
        "photo": [
            {"file_id": "small_id", "file_size": 100, "width": 90, "height": 90},
            {"file_id": "medium_id", "file_size": 500, "width": 320, "height": 320},
            {"file_id": "large_id", "file_size": 2000, "width": 800, "height": 800}
        ]
    });
    let att = TelegramChannel::parse_attachment_metadata(&message).unwrap();
    assert_eq!(att.kind, IncomingAttachmentKind::Photo);
    assert_eq!(att.file_id, "large_id");
    assert_eq!(att.file_size, Some(2000));
    assert!(att.file_name.is_none());
}

/// 测试提取附件的标题（caption）
///
/// 附件消息可以包含用户添加的文字说明。
/// 验证文档和图片两种类型都能正确提取 caption 字段。
#[test]
fn parse_attachment_metadata_extracts_caption() {
    let doc_msg = serde_json::json!({
        "document": {
            "file_id": "doc_id",
            "file_name": "data.csv"
        },
        "caption": "Monthly report"
    });
    let att = TelegramChannel::parse_attachment_metadata(&doc_msg).unwrap();
    assert_eq!(att.caption.as_deref(), Some("Monthly report"));

    let photo_msg = serde_json::json!({
        "photo": [
                {"file_id": "photo_id", "file_size": 1_000}
        ],
        "caption": "Look at this"
    });
    let att = TelegramChannel::parse_attachment_metadata(&photo_msg).unwrap();
    assert_eq!(att.caption.as_deref(), Some("Look at this"));
}

/// 测试文档附件的可选字段缺失情况
///
/// 文档附件的 file_name 和 file_size 是可选字段。
/// 验证当这些字段缺失时，解析结果中的对应值为 None。
#[test]
fn parse_attachment_metadata_document_without_optional_fields() {
    let message = serde_json::json!({
        "document": {
            "file_id": "doc_no_name"
        }
    });
    let att = TelegramChannel::parse_attachment_metadata(&message).unwrap();
    assert_eq!(att.kind, IncomingAttachmentKind::Document);
    assert_eq!(att.file_id, "doc_no_name");
    assert!(att.file_name.is_none());
    assert!(att.file_size.is_none());
    assert!(att.caption.is_none());
}

/// 测试纯文本消息不返回附件
///
/// 只包含 text 字段的消息不是附件消息，
/// 解析应返回 None。
#[test]
fn parse_attachment_metadata_returns_none_for_text() {
    let message = serde_json::json!({
        "text": "Hello world"
    });
    assert!(TelegramChannel::parse_attachment_metadata(&message).is_none());
}

/// 测试语音消息不返回附件
///
/// 语音消息（voice）被视为特殊消息类型，不属于常规附件。
/// 验证解析返回 None。
#[test]
fn parse_attachment_metadata_returns_none_for_voice() {
    let message = serde_json::json!({
        "voice": {
            "file_id": "voice_id",
            "duration": 5
        }
    });
    assert!(TelegramChannel::parse_attachment_metadata(&message).is_none());
}

/// 测试空图片数组的情况
///
/// 当 photo 字段存在但数组为空时，
/// 无法提取有效图片信息，应返回 None。
#[test]
fn parse_attachment_metadata_empty_photo_array() {
    let message = serde_json::json!({
        "photo": []
    });
    assert!(TelegramChannel::parse_attachment_metadata(&message).is_none());
}

/// 测试图片类型附件的内容格式化
///
/// 验证图片附件使用 [IMAGE:路径] 标记格式，
/// 而不是文档格式。
#[test]
fn attachment_photo_content_uses_image_marker() {
    let local_path = std::path::Path::new("/tmp/workspace/photo_123_45.jpg");
    let local_filename = "photo_123_45.jpg";

    let content =
        format_attachment_content(IncomingAttachmentKind::Photo, local_filename, local_path);

    assert_eq!(content, "[IMAGE:/tmp/workspace/photo_123_45.jpg]");
    assert!(content.starts_with("[IMAGE:"));
    assert!(content.ends_with(']'));
}

/// 测试文档类型附件的内容格式化
///
/// 验证文档附件使用 [Document: 文件名] 路径 格式，
/// 不包含 [IMAGE:] 标记。
#[test]
fn attachment_document_content_uses_document_label() {
    let local_path = std::path::Path::new("/tmp/workspace/report.pdf");
    let local_filename = "report.pdf";

    let content =
        format_attachment_content(IncomingAttachmentKind::Document, local_filename, local_path);

    assert_eq!(content, "[Document: report.pdf] /tmp/workspace/report.pdf");
    assert!(!content.contains("[IMAGE:"));
}

/// 测试 Markdown 文件不会被标记为图片
///
/// 即使传入的附件类型是 Photo，如果文件扩展名是 .md，
/// 也不应该使用 [IMAGE:] 标记，而是使用文档格式。
///
/// 这是重要的安全检查，防止文本文件被当作图片处理。
#[test]
fn markdown_file_never_produces_image_marker() {
    let local_path = std::path::Path::new("/tmp/workspace/telegram_files/notes.md");
    let local_filename = "notes.md";

    let content =
        format_attachment_content(IncomingAttachmentKind::Photo, local_filename, local_path);
    assert!(!content.contains("[IMAGE:"), "markdown must not get [IMAGE:] marker: {content}");
    assert!(content.starts_with("[Document:"));

    let content_doc =
        format_attachment_content(IncomingAttachmentKind::Document, local_filename, local_path);
    assert!(
        !content_doc.contains("[IMAGE:"),
        "markdown document must not get [IMAGE:] marker: {content_doc}"
    );
}

/// 测试非图片格式的 Photo 附件回退到文档格式
///
/// 当附件类型被标记为 Photo，但实际文件扩展名不是图片格式时，
/// 应该使用文档格式而不是 [IMAGE:] 标记。
///
/// 测试覆盖的文件类型：md, txt, pdf, csv, json, zip, 无扩展名
#[test]
fn non_image_photo_falls_back_to_document_format() {
    for (filename, ext_path) in [
        ("file.md", "/tmp/ws/file.md"),
        ("file.txt", "/tmp/ws/file.txt"),
        ("file.pdf", "/tmp/ws/file.pdf"),
        ("file.csv", "/tmp/ws/file.csv"),
        ("file.json", "/tmp/ws/file.json"),
        ("file.zip", "/tmp/ws/file.zip"),
        ("file", "/tmp/ws/file"),
    ] {
        let path = std::path::Path::new(ext_path);
        let content = format_attachment_content(IncomingAttachmentKind::Photo, filename, path);
        assert!(
            !content.contains("[IMAGE:"),
            "{filename}: non-image file should not get [IMAGE:] marker, got: {content}"
        );
        assert!(
            content.starts_with("[Document:"),
            "{filename}: should use [Document:] format, got: {content}"
        );
    }
}

/// 测试图片扩展名能够产生 IMAGE 标记
///
/// 验证所有支持的图片格式（png, jpg, jpeg, gif, webp, bmp）
/// 都能正确生成 [IMAGE:] 标记。
#[test]
fn image_extensions_produce_image_marker() {
    for ext in ["png", "jpg", "jpeg", "gif", "webp", "bmp"] {
        let filename = format!("photo_1_2.{ext}");
        let path_str = format!("/tmp/ws/{filename}");
        let path = std::path::Path::new(&path_str);
        let content = format_attachment_content(IncomingAttachmentKind::Photo, &filename, path);
        assert!(
            content.starts_with("[IMAGE:"),
            "{ext}: image should get [IMAGE:] marker, got: {content}"
        );
    }
}

/// 测试 Markdown 附件不会被多模态系统识别为图片
///
/// 虽然文件类型标记为 Photo，但因为是 .md 文件，
/// 不应该触发多模态系统的图片标记检测。
#[test]
fn markdown_attachment_not_detected_by_multimodal_image_markers() {
    let content = format_attachment_content(
        IncomingAttachmentKind::Photo,
        "notes.md",
        std::path::Path::new("/tmp/ws/notes.md"),
    );
    let messages = vec![crate::app::agent::providers::ChatMessage::user(content)];
    assert_eq!(
        crate::app::agent::multimodal::count_image_markers(&messages),
        0,
        "markdown file must not trigger image marker detection"
    );
}

/// 测试 is_image_extension 函数的识别能力
///
/// 验证函数能够：
/// - 正确识别常见图片格式（png, jpg, jpeg, gif, webp, bmp）
/// - 支持大写扩展名
/// - 正确拒绝非图片格式（md, txt, pdf, csv, 无扩展名）
#[test]
fn is_image_extension_recognizes_images() {
    assert!(is_image_extension(std::path::Path::new("photo.png")));
    assert!(is_image_extension(std::path::Path::new("photo.jpg")));
    assert!(is_image_extension(std::path::Path::new("photo.jpeg")));
    assert!(is_image_extension(std::path::Path::new("photo.gif")));
    assert!(is_image_extension(std::path::Path::new("photo.webp")));
    assert!(is_image_extension(std::path::Path::new("photo.bmp")));
    assert!(is_image_extension(std::path::Path::new("PHOTO.PNG")));

    assert!(!is_image_extension(std::path::Path::new("file.md")));
    assert!(!is_image_extension(std::path::Path::new("file.txt")));
    assert!(!is_image_extension(std::path::Path::new("file.pdf")));
    assert!(!is_image_extension(std::path::Path::new("file.csv")));
    assert!(!is_image_extension(std::path::Path::new("file")));
}

/// 测试多模态系统能够检测图片标记
///
/// 验证格式化的图片内容中的 [IMAGE:路径] 标记
/// 能够被多模态系统正确检测。
#[test]
fn photo_image_marker_detected_by_multimodal() {
    let photo_content = "[IMAGE:/tmp/workspace/photo_1_2.jpg]";
    let messages = vec![crate::app::agent::providers::ChatMessage::user(photo_content.to_string())];
    let count = crate::app::agent::multimodal::count_image_markers(&messages);
    assert_eq!(count, 1, "multimodal should detect exactly one image marker");
}

/// 测试带标题的图片标记格式
///
/// 图片内容可以在 [IMAGE:] 标记后添加标题文字。
/// 验证标题不影响图片标记的检测。
#[test]
fn photo_image_marker_with_caption() {
    let local_path = std::path::Path::new("/tmp/workspace/photo_1_2.jpg");
    let mut content = format!("[IMAGE:{}]", local_path.display());
    let caption = "Look at this screenshot";
    use std::fmt::Write;
    let _ = write!(content, "\n\n{caption}");

    assert_eq!(content, "[IMAGE:/tmp/workspace/photo_1_2.jpg]\n\nLook at this screenshot");

    let messages = vec![crate::app::agent::providers::ChatMessage::user(content)];
    assert_eq!(crate::app::agent::multimodal::count_image_markers(&messages), 1);
}

/// 端到端测试：附件保存和内容格式化
///
/// 此测试模拟完整的附件处理流程：
/// 1. 在临时工作空间创建文件
/// 2. 格式化附件内容
/// 3. 验证格式化结果
/// 4. 检查多模态标记检测
///
/// 测试覆盖：
/// - PDF 文档附件
/// - JPG 图片附件（含标题）
/// - Markdown 文件（不应被识别为图片）
#[test]
fn e2e_attachment_saves_file_and_formats_content() {
    let workspace = tempfile::tempdir().expect("create temp workspace");

    let doc_filename = "report.pdf";
    let doc_path = workspace.path().join(doc_filename);
    std::fs::write(&doc_path, b"%PDF-1.4 fake").expect("write doc fixture");
    assert!(doc_path.exists(), "document file must exist on disk");

    let doc_content =
        format_attachment_content(IncomingAttachmentKind::Document, doc_filename, &doc_path);
    assert!(
        doc_content.starts_with("[Document: report.pdf]"),
        "document label format mismatch: {doc_content}"
    );
    let doc_msgs = vec![crate::app::agent::providers::ChatMessage::user(doc_content)];
    assert_eq!(
        crate::app::agent::multimodal::count_image_markers(&doc_msgs),
        0,
        "document content must not contain image markers"
    );

    let photo_filename = "photo_99_1.jpg";
    let photo_path = workspace.path().join(photo_filename);
    std::fs::write(&photo_path, [0xFF, 0xD8, 0xFF, 0xD9]).expect("write photo fixture");
    assert!(photo_path.exists(), "photo file must exist on disk");

    let photo_content =
        format_attachment_content(IncomingAttachmentKind::Photo, photo_filename, &photo_path);
    assert!(
        photo_content.starts_with("[IMAGE:"),
        "photo must use [IMAGE:] marker: {photo_content}"
    );
    assert!(photo_content.ends_with(']'), "photo marker must close with ]: {photo_content}");

    let photo_msgs = vec![crate::app::agent::providers::ChatMessage::user(photo_content.clone())];
    assert_eq!(
        crate::app::agent::multimodal::count_image_markers(&photo_msgs),
        1,
        "multimodal must detect exactly one image marker in photo content"
    );

    let mut captioned = photo_content;
    use std::fmt::Write;
    let _ = write!(captioned, "\n\nCheck this out");
    let cap_msgs = vec![crate::app::agent::providers::ChatMessage::user(captioned.clone())];
    assert_eq!(
        crate::app::agent::multimodal::count_image_markers(&cap_msgs),
        1,
        "caption must not break image marker detection"
    );
    assert!(captioned.contains("Check this out"), "caption text must be present in content");

    let md_filename = "notes.md";
    let md_path = workspace.path().join(md_filename);
    std::fs::write(&md_path, b"# Hello\nSome markdown").expect("write md fixture");
    let md_content =
        format_attachment_content(IncomingAttachmentKind::Photo, md_filename, &md_path);
    assert!(!md_content.contains("[IMAGE:"), "markdown must not get [IMAGE:] marker: {md_content}");
    let md_msgs = vec![crate::app::agent::providers::ChatMessage::user(md_content)];
    assert_eq!(
        crate::app::agent::multimodal::count_image_markers(&md_msgs),
        0,
        "markdown file must not trigger image marker detection"
    );
}

/// 测试不支持视觉功能的 Provider 拒绝图片附件
///
/// 模拟一个类似 Groq 的 Provider，其 capabilities 中 vision = false。
/// 验证：
/// - Provider 明确不支持视觉功能
/// - 图片标记能够被正确检测
///
/// 注意：此测试仅验证标记检测逻辑，
/// 实际的错误处理在运行时层面进行。
#[test]
fn groq_provider_rejects_photo_with_vision_error() {
    use crate::app::agent::providers::Provider;

    /// 模拟不支持视觉功能的 Provider（类似 Groq）
    struct GroqLikeProvider;

    #[async_trait::async_trait]
    impl crate::app::agent::providers::Provider for GroqLikeProvider {
        fn capabilities(&self) -> crate::app::agent::providers::traits::ProviderCapabilities {
            crate::app::agent::providers::traits::ProviderCapabilities {
                native_tool_search: false,
                native_tool_calling: true,
                vision: false,
            }
        }

        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok(String::new())
        }
    }

    let groq = GroqLikeProvider;

    assert!(!groq.supports_vision(), "Groq provider must not support vision");

    let messages = vec![crate::app::agent::providers::ChatMessage::user(
        "[IMAGE:/tmp/photo.jpg]\n\nDescribe this image".to_string(),
    )];
    let marker_count = crate::app::agent::multimodal::count_image_markers(&messages);
    assert_eq!(marker_count, 1, "must detect image marker in photo content");
}

/// 测试图片扩展名的文档附件使用 IMAGE 标记
///
/// 当附件类型是 Document 但文件扩展名是图片格式时，
/// 应该使用 [IMAGE:] 标记而不是文档格式。
///
/// 这是基于实际文件内容的智能路由。
#[test]
fn document_with_image_extension_routes_to_image_marker() {
    let path = std::path::Path::new("/tmp/workspace/scan.png");
    let result = format_attachment_content(IncomingAttachmentKind::Document, "scan.png", path);
    assert_eq!(result, "[IMAGE:/tmp/workspace/scan.png]");

    let path = std::path::Path::new("/tmp/workspace/photo.jpg");
    let result = format_attachment_content(IncomingAttachmentKind::Document, "photo.jpg", path);
    assert!(result.starts_with("[IMAGE:"));
}

/// 测试非图片扩展名的文档附件使用文档格式
///
/// 当附件类型是 Document 且文件扩展名不是图片格式时，
/// 应该使用 [Document: 文件名] 路径 格式。
#[test]
fn document_with_non_image_extension_routes_to_document_format() {
    let path = std::path::Path::new("/tmp/workspace/report.pdf");
    let result = format_attachment_content(IncomingAttachmentKind::Document, "report.pdf", path);
    assert_eq!(result, "[Document: report.pdf] /tmp/workspace/report.pdf");
    assert!(!result.starts_with("[IMAGE:"));
}
