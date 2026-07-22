#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("tests"));
}

#[test]
fn dismiss_preview_popup_menus_clears_all_preview_menu_state() {
    let mut app = crate::app::App::new().0;
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

    super::dismiss_preview_popup_menus(&mut app);

    assert!(!app.show_preview_context_menu);
    assert!(app.preview_context_menu_pos.is_none());
    assert!(app.preview_nav_popup.is_none());
    assert!(app.preview_tab_menu_path.is_none());
    assert!(app.preview_tab_menu_pos.is_none());
}

#[test]
fn update_routes_search_message_without_side_effects() {
    let mut app = crate::app::App::new().0;
    app.active_preview_path = Some("/tmp/a.txt".to_string());

    let _ = super::update(&mut app, super::PreviewMessage::SearchNext);

    assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/a.txt"));
}

#[test]
fn update_routes_tab_message_to_tab_handler() {
    let mut app = crate::app::App::new().0;
    app.active_preview_path = Some("/tmp/a.txt".to_string());

    let _ = super::update(&mut app, super::PreviewMessage::Select(String::new()));

    assert!(app.active_preview_path.is_none());
}

#[test]
fn update_routes_editor_message_to_editor_handler() {
    let mut app = crate::app::App::new().0;
    app.show_preview_fullscreen_overlay = false;

    let _ = super::update(&mut app, super::PreviewMessage::FullscreenOverlayEntered);

    assert!(app.show_preview_fullscreen_overlay);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn update_routes_editor_mouse_message_to_editor_handler() {
    let mut app = crate::app::App::new().0;
    app.active_preview_path = Some("/tmp/a.rs".to_string());
    app.lsp_overlay_path = Some("/tmp/a.rs".to_string());
    app.lsp_hover_hide_deadline = Some(std::time::Instant::now());

    let _ = super::update(&mut app, super::PreviewMessage::EditorMouseEntered);

    assert!(app.lsp_hover_hide_deadline.is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn update_routes_lsp_message_to_editor_handler() {
    let mut app = crate::app::App::new().0;
    app.lsp_overlay.all_completions = vec!["item".to_string()];
    app.lsp_overlay.completion_items = vec!["item".to_string()];
    app.lsp_overlay.completion_visible = true;

    let _ = super::update(&mut app, super::PreviewMessage::LspCompletionClosed);

    assert!(app.lsp_overlay.all_completions.is_empty());
    assert!(app.lsp_overlay.completion_items.is_empty());
    assert!(!app.lsp_overlay.completion_visible);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn tick_lsp_routes_to_lsp_tick_when_disabled() {
    let mut app = crate::app::App::new().0;
    app.lsp_disabled = true;
    let initial_spinner_frame = app.spinner_frame;

    let _ = super::tick_lsp(&mut app);

    assert_eq!(app.spinner_frame, initial_spinner_frame.wrapping_add(1));
}
