//! 验证文件搜索标签、结果分页、路径预览、快捷键和窄面板布局。
//! fixture 包含零长度、越界和 Unicode 命中，确保高亮失败时仍能构造稳定预览卡片。

use super::find_results::{
    FIND_CASE_SHORTCUT, FIND_REGEX_SHORTCUT, FIND_RESULT_ITEM_ICON_SIZE,
    FIND_RESULTS_SCROLLBAR_GUTTER, FIND_RESULTS_TAB_HEIGHT, FIND_WHOLE_WORD_SHORTCUT,
    build_find_results_list, build_find_results_tabs, find_match_highlight_bg, find_option_button,
    find_option_button_style, find_results_tab_add_message, find_results_tab_title,
    find_run_button_style, find_scope_input_style, find_scope_input_value, find_tab_menu_messages,
    to_relative_path,
};
use crate::app::state::{
    FIND_RESULTS_PAGE_SIZE, FindInFolderMatch, FindInFolderTab, find_results_page_bounds,
};
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::Layout;
use iced::advanced::layout;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::{Color, Font, Pixels, Size, Theme};

fn find_tab(id: &str) -> FindInFolderTab {
    FindInFolderTab {
        id: id.to_string(),
        title: "查找".to_string(),
        scope_path: "/tmp/project".to_string(),
        query_input: "main".to_string(),
        replace_input: String::new(),
        query_editor: iced::widget::text_editor::Content::with_text("main"),
        replace_editor: iced::widget::text_editor::Content::new(),
        query: "main".to_string(),
        replace_text: String::new(),
        case_sensitive: false,
        whole_word: false,
        use_regex: false,
        running: false,
        error: None,
        limit_reached: false,
        matches: Vec::new(),
        results_page: 0,
    }
}

fn find_match(
    path: &str,
    line: usize,
    column: usize,
    preview: &str,
    match_len: usize,
) -> FindInFolderMatch {
    FindInFolderMatch {
        path: path.to_string(),
        line,
        column,
        preview: preview.to_string(),
        match_len,
    }
}

// 只读取控件树直接子节点，测试关注页面骨架而不绑定子控件内部实现。
fn child_count(element: Element<'_, Message>) -> usize {
    Tree::new(element.as_widget()).children.len()
}

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
fn find_results_tabs_build_empty_and_populated_tab_bar() {
    let empty = App::new().0;
    assert_eq!(child_count(build_find_results_tabs(&empty)), 1);

    let mut app = App::new().0;
    app.find_results_tabs.push(find_tab("first"));
    app.find_results_tabs.push(find_tab("second"));
    app.active_find_results_tab_id = Some("second".to_string());

    assert_eq!(child_count(build_find_results_tabs(&app)), 1);
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
        Message::Project(crate::app::message::project::ProjectMessage::FileTreeFindInProject)
    ));
}

#[test]
fn find_tab_menu_messages_preserve_id_and_action_order() {
    let [close, close_left, close_right, close_all] = find_tab_menu_messages("搜索 标签");

    assert!(matches!(
        close,
        Message::Project(crate::app::message::project::ProjectMessage::FileTreeFindTabClosed(id))
            if id == "搜索 标签"
    ));
    assert!(matches!(
        close_left,
        Message::Project(crate::app::message::project::ProjectMessage::FileTreeFindTabMenuCloseLeft(id))
            if id == "搜索 标签"
    ));
    assert!(matches!(
        close_right,
        Message::Project(crate::app::message::project::ProjectMessage::FileTreeFindTabMenuCloseRight(id))
            if id == "搜索 标签"
    ));
    assert!(matches!(
        close_all,
        Message::Project(crate::app::message::project::ProjectMessage::FileTreeFindTabMenuCloseAll)
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

#[test]
fn find_results_list_builds_empty_states() {
    let app = App::new().0;
    assert_eq!(child_count(build_find_results_list(&app)), 1);

    let mut app = App::new().0;
    app.active_find_results_tab_id = Some("missing".to_string());
    assert_eq!(child_count(build_find_results_list(&app)), 1);
}

#[test]
fn find_results_list_builds_controls_empty_results_and_limit_notice() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/project".to_string());

    let mut tab = find_tab("active");
    tab.replace_editor = iced::widget::text_editor::Content::with_text("new_name");
    tab.replace_text = "new\nname".to_string();
    tab.case_sensitive = true;
    tab.whole_word = true;
    tab.use_regex = true;
    tab.running = true;
    tab.error = Some("bad regex".to_string());
    tab.limit_reached = true;

    app.active_find_results_tab_id = Some(tab.id.clone());
    app.find_results_tabs.push(tab);

    let element = build_find_results_list(&app);
    assert_eq!(child_count(build_find_results_list(&app)), 3);
    let tree = Tree::new(element.as_widget());
    assert_eq!(
        tree.children[0].children.len(),
        6,
        "controls should contain query, replace, options, scope, run, and error without a summary"
    );
    assert_eq!(
        tree.children[1].children.len(),
        2,
        "the fixed results status should contain the header and limit notice"
    );
}

#[test]
fn find_results_list_builds_non_running_empty_match_state() {
    let mut app = App::new().0;
    let tab = find_tab("active");
    app.active_find_results_tab_id = Some(tab.id.clone());
    app.find_results_tabs.push(tab);

    assert_eq!(child_count(build_find_results_list(&app)), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn find_results_keeps_header_above_scrollable_cards_in_narrow_panel() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.project_path = Some("/tmp/project".to_string());
    let mut tab = find_tab("active");
    tab.matches.push(find_match(
        "/tmp/project/src/a_very_long_module_name.rs",
        18,
        7,
        "let long_preview_content = build_result();",
        7,
    ));
    app.active_find_results_tab_id = Some(tab.id.clone());
    app.find_results_tabs.push(tab);

    let mut element = build_find_results_list(&app);
    let limits = layout::Limits::new(Size::ZERO, Size::new(260.0, 600.0));
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let sections = Layout::new(&node).children().collect::<Vec<_>>();

    assert_eq!(sections.len(), 3, "controls, fixed results header, and scrollable cards");
    assert!(sections[1].bounds().height > 0.0);
    assert!(sections[1].bounds().y + sections[1].bounds().height <= sections[2].bounds().y);
    assert_eq!(FIND_RESULTS_SCROLLBAR_GUTTER, 10.0);
}

#[test]
fn find_results_list_builds_matches_highlight_fallback_and_menu_overlay() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/project".to_string());

    let mut tab = find_tab("active");
    tab.matches = vec![
        find_match("/tmp/project/src/main.rs", 8, 4, "fn main()", 4),
        find_match("/tmp/project/src/lib.rs", 11, 2, "éclair", 2),
    ];

    app.file_tree_menu_path = Some("/tmp/project/src/main.rs".to_string());
    app.file_tree_menu_source = Some("find:/tmp/project/src/main.rs:8:4".to_string());
    app.file_tree_menu_anchor = Some(iced::Point::new(12.0, 24.0));
    app.active_find_results_tab_id = Some(tab.id.clone());
    app.find_results_tabs.push(tab);

    let _ = build_find_results_list(&app);
}

#[test]
fn find_results_list_builds_zero_length_and_out_of_range_previews_without_menu_anchor() {
    let mut app = App::new().0;
    app.project_path = Some("/tmp/project".to_string());

    let mut tab = find_tab("active");
    tab.matches = vec![
        find_match("/tmp/project/src/empty.rs", 1, 1, "empty", 0),
        find_match("/tmp/project/src/short.rs", 2, 99, "short", 4),
        find_match("/tmp/project/src/unicode.rs", 3, 2, "éclair", 1),
    ];

    app.file_tree_menu_path = Some("/tmp/project/src/empty.rs".to_string());
    app.file_tree_menu_source = Some("find:/tmp/project/src/empty.rs:1:1".to_string());
    app.file_tree_menu_anchor = None;
    app.active_find_results_tab_id = Some(tab.id.clone());
    app.find_results_tabs.push(tab);

    std::hint::black_box(build_find_results_list(&app));
}
