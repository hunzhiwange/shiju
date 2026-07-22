use super::find_results::{
    build_find_results_list, build_find_results_tabs, find_match_highlight_bg, to_relative_path,
};
use crate::app::state::{FindInFolderMatch, FindInFolderTab};
use crate::app::{App, Message};
use iced::Element;
use iced::Theme;
use iced::advanced::widget::Tree;

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
fn find_results_tabs_builds_empty_and_populated_tab_bar() {
    let empty = App::new().0;
    assert_eq!(child_count(build_find_results_tabs(&empty)), 1);

    let mut app = App::new().0;
    app.find_results_tabs.push(find_tab("first"));
    app.find_results_tabs.push(find_tab("second"));
    app.find_results_tabs[1].matches.push(find_match(
        "/tmp/project/src/main.rs",
        3,
        4,
        "fn main()",
        4,
    ));
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

    assert_eq!(child_count(build_find_results_list(&app)), 2);
}

#[test]
fn find_results_list_builds_non_running_empty_match_state() {
    let mut app = App::new().0;
    let tab = find_tab("active");
    app.active_find_results_tab_id = Some(tab.id.clone());
    app.find_results_tabs.push(tab);

    assert_eq!(child_count(build_find_results_list(&app)), 2);
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
