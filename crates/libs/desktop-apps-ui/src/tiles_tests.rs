//! 应用中心磁贴测试，验证响应式列数、筛选匹配和内置操作布局。
//! 极窄或退化尺寸至少保留一列，完整行与尾行都不得丢失书签磁贴。

struct App { window_size: (f32, f32), apps_search_query: String, web_bookmarks: Vec<WebBookmark> }
struct WebBookmark { title: String, url: String, width: Option<i32>, height: Option<i32> }
fn test_app() -> App { App { window_size: (800.0, 700.0), apps_search_query: String::new(), web_bookmarks: vec![] } }
fn render_tiles_grid(app: &App, blocked: bool) -> iced::Element<'static, super::Event> {
    super::render_tiles_grid(app.window_size.0, &app.apps_search_query, desktop_i18n::I18n::new(Default::default()), app.web_bookmarks.iter().map(|bm| super::Bookmark { title: &bm.title, url: &bm.url, width: bm.width, height: bm.height }), blocked)
}
fn render_header(app: &App, blocked: bool) -> iced::Element<'static, super::Event> {
    super::render_header(&app.apps_search_query, desktop_i18n::I18n::new(Default::default()), blocked)
}
fn bookmark(title: &str, url: &str) -> WebBookmark {
    WebBookmark {
        title: title.to_string(),
        url: url.to_string(),
        width: Some(900),
        height: Some(700),
    }
}

fn grid_row_tile_counts(app: &App, blocked: bool) -> Vec<usize> {
    render_tiles_grid(app, blocked)
        .as_widget()
        .children()
        .into_iter()
        .map(|row| row.children.len())
        .collect()
}

fn clear_control_child_count(app: &App, blocked: bool) -> usize {
    let header_children = render_header(app, blocked).as_widget().children();
    let search_box = &header_children[2];
    let clear_overlay = &search_box.children[1];
    clear_overlay.children.len()
}

fn first_tile_action_count(app: &App) -> usize {
    let grid_children = render_tiles_grid(app, false).as_widget().children();
    let tile_button = &grid_children[0].children[0];
    let tile_content = &tile_button.children[0];
    tile_content.children[2].children.len()
}

#[test]
fn compute_tile_columns_clamps_widths_below_one_tile_to_one() {
    let mut app = test_app();

    for width in [-100.0, 0.0, 47.0, 100.0, 215.0] {
        app.window_size = (width, 700.0);
        assert_eq!(super::compute_tile_columns(app.window_size.0, 168.0, 20.0), 1);
    }
}

#[test]
fn compute_tile_columns_changes_at_exact_three_column_threshold() {
    let mut app = test_app();

    app.window_size = (591.9, 700.0);
    assert_eq!(super::compute_tile_columns(app.window_size.0, 168.0, 20.0), 2);

    app.window_size = (592.0, 700.0);
    assert_eq!(super::compute_tile_columns(app.window_size.0, 168.0, 20.0), 3);
}

#[test]
fn compute_tile_columns_clamps_degenerate_dimensions_to_one() {
    let mut app = test_app();
    app.window_size = (0.0, 700.0);

    assert_eq!(super::compute_tile_columns(app.window_size.0, 0.0, 0.0), 1);
}

#[test]
fn render_tiles_grid_arranges_builtin_tiles_across_complete_and_partial_rows() {
    let mut app = test_app();
    app.window_size = (800.0, 700.0);

    assert_eq!(grid_row_tile_counts(&app, false), vec![4, 2]);
}

#[test]
fn render_tiles_grid_matches_english_knowledge_query_case_insensitively() {
    let mut app = test_app();
    app.apps_search_query = "  KNOWLEDGE  ".to_string();

    assert_eq!(grid_row_tile_counts(&app, false), vec![1]);
}

#[test]
fn project_management_tile_uses_new_title_and_exposes_three_actions() {
    let mut app = test_app();
    app.apps_search_query = "项目管理".to_string();

    assert_eq!(grid_row_tile_counts(&app, false), vec![1]);
    assert_eq!(first_tile_action_count(&app), 3);
    assert!(super::builtin_tile_titles().contains(&"项目管理"));
    assert!(!super::builtin_tile_titles().contains(&"历史项目"));
}

#[test]
fn render_tiles_grid_matches_bookmarks_by_title_or_url() {
    let mut app = test_app();
    app.window_size = (800.0, 700.0);
    app.apps_search_query = "docs".to_string();
    app.web_bookmarks = vec![
        bookmark("Docs", "https://example.com"),
        bookmark("Other", "https://docs.example.com"),
        bookmark("Unrelated", "https://example.org"),
    ];

    assert_eq!(grid_row_tile_counts(&app, false), vec![2]);
}

#[test]
fn render_tiles_grid_uses_url_for_blank_bookmark_title() {
    let mut app = test_app();
    app.apps_search_query = "portal".to_string();
    app.web_bookmarks = vec![bookmark("  ", "https://portal.example.com")];

    assert_eq!(grid_row_tile_counts(&app, false), vec![1]);
}

#[test]
fn render_tiles_grid_returns_no_rows_when_nothing_matches() {
    let mut app = test_app();
    app.apps_search_query = "no such tile".to_string();
    app.web_bookmarks = vec![bookmark("Docs", "https://example.com")];

    assert!(grid_row_tile_counts(&app, false).is_empty());
}

#[test]
fn render_tiles_grid_blocking_preserves_filtered_layout() {
    let mut app = test_app();
    app.apps_search_query = "工作流".to_string();

    assert_eq!(grid_row_tile_counts(&app, true), grid_row_tile_counts(&app, false));
    assert_eq!(grid_row_tile_counts(&app, true), vec![1]);
}

#[test]
fn render_header_hides_clear_button_for_blank_or_blocked_search() {
    let mut app = test_app();
    app.apps_search_query = "   ".to_string();
    assert_eq!(clear_control_child_count(&app, false), 0);

    app.apps_search_query = "docs".to_string();
    assert_eq!(clear_control_child_count(&app, true), 0);
}

#[test]
fn render_header_shows_clear_button_for_nonempty_unblocked_search() {
    let mut app = test_app();
    app.apps_search_query = "docs".to_string();

    assert_eq!(clear_control_child_count(&app, false), 1);
}

#[test]
fn builtin_tiles_omit_design_entry() {
    assert!(!super::builtin_tile_titles().contains(&concat!("设", "计")));
}

#[test]
fn third_party_agents_tile_is_searchable_as_an_independent_tool() {
    let mut app = test_app();
    app.apps_search_query = "第三方智能体".into();
    assert_eq!(grid_row_tile_counts(&app, false), vec![1]);
}
