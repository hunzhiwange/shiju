use super::*;

/// 测试空附件列表返回空字符串
/// 没有附件时应该返回空字符串
#[tokio::test]
async fn process_attachments_empty_list_returns_empty() {
    let client = reqwest::Client::new();
    let result = attachments::process_attachments(&[], &client).await;
    assert!(result.is_empty());
}

/// 测试跳过不支持的附件类型
/// 非图片/音频类型应该被跳过
#[tokio::test]
async fn process_attachments_skips_unsupported_types() {
    let client = reqwest::Client::new();
    let attachments = vec![serde_json::json!({
        "url": "https://cdn.discordapp.com/attachments/123/456/doc.pdf",
        "filename": "doc.pdf",
        "content_type": "application/pdf"
    })];
    let result = attachments::process_attachments(&attachments, &client).await;
    assert!(result.is_empty());
}

/// 测试图片内容类型产生图片标记
/// 图片附件应该生成 [IMAGE:url] 标记
#[tokio::test]
async fn process_attachments_emits_image_marker_for_image_content_type() {
    let client = reqwest::Client::new();
    let attachments = vec![serde_json::json!({
        "url": "https://cdn.discordapp.com/attachments/123/456/photo.png",
        "filename": "photo.png",
        "content_type": "image/png"
    })];
    let result = attachments::process_attachments(&attachments, &client).await;
    assert_eq!(result, "[IMAGE:https://cdn.discordapp.com/attachments/123/456/photo.png]");
}

/// 测试多个图片标记的生成
/// 多个图片应该用分隔符连接
#[tokio::test]
async fn process_attachments_emits_multiple_image_markers() {
    let client = reqwest::Client::new();
    let attachments = vec![
        serde_json::json!({
            "url": "https://cdn.discordapp.com/attachments/123/456/one.jpg",
            "filename": "one.jpg",
            "content_type": "image/jpeg"
        }),
        serde_json::json!({
            "url": "https://cdn.discordapp.com/attachments/123/456/two.webp",
            "filename": "two.webp",
            "content_type": "image/webp"
        }),
    ];
    let result = attachments::process_attachments(&attachments, &client).await;
    assert_eq!(
        result,
        "[IMAGE:https://cdn.discordapp.com/attachments/123/456/one.jpg]\n---\n[IMAGE:https://cdn.discordapp.com/attachments/123/456/two.webp]"
    );
}

/// 测试没有内容类型时从文件名推断图片
/// 当缺少 content_type 时，应该根据文件扩展名判断
#[tokio::test]
async fn process_attachments_emits_image_marker_from_filename_without_content_type() {
    let client = reqwest::Client::new();
    let attachments = vec![serde_json::json!({
        "url": "https://cdn.discordapp.com/attachments/123/456/photo.jpeg?size=1024",
        "filename": "photo.jpeg"
    })];
    let result = attachments::process_attachments(&attachments, &client).await;
    assert_eq!(
        result,
        "[IMAGE:https://cdn.discordapp.com/attachments/123/456/photo.jpeg?size=1024]"
    );
}

/// 测试图片附件检测优先使用内容类型而非扩展名
/// content_type 非 image 时，即使扩展名是图片也不应识别为图片
#[test]
fn is_image_attachment_prefers_non_image_content_type_over_extension() {
    assert!(!attachments::is_image_attachment(
        "text/plain",
        "photo.png",
        "https://cdn.discordapp.com/attachments/123/456/photo.png"
    ));
}

/// 测试图片附件允许 octet-stream 时使用扩展名回退
/// content_type 为 application/octet-stream 时，应该使用扩展名判断
#[test]
fn is_image_attachment_allows_octet_stream_extension_fallback() {
    assert!(attachments::is_image_attachment(
        "application/octet-stream",
        "photo.png",
        "https://cdn.discordapp.com/attachments/123/456/photo.png"
    ));
}
