//! 验证文件搜索路径展示、结果上限、父目录行与选中样式。
//! 根目录归一化和二十项上限用例锁定展示层的截断策略。

use super::file_search::file_search_overlay;
use crate::app::App;

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
