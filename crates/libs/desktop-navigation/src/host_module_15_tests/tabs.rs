use crate::panel::tabs as domain;
use crate::{components::top_bar::types::AppTab, ui_types::Screen};
fn test_app() -> domain::State {
    domain::State {
        apps_search_query: String::new(),        project_path: None,
        project_preview_tabs: Default::default(),
        preview_tabs: Vec::new(),
        project_preview_active_path: Default::default(),
        active_preview_path: None,
        screen: Screen::Home,
        active_tab_id: Some("home".into()),
        preview_tab_menu_path: None,
        preview_tab_menu_pos: None,
        home_apps_bar_scroll_x: 0.0,
        home_apps_bar_scroll_id: iced::widget::Id::new("home_apps_bar"),
        open_tabs: vec![tab("home", Screen::Home, None)],
        hovered_tab_id: None,
    }
}
fn tab(id: &str, screen: Screen, project_path: Option<&str>) -> AppTab {
    AppTab {
        id: id.into(),
        title: id.into(),
        screen,
        project_path: project_path.map(str::to_owned),
    }
}
fn update(state: &mut domain::State, message: domain::Message) -> iced::Task<()> {
    domain::update(state, &mut false, &mut crate::panel::settings::State::default(), message).task
}
fn assert_scroll_offset(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < f32::EPSILON);
}
#[test]
fn home_apps_bar_scroll_messages_update_and_clamp_offsets() {
    let mut app = test_app();

    let _ = update(&mut app, domain::Message::HomeAppsBarScrollChanged(0.6));
    assert_scroll_offset(app.home_apps_bar_scroll_x, 0.6);

    let _ = update(&mut app, domain::Message::HomeAppsBarPrev);
    assert_scroll_offset(app.home_apps_bar_scroll_x, 0.35);

    let _ = update(&mut app, domain::Message::HomeAppsBarNext);
    assert_scroll_offset(app.home_apps_bar_scroll_x, 0.6);

    app.home_apps_bar_scroll_x = 0.1;
    let _ = update(&mut app, domain::Message::HomeAppsBarPrev);
    assert_scroll_offset(app.home_apps_bar_scroll_x, 0.0);

    app.home_apps_bar_scroll_x = 0.9;
    let _ = update(&mut app, domain::Message::HomeAppsBarNext);
    assert_scroll_offset(app.home_apps_bar_scroll_x, 1.0);
}

#[test]
fn tab_hovered_tracks_and_clears_hover() {
    let mut app = test_app();

    let _ = update(&mut app, domain::Message::TabHovered(Some("project".to_string())));
    assert_eq!(app.hovered_tab_id.as_deref(), Some("project"));

    let _ = update(&mut app, domain::Message::TabHovered(None));
    assert_eq!(app.hovered_tab_id, None);
}

#[test]
fn tab_selected_home_archives_previous_project_preview() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.project_path = Some("/project-a".to_string());
    app.active_preview_path = Some("/project-a/README.md".to_string());
    app.preview_tab_menu_path = Some("/project-a/README.md".to_string());

    let _ = update(&mut app, domain::Message::TabSelected("home".to_string()));

    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.active_tab_id.as_deref(), Some("home"));
    assert_eq!(app.project_path, None);
    assert_eq!(app.active_preview_path, None);
    assert_eq!(
        app.project_preview_active_path.get("/project-a"),
        Some(&Some("/project-a/README.md".to_string()))
    );
    assert_eq!(app.preview_tab_menu_path, None);
}

#[test]
fn tab_selected_project_restores_cached_preview_state() {
    let mut app = test_app();
    app.open_tabs.push(tab("project-b", Screen::Project, Some("/project-b")));
    app.project_preview_tabs.insert("/project-b".to_string(), Vec::new());
    app.project_preview_active_path
        .insert("/project-b".to_string(), Some("/project-b/src/lib.rs".to_string()));

    let _ = update(&mut app, domain::Message::TabSelected("project-b".to_string()));

    assert_eq!(app.screen, Screen::Project);
    assert_eq!(app.active_tab_id.as_deref(), Some("project-b"));
    assert_eq!(app.project_path.as_deref(), Some("/project-b"));
    assert_eq!(app.active_preview_path.as_deref(), Some("/project-b/src/lib.rs"));
    assert!(!app.project_preview_tabs.contains_key("/project-b"));
    assert!(!app.project_preview_active_path.contains_key("/project-b"));
}

#[test]
fn tab_selected_unknown_id_preserves_current_project_preview_state() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.project_path = Some("/project-a".to_string());
    app.active_preview_path = Some("/project-a/src/main.rs".to_string());

    let _ = update(&mut app, domain::Message::TabSelected("missing".to_string()));

    assert_eq!(app.screen, Screen::Project);
    assert_eq!(app.active_tab_id.as_deref(), Some("missing"));
    assert_eq!(app.project_path.as_deref(), Some("/project-a"));
    assert_eq!(app.active_preview_path.as_deref(), Some("/project-a/src/main.rs"));
    assert!(!app.project_preview_tabs.contains_key("/project-a"));
    assert!(!app.project_preview_active_path.contains_key("/project-a"));
}

#[test]
fn tab_selected_same_project_changes_screen_without_losing_preview_state() {
    let mut app = test_app();
    app.open_tabs.push(tab("preview", Screen::Preview, Some("/project-a")));
    app.screen = Screen::Project;
    app.project_path = Some("/project-a".to_string());
    app.active_preview_path = Some("/project-a/src/main.rs".to_string());

    let _ = update(&mut app, domain::Message::TabSelected("preview".to_string()));

    assert_eq!(app.screen, Screen::Preview);
    assert_eq!(app.active_tab_id.as_deref(), Some("preview"));
    assert_eq!(app.project_path.as_deref(), Some("/project-a"));
    assert_eq!(app.active_preview_path.as_deref(), Some("/project-a/src/main.rs"));
    assert!(!app.project_preview_tabs.contains_key("/project-a"));
    assert!(!app.project_preview_active_path.contains_key("/project-a"));
}

#[test]
fn tab_closed_inactive_removes_only_requested_tab() {
    let mut app = test_app();
    app.open_tabs.push(tab("apps", Screen::Apps, None));

    let _ = update(&mut app, domain::Message::TabClosed("apps".to_string()));

    assert_eq!(app.open_tabs.len(), 1);
    assert_eq!(app.open_tabs[0].id, "home");
    assert_eq!(app.active_tab_id.as_deref(), Some("home"));
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn tab_closed_missing_id_preserves_tab_selection() {
    let mut app = test_app();

    let _ = update(&mut app, domain::Message::TabClosed("missing".to_string()));

    assert_eq!(app.open_tabs.len(), 1);
    assert_eq!(app.open_tabs[0].id, "home");
    assert_eq!(app.active_tab_id.as_deref(), Some("home"));
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn tab_closed_active_project_falls_back_home_and_archives_preview() {
    let mut app = test_app();
    app.open_tabs.push(tab("project-a", Screen::Project, Some("/project-a")));
    app.active_tab_id = Some("project-a".to_string());
    app.screen = Screen::Project;
    app.project_path = Some("/project-a".to_string());
    app.active_preview_path = Some("/project-a/src/main.rs".to_string());
    app.preview_tab_menu_path = Some("/project-a/src/main.rs".to_string());

    let _ = update(&mut app, domain::Message::TabClosed("project-a".to_string()));

    assert_eq!(app.open_tabs.len(), 1);
    assert_eq!(app.active_tab_id.as_deref(), Some("home"));
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.project_path, None);
    assert_eq!(app.active_preview_path, None);
    assert_eq!(
        app.project_preview_active_path.get("/project-a"),
        Some(&Some("/project-a/src/main.rs".to_string()))
    );
    assert_eq!(app.preview_tab_menu_path, None);
}

#[test]
fn tab_closed_active_tab_on_same_project_preserves_preview_state() {
    let mut app = test_app();
    app.open_tabs = vec![
        tab("project-summary", Screen::Project, Some("/project-a")),
        tab("project-preview", Screen::Preview, Some("/project-a")),
    ];
    app.active_tab_id = Some("project-preview".to_string());
    app.screen = Screen::Preview;
    app.project_path = Some("/project-a".to_string());
    app.active_preview_path = Some("/project-a/src/main.rs".to_string());

    let _ = update(&mut app, domain::Message::TabClosed("project-preview".to_string()));

    assert_eq!(app.open_tabs.len(), 1);
    assert_eq!(app.active_tab_id.as_deref(), Some("project-summary"));
    assert_eq!(app.screen, Screen::Project);
    assert_eq!(app.project_path.as_deref(), Some("/project-a"));
    assert_eq!(app.active_preview_path.as_deref(), Some("/project-a/src/main.rs"));
    assert!(!app.project_preview_tabs.contains_key("/project-a"));
    assert!(!app.project_preview_active_path.contains_key("/project-a"));
}

#[test]
fn tab_closed_active_project_switches_to_last_project_and_restores_preview() {
    let mut app = test_app();
    app.open_tabs = vec![
        tab("project-b", Screen::Project, Some("/project-b")),
        tab("project-a", Screen::Project, Some("/project-a")),
    ];
    app.active_tab_id = Some("project-a".to_string());
    app.screen = Screen::Project;
    app.project_path = Some("/project-a".to_string());
    app.active_preview_path = Some("/project-a/src/main.rs".to_string());
    app.project_preview_active_path
        .insert("/project-b".to_string(), Some("/project-b/src/lib.rs".to_string()));

    let _ = update(&mut app, domain::Message::TabClosed("project-a".to_string()));

    assert_eq!(app.open_tabs.len(), 1);
    assert_eq!(app.active_tab_id.as_deref(), Some("project-b"));
    assert_eq!(app.screen, Screen::Project);
    assert_eq!(app.project_path.as_deref(), Some("/project-b"));
    assert_eq!(app.active_preview_path.as_deref(), Some("/project-b/src/lib.rs"));
    assert_eq!(
        app.project_preview_active_path.get("/project-a"),
        Some(&Some("/project-a/src/main.rs".to_string()))
    );
}

#[test]
fn tab_closed_last_tab_falls_back_to_unselected_home_screen() {
    let mut app = test_app();
    app.open_tabs = vec![tab("apps", Screen::Apps, None)];
    app.active_tab_id = Some("apps".to_string());
    app.screen = Screen::Apps;

    let _ = update(&mut app, domain::Message::TabClosed("apps".to_string()));

    assert!(app.open_tabs.is_empty());
    assert_eq!(app.active_tab_id, None);
    assert_eq!(app.screen, Screen::Home);
}
