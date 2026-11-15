use super::*;
#[derive(Default)]
struct Fixture {
    active_preview_path: Option<String>,
    preview_tabs: Vec<PreviewTab>,
    preview_context_target: Option<(String, usize, usize, usize, usize)>,
    preview_trace_back: Vec<(String, usize, usize)>,
    preview_trace_forward: Vec<(String, usize, usize)>,
    preview_trace_navigating: bool,
    pending_preview_goto: Option<(String, usize, usize)>,
    preview_messages: std::collections::HashMap<String, desktop_i18n::format::Message>,
    preview_auto_save_mode: config_ui::PreviewAutoSaveMode,
    show_preview_context_menu: bool,
    preview_context_menu_pos: Option<(f32, f32)>,
    preview_nav_popup: Option<(String, f32, f32, Vec<(String, bool)>)>,
    preview_tab_menu_path: Option<String>,
    preview_tab_menu_pos: Option<iced::Point>,
}
impl Fixture {
 fn state(&mut self) -> TabState<'_> { TabState {
    active_preview_path: &mut self.active_preview_path,
    preview_tabs: &mut self.preview_tabs,
    preview_context_target: &mut self.preview_context_target,
    preview_trace_back: &mut self.preview_trace_back,
    preview_trace_forward: &mut self.preview_trace_forward,
    preview_trace_navigating: &mut self.preview_trace_navigating,
    pending_preview_goto: &mut self.pending_preview_goto,
    preview_messages: &mut self.preview_messages,
    preview_auto_save_mode: &mut self.preview_auto_save_mode,
    show_preview_context_menu: &mut self.show_preview_context_menu,
    preview_context_menu_pos: &mut self.preview_context_menu_pos,
    preview_nav_popup: &mut self.preview_nav_popup,
    preview_tab_menu_path: &mut self.preview_tab_menu_path,
    preview_tab_menu_pos: &mut self.preview_tab_menu_pos,
 } }
}
fn app() -> Fixture { Fixture::default() }
fn preview_tab(path: &str, content: &str) -> PreviewTab {
    PreviewTab {
        path: path.to_string(),
        title: std::path::Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(path)
            .to_string(),
        content: content.to_string(),
        is_dirty: false,
        truncated: false,
        read_only: false,
        auto_save_revision: 0,
        editor: crate::components::editor::Editor::new(content, "txt"),
        scroll_id: iced::widget::Id::unique(),
        #[cfg(not(target_arch = "wasm32"))]
        lsp_server_key: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_uri: None,
        #[cfg(not(target_arch = "wasm32"))]
        lsp_language_id: None,
    }
}

fn has_scheduled_action(task: Task<Effect>) -> bool { task.units() > 0 }
#[test]
fn gateway_target_uses_project_relative_path_for_file_inside_project() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    let project_path = project.path().to_string_lossy().to_string();
    let file_path = project.path().join("src").join("main.rs").to_string_lossy().to_string();
    let target = gateway_preview_target(Some(&project_path), &file_path)
        .expect("path inside project should resolve");

    assert_eq!(target, (project_path, "src/main.rs".to_string()));
}

#[test]
fn gateway_target_joins_relative_path_to_project_and_normalizes_separators() {
    let target = gateway_preview_target(Some("/workspace/project"), "src\\main.rs")
        .expect("relative project path should resolve");

    assert_eq!(target, ("/workspace/project".to_string(), "src/main.rs".to_string()));
}

#[test]
fn gateway_target_falls_back_to_parent_for_absolute_path_outside_project() {
    let project = tempfile::tempdir().expect("temporary project should be created");
    let outside = tempfile::tempdir().expect("temporary outside directory should be created");
    let project_path = project.path().to_string_lossy().to_string();
    let file_path = outside.path().join("notes.txt").to_string_lossy().to_string();
    let target = gateway_preview_target(Some(&project_path), &file_path)
        .expect("absolute path should resolve from its parent");

    assert_eq!(target, (outside.path().to_string_lossy().to_string(), "notes.txt".to_string()));
}

#[test]
fn gateway_target_rejects_path_without_directory_or_file_name() {
    let no_directory = gateway_preview_target(None, "notes.txt")
        .expect_err("bare path without a project should be rejected");
    let temp = tempfile::tempdir().expect("temporary directory should be created");
    let path_without_name = temp.path().join("..").to_string_lossy().to_string();
    let no_file_name = gateway_preview_target(None, &path_without_name)
        .expect_err("path without a file name should be rejected");

    assert!(no_directory.contains("无法确定预览文件目录"));
    assert!(no_file_name.contains("无法确定预览文件名"));
}

#[test]
fn push_trace_entry_deduplicates_adjacent_entries() {
    let entry = ("/tmp/a.rs".to_string(), 3, 5);
    let mut entries = vec![entry.clone()];

    push_trace_entry(&mut entries, entry.clone());

    assert_eq!(entries, vec![entry]);
}

#[test]
fn push_trace_entry_keeps_only_latest_history_limit() {
    let mut entries = Vec::new();
    for line in 0..=TRACE_HISTORY_LIMIT {
        push_trace_entry(&mut entries, ("/tmp/a.rs".to_string(), line, 0));
    }

    assert_eq!(entries.len(), TRACE_HISTORY_LIMIT);
    assert_eq!(entries.first().map(|entry| entry.1), Some(1));
    assert_eq!(entries.last().map(|entry| entry.1), Some(TRACE_HISTORY_LIMIT));
}

#[test]
fn current_preview_location_requires_active_matching_tab() {
    let mut app = app();
    assert!(current_preview_location(&app.state()).is_none());

    app.active_preview_path = Some("/tmp/missing.rs".to_string());
    assert!(current_preview_location(&app.state()).is_none());

    app.preview_tabs.push(preview_tab("/tmp/missing.rs", "fn main() {}"));
    assert_eq!(current_preview_location(&app.state()), Some(("/tmp/missing.rs".to_string(), 0, 0)));
}

#[test]
fn preview_is_dirty_reflects_tab_state() {
    let mut tab = preview_tab("/tmp/a.txt", "a");
    assert!(!preview_is_dirty(&tab));

    tab.is_dirty = true;
    assert!(preview_is_dirty(&tab));
}

#[test]
fn record_trace_before_jump_records_current_and_clears_forward_history() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/current.rs", "current"));
    app.active_preview_path = Some("/tmp/current.rs".to_string());
    app.preview_trace_forward.push(("/tmp/future.rs".to_string(), 8, 2));

    record_trace_before_jump(&mut app.state(), &("/tmp/target.rs".to_string(), 4, 1));

    assert_eq!(app.preview_trace_back, vec![("/tmp/current.rs".to_string(), 0, 0)]);
    assert!(app.preview_trace_forward.is_empty());
}

#[test]
fn record_trace_before_jump_ignores_navigation_and_same_location() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/current.rs", "current"));
    app.active_preview_path = Some("/tmp/current.rs".to_string());
    let current = ("/tmp/current.rs".to_string(), 0, 0);

    record_trace_before_jump(&mut app.state(), &current);
    assert!(app.preview_trace_back.is_empty());

    app.preview_trace_navigating = true;
    record_trace_before_jump(&mut app.state(), &("/tmp/other.rs".to_string(), 1, 1));
    assert!(app.preview_trace_back.is_empty());
}

#[test]
fn navigate_trace_back_skips_current_and_prepares_previous_target() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/current.rs", "current"));
    app.active_preview_path = Some("/tmp/current.rs".to_string());
    app.preview_trace_back =
        vec![("/tmp/previous.rs".to_string(), 7, 3), ("/tmp/current.rs".to_string(), 0, 0)];

    let task = navigate_trace_back(&mut app.state());

    assert_eq!(app.pending_preview_goto, Some(("/tmp/previous.rs".to_string(), 7, 3)));
    assert_eq!(app.preview_trace_forward, vec![("/tmp/current.rs".to_string(), 0, 0)]);
    assert!(app.preview_trace_navigating);
    assert!(has_scheduled_action(task));
}

#[test]
fn navigate_trace_forward_prepares_next_target_and_records_current() {
    let mut app = app();
    app.preview_tabs.push(preview_tab("/tmp/current.rs", "current"));
    app.active_preview_path = Some("/tmp/current.rs".to_string());
    app.preview_trace_forward = vec![("/tmp/next.rs".to_string(), 9, 4)];

    let task = navigate_trace_forward(&mut app.state());

    assert_eq!(app.pending_preview_goto, Some(("/tmp/next.rs".to_string(), 9, 4)));
    assert_eq!(app.preview_trace_back, vec![("/tmp/current.rs".to_string(), 0, 0)]);
    assert!(app.preview_trace_navigating);
    assert!(has_scheduled_action(task));
}

#[test]
fn navigate_trace_with_empty_history_stays_idle() {
    let mut app = app();

    assert!(!has_scheduled_action(navigate_trace_back(&mut app.state())));
    assert!(!has_scheduled_action(navigate_trace_forward(&mut app.state())));
    assert!(!app.preview_trace_navigating);
    assert!(app.pending_preview_goto.is_none());
}

#[test]
fn navigate_trace_without_current_location_opens_target_without_recording_opposite_history() {
    let mut app = app();
    app.preview_trace_back = vec![("/tmp/previous.rs".to_string(), 6, 2)];

    let task = navigate_trace_back(&mut app.state());

    assert_eq!(app.pending_preview_goto, Some(("/tmp/previous.rs".to_string(), 6, 2)));
    assert!(app.preview_trace_forward.is_empty());
    assert!(app.preview_trace_navigating);
    assert!(has_scheduled_action(task));
}

#[test]
fn dismiss_preview_popup_menus_clears_all_preview_menu_state() {
    let mut app = app();
    app.show_preview_context_menu = true;
    app.preview_context_menu_pos = Some((1.0, 2.0));
    app.preview_nav_popup = Some((
        "/tmp".to_string(),
        3.0,
        4.0,
        vec![("src".to_string(), true), ("main.rs".to_string(), false)],
    ));
    app.preview_tab_menu_path = Some("/tmp/main.rs".to_string());
    app.preview_tab_menu_pos = Some(iced::Point::new(5.0, 6.0));

    dismiss_preview_popup_menus(&mut app.state());

    assert!(!app.show_preview_context_menu);
    assert!(app.preview_context_menu_pos.is_none());
    assert!(app.preview_nav_popup.is_none());
    assert!(app.preview_tab_menu_path.is_none());
    assert!(app.preview_tab_menu_pos.is_none());
}
