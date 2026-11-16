//! Local LSP state behavior, independent of desktop App and message mapping.
#![cfg(not(target_arch = "wasm32"))]
use crate::preview::lsp as domain;
use crate::preview::{LspHoverPending, LspProgress, PreviewTab};
use desktop_lsp::LspServiceManager;
use std::collections::HashMap;
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
        lsp_server_key: None,
        lsp_uri: None,
        lsp_language_id: None,
    }
}

#[test]
fn sync_lsp_for_path_rejects_disabled_lsp_and_detaches_matching_tab() {
    let mut notifications = Vec::new();
    let mut lsp_manager: Option<LspServiceManager> = Default::default();
    let mut lsp_status: Option<String> = Default::default();
    let mut lsp_status_message: Option<(String, desktop_i18n::format::Message)> =
        Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut project_path: Option<String> = Default::default();
    let mut lsp_disabled: bool = true;
    let mut tab = preview_tab("/tmp/main.rs", "fn main() {}");
    tab.lsp_server_key = Some("rust-analyzer");
    tab.lsp_uri = Some("file:///tmp/main.rs".to_string());
    tab.lsp_language_id = Some("rust".to_string());
    preview_tabs.push(tab);

    let synced = domain::sync_lsp_for_path(
        &mut lsp_disabled,
        &mut lsp_manager,
        &mut lsp_status,
        &mut lsp_status_message,
        &mut notifications,
        &mut preview_tabs,
        &mut project_path,
        "/tmp/main.rs",
    );

    assert!(!synced);
    assert!(preview_tabs[0].lsp_server_key.is_none());
    assert!(preview_tabs[0].lsp_uri.is_none());
    assert!(preview_tabs[0].lsp_language_id.is_none());
}

#[test]
fn sync_lsp_for_path_rejects_missing_tab_without_changing_status() {
    let mut notifications = Vec::new();
    let mut lsp_manager: Option<LspServiceManager> = Default::default();
    let mut lsp_status_message: Option<(String, desktop_i18n::format::Message)> =
        Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut project_path: Option<String> = Default::default();
    let mut lsp_disabled: bool = false;
    let mut lsp_status: Option<String> = Some("unchanged".to_string());

    let synced = domain::sync_lsp_for_path(
        &mut lsp_disabled,
        &mut lsp_manager,
        &mut lsp_status,
        &mut lsp_status_message,
        &mut notifications,
        &mut preview_tabs,
        &mut project_path,
        "/tmp/missing.rs",
    );

    assert!(!synced);
    assert_eq!(lsp_status.as_deref(), Some("unchanged"));
}

#[test]
fn sync_lsp_for_path_rejects_unknown_language_and_detaches_metadata() {
    let mut notifications = Vec::new();
    let mut lsp_manager: Option<LspServiceManager> = Default::default();
    let mut lsp_status: Option<String> = Default::default();
    let mut lsp_status_message: Option<(String, desktop_i18n::format::Message)> =
        Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut project_path: Option<String> = Default::default();
    let mut lsp_disabled: bool = false;
    let mut tab = preview_tab("/tmp/data.unknown-language", "content");
    tab.lsp_server_key = Some("rust-analyzer");
    tab.lsp_uri = Some("file:///tmp/data.unknown-language".to_string());
    tab.lsp_language_id = Some("rust".to_string());
    preview_tabs.push(tab);

    let synced = domain::sync_lsp_for_path(
        &mut lsp_disabled,
        &mut lsp_manager,
        &mut lsp_status,
        &mut lsp_status_message,
        &mut notifications,
        &mut preview_tabs,
        &mut project_path,
        "/tmp/data.unknown-language",
    );

    assert!(!synced);
    assert!(preview_tabs[0].lsp_server_key.is_none());
    assert!(preview_tabs[0].lsp_uri.is_none());
    assert!(preview_tabs[0].lsp_language_id.is_none());
}

#[test]
fn sync_lsp_for_path_rejects_known_language_without_manager_after_detach() {
    let mut notifications = Vec::new();
    let mut lsp_status: Option<String> = Default::default();
    let mut lsp_status_message: Option<(String, desktop_i18n::format::Message)> =
        Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut project_path: Option<String> = Default::default();
    let mut lsp_disabled: bool = false;
    let mut lsp_manager: Option<LspServiceManager> = None;
    let mut tab = preview_tab("/tmp/main.rs", "fn main() {}");
    tab.lsp_server_key = Some("rust-analyzer");
    tab.lsp_uri = Some("file:///tmp/main.rs".to_string());
    tab.lsp_language_id = Some("rust".to_string());
    preview_tabs.push(tab);

    let synced = domain::sync_lsp_for_path(
        &mut lsp_disabled,
        &mut lsp_manager,
        &mut lsp_status,
        &mut lsp_status_message,
        &mut notifications,
        &mut preview_tabs,
        &mut project_path,
        "/tmp/main.rs",
    );

    assert!(!synced);
    assert!(preview_tabs[0].lsp_server_key.is_none());
    assert!(preview_tabs[0].lsp_uri.is_none());
    assert!(preview_tabs[0].lsp_language_id.is_none());
}

#[test]
fn enable_lsp_without_active_preview_only_marks_lsp_enabled() {
    let mut notifications = Vec::new();
    let mut lsp_status: Option<String> = Default::default();
    let mut lsp_status_message: Option<(String, desktop_i18n::format::Message)> =
        Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut project_path: Option<String> = Default::default();
    let mut lsp_disabled: bool = true;
    let mut lsp_manager: Option<LspServiceManager> = None;
    let mut active_preview_path: Option<String> = None;

    domain::enable_lsp(
        &mut active_preview_path,
        &mut lsp_disabled,
        &mut lsp_manager,
        &mut lsp_status,
        &mut lsp_status_message,
        &mut notifications,
        &mut preview_tabs,
        &mut project_path,
    );

    assert!(!lsp_disabled);
    assert_eq!(lsp_status.as_deref(), Some("LSP 已启用"));
}

#[test]
fn enable_lsp_with_active_unknown_language_marks_enabled_and_detaches_tab() {
    let mut notifications = Vec::new();
    let mut lsp_status: Option<String> = Default::default();
    let mut lsp_status_message: Option<(String, desktop_i18n::format::Message)> =
        Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut project_path: Option<String> = Default::default();
    let mut lsp_disabled: bool = true;
    let mut lsp_manager: Option<LspServiceManager> = None;
    let mut active_preview_path: Option<String> = Some("/tmp/data.unknown-language".to_string());
    let mut tab = preview_tab("/tmp/data.unknown-language", "content");
    tab.lsp_server_key = Some("rust-analyzer");
    tab.lsp_uri = Some("file:///tmp/data.unknown-language".to_string());
    tab.lsp_language_id = Some("rust".to_string());
    preview_tabs.push(tab);

    domain::enable_lsp(
        &mut active_preview_path,
        &mut lsp_disabled,
        &mut lsp_manager,
        &mut lsp_status,
        &mut lsp_status_message,
        &mut notifications,
        &mut preview_tabs,
        &mut project_path,
    );

    assert!(!lsp_disabled);
    assert_eq!(lsp_status.as_deref(), Some("LSP 已启用"));
    assert!(preview_tabs[0].lsp_server_key.is_none());
}

#[test]
fn clear_lsp_hover_clears_hover_state_and_path_when_completion_hidden() {
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    lsp_overlay.show_hover("hover".to_string());
    let mut lsp_overlay_path: Option<String> = Some("/tmp/main.rs".to_string());
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> =
        Some(("/tmp/main.rs".to_string(), iced_code_editor::LspPosition { line: 1, character: 2 }));
    let mut lsp_hover_pending: Option<LspHoverPending> = Some(LspHoverPending {
        path: "/tmp/main.rs".to_string(),
        position: iced_code_editor::LspPosition { line: 1, character: 2 },
        point: iced::Point::new(10.0, 20.0),
        ready_at: std::time::Instant::now(),
    });
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> = Some(std::time::Instant::now());

    domain::clear_lsp_hover(
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
    );

    assert!(!lsp_overlay.hover_visible);
    assert!(lsp_hover_anchor.is_none());
    assert!(lsp_hover_pending.is_none());
    assert!(lsp_hover_hide_deadline.is_none());
    assert!(lsp_overlay_path.is_none());
}

#[test]
fn clear_lsp_hover_preserves_path_while_completion_is_visible() {
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> = Default::default();
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> = Default::default();
    let mut lsp_hover_pending: Option<LspHoverPending> = Default::default();
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    lsp_overlay.show_hover("hover".to_string());
    lsp_overlay.completion_visible = true;
    let mut lsp_overlay_path: Option<String> = Some("/tmp/main.rs".to_string());

    domain::clear_lsp_hover(
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
    );

    assert!(!lsp_overlay.hover_visible);
    assert_eq!(lsp_overlay_path.as_deref(), Some("/tmp/main.rs"));
}

#[test]
fn clear_lsp_completion_clears_items_and_preserves_hover_path() {
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    lsp_overlay.all_completions = vec!["Alpha".to_string(), "Beta".to_string()];
    lsp_overlay.completion_items = vec!["Alpha".to_string()];
    lsp_overlay.completion_filter = "a".to_string();
    lsp_overlay.completion_visible = true;
    lsp_overlay.completion_selected = 9;
    lsp_overlay.hover_visible = true;
    let mut lsp_overlay_path: Option<String> = Some("/tmp/main.rs".to_string());

    domain::clear_lsp_completion(&mut lsp_overlay, &mut lsp_overlay_path, true);

    assert!(lsp_overlay.all_completions.is_empty());
    assert!(lsp_overlay.completion_items.is_empty());
    assert!(lsp_overlay.completion_filter.is_empty());
    assert!(!lsp_overlay.completion_visible);
    assert!(lsp_overlay.completion_suppressed);
    assert_eq!(lsp_overlay.completion_selected, 0);
    assert_eq!(lsp_overlay_path.as_deref(), Some("/tmp/main.rs"));
}

#[test]
fn clear_lsp_completion_clears_path_when_hover_is_hidden() {
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    lsp_overlay.completion_visible = true;
    lsp_overlay.completion_suppressed = true;
    lsp_overlay.hover_visible = false;
    let mut lsp_overlay_path: Option<String> = Some("/tmp/main.rs".to_string());

    domain::clear_lsp_completion(&mut lsp_overlay, &mut lsp_overlay_path, false);

    assert!(!lsp_overlay.completion_suppressed);
    assert!(lsp_overlay_path.is_none());
}

#[test]
fn filter_completions_matches_case_insensitively_and_clamps_selection() {
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    lsp_overlay.all_completions =
        vec!["ReadFile".to_string(), "write_file".to_string(), "Close".to_string()];
    lsp_overlay.completion_filter = "FILE".to_string();
    lsp_overlay.completion_selected = 8;

    domain::filter_completions(&mut lsp_overlay);

    assert_eq!(lsp_overlay.completion_filter, "file");
    assert_eq!(
        lsp_overlay.completion_items,
        vec!["ReadFile".to_string(), "write_file".to_string()]
    );
    assert!(lsp_overlay.completion_visible);
    assert_eq!(lsp_overlay.completion_selected, 1);
}

#[test]
fn filter_completions_hides_empty_result() {
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    lsp_overlay.all_completions = vec!["Alpha".to_string()];
    lsp_overlay.completion_filter = "zzz".to_string();
    lsp_overlay.completion_selected = 4;

    domain::filter_completions(&mut lsp_overlay);

    assert!(lsp_overlay.completion_items.is_empty());
    assert!(!lsp_overlay.completion_visible);
    assert_eq!(lsp_overlay.completion_selected, 0);
}

#[test]
fn filter_completions_with_empty_filter_restores_all_items_and_keeps_valid_selection() {
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    lsp_overlay.all_completions = vec!["Alpha".to_string(), "Beta".to_string()];
    lsp_overlay.completion_items = vec!["stale".to_string()];
    lsp_overlay.completion_filter = String::new();
    lsp_overlay.completion_selected = 1;

    domain::filter_completions(&mut lsp_overlay);

    assert_eq!(lsp_overlay.completion_items, lsp_overlay.all_completions);
    assert!(lsp_overlay.completion_visible);
    assert_eq!(lsp_overlay.completion_selected, 1);
}

#[test]
fn apply_completion_without_matching_active_preview_preserves_editor_content() {
    let mut active_preview_path: Option<String> = Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    preview_tabs.push(preview_tab("/tmp/main.rs", "let value = 1;"));
    let original = preview_tabs[0].editor.content();

    domain::apply_completion(&mut active_preview_path, &mut preview_tabs, "replacement");
    assert_eq!(preview_tabs[0].editor.content(), original);

    active_preview_path = Some("/tmp/missing.rs".to_string());
    domain::apply_completion(&mut active_preview_path, &mut preview_tabs, "replacement");

    assert_eq!(preview_tabs[0].editor.content(), original);
}

#[test]
fn process_lsp_hover_timers_clears_visible_hover_without_path() {
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> = Default::default();
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> = Default::default();
    let mut lsp_hover_pending: Option<LspHoverPending> = Default::default();
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    lsp_overlay.show_hover("hover".to_string());
    let mut lsp_overlay_path: Option<String> = None;

    domain::process_lsp_hover_timers(
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
        &mut preview_tabs,
    );

    assert!(!lsp_overlay.hover_visible);
    assert!(lsp_hover_anchor.is_none());
}

#[test]
fn process_lsp_hover_timers_retains_request_before_deadline() {
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> = Default::default();
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> = Default::default();
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    let mut lsp_overlay_path: Option<String> = Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut lsp_hover_pending: Option<LspHoverPending> = Some(LspHoverPending {
        path: "/tmp/main.rs".to_string(),
        position: iced_code_editor::LspPosition { line: 2, character: 3 },
        point: iced::Point::new(8.0, 9.0),
        ready_at: std::time::Instant::now() + std::time::Duration::from_secs(60),
    });

    domain::process_lsp_hover_timers(
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
        &mut preview_tabs,
    );

    let pending = lsp_hover_pending.expect("request should remain pending");
    assert_eq!(pending.path, "/tmp/main.rs");
    assert_eq!(pending.position, iced_code_editor::LspPosition { line: 2, character: 3 });
}

#[test]
fn process_lsp_hover_timers_clears_anchor_when_ready_tab_is_missing() {
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> = Default::default();
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    let mut lsp_overlay_path: Option<String> = Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let position = iced_code_editor::LspPosition { line: 2, character: 3 };
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> =
        Some(("/tmp/main.rs".to_string(), position));
    let mut lsp_hover_pending: Option<LspHoverPending> = Some(LspHoverPending {
        path: "/tmp/main.rs".to_string(),
        position,
        point: iced::Point::new(8.0, 9.0),
        ready_at: std::time::Instant::now() - std::time::Duration::from_secs(1),
    });

    domain::process_lsp_hover_timers(
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
        &mut preview_tabs,
    );

    assert!(lsp_hover_pending.is_none());
    assert!(lsp_hover_anchor.is_none());
    assert!(lsp_overlay_path.is_none());
}

#[test]
fn process_lsp_hover_timers_hides_expired_noninteractive_hover() {
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> = Default::default();
    let mut lsp_hover_pending: Option<LspHoverPending> = Default::default();
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    lsp_overlay.show_hover("hover".to_string());
    lsp_overlay.hover_interactive = false;
    let mut lsp_overlay_path: Option<String> = Some("/tmp/main.rs".to_string());
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(1));

    domain::process_lsp_hover_timers(
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
        &mut preview_tabs,
    );

    assert!(!lsp_overlay.hover_visible);
    assert!(lsp_hover_hide_deadline.is_none());
    assert!(lsp_overlay_path.is_none());
}

#[test]
fn process_lsp_hover_timers_preserves_expired_interactive_hover() {
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> = Default::default();
    let mut lsp_hover_pending: Option<LspHoverPending> = Default::default();
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    lsp_overlay.show_hover("hover".to_string());
    lsp_overlay.hover_interactive = true;
    let mut lsp_overlay_path: Option<String> = Some("/tmp/main.rs".to_string());
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(1));

    domain::process_lsp_hover_timers(
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
        &mut preview_tabs,
    );

    assert!(lsp_overlay.hover_visible);
    assert!(lsp_overlay.hover_interactive);
    assert!(lsp_hover_hide_deadline.is_some());
    assert_eq!(lsp_overlay_path.as_deref(), Some("/tmp/main.rs"));
}

#[test]
fn handle_lsp_hover_from_mouse_ignores_missing_active_preview() {
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> = Default::default();
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> = Default::default();
    let mut lsp_hover_pending: Option<LspHoverPending> = Default::default();
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    let mut lsp_overlay_path: Option<String> = Default::default();
    let mut preview_nav_popup: Option<(String, f32, f32, Vec<(String, bool)>)> = Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut show_preview_context_menu: bool = Default::default();
    let mut active_preview_path: Option<String> = None;

    domain::handle_lsp_hover_from_mouse(
        &mut active_preview_path,
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
        &mut preview_nav_popup,
        &mut preview_tabs,
        &mut show_preview_context_menu,
        iced::Point::new(10.0, 20.0),
    );

    assert!(lsp_hover_pending.is_none());
    assert!(lsp_hover_anchor.is_none());
}

#[test]
fn handle_lsp_hover_from_mouse_ignores_context_menu() {
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> = Default::default();
    let mut lsp_hover_pending: Option<LspHoverPending> = Default::default();
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    let mut lsp_overlay_path: Option<String> = Default::default();
    let mut preview_nav_popup: Option<(String, f32, f32, Vec<(String, bool)>)> = Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut show_preview_context_menu: bool = true;
    let mut active_preview_path: Option<String> = Some("/tmp/main.rs".to_string());
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> = Some(std::time::Instant::now());

    domain::handle_lsp_hover_from_mouse(
        &mut active_preview_path,
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
        &mut preview_nav_popup,
        &mut preview_tabs,
        &mut show_preview_context_menu,
        iced::Point::new(10.0, 20.0),
    );

    assert!(lsp_hover_hide_deadline.is_some());
    assert!(lsp_hover_pending.is_none());
}

#[test]
fn handle_lsp_hover_from_mouse_releases_stale_interactive_overlay() {
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> = Default::default();
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> = Default::default();
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    let mut preview_nav_popup: Option<(String, f32, f32, Vec<(String, bool)>)> = Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut show_preview_context_menu: bool = Default::default();
    let mut active_preview_path: Option<String> = Some("/tmp/main.rs".to_string());
    lsp_overlay.show_hover("hover".to_string());
    lsp_overlay.hover_interactive = true;
    let mut lsp_overlay_path: Option<String> = Some("/tmp/other.rs".to_string());
    let mut lsp_hover_pending: Option<LspHoverPending> = Some(LspHoverPending {
        path: "/tmp/other.rs".to_string(),
        position: iced_code_editor::LspPosition { line: 1, character: 1 },
        point: iced::Point::new(1.0, 1.0),
        ready_at: std::time::Instant::now(),
    });

    domain::handle_lsp_hover_from_mouse(
        &mut active_preview_path,
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
        &mut preview_nav_popup,
        &mut preview_tabs,
        &mut show_preview_context_menu,
        iced::Point::new(10.0, 20.0),
    );

    assert!(!lsp_overlay.hover_interactive);
    assert!(lsp_hover_pending.is_none());
    assert!(lsp_hover_anchor.is_none());
}

#[test]
fn handle_lsp_hover_from_mouse_keeps_visible_hover_for_same_path_without_anchor() {
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> = Default::default();
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> = Default::default();
    let mut lsp_hover_pending: Option<LspHoverPending> = Default::default();
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    let mut preview_nav_popup: Option<(String, f32, f32, Vec<(String, bool)>)> = Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut show_preview_context_menu: bool = Default::default();
    let mut active_preview_path: Option<String> = Some("/tmp/main.rs".to_string());
    lsp_overlay.show_hover("hover".to_string());
    let mut lsp_overlay_path: Option<String> = Some("/tmp/main.rs".to_string());

    domain::handle_lsp_hover_from_mouse(
        &mut active_preview_path,
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
        &mut preview_nav_popup,
        &mut preview_tabs,
        &mut show_preview_context_menu,
        iced::Point::new(10.0, 20.0),
    );

    assert!(lsp_overlay.hover_visible);
    assert!(lsp_hover_hide_deadline.is_none());
}

#[test]
fn handle_lsp_hover_from_mouse_schedules_hide_for_other_visible_hover_without_anchor() {
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> = Default::default();
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> = Default::default();
    let mut lsp_hover_pending: Option<LspHoverPending> = Default::default();
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    let mut preview_nav_popup: Option<(String, f32, f32, Vec<(String, bool)>)> = Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut show_preview_context_menu: bool = Default::default();
    let mut active_preview_path: Option<String> = Some("/tmp/main.rs".to_string());
    lsp_overlay.show_hover("hover".to_string());
    let mut lsp_overlay_path: Option<String> = Some("/tmp/other.rs".to_string());

    domain::handle_lsp_hover_from_mouse(
        &mut active_preview_path,
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
        &mut preview_nav_popup,
        &mut preview_tabs,
        &mut show_preview_context_menu,
        iced::Point::new(10.0, 20.0),
    );

    assert!(lsp_overlay.hover_visible);
    assert!(lsp_hover_hide_deadline.is_some());
}

#[test]
fn handle_lsp_hover_from_mouse_keeps_pending_deadline_for_same_symbol() {
    let path = "/tmp/main.rs";
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> = Default::default();
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> = Default::default();
    let mut lsp_hover_pending: Option<LspHoverPending> = Default::default();
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    let mut lsp_overlay_path: Option<String> = Default::default();
    let mut preview_nav_popup: Option<(String, f32, f32, Vec<(String, bool)>)> = Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut show_preview_context_menu: bool = Default::default();
    let mut active_preview_path: Option<String> = Some(path.to_string());
    preview_tabs.push(preview_tab(path, "fn main() {}"));
    let point = iced::Point::new(100.0, 10.0);

    domain::handle_lsp_hover_from_mouse(
        &mut active_preview_path,
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
        &mut preview_nav_popup,
        &mut preview_tabs,
        &mut show_preview_context_menu,
        point,
    );
    let first_ready_at =
        lsp_hover_pending.as_ref().expect("first hover should schedule a request").ready_at;

    domain::handle_lsp_hover_from_mouse(
        &mut active_preview_path,
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
        &mut preview_nav_popup,
        &mut preview_tabs,
        &mut show_preview_context_menu,
        point,
    );

    assert_eq!(
        lsp_hover_pending
            .as_ref()
            .expect("same-symbol hover should keep the pending request")
            .ready_at,
        first_ready_at
    );
}

#[test]
fn disable_lsp_detaches_state_and_marks_status() {
    let mut lsp_hover_anchor: Option<(String, iced_code_editor::LspPosition)> = Default::default();
    let mut lsp_hover_hide_deadline: Option<std::time::Instant> = Default::default();
    let mut lsp_hover_pending: Option<LspHoverPending> = Default::default();
    let mut lsp_manager: Option<LspServiceManager> = Default::default();
    let mut lsp_overlay: iced_code_editor::LspOverlayState =
        iced_code_editor::LspOverlayState::new();
    let mut lsp_progress: HashMap<String, HashMap<String, LspProgress>> = Default::default();
    let mut lsp_status: Option<String> = Default::default();
    let mut lsp_status_message: Option<(String, desktop_i18n::format::Message)> =
        Default::default();
    let mut preview_tabs: Vec<PreviewTab> = Default::default();
    let mut lsp_disabled: bool = false;
    lsp_overlay.show_hover("hover".to_string());
    lsp_overlay.all_completions = vec!["item".to_string()];
    lsp_overlay.completion_items = vec!["item".to_string()];
    lsp_overlay.completion_visible = true;
    let mut lsp_overlay_path: Option<String> = Some("/tmp/main.rs".to_string());
    lsp_progress.entry("rust-analyzer".to_string()).or_default().insert(
        "token".to_string(),
        LspProgress { title: "index".to_string(), message: None, percentage: Some(10) },
    );
    let mut tab = preview_tab("/tmp/main.rs", "fn main() {}");
    tab.lsp_server_key = Some("rust-analyzer");
    tab.lsp_uri = Some("file:///tmp/main.rs".to_string());
    tab.lsp_language_id = Some("rust".to_string());
    preview_tabs.push(tab);

    domain::disable_lsp(
        &mut lsp_disabled,
        &mut lsp_hover_anchor,
        &mut lsp_hover_hide_deadline,
        &mut lsp_hover_pending,
        &mut lsp_manager,
        &mut lsp_overlay,
        &mut lsp_overlay_path,
        &mut lsp_progress,
        &mut lsp_status,
        &mut lsp_status_message,
        &mut preview_tabs,
    );

    assert!(lsp_disabled);
    assert!(!lsp_overlay.hover_visible);
    assert!(!lsp_overlay.completion_visible);
    assert!(lsp_progress.is_empty());
    assert_eq!(lsp_status.as_deref(), Some("LSP 已禁用"));
    assert!(preview_tabs[0].lsp_server_key.is_none());
}
