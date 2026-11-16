//! 文件搜索领域绘制及排序夹具。
use crate::input_panel::file_search::{self, FileSearchEntry};
use iced::Length;
#[derive(Default)]
struct Fixture {
    project: Option<String>,
    file_search_query: String,
    file_search_selected_index: usize,
    files: Vec<String>,
}
impl Fixture {
    fn set_file_index(&mut self, _: &str, files: Vec<String>) {
        self.files = files;
    }
}
fn overlay(app: &Fixture) -> iced::Element<'static, crate::input_panel::slice_message::Message> {
    let ranked =
        desktop_session::message::chat::input::file_search::build_ranked_file_search_entries(
            &app.files,
            &app.file_search_query,
        );
    let entries = ranked
        .iter()
        .map(|e| FileSearchEntry { path: &e.path, is_dir: e.is_dir })
        .collect::<Vec<_>>();
    file_search::file_search_overlay(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &entries,
        &app.file_search_query,
        app.project.as_deref(),
        app.file_search_selected_index,
        iced::widget::Id::new("search-fixture"),
    )
}
fn file_search_list_len(app: &Fixture) -> usize {
    let overlay = overlay(app);
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
    let mut app = Fixture::default();
    app.file_search_query = "missing".to_string();

    let overlay = overlay(&app);

    assert_eq!(overlay.as_widget().size().width, Length::Fixed(320.0));
    assert_eq!(file_search_list_len(&app), 1);
}

#[test]
fn file_search_overlay_non_empty_results_render_each_ranked_entry() {
    let mut app = Fixture::default();
    app.project = Some("/repo".to_string());
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

    let overlay = overlay(&app);

    assert_eq!(overlay.as_widget().size().width, Length::Fixed(320.0));
    assert_eq!(file_search_list_len(&app), 2);
}

#[test]
fn file_search_overlay_includes_matching_parent_directory_rows() {
    let mut app = Fixture::default();
    app.project = Some("/repo".to_string());
    app.file_search_query = "src".to_string();
    app.set_file_index("/repo", vec!["/repo/src/main.rs".to_string()]);

    assert_eq!(file_search_list_len(&app), 2);
}

#[test]
fn file_search_overlay_limits_large_result_sets_to_twenty_rows() {
    let mut app = Fixture::default();
    app.project = Some("/repo".to_string());
    app.file_search_query = "file".to_string();
    app.set_file_index("/repo", (0..30).map(|i| format!("src/file_{i}.rs")).collect());

    let overlay = overlay(&app);

    assert_eq!(overlay.as_widget().size().width, Length::Fixed(320.0));
    assert_eq!(file_search_list_len(&app), 20);
}
