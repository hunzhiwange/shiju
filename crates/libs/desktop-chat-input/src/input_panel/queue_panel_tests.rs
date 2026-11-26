//! 验证队列面板的任务分组、状态展示、操作消息与空态渲染。
//! 队列 fixture 固定发送行为和历史策略，便于单独验证顺序与移动可用性。

use super::attachment::AttachmentDisplayItem;
use super::queue_panel::{
    QueuePreview, is_dark_theme, queue_attachment_previews, queue_item_label, queue_item_style,
    queue_move_availability, queue_panel_style, queue_preview_label,
};

use desktop_i18n::I18n;
use iced::{Background, Color, Theme};

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        _ => panic!("expected a solid color background"),
    }
}

#[test]
fn queue_item_label_uses_trimmed_first_line_and_preserves_empty_first_line() {
    assert_eq!(queue_item_label("  first line  \nsecond line"), "first line");
    assert_eq!(queue_item_label("\nsecond line"), "");
    assert_eq!(queue_item_label("   "), "");
}

#[test]
fn queue_item_label_preserves_exact_limit_without_ellipsis() {
    let exact_limit = "界".repeat(42);

    assert_eq!(queue_item_label(&exact_limit), exact_limit);
}

#[test]
fn queue_item_label_truncates_unicode_by_character_count() {
    let prefix = "界".repeat(42);
    let over_limit = format!("{prefix}尾");

    assert_eq!(queue_item_label(&over_limit), format!("{prefix}…"));
}

#[test]
fn queue_item_label_handles_empty_input_and_ascii_overflow() {
    let exact_limit = "a".repeat(42);
    let over_limit = format!("{exact_limit}b");

    assert_eq!(queue_item_label(""), "");
    assert_eq!(queue_item_label(&exact_limit), exact_limit);
    assert_eq!(queue_item_label(&over_limit), format!("{exact_limit}…"));
}

#[test]
fn attachment_only_queue_titles_distinguish_images_and_documents() {
    let i18n = I18n::new(config_ui::DesktopLocale::SimplifiedChinese);
    let mut item = QueuePreview {
        query: String::new(),
        is_guide: false,
        attachments: (0..8)
            .map(|index| AttachmentDisplayItem::from_local_path(format!("{index}.png")))
            .collect(),
    };
    assert_eq!(queue_preview_label(i18n, &item), "8 张图片");
    item.attachments[0] = AttachmentDisplayItem::from_local_path("notes.pdf");
    assert_eq!(queue_preview_label(i18n, &item), "8 个附件");
    item.attachments.truncate(1);
    assert_eq!(queue_preview_label(i18n, &item), "notes.pdf");
    item.query = "Summarize the attachments".into();
    assert_eq!(queue_preview_label(i18n, &item), "Summarize the attachments");
}

#[test]
fn attachment_rows_reserve_the_second_slot_for_overflow() {
    let i18n = I18n::new(config_ui::DesktopLocale::SimplifiedChinese);
    let attachments: Vec<_> = (0..8)
        .map(|index| AttachmentDisplayItem::from_local_path(format!("{index}.png")))
        .collect();
    assert!(queue_attachment_previews(i18n, &[]).is_none());
    for (length, slots) in [(1, 1), (2, 2), (8, 2)] {
        let previews = queue_attachment_previews(i18n, &attachments[..length]).unwrap();
        let children = previews.as_widget().children();
        assert_eq!(children.len(), slots);
    }
    assert_eq!(attachments.len(), 8);
}

#[test]
fn queue_move_availability_disables_only_boundary_directions() {
    assert_eq!(queue_move_availability(0, 0), (false, false));
    assert_eq!(queue_move_availability(0, 1), (false, false));
    assert_eq!(queue_move_availability(0, 3), (false, true));
    assert_eq!(queue_move_availability(1, 3), (true, true));
    assert_eq!(queue_move_availability(2, 3), (true, false));
    assert_eq!(queue_move_availability(9, 3), (false, false));
}

#[test]
fn queue_theme_detection_distinguishes_light_and_dark() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
}

#[test]
fn queue_item_style_highlights_next_item_without_an_outline() {
    for theme in [Theme::Light, Theme::Dark] {
        let next = queue_item_style(&theme, true);
        let waiting = queue_item_style(&theme, false);
        assert_ne!(next.background, waiting.background);
        assert_eq!(next.border.width, 0.0);
        assert_eq!(waiting.border.width, 0.0);
    }
}

#[test]
fn queue_panel_style_uses_theme_text_for_its_surface_tint() {
    for theme in [Theme::Light, Theme::Dark] {
        let style = queue_panel_style(&theme);
        assert_eq!(background_color(style.background), theme.palette().text.scale_alpha(0.045),);
        assert_eq!(style.border.width, 0.0);
    }
}
