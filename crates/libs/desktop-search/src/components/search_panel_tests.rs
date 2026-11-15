use super::search_panel::{result_button_style, search_input_style, SessionSearchMatch};
use iced::widget::{button, text_input};
use iced::{Background, Theme};
#[derive(Default)]
struct Fixture {
    sessions: Vec<session_info::Info>,
    session_previews: std::collections::HashMap<String, String>,
}
fn matching_sessions(app: &Fixture, query: &str) -> Vec<SessionSearchMatch> {
    super::search_panel::matching_sessions(&app.sessions, &app.session_previews, |s| s.title.clone(), query)
}
fn session(id: &str, title: &str, directory: &str) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project-1".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "0.0.0".to_string(),
        time: session_info::TimeInfo {
            created: 0,
            updated: 0,
            compacting: None,
            archived: None,
        },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

fn bg(style: &iced::widget::button::Style) -> iced::Color {
    match style.background {
        Some(Background::Color(color)) => color,
        _ => panic!("button style should use a color background"),
    }
}

#[test]
fn result_button_style_changes_background_by_status() {
    let theme = Theme::Light;

    let active = result_button_style(&theme, button::Status::Active);
    let hovered = result_button_style(&theme, button::Status::Hovered);
    let pressed = result_button_style(&theme, button::Status::Pressed);

    assert_ne!(bg(&active), bg(&hovered));
    assert_ne!(bg(&hovered), bg(&pressed));
    assert_eq!(active.border.width, 0.0);
    assert_eq!(active.border.radius.top_left, 6.0);
}

#[test]
fn result_button_style_uses_base_surface_for_fallback_statuses() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let active = result_button_style(&theme, button::Status::Active);
        let disabled = result_button_style(&theme, button::Status::Disabled);

        assert_eq!(bg(&active), palette.background.base.color);
        assert_eq!(bg(&disabled), palette.background.base.color);
        assert_eq!(active.text_color, palette.background.base.text);
        assert_eq!(disabled.text_color, palette.background.base.text);
    }
}

#[test]
fn search_input_style_tracks_focus_and_hover() {
    let theme = Theme::Light;

    let active = search_input_style(&theme, text_input::Status::Active);
    let hovered = search_input_style(&theme, text_input::Status::Hovered);
    let focused = search_input_style(&theme, text_input::Status::Focused { is_hovered: false });
    let focused_hovered =
        search_input_style(&theme, text_input::Status::Focused { is_hovered: true });

    assert_ne!(active.border.color, hovered.border.color);
    assert_ne!(hovered.border.color, focused.border.color);
    assert_eq!(focused.border.color, focused_hovered.border.color);
    assert_eq!(active.border.width, 1.0);
    assert_eq!(active.border.radius.top_left, 8.0);
}

#[test]
fn matching_sessions_uses_title_preview_and_directory() {
    let mut app = Fixture::default();
    app.sessions = vec![
        session("session-1", "Alpha Plan", "/workspace/alpha"),
        session("session-2", "Beta", "/workspace/beta"),
    ];
    app.session_previews = std::collections::HashMap::from([(
        "session-2".to_string(),
        "Discuss release checklist".to_string(),
    )]);

    let title_hits = matching_sessions(&app, "alpha");
    let preview_hits = matching_sessions(&app, "release");
    let directory_hits = matching_sessions(&app, "beta");

    assert_eq!(title_hits[0].id, "session-1");
    assert_eq!(preview_hits[0].id, "session-2");
    assert_eq!(directory_hits[0].id, "session-2");
}

#[test]
fn child_session_navigation_search_hides_internal_sources() {
    let mut app = Fixture::default();
    let mut agent = session("agent", "任务", "/workspace");
    agent.mark_agent_tool();
    let mut workflow = session("workflow", "任务", "/workspace");
    workflow.slug = session_info::Info::plan_run_slug(&workflow.slug);
    let mut fork = session("fork", "任务", "/workspace");
    fork.parent_id = Some("parent".into());
    let mut task = session("task", "任务", "/workspace");
    task.parent_id = Some("parent".into());
    task.slug = session_info::Info::visible_task_slug(&task.slug);
    app.sessions = vec![agent, workflow, fork, task];

    let matches = matching_sessions(&app, "任务");
    assert_eq!(matches.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(), ["fork", "task"]);
}

#[test]
fn matching_sessions_rejects_blank_queries() {
    let mut app = Fixture::default();
    app.sessions = vec![session("session-1", "Alpha", "/workspace/alpha")];

    assert!(matching_sessions(&app, "").is_empty());
    assert!(matching_sessions(&app, " \t\n ").is_empty());
}

#[test]
fn matching_sessions_matches_id_case_insensitively_and_preserves_fields() {
    let mut app = Fixture::default();
    app.sessions = vec![session("Session-ALPHA", "Planning", "/workspace/demo")];
    app.session_previews.insert("Session-ALPHA".to_string(), "Release notes".to_string());

    let matches = matching_sessions(&app, "  session-alpha  ");

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].id, "Session-ALPHA");
    assert_eq!(matches[0].title, "Planning");
    assert_eq!(matches[0].preview.as_deref(), Some("Release notes"));
    assert_eq!(matches[0].directory, "/workspace/demo");
}

#[test]
fn matching_sessions_limits_results_to_first_eight() {
    let mut app = Fixture::default();
    app.sessions = (0..10)
        .map(|index| session(&format!("session-{index}"), "Needle", "/workspace/demo"))
        .collect();

    let matches = matching_sessions(&app, "needle");
    let ids = matches.iter().map(|session| session.id.clone()).collect::<Vec<_>>();

    assert_eq!(ids, (0..8).map(|index| format!("session-{index}")).collect::<Vec<_>>());
}

#[test]
fn matching_sessions_preserves_session_order_across_different_matching_fields() {
    let mut app = Fixture::default();
    app.sessions = vec![
        session("needle-id", "First", "/workspace/one"),
        session("session-2", "Needle title", "/workspace/two"),
        session("session-3", "Third", "/workspace/needle-directory"),
    ];
    app.session_previews.insert("session-2".to_string(), "unrelated".to_string());

    let ids =
        matching_sessions(&app, "needle").into_iter().map(|session| session.id).collect::<Vec<_>>();

    assert_eq!(ids, ["needle-id", "session-2", "session-3"].map(str::to_string));
}
