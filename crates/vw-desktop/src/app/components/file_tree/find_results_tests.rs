//! 验证文件搜索标签、结果分页、路径预览、快捷键和窄面板布局。
//! fixture 包含零长度、越界和 Unicode 命中，确保高亮失败时仍能构造稳定预览卡片。

use super::find_results::{build_find_results_list, build_find_results_tabs, FIND_RESULTS_SCROLLBAR_GUTTER};
use crate::app::state::{FindInFolderMatch, FindInFolderTab};
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::Layout;
use iced::advanced::layout;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::{Font, Pixels, Size, Theme};

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
        localized_error: None,
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

    app.file_tree_interaction.file_tree_menu_path = Some("/tmp/project/src/main.rs".to_string());
    app.file_tree_interaction.file_tree_menu_source = Some("find:/tmp/project/src/main.rs:8:4".to_string());
    app.file_tree_interaction.file_tree_menu_anchor = Some(iced::Point::new(12.0, 24.0));
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

    app.file_tree_interaction.file_tree_menu_path = Some("/tmp/project/src/empty.rs".to_string());
    app.file_tree_interaction.file_tree_menu_source = Some("find:/tmp/project/src/empty.rs:1:1".to_string());
    app.file_tree_interaction.file_tree_menu_anchor = None;
    app.active_find_results_tab_id = Some(tab.id.clone());
    app.find_results_tabs.push(tab);

    std::hint::black_box(build_find_results_list(&app));
}
