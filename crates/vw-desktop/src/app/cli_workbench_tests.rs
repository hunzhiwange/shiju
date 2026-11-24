use super::*;
use vw_api_types::cli_workbench::{
    CliWorkbenchCapabilities, CliWorkbenchMessageId, CliWorkbenchMessageState, CliWorkbenchTarget,
};

fn session(project: &str, id: &str, state: Process) -> CliWorkbenchSession {
    CliWorkbenchSession {
        id: id.into(),
        project_id: project.into(),
        directory: format!("/projects/{project}"),
        target: CliWorkbenchTarget::Acp { agent: "acp-codex".into() },
        state,
        agent_activity: None,
        capabilities: CliWorkbenchCapabilities {
            available: true,
            terminal: false,
            delivery: CliWorkbenchDeliveryCapability::Structured,
        },
    }
}

fn project(id: &str, sessions: Vec<CliWorkbenchSession>) -> Project {
    let mut project = Project::new(id.into(), format!("/projects/{id}"));
    project.merge_sessions(sessions);
    project
}

fn key(project: &str, session: &str) -> SessionKey {
    SessionKey { project: project.into(), session: session.into() }
}

#[test]
fn cli_workbench_sidebar_defaults_to_twenty_percent_and_clamps_to_ten() {
    use iced::widget::pane_grid::{Node, ResizeEvent};

    let mut sidebar = SidebarLayout::default();
    let Node::Split { id, ratio, .. } = sidebar.workspace.layout() else {
        panic!("expected a sidebar divider");
    };
    let (split, default) = (*id, *ratio);
    assert_eq!(default, 0.20);

    sidebar.resize_width(ResizeEvent { split, ratio: 0.0 });

    let Node::Split { ratio, .. } = sidebar.workspace.layout() else {
        panic!("expected a sidebar divider after resizing");
    };
    assert_eq!(*ratio, 0.10);
}

#[test]
fn cli_workbench_sidebar_shows_new_and_untracked_agents() {
    use view::AgentStatus;
    use vw_api_types::cli_workbench::CliWorkbenchAgentActivity;

    let project = project("a", Vec::new());
    let mut tab = Tab::new(session("a", "agent", Process::Ready));
    assert_eq!(AgentStatus::for_tab(&project, &tab), Some(AgentStatus::Ready));

    tab.session.target = CliWorkbenchTarget::NativeCli { app_id: "codex".into() };
    tab.session.state = Process::Running;
    assert_eq!(AgentStatus::for_tab(&project, &tab), Some(AgentStatus::Running));

    tab.session.agent_activity = Some(CliWorkbenchAgentActivity::NeedsInput);
    assert_eq!(AgentStatus::for_tab(&project, &tab), Some(AgentStatus::NeedsInput));

    tab.session.agent_activity = Some(CliWorkbenchAgentActivity::Idle);
    assert_eq!(AgentStatus::for_tab(&project, &tab), Some(AgentStatus::Idle));

    tab.session.target = CliWorkbenchTarget::Shell;
    assert_eq!(AgentStatus::for_tab(&project, &tab), None);
}

fn message(target: &str, revision: u64, output: &str) -> CliWorkbenchMessage {
    CliWorkbenchMessage {
        id: CliWorkbenchMessageId {
            request_id: "req".into(),
            sender_user_id: "user".into(),
            target_session_id: target.into(),
        },
        turn_id: "turn".into(),
        state: CliWorkbenchMessageState::Completed,
        manual_submission: false,
        revision,
        approval_request_id: None,
        approval_history: Vec::new(),
        assistant_message_id: None,
        output: output.into(),
        output_truncated: false,
        error: None,
    }
}

#[test]
fn cli_workbench_window_from_catalog_accepts_acp_profiles_across_apps() {
    let agents: Vec<update::Agent> = serde_json::from_value(serde_json::json!([
        {"key": "main"},
        {"key": "work-codex", "runtime": {"kind": "acp", "backend": "codex"}},
        {"key": "work-claude", "runtime": {"kind": "acp", "backend": "claude"}}
    ]))
    .unwrap();

    let matching_backends = |filter: Option<&str>| {
        agents.iter().filter_map(|agent| agent.matching_backend(filter)).collect::<Vec<_>>()
    };
    assert_eq!(matching_backends(None), ["codex", "claude"]);
    assert_eq!(matching_backends(Some("codex")), ["codex"]);
}

#[test]
fn cli_workbench_window_applies_newer_updates_to_a_background_tab() {
    let mut workbench = Workbench::default();
    workbench.projects.insert(
        "a".into(),
        project(
            "a",
            vec![session("a", "front", Process::Ready), session("a", "other", Process::Running)],
        ),
    );
    let visible = key("a", "front");
    let other = key("a", "other");
    assert!(workbench.locate(&visible));

    workbench.tab_mut(&other).unwrap().turns.push(Turn {
        request: "req".into(),
        prompt: "hello".into(),
        snapshot: None,
        error: None,
        recover: true,
        submitting: None,
    });
    workbench.apply_turn(&other, "req", Ok(message("other", 3, "new result")), false);
    workbench.apply_turn(&other, "req", Ok(message("other", 2, "old result")), false);
    workbench.apply_turn(&other, "req", Ok(message("front", 4, "wrong target")), false);

    let project = &workbench.projects["a"];
    assert_eq!(project.active_tab().map(|tab| tab.session.id.as_str()), Some("front"));
    assert!(project.tabs["front"].turns.is_empty());
    assert_eq!(project.tabs["other"].turns[0].snapshot.as_ref().unwrap().output, "new result",);
}

#[test]
fn cli_workbench_window_pending_navigation_switches_project_and_reopens_tab() {
    let mut workbench = Workbench::default();
    for (project_id, status) in [("a", Process::Ready), ("b", Process::WaitingForApproval)] {
        workbench.projects.insert(
            project_id.into(),
            project(project_id, vec![session(project_id, "matching-id", status)]),
        );
    }
    assert!(workbench.locate(&key("a", "matching-id")));
    workbench.projects.get_mut("b").unwrap().hide("matching-id");

    assert!(workbench.locate(&key("b", "matching-id")));
    assert_eq!(workbench.selected.as_deref(), Some("b"));
    let target = &workbench.projects["b"];
    assert!(!target.tabs["matching-id"].hidden);
    assert_eq!(target.active_tab().map(|tab| tab.session.id.as_str()), Some("matching-id"));
    assert_eq!(workbench.projects["a"].tabs["matching-id"].session.state, Process::Ready);
}

#[test]
fn cli_workbench_window_stop_is_independent_of_hiding_the_tab() {
    let mut project = project("a", vec![session("a", "session", Process::Ready)]);
    project.hide("session");
    let tab = project.tabs.get_mut("session").unwrap();
    assert!(tab.hidden);
    assert!(tab.can_send());
    assert_eq!(tab.session.state, Process::Ready);

    assert!(tab.begin_stop());
    assert!(!tab.begin_stop());
    assert!(!tab.can_send());

    tab.finish_stop(Ok(session("a", "session", Process::Exited)));
    assert_eq!(tab.session.state, Process::Exited);
    assert!(tab.hidden);
    assert!(!tab.stopping);
    assert!(!tab.begin_stop());
}

#[test]
fn cli_workbench_window_routes_terminal_drafts_only_to_native_cli() {
    let mut native = session("a", "native", Process::Running);
    native.target = CliWorkbenchTarget::NativeCli { app_id: "codex".into() };
    native.capabilities.terminal = true;
    native.capabilities.delivery = CliWorkbenchDeliveryCapability::Draft;
    let mut shell = native.clone();
    shell.id = "shell".into();
    shell.target = CliWorkbenchTarget::Shell;
    shell.capabilities.delivery = CliWorkbenchDeliveryCapability::Unavailable;
    let mut workbench = Workbench::default();
    workbench.projects.insert("a".into(), project("a", vec![native, shell]));
    let key = key("a", "native");
    let tab = workbench.tab_mut(&key).unwrap();
    assert!(tab.can_send());
    let mut snapshot = message("native", 1, "");
    snapshot.manual_submission = true;
    snapshot.state = CliWorkbenchMessageState::PendingSubmission;
    tab.turns.push(Turn {
        request: "req".into(),
        prompt: "review changes".into(),
        snapshot: Some(snapshot.clone()),
        error: None,
        recover: true,
        submitting: None,
    });
    assert!(tab.can_send());
    snapshot.revision += 1;
    snapshot.state = CliWorkbenchMessageState::Submitted;
    workbench.apply_turn(&key, "req", Ok(snapshot), false);
    assert!(!workbench.tab_mut(&key).unwrap().turns[0].recover);
    assert!(!workbench.projects["a"].tabs["shell"].can_send());
}


#[test]
fn cli_workbench_catalog_uses_one_main_tab_and_preserves_sessions_when_closed() {
    use crate::app::{Screen, message::ViewMessage};

    let mut app = App::new().0;
    let main = window::Id::unique();
    app.register_window_ids(main, None);
    let open = Message::CliWorkbench(Event::Open { backend: None, native_app: None });
    let _ = app.update(open.clone());
    let _ = app.update(open.clone());

    let tab_id = app.navigation_tabs.active_tab_id.clone().unwrap();
    assert_eq!(tab_id, update::TAB_ID);
    assert_eq!(app.navigation_tabs.screen, Screen::CliWorkbench);
    assert_eq!(app.navigation_tabs.open_tabs.iter().filter(|tab| tab.id == tab_id).count(), 1);

    let target = key("a", "session");
    app.cli_workbench.projects.insert(
        "a".into(),
        project("a", vec![session("a", "session", Process::Running)]),
    );
    app.cli_workbench.tab_mut(&target).unwrap().name = Some("Build".into());

    let _ = app.update(Message::View(ViewMessage::TabClosed(tab_id.clone())));
    assert_ne!(app.navigation_tabs.screen, Screen::CliWorkbench);
    assert_eq!(app.main_window_id, Some(main));
    assert!(!app.shutdown_requested);

    let _ = app.update(open);
    let session = app.cli_workbench.tab_mut(&target).unwrap();
    assert_eq!(session.name.as_deref(), Some("Build"));
    assert_eq!(session.session.state, Process::Running);
}

#[test]
fn cli_workbench_native_launch_requires_the_selected_installed_app() {
    let mut workbench = Workbench {
        terminal_available: true,
        native_apps: vec![CliWorkbenchNativeApp { id: "codex".into(), name: "Codex".into() }],
        ..Workbench::default()
    };
    let codex = CliWorkbenchTarget::NativeCli { app_id: "codex".into() };
    let claude = CliWorkbenchTarget::NativeCli { app_id: "claude".into() };
    assert!(update::launch_target_available(&workbench, &codex));
    assert!(!update::launch_target_available(&workbench, &claude));
    assert!(update::launch_target_available(&workbench, &CliWorkbenchTarget::Shell));

    workbench.terminal_available = false;
    assert!(!update::launch_target_available(&workbench, &codex));
    assert!(!update::launch_target_available(&workbench, &CliWorkbenchTarget::Shell));
}

#[test]
fn cli_workbench_codex_launch_uses_native_pty_even_without_a_gateway_catalog() {
    let mut workbench = Workbench {
        terminal_available: true,
        agents: serde_json::from_value(serde_json::json!([
            {"key": "acp-codex", "runtime": {"kind": "acp", "backend": "codex"}},
            {"key": "acp-claude", "runtime": {"kind": "acp", "backend": "claude"}}
        ])).unwrap(),
        ..Workbench::default()
    };
    let locale = crate::app::i18n::I18n::new(Default::default());
    let codex = CliWorkbenchTarget::NativeCli { app_id: "codex".into() };
    let acp_codex = CliWorkbenchTarget::Acp { agent: "acp-codex".into() };
    let acp_claude = CliWorkbenchTarget::Acp { agent: "acp-claude".into() };
    let choices = view::launch_choices(&workbench, locale);
    assert_eq!(choices[0].target, CliWorkbenchTarget::Shell);
    let codex_choice = choices.iter().find(|choice| choice.target == codex).unwrap();
    assert_eq!(codex_choice.to_string(), format!("Codex · {}", locale.__("终端")));
    assert!(update::launch_target_available(&workbench, &codex_choice.target));
    assert!(!choices.iter().any(|choice| choice.target == acp_codex));
    assert!(choices.iter().any(|choice| choice.target == acp_claude));

    workbench.native_apps.push(CliWorkbenchNativeApp { id: "codex".into(), name: "Codex".into() });
    assert_eq!(view::launch_choices(&workbench, locale).iter().filter(|choice| choice.target == codex).count(), 1);

    workbench.terminal_available = false;
    assert!(view::launch_choices(&workbench, locale).iter().all(|choice| choice.target != codex));
    assert!(!update::launch_target_available(&workbench, &codex));
}

#[test]
fn cli_workbench_uses_shared_panes_for_terminal_tabs() {
    use iced::widget::pane_grid::Direction;

    let mut project = project("a", vec![
        session("a", "one", Process::Ready),
        session("a", "two", Process::Ready),
    ]);
    let first_pane = project.panes.focused;
    project.select("one");
    let new_pane = project.panes.split(first_pane, Direction::Down).unwrap();
    let new_tab = Tab::new(session("a", "three", Process::Ready));
    project.panes.add(new_pane, new_tab.view_id);
    project.tabs.insert("three".into(), new_tab);

    assert_eq!(project.active_tab().unwrap().session.id, "three");
    assert_eq!(project.panes.grid.get(first_pane).unwrap().tabs.len(), 2);
    project.panes.grid.maximize(new_pane);
    project.select("two");
    assert!(project.panes.grid.maximized().is_none());
    assert_eq!(project.active_tab().unwrap().session.id, "two");

    project.hide("three");
    assert_eq!(project.panes.grid.len(), 1);
    assert!(project.tabs["three"].hidden);
}

#[test]
fn cli_workbench_split_moves_a_session_observed_before_start_reply() {
    use iced::widget::pane_grid::Direction;

    let old = session("a", "original", Process::Ready);
    let started = session("a", "started", Process::Running);
    let mut project = project("a", vec![old.clone()]);
    let original_pane = project.panes.focused;
    project.merge_sessions(vec![old, started.clone()]);
    let started_view = project.tabs["started"].view_id;
    assert!(project.panes.grid.get(original_pane).unwrap().tabs.contains(&started_view));

    project.place_started(started, project.active_layout, original_pane, Some(Direction::Down));

    let split_pane = project.panes.focused;
    assert_ne!(split_pane, original_pane);
    assert_eq!(project.active_tab().unwrap().session.id, "started");
    assert_eq!(project.panes.grid.get(original_pane).unwrap().tabs, [project.tabs["original"].view_id]);
    assert_eq!(project.panes.grid.get(split_pane).unwrap().tabs, [started_view]);
}

#[test]
fn cli_workbench_layout_tabs_keep_independent_splits_and_reopen_hidden_sessions() {
    use iced::widget::pane_grid::Direction;

    let a = session("a", "original", Process::Running);
    let mut project = project("a", vec![a.clone()]);
    let first = project.active_layout;
    let first_pane = project.panes.focused;
    project.place_started(session("a", "split", Process::Running), first, first_pane, Some(Direction::Right));
    assert_eq!(project.panes.grid.len(), 2);

    project.new_layout();
    let second = project.active_layout;
    let second_pane = project.panes.focused;
    assert_eq!(project.layout_ids().collect::<Vec<_>>(), [first, second]);
    assert!(project.active_tab().is_none());
    project.place_started(session("a", "another", Process::Running), second, second_pane, None);

    project.select("original");
    assert_eq!(project.active_layout, first);
    assert_eq!(project.panes.grid.len(), 2);
    project.select("another");
    assert_eq!(project.active_layout, second);
    assert_eq!(project.panes.grid.len(), 1);

    assert!(project.close_layout(second));
    assert!(!project.close_layout(first));
    assert!(project.tabs["another"].hidden);
    assert_eq!(project.tabs["another"].session.state, Process::Running);
    project.select("another");
    assert_eq!(project.active_layout, first);
    assert_eq!(project.active_tab().unwrap().session.id, "another");
}

#[test]
fn cli_workbench_layout_switch_restores_nested_split_geometry_and_focus() {
    use desktop_terminal::panes::PANE_SPACING;
    use iced::{Size, widget::pane_grid::{Direction, Node}};

    let mut project = project("a", vec![session("a", "left", Process::Running)]);
    let first = project.active_layout;
    let left = project.panes.focused;
    project.place_started(session("a", "top", Process::Running), first, left, Some(Direction::Right));
    let top = project.panes.focused;
    project.place_started(session("a", "bottom", Process::Running), first, top, Some(Direction::Down));
    let bottom = project.panes.focused;
    let Node::Split { id, b, .. } = project.panes.grid.layout() else {
        panic!("expected a left/right split");
    };
    let Node::Split { id: nested, .. } = b.as_ref() else {
        panic!("expected the right pane to be split vertically");
    };
    let (root, nested) = (*id, *nested);
    project.panes.grid.resize(root, 0.37);
    project.panes.grid.resize(nested, 0.61);
    let size = Size::new(800.0, 600.0);
    let regions = project.panes.grid.layout().pane_regions(PANE_SPACING, 0.0, size);
    assert_eq!(regions.len(), 3);
    assert_eq!(regions[&top].x, regions[&bottom].x);
    assert!(regions[&left].x < regions[&top].x && regions[&top].y < regions[&bottom].y);

    project.new_layout();
    assert_eq!(project.panes.grid.len(), 1);
    assert!(project.select_layout(first));

    assert_eq!(project.panes.grid.layout().pane_regions(PANE_SPACING, 0.0, size), regions);
    assert_eq!(project.panes.focused, bottom);
    assert_eq!(project.active_tab().unwrap().session.id, "bottom");
}

#[test]
fn cli_workbench_late_launch_reply_stays_with_its_original_layout() {
    let original = session("a", "original", Process::Running);
    let started = session("a", "late", Process::Running);
    let mut project = project("a", vec![original.clone()]);
    project.new_layout();
    let requested_layout = project.active_layout;
    let requested_pane = project.panes.focused;
    project.select_layout(1);
    project.merge_sessions(vec![original, started.clone()]);
    assert_eq!(project.layout_with_tab(project.tabs["late"].view_id), Some(1));

    project.place_started(started, requested_layout, requested_pane, None);

    assert_eq!(project.active_layout, 1);
    assert_eq!(project.layout_with_tab(project.tabs["late"].view_id), Some(requested_layout));
    assert_eq!(project.active_tab().unwrap().session.id, "original");
    project.select_layout(requested_layout);
    assert_eq!(project.active_tab().unwrap().session.id, "late");
}

#[test]
fn cli_workbench_hide_clears_context_menus_before_reattaching() {
    let mut project = project("a", vec![session("a", "session", Process::Running)]);
    let view_id = project.tabs["session"].view_id;
    project.tab_menu = Some((view_id, 10.0, 10.0));
    project.content_menu = Some((view_id, iced::Point::ORIGIN));

    project.hide("session");
    project.select("session");

    assert!(project.tab_menu.is_none());
    assert!(project.content_menu.is_none());
    assert_eq!(project.active_tab().unwrap().view_id, view_id);
}

#[test]
fn cli_workbench_snapshot_populates_project_and_pending_session_together() {
    use vw_api_types::cli_workbench::{CliWorkbenchProject, CliWorkbenchProjects};

    let snapshot = CliWorkbenchProjects {
        projects: vec![CliWorkbenchProject {
            id: "a".into(), name: "Project A".into(), directory: "/projects/a".into(),
            sessions: vec![session("a", "pending", Process::WaitingForApproval)],
        }],
        preferred_project: Some("a".into()),
        terminal_available: true,
    };
    let mut workbench = Workbench::default();
    update::apply_projects(&mut workbench, snapshot.clone(), true);
    assert_eq!(workbench.selected.as_deref(), Some("a"));
    assert!(workbench.terminal_available);
    let target = key("a", "pending");
    assert_eq!(workbench.tab_mut(&target).unwrap().session.state, Process::WaitingForApproval);

    workbench.tab_mut(&target).unwrap().session.state = Process::Running;
    update::apply_projects(&mut workbench, snapshot, false);
    assert_eq!(workbench.tab_mut(&target).unwrap().session.state, Process::Running);
}

#[test]
fn cli_workbench_legacy_poll_prioritizes_current_project_and_bounds_each_batch() {
    let mut workbench = Workbench::default();
    assert!(update::legacy_poll_targets(&mut workbench).is_empty());

    for number in 0..=10 {
        let id = format!("p{number:02}");
        workbench.projects.insert(id.clone(), project(&id, Vec::new()));
    }
    workbench.selected = Some("p05".into());
    let first = update::legacy_poll_targets(&mut workbench);
    let second = update::legacy_poll_targets(&mut workbench);
    assert_eq!(first, ["p05", "p00", "p01", "p02", "p03", "p04"]);
    assert_eq!(second, ["p05", "p06", "p07", "p08", "p09", "p10"]);
    assert_eq!(update::legacy_poll_targets(&mut workbench), first);
}
