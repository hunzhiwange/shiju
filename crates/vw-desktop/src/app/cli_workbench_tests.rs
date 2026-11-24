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
fn cli_workbench_window_reuses_the_attached_window() {
    let mut workbench = Workbench::default();
    let first = window::Id::unique();
    let second = window::Id::unique();

    assert_eq!(workbench.attach(first), first);
    assert_eq!(workbench.attach(second), first);
    assert!(!workbench.owns_window(second));

    assert!(workbench.request_close(first));
    assert!(workbench.owns_window(first));
    assert_eq!(workbench.attach(second), second);
    workbench.closed(first);
    assert_eq!(workbench.window, Some(second));
    assert!(!workbench.owns_window(first));
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
    assert_eq!(project.panes.get(project.active_pane).unwrap().as_deref(), Some("front"));
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
    assert_eq!(target.panes.get(target.active_pane).unwrap().as_deref(), Some("matching-id"));
    assert_eq!(workbench.projects["a"].tabs["matching-id"].session.state, Process::Ready);
}

#[test]
fn cli_workbench_window_close_preserves_sessions_for_reattachment() {
    let mut workbench = Workbench::default();
    let first = workbench.attach(window::Id::unique());
    let target = key("a", "session");
    workbench
        .projects
        .insert("a".into(), project("a", vec![session("a", "session", Process::Running)]));
    workbench.tab_mut(&target).unwrap().name = Some("Build".into());

    assert!(workbench.request_close(first));
    workbench.closed(first);
    assert_eq!(workbench.window, None);
    assert_eq!(workbench.tab_mut(&target).unwrap().session.state, Process::Running);

    workbench.attach(window::Id::unique());
    workbench.projects.get_mut("a").unwrap().merge_sessions(vec![session(
        "a",
        "session",
        Process::Ready,
    )]);
    assert!(workbench.locate(&target));
    let tab = workbench.tab_mut(&target).unwrap();
    assert_eq!(tab.name.as_deref(), Some("Build"));
    assert_eq!(tab.session.state, Process::Ready);
    assert!(!tab.stopping);
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
fn cli_workbench_window_close_routes_away_from_main_window() {
    use desktop_navigation::global_events::Message as WindowEvent;

    let mut app = App::new().0;
    let main = window::Id::unique();
    app.register_window_ids(main, None);
    let (first, _) = app.open_cli_workbench_window();
    let (reused, _) = app.open_cli_workbench_window();
    assert_eq!(first, reused);

    let _ = app.update(Message::NativeWindowEvent(first, WindowEvent::CloseRequested(first)));
    assert_eq!(app.cli_workbench.window, None);
    assert_eq!(app.main_window_id, Some(main));
    assert!(!app.shutdown_requested);

    let (reopened, _) = app.open_cli_workbench_window();
    let _ = app.update(Message::NativeWindowEvent(first, WindowEvent::WindowClosed(first)));
    assert_eq!(app.cli_workbench.window, Some(reopened));
    assert_eq!(app.main_window_id, Some(main));
    assert!(!app.shutdown_requested);
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
