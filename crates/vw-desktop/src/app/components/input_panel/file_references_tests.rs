//! 验证文件引用的解析、提取、位置格式化与卡片样式边界。
//! 精确范围、溢出数字、重复提及和非法后缀用于约束降级为普通路径的规则。

use super::file_references::{
    FileReferenceLocation, extract_file_mentions, file_reference_card, file_reference_style,
    format_reference_location, format_reference_location_compact, parse_file_reference,
    render_file_references, tooltip_dark_style,
};
use crate::app::Message;
use iced::{Background, Color, Element, Length, Theme};

fn assert_size(element: &Element<'_, Message>, width: Length, height: Length) {
    let size = element.as_widget().size();

    assert_eq!(size.width, width);
    assert_eq!(size.height, height);
}

#[test]
fn parses_file_reference_locations_from_most_specific_to_plain_path() {
    let range = parse_file_reference("src/main.rs:10:5-20:8");
    assert_eq!(range.path, "src/main.rs");
    assert!(matches!(
        range.location,
        Some(FileReferenceLocation::Range {
            start_line: 10,
            start_column: 5,
            end_line: 20,
            end_column: 8
        })
    ));

    let line_column = parse_file_reference("src/lib.rs:7:2");
    assert_eq!(line_column.path, "src/lib.rs");
    assert!(matches!(
        line_column.location,
        Some(FileReferenceLocation::LineColumn { line: 7, column: 2 })
    ));

    let line_range = parse_file_reference("src/ui.rs:3-9");
    assert_eq!(line_range.path, "src/ui.rs");
    assert!(matches!(
        line_range.location,
        Some(FileReferenceLocation::LineRange { start_line: 3, end_line: 9 })
    ));

    let line = parse_file_reference("README.md:42");
    assert_eq!(line.path, "README.md");
    assert!(matches!(line.location, Some(FileReferenceLocation::Line { line: 42 })));

    let plain = parse_file_reference("docs/guide.md");
    assert_eq!(plain.path, "docs/guide.md");
    assert!(plain.location.is_none());
}

#[test]
fn parse_file_reference_keeps_invalid_suffix_as_plain_path() {
    for reference in ["src/main.rs:", ":12", "src/main.rs:abc", "src/main.rs:1-", ""] {
        let parsed = parse_file_reference(reference);

        assert_eq!(parsed.path, reference);
        assert!(parsed.location.is_none());
    }
}

#[test]
fn parse_file_reference_accepts_zeroes_and_colons_in_path() {
    let zero_range = parse_file_reference("src/main.rs:0:0-0:0");
    assert_eq!(zero_range.path, "src/main.rs");
    assert!(matches!(
        zero_range.location,
        Some(FileReferenceLocation::Range {
            start_line: 0,
            start_column: 0,
            end_line: 0,
            end_column: 0
        })
    ));

    let drive_path = parse_file_reference(r"C:\src\main.rs:9");
    assert_eq!(drive_path.path, r"C:\src\main.rs");
    assert!(matches!(drive_path.location, Some(FileReferenceLocation::Line { line: 9 })));
}

#[test]
fn parse_file_reference_accepts_maximum_usize_location_values() {
    let max = usize::MAX;
    let reference = format!("src/lib.rs:{max}:{max}-{max}:{max}");
    let parsed = parse_file_reference(&reference);

    assert_eq!(parsed.path, "src/lib.rs");
    assert!(matches!(
        parsed.location,
        Some(FileReferenceLocation::Range {
            start_line,
            start_column,
            end_line,
            end_column,
        }) if start_line == max
            && start_column == max
            && end_line == max
            && end_column == max
    ));
}

#[test]
fn parse_file_reference_rejects_overflow_and_incomplete_precise_ranges() {
    for reference in
        ["src/main.rs:184467440737095516160", "src/main.rs:1:2-3:", "src/main.rs:1:2-three:x"]
    {
        let parsed = parse_file_reference(reference);

        assert_eq!(parsed.path, reference);
        assert!(parsed.location.is_none());
    }
}

#[test]
fn formats_reference_locations_for_tooltip_and_compact_badge() {
    let cases = [
        (FileReferenceLocation::Line { line: 8 }, "第 8 行", "8"),
        (FileReferenceLocation::LineColumn { line: 8, column: 4 }, "第 8 行, 第 4 列", "8:4"),
        (FileReferenceLocation::LineRange { start_line: 8, end_line: 12 }, "第 8-12 行", "8-12"),
        (
            FileReferenceLocation::Range {
                start_line: 8,
                start_column: 4,
                end_line: 12,
                end_column: 2,
            },
            "第 8 行, 第 4 列 到 第 12 行, 第 2 列",
            "8:4-12:2",
        ),
    ];

    for (location, verbose, compact) in cases {
        assert_eq!(format_reference_location(location), verbose);
        assert_eq!(format_reference_location_compact(location), compact);
    }
}

#[test]
fn extract_file_mentions_scans_valid_path_characters_and_boundaries() {
    let text = "看 @src/main.rs:10:2-12:8, @docs\\guide.md#intro 和 @bad @";

    assert_eq!(
        extract_file_mentions(text),
        vec![
            "src/main.rs:10:2-12:8".to_string(),
            "docs\\guide.md#intro".to_string(),
            "bad".to_string()
        ]
    );
}

#[test]
fn extract_file_mentions_ignores_bare_at_and_stops_on_spaces() {
    assert_eq!(
        extract_file_mentions("@ @src/main.rs next @a-b_c.1:2"),
        vec!["src/main.rs".to_string(), "a-b_c.1:2".to_string(),]
    );
}

#[test]
fn extract_file_mentions_preserves_order_duplicates_and_ascii_boundaries() {
    assert_eq!(
        extract_file_mentions("@same.rs,@same.rs; @@last.rs? @目录/file.rs"),
        vec!["same.rs".to_string(), "same.rs".to_string(), "last.rs".to_string(),]
    );
    assert!(extract_file_mentions("plain text without mentions").is_empty());
}

#[test]
fn file_reference_styles_cover_hovered_and_idle_states() {
    let theme = Theme::Dark;
    let hovered = file_reference_style(&theme, true);
    let idle = file_reference_style(&theme, false);

    assert!(hovered.background.is_some());
    assert!(idle.background.is_some());
    assert_ne!(hovered.border.color, idle.border.color);
    assert_eq!(hovered.border.width, 1.0);
}

#[test]
fn tooltip_dark_style_uses_dark_background_and_white_text() {
    let style = tooltip_dark_style(&Theme::Light);

    assert_eq!(style.text_color, Some(Color::WHITE));
    assert!(matches!(style.background, Some(Background::Color(_))));
    assert_eq!(style.border.width, 0.0);
}

#[test]
fn file_reference_styles_keep_theme_text_and_dark_tooltip_contracts() {
    let theme = Theme::Dark;
    let idle = file_reference_style(&theme, false);
    let hovered = file_reference_style(&theme, true);

    assert_eq!(idle.text_color, Some(theme.palette().text));
    assert_eq!(hovered.text_color, Some(theme.palette().text));
    let Some(Background::Color(idle_background)) = idle.background else {
        panic!("idle style should use a color background");
    };
    let Some(Background::Color(hovered_background)) = hovered.background else {
        panic!("hovered style should use a color background");
    };
    assert_ne!(idle_background, hovered_background);

    let tooltip = tooltip_dark_style(&theme);
    assert_eq!(tooltip.text_color, Some(Color::WHITE));
    assert_eq!(tooltip.border.color, Color::TRANSPARENT);
    assert_eq!(tooltip.border.width, 0.0);
    assert!(tooltip.shadow.blur_radius > 0.0);
}

#[test]
fn file_reference_card_and_list_render_expected_outer_shapes() {
    let card = file_reference_card("src/main.rs:10".to_string(), false);
    assert_size(&card, Length::Shrink, Length::Shrink);

    let hovered = file_reference_card("src/main.rs:10".to_string(), true);
    assert_size(&hovered, Length::Shrink, Length::Shrink);

    let empty = render_file_references(&[], None);
    assert_size(&empty, Length::Shrink, Length::Shrink);

    let mentions = vec!["src/main.rs:10".to_string(), "README.md".to_string()];
    let rendered = render_file_references(&mentions, Some(1));
    assert_size(&rendered, Length::Fill, Length::Shrink);
}

#[test]
fn file_reference_cards_accept_empty_paths_and_out_of_range_hover_indices() {
    let empty_path = file_reference_card(String::new(), true);
    assert_size(&empty_path, Length::Shrink, Length::Shrink);

    let mentions = vec!["README.md".to_string(), "src/lib.rs:0:0".to_string()];
    let rendered = render_file_references(&mentions, Some(mentions.len()));
    assert_size(&rendered, Length::Fill, Length::Shrink);
}
