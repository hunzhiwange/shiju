use crate::find_results::*;
use crate::{Message, ProjectMessage};
use crate::state::{FIND_RESULTS_PAGE_SIZE, find_results_page_bounds};
use iced::{Color, Theme};
#[test]
fn to_relative_path_strips_project_root() {
    assert_eq!(to_relative_path("/tmp/project", "/tmp/project/src/main.rs"), "src/main.rs");
    assert_eq!(to_relative_path("/tmp/project", "/other/main.rs"), "/other/main.rs");
    assert_eq!(to_relative_path(" /tmp/project/ ", " /tmp/project/src/lib.rs "), "src/lib.rs");
    assert_eq!(to_relative_path("/tmp/project", "/tmp/project"), "/tmp/project");
}

#[test]
fn find_match_highlight_bg_is_visible() {
    let dark = find_match_highlight_bg(&Theme::Dark);
    let light = find_match_highlight_bg(&Theme::Light);

    assert_eq!(dark.a, 0.34);
    assert_eq!(light.a, 0.70);
    assert_ne!(dark, light);
}

#[test]
fn find_option_buttons_are_tooltip_wrapped_and_highlight_active_state() {
    let button = find_option_button("Aa", "区分大小写", FIND_CASE_SHORTCUT, false, Message::None);
    assert_eq!(button.as_widget().children().len(), 2);

    for theme in [Theme::Dark, Theme::Light] {
        let active = find_option_button_style(&theme, iced::widget::button::Status::Active, true);
        let inactive =
            find_option_button_style(&theme, iced::widget::button::Status::Active, false);
        assert_ne!(active.background, inactive.background);
        assert_ne!(active.text_color, inactive.text_color);
    }
}

#[test]
fn find_option_shortcut_labels_match_platform_conventions() {
    #[cfg(target_os = "macos")]
    assert_eq!(
        [FIND_CASE_SHORTCUT, FIND_WHOLE_WORD_SHORTCUT, FIND_REGEX_SHORTCUT],
        ["⌥⌘C", "⌥⌘W", "⌥⌘X"]
    );
    #[cfg(not(target_os = "macos"))]
    assert_eq!(
        [FIND_CASE_SHORTCUT, FIND_WHOLE_WORD_SHORTCUT, FIND_REGEX_SHORTCUT],
        ["Ctrl+Alt+C", "Ctrl+Alt+W", "Ctrl+Alt+X"]
    );
}

#[test]
fn find_results_tab_title_normalizes_to_the_first_line_and_ten_unicode_characters() {
    assert_eq!(find_results_tab_title("  alpha  \n beta  "), "alpha");
    assert_eq!(find_results_tab_title("搜索标签交互增强测试"), "搜索标签交互增强测试");
    assert_eq!(find_results_tab_title(" \n "), "搜索");
}

#[test]
fn find_results_tabs_use_the_preview_tab_height() {
    assert_eq!(FIND_RESULTS_TAB_HEIGHT, 30.0);
}

#[test]
fn find_result_item_icon_is_two_thirds_of_the_standard_file_icon() {
    assert_eq!(FIND_RESULT_ITEM_ICON_SIZE, 14.0 * 2.0 / 3.0);
}

#[test]
fn find_results_tab_add_button_creates_a_new_project_search() {
    assert!(matches!(
        find_results_tab_add_message(),
        Message::Project(ProjectMessage::FileTreeFindInProject)
    ));
}

#[test]
fn find_tab_menu_messages_preserve_id_and_action_order() {
    let [close, close_left, close_right, close_all] = find_tab_menu_messages("搜索 标签");

    assert!(matches!(
        close,
        Message::Project(ProjectMessage::FileTreeFindTabClosed(id))
            if id == "搜索 标签"
    ));
    assert!(matches!(
        close_left,
        Message::Project(ProjectMessage::FileTreeFindTabMenuCloseLeft(id))
            if id == "搜索 标签"
    ));
    assert!(matches!(
        close_right,
        Message::Project(ProjectMessage::FileTreeFindTabMenuCloseRight(id))
            if id == "搜索 标签"
    ));
    assert!(matches!(
        close_all,
        Message::Project(ProjectMessage::FileTreeFindTabMenuCloseAll)
    ));
}

#[test]
fn find_run_button_uses_white_text_in_all_themes() {
    for theme in [Theme::Dark, Theme::Light] {
        for status in [
            iced::widget::button::Status::Active,
            iced::widget::button::Status::Hovered,
            iced::widget::button::Status::Pressed,
        ] {
            assert_eq!(find_run_button_style(&theme, status).text_color, Color::WHITE);
        }
    }
}

#[test]
fn find_scope_input_uses_pill_shaped_border_in_light_and_dark_themes() {
    for theme in [Theme::Dark, Theme::Light] {
        let style = find_scope_input_style(&theme, iced::widget::text_input::Status::Active);
        assert_eq!(style.border.radius, 999.0.into());
    }
}

#[test]
fn find_results_page_bounds_limit_each_ui_build_to_one_page() {
    assert_eq!(find_results_page_bounds(0, usize::MAX), (0..0, 0, 0));
    assert_eq!(
        find_results_page_bounds(FIND_RESULTS_PAGE_SIZE * 3 + 17, 0),
        (0..FIND_RESULTS_PAGE_SIZE, 0, 4)
    );
    assert_eq!(
        find_results_page_bounds(FIND_RESULTS_PAGE_SIZE * 3 + 17, usize::MAX),
        (FIND_RESULTS_PAGE_SIZE * 3..FIND_RESULTS_PAGE_SIZE * 3 + 17, 3, 4)
    );
}

#[test]
fn find_results_page_size_limits_tab_switch_rendering_to_twenty_four_items() {
    assert_eq!(FIND_RESULTS_PAGE_SIZE, 24);
}

#[test]
fn to_relative_path_normalizes_windows_separators() {
    assert_eq!(
        to_relative_path("C:/work/project", "C:\\work\\project\\src\\main.rs"),
        "src/main.rs"
    );
}

#[test]
fn to_relative_path_handles_root_slashes_whitespace_and_empty_values() {
    assert_eq!(to_relative_path("/tmp/project///", "/tmp/project/src/lib.rs"), "src/lib.rs");
    assert_eq!(to_relative_path("", " /tmp/project/src/lib.rs "), "tmp/project/src/lib.rs");
    assert_eq!(to_relative_path(" /tmp/project ", "   "), "");
    assert_eq!(to_relative_path("C:\\work\\project\\", "C:\\work\\project"), "C:/work/project");
}

#[test]
fn find_scope_input_value_is_empty_for_project_and_relative_for_folder() {
    assert_eq!(find_scope_input_value("/tmp/project", "/tmp/project"), "");
    assert_eq!(find_scope_input_value("/tmp/project", "/tmp/project/src/nested"), "src/nested");
    assert_eq!(find_scope_input_value("C:\\work\\project", "C:\\work\\project\\src"), "src");
}
