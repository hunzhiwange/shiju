//! attachment_tests.rs 测试模块。
//!
//! 这些测试固定相邻解析器、视图辅助函数或状态计算的行为，防止后续 UI 重排时破坏边界契约。

use super::{
    AttachmentDisplayItem, AttachmentDisplayKind, attachment_button, attachment_chip,
    attachment_preview_strip, attachment_strip, attachment_thumbnail, chip_close_style, chip_style,
    chip_thumb_style, is_supported_image_attachment, parse_attachment_markers,
    open_attachment_message, split_attachment_name_extension, truncate_attachment_name_middle,
};
use crate::app::{App, Message, message};
use iced::widget::button;
use iced::{Color, Element, Length, Theme};

fn assert_size(element: &Element<'_, Message>, width: Length, height: Length) {
    let size = element.as_widget().size();

    assert_eq!(size.width, width);
    assert_eq!(size.height, height);
}

/// 验证 truncate attachment name middle preserves short name 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn truncate_attachment_name_middle_preserves_short_name() {
    assert_eq!(truncate_attachment_name_middle("short-name.png", 20), "short-name.png");
}

/// 验证 truncate attachment name middle preserves extension and total length 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn truncate_attachment_name_middle_preserves_extension_and_total_length() {
    let result = truncate_attachment_name_middle("1234567890abcdefghidef.png", 20);

    assert_eq!(result, "1234567890...def.png");
    assert_eq!(result.chars().count(), 20);
}

/// 验证 truncate attachment name middle falls back without extension 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn truncate_attachment_name_middle_falls_back_without_extension() {
    let result = truncate_attachment_name_middle("1234567890123456789012345", 20);

    assert_eq!(result, "123456789...89012345");
    assert_eq!(result.chars().count(), 20);
}

#[test]
fn image_attachment_detection_is_case_insensitive_and_safe_for_missing_extensions() {
    for path in ["a.png", "b.JPG", "c.Jpeg", "d.webp", "e.GIF", "f.bmp"] {
        assert!(is_supported_image_attachment(path));
    }

    for path in ["README", "archive.zip", ".png", "image.svg", "folder/name"] {
        assert!(!is_supported_image_attachment(path));
    }
}

#[test]
fn attachment_display_item_classifies_local_paths() {
    let image = AttachmentDisplayItem::from_local_path("/tmp/screenshot.PNG");
    assert_eq!(image.kind, AttachmentDisplayKind::Image);
    assert_eq!(image.path, "/tmp/screenshot.PNG");
    assert_eq!(image.display_name, None);

    let document = AttachmentDisplayItem::from_local_path("/tmp/report.pdf");
    assert_eq!(document.kind, AttachmentDisplayKind::Document);
}

#[test]
fn split_attachment_name_extension_rejects_edge_dots() {
    assert_eq!(split_attachment_name_extension("report.final.pdf"), Some(("report.final", ".pdf")));
    assert_eq!(split_attachment_name_extension(".env"), None);
    assert_eq!(split_attachment_name_extension("trailing."), None);
    assert_eq!(split_attachment_name_extension("README"), None);
}

#[test]
fn truncate_attachment_name_middle_handles_tiny_limits_and_unicode() {
    assert_eq!(truncate_attachment_name_middle("abcdef", 3), "abc");
    assert_eq!(truncate_attachment_name_middle("abcdef", 5), "a...f");

    let result = truncate_attachment_name_middle("项目截图长文件名.png", 8);
    assert_eq!(result.chars().count(), 8);
    assert!(result.contains("..."));
}

#[test]
fn truncate_attachment_name_middle_handles_zero_and_ellipsis_sized_limits() {
    assert_eq!(truncate_attachment_name_middle("abcdef", 0), "");
    assert_eq!(truncate_attachment_name_middle("abcdef", 1), "a");
    assert_eq!(truncate_attachment_name_middle("abcdef", 2), "ab");
    assert_eq!(truncate_attachment_name_middle("abcdef", 3), "abc");
    assert_eq!(truncate_attachment_name_middle("abcdef", 4), "a...");
}

#[test]
fn truncate_attachment_name_middle_limits_tail_when_extension_exceeds_budget() {
    let result = truncate_attachment_name_middle("abcdefgh.longextension", 8);

    assert_eq!(result, "ab...ion");
    assert_eq!(result.chars().count(), 8);
}

#[test]
fn attachment_chip_styles_follow_theme_and_hover_contracts() {
    for theme in [Theme::Light, Theme::Dark] {
        let chip = chip_style(&theme);
        assert!(chip.background.is_some());
        assert_eq!(chip.border.width, 1.0);
        assert_eq!(chip.text_color, Some(theme.palette().text));

        let thumbnail = chip_thumb_style(&theme);
        assert!(thumbnail.background.is_some());
        assert_eq!(thumbnail.border.width, 1.0);

        let idle_close = chip_close_style(&theme, button::Status::Active);
        let hovered_close = chip_close_style(&theme, button::Status::Hovered);
        assert!(idle_close.background.is_none());
        assert!(hovered_close.background.is_some());
        assert_eq!(idle_close.border.width, 0.0);
        assert_eq!(idle_close.border.color, Color::TRANSPARENT);
        assert_eq!(idle_close.text_color, theme.palette().text.scale_alpha(0.78));
    }
}

#[test]
fn attachment_thumbnail_uses_image_and_icon_fallback_shapes() {
    let existing_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/components/input_panel/attachment_tests.rs");
    assert!(existing_path.exists());
    let existing_image = attachment_thumbnail(
        existing_path.to_string_lossy().into_owned(),
        AttachmentDisplayKind::Image,
    );
    assert_size(&existing_image, Length::Fixed(28.0), Length::Fixed(28.0));

    let missing_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/components/input_panel/__missing_attachment_thumbnail__.png");
    assert!(!missing_path.exists());
    let missing_image = attachment_thumbnail(
        missing_path.to_string_lossy().into_owned(),
        AttachmentDisplayKind::Image,
    );
    assert_size(&missing_image, Length::Fixed(28.0), Length::Fixed(28.0));

    let document = attachment_thumbnail("report.pdf".to_string(), AttachmentDisplayKind::Document);
    assert_size(&document, Length::Fixed(28.0), Length::Fixed(28.0));
}

#[test]
fn attachment_chip_builds_with_optional_remove_action_and_display_name_sources() {
    let without_remove = attachment_chip(
        AttachmentDisplayItem {
            path: "/tmp/report.pdf".to_string(),
            display_name: Some("Quarterly report.pdf".to_string()),
            kind: AttachmentDisplayKind::Document,
        },
        None,
    );
    assert_size(&without_remove, Length::Shrink, Length::Shrink);

    let missing_image_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/components/input_panel/__missing_attachment_chip__.png");
    assert!(!missing_image_path.exists());
    let with_remove = attachment_chip(
        AttachmentDisplayItem::from_local_path(missing_image_path.to_string_lossy()),
        Some(Message::Project(message::ProjectMessage::RemoveAttachedFile(
            missing_image_path.to_string_lossy().into_owned(),
        ))),
    );
    assert_size(&with_remove, Length::Shrink, Length::Shrink);

    let empty_path = attachment_chip(
        AttachmentDisplayItem {
            path: String::new(),
            display_name: None,
            kind: AttachmentDisplayKind::Document,
        },
        None,
    );
    assert_size(&empty_path, Length::Shrink, Length::Shrink);
}

#[test]
fn attachment_click_opens_file_in_preview_editor() {
    let path = "/tmp/vibewindow-paste-example.txt".to_string();

    assert!(matches!(
        open_attachment_message(path.clone()),
        Message::Preview(message::PreviewMessage::Open(opened_path)) if opened_path == path
    ));
}

#[test]
fn parse_attachment_markers_extracts_known_marker_forms_and_cleans_text() {
    let input =
        "before [IMAGE: /tmp/a.png]\n[DOCUMENT: /tmp/b.pdf]\n[Document: spec] /tmp/c.txt\n after ";
    let (cleaned, attachments) = parse_attachment_markers(input);

    assert_eq!(cleaned, "before\n\n\n after");
    assert_eq!(attachments.len(), 3);
    assert_eq!(attachments[0].path, "/tmp/a.png");
    assert_eq!(attachments[0].kind, AttachmentDisplayKind::Image);
    assert_eq!(attachments[1].kind, AttachmentDisplayKind::Document);
    assert_eq!(attachments[2].display_name.as_deref(), Some("spec"));
    assert_eq!(attachments[2].path, "/tmp/c.txt");
}

#[test]
fn parse_attachment_markers_preserves_unknown_empty_and_unclosed_markers() {
    let input = "keep [IMAGE: ] and [DOCUMENT:] and [UNKNOWN: value] plus [DOCUMENT: /x";
    let (cleaned, attachments) = parse_attachment_markers(input);

    assert!(attachments.is_empty());
    assert_eq!(cleaned, input);
}

#[test]
fn parse_attachment_markers_extracts_adjacent_markers_and_trims_remaining_text() {
    let (cleaned, attachments) =
        parse_attachment_markers("  [IMAGE: a.png][DOCUMENT:b.pdf] tail  ");

    assert_eq!(cleaned, "tail");
    assert_eq!(attachments.len(), 2);
    assert_eq!(attachments[0].path, "a.png");
    assert_eq!(attachments[0].kind, AttachmentDisplayKind::Image);
    assert_eq!(attachments[1].path, "b.pdf");
    assert_eq!(attachments[1].kind, AttachmentDisplayKind::Document);
}

#[test]
fn parse_attachment_markers_supports_legacy_empty_name_unicode_space_and_crlf() {
    let input = "before [Document: ]\u{3000}/tmp/报告 final.pdf\r\nafter";
    let (cleaned, attachments) = parse_attachment_markers(input);

    assert_eq!(cleaned, "before\nafter");
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].path, "/tmp/报告 final.pdf");
    assert_eq!(attachments[0].display_name, None);
    assert_eq!(attachments[0].kind, AttachmentDisplayKind::Document);
}

#[test]
fn parse_attachment_markers_preserves_legacy_marker_without_same_line_path() {
    let (cleaned, attachments) = parse_attachment_markers("keep [Document: report]   \nnext");

    assert!(attachments.is_empty());
    assert_eq!(cleaned, "keep [Document: report]\nnext");
}

#[test]
fn parse_attachment_markers_normalizes_trailing_space_without_markers() {
    let (cleaned, attachments) = parse_attachment_markers("  first  \n second\t \n\n");

    assert!(attachments.is_empty());
    assert_eq!(cleaned, "first\n second");

    let (cleaned, attachments) = parse_attachment_markers("");
    assert!(cleaned.is_empty());
    assert!(attachments.is_empty());
}

#[test]
fn attachment_preview_strip_builds_empty_and_non_empty_elements() {
    let empty = attachment_preview_strip(vec![]);
    assert_size(&empty, Length::Shrink, Length::Fixed(0.0));

    let missing_image_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/app/components/input_panel/__missing_attachment_preview__.png");
    assert!(!missing_image_path.exists());
    let populated = attachment_preview_strip(vec![
        AttachmentDisplayItem::from_local_path(missing_image_path.to_string_lossy()),
        AttachmentDisplayItem::from_local_path("/tmp/readme.txt"),
    ]);
    assert_size(&populated, Length::Shrink, Length::Shrink);
}

#[test]
fn attachment_button_and_strip_reflect_empty_and_populated_app_files() {
    let mut app = App::new().0;

    let button = attachment_button(&app);
    assert_size(&button, Length::Shrink, Length::Shrink);
    drop(button);

    let empty_strip = attachment_strip(&app);
    assert_size(&empty_strip, Length::Shrink, Length::Fixed(0.0));
    drop(empty_strip);

    app.files = vec!["/tmp/report.pdf".to_string(), "/tmp/screenshot.PNG".to_string()];
    let populated_strip = attachment_strip(&app);
    assert_size(&populated_strip, Length::Shrink, Length::Shrink);
}

#[test]
fn attachment_button_clamps_multimodal_limits_while_counting_images() {
    let mut app = App::new().0;
    app.files =
        vec!["/tmp/one.png".to_string(), "/tmp/two.JPG".to_string(), "/tmp/readme.md".to_string()];
    app.multimodal_settings.max_images = 0;
    app.multimodal_settings.max_image_size_mb = 99;

    let lower_clamped = attachment_button(&app);
    assert_size(&lower_clamped, Length::Shrink, Length::Shrink);
    drop(lower_clamped);

    app.multimodal_settings.max_images = 99;
    app.multimodal_settings.max_image_size_mb = 0;
    let upper_clamped = attachment_button(&app);
    assert_size(&upper_clamped, Length::Shrink, Length::Shrink);
}
