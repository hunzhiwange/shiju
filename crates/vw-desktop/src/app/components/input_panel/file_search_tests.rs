//! 验证文件搜索路径展示、结果上限、父目录行与选中样式。
//! 根目录归一化和二十项上限用例锁定展示层的截断策略。

use super::file_search::{
    display_file_search_path, empty_file_search_text_style, file_search_entry_style,
    file_search_icon_style, file_search_overlay, file_search_overlay_style,
    file_search_query_text_style, highlighted_file_search_query,
};
use crate::app::App;
use iced::widget::{button, svg};
use iced::{Background, Color, Length, Theme};

fn file_search_list_len(app: &App) -> usize {
    let overlay = file_search_overlay(app);
    let root_children = overlay.as_widget().children();
    root_children
        .get(1)
        .and_then(|scrollable| scrollable.children.first())
        .expect("overlay should contain the result list")
        .children
        .len()
}

#[test]
fn file_search_query_and_paths_are_normalized_for_display() {
    assert_eq!(highlighted_file_search_query("  src\\main  "), "@src/main");
    assert_eq!(highlighted_file_search_query(" \t "), "@");
    assert_eq!(display_file_search_path("/repo/src/main.rs", Some("/repo")), "src/main.rs");
    assert_eq!(display_file_search_path("/repo", Some("/repo")), "");
    assert_eq!(display_file_search_path("/other/main.rs", Some("/repo")), "/other/main.rs");
    assert_eq!(display_file_search_path("src\\main.rs", None), "src/main.rs");
}

#[test]
fn display_file_search_path_respects_complete_root_components_and_trailing_separators() {
    assert_eq!(display_file_search_path("/repo/src/lib.rs", Some("/repo/")), "src/lib.rs");
    assert_eq!(
        display_file_search_path("/repository/src/lib.rs", Some("/repo")),
        "/repository/src/lib.rs"
    );
    assert_eq!(display_file_search_path("/repo/src/", Some("/repo")), "src");
}

#[test]
fn file_search_overlay_empty_results_render_one_empty_state_row() {
    let mut app = App::new().0;
    app.file_search_query = "missing".to_string();

    let overlay = file_search_overlay(&app);

    assert_eq!(overlay.as_widget().size().width, Length::Fixed(320.0));
    assert_eq!(file_search_list_len(&app), 1);
}

#[test]
fn file_search_overlay_non_empty_results_render_each_ranked_entry() {
    let mut app = App::new().0;
    app.project_path = Some("/repo".to_string());
    app.file_search_query = "main".to_string();
    app.file_search_selected_index = 1;
    app.set_file_index(
        "/repo",
        vec![
            "/repo/src/main.rs".to_string(),
            "/repo/src/bin/tool.rs".to_string(),
            "/other/main.txt".to_string(),
        ],
    );

    let overlay = file_search_overlay(&app);

    assert_eq!(overlay.as_widget().size().width, Length::Fixed(320.0));
    assert_eq!(file_search_list_len(&app), 2);
}

#[test]
fn file_search_overlay_includes_matching_parent_directory_rows() {
    let mut app = App::new().0;
    app.project_path = Some("/repo".to_string());
    app.file_search_query = "src".to_string();
    app.set_file_index("/repo", vec!["/repo/src/main.rs".to_string()]);

    assert_eq!(file_search_list_len(&app), 2);
}

#[test]
fn file_search_overlay_limits_large_result_sets_to_twenty_rows() {
    let mut app = App::new().0;
    app.project_path = Some("/repo".to_string());
    app.file_search_query = "file".to_string();
    app.set_file_index("/repo", (0..30).map(|i| format!("src/file_{i}.rs")).collect());

    let overlay = file_search_overlay(&app);

    assert_eq!(overlay.as_widget().size().width, Length::Fixed(320.0));
    assert_eq!(file_search_list_len(&app), 20);
}

#[test]
fn file_search_entry_style_highlights_hovered_or_selected_rows() {
    for theme in [Theme::Light, Theme::Dark] {
        let idle = file_search_entry_style(&theme, button::Status::Active, false);
        let hovered = file_search_entry_style(&theme, button::Status::Hovered, false);
        let selected = file_search_entry_style(&theme, button::Status::Active, true);
        let expected = Some(Background::Color(theme.extended_palette().background.weak.color));

        assert!(idle.background.is_none());
        assert_eq!(hovered.background, expected);
        assert_eq!(selected.background, expected);
        assert_eq!(idle.text_color, theme.palette().text);
        assert_eq!(idle.border.radius, 6.0.into());
    }
}

#[test]
fn file_search_styles_follow_light_and_dark_theme_palettes() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let empty_text = empty_file_search_text_style(&theme);
        let query_text = file_search_query_text_style(&theme);
        let icon = file_search_icon_style(&theme, svg::Status::Idle);
        let overlay = file_search_overlay_style(&theme);

        assert_eq!(empty_text.color, Some(palette.secondary.base.text));
        assert_eq!(query_text.color, Some(theme.palette().primary));
        assert_eq!(icon.color, Some(theme.palette().text.scale_alpha(0.65)));
        assert_eq!(overlay.background, Some(Background::Color(palette.background.base.color)));
        assert_eq!(overlay.border.width, 1.0);
        assert_eq!(overlay.border.color, palette.background.strong.color);
        assert_eq!(overlay.border.radius, 12.0.into());
        assert_eq!(overlay.shadow.color, Color::BLACK.scale_alpha(0.12));
        assert_eq!(overlay.shadow.offset, iced::Vector::new(0.0, 4.0));
        assert_eq!(overlay.shadow.blur_radius, 12.0);
    }
}
