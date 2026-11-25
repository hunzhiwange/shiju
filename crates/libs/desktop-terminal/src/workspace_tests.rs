use crate::{Shell, TerminalState, TerminalTheme};
use iced::widget::pane_grid::{Axis, Direction, Node};
use iced_term::{BackendCommand, Command};
use std::time::{Duration, Instant};

fn new_state() -> TerminalState<Shell> {
    TerminalState::blank_with_settings(
        true,
        Shell::Bash,
        TerminalTheme::System,
        "JetBrains Mono".into(),
        15.0,
        200.0,
    )
}

#[test]
fn project_workspace_restores_split_tabs_and_last_directories() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().canonicalize().unwrap();
    let snapshot = project.join("terminals.json");
    let changed_dir = project.join("changed");
    let removed_dir = project.join("removed");
    std::fs::create_dir(&changed_dir).unwrap();
    std::fs::create_dir(&removed_dir).unwrap();

    let mut original = new_state();
    assert!(original.add_terminal(Some(project.clone())));
    let first_pane = original.panes.focused;
    let first_id = original.active_id.unwrap();
    original.tabs[0].title = "My shell".into();
    original.tabs[0].default_title = false;
    let _ = original.tabs[0]
        .term
        .handle(Command::ProxyToBackend(BackendCommand::Write(b"cd ./changed\r".to_vec())));
    let deadline = Instant::now() + Duration::from_secs(5);
    while original.tabs[0].term.working_directory().as_ref() != Some(&changed_dir) {
        assert!(Instant::now() < deadline, "shell did not change directories");
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(original.add_terminal_to_pane(first_pane, Some(project.clone())));
    let second_id = original.active_id.unwrap();
    original.split_terminal(first_pane, Direction::Right, Some(removed_dir.clone())).unwrap();
    let split_tab_id = original.active_id.unwrap();
    let Node::Split { id, .. } = original.panes.grid.layout() else {
        panic!("expected two panes");
    };
    let split = *id;
    original.panes.grid.resize(split, 0.37);
    assert!(original.select_terminal(first_id));
    original.height = 276.0;
    original.save_workspace(&snapshot).unwrap();
    drop(original);
    let mut legacy: serde_json::Value = super::read_json(&snapshot).unwrap().unwrap();
    legacy.as_object_mut().unwrap().remove("active_group");
    super::write_json(&snapshot, &legacy).unwrap();
    std::fs::remove_dir(&removed_dir).unwrap();

    let mut restored = new_state();
    assert!(restored.restore_workspace(&snapshot, &project).unwrap());
    assert_eq!(restored.active_id, Some(first_id));
    assert_eq!(restored.height, 276.0);
    assert_eq!(restored.tabs[0].title, "My shell");
    assert!(!restored.tabs[0].default_title);
    assert_eq!(restored.tabs[0].term.working_directory(), Some(changed_dir));
    assert_eq!(restored.tabs[2].term.working_directory(), Some(project));

    let grid = &restored.panes.grid;
    let Node::Split { axis, ratio, a, b, .. } = grid.layout() else {
        panic!("expected restored split");
    };
    assert_eq!((*axis, *ratio), (Axis::Vertical, 0.37));
    let (Node::Pane(first), Node::Pane(second)) = (&**a, &**b) else {
        panic!("expected restored left and right panes");
    };
    assert_eq!(restored.panes.focused, *first);
    assert_eq!(grid.get(*first).unwrap().tabs, [first_id, second_id]);
    assert_eq!(grid.get(*second).unwrap().tabs, [split_tab_id]);
}

#[test]
fn project_workspace_restores_active_and_parked_groups() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().canonicalize().unwrap();
    let snapshot = project.join("groups.json");
    let mut original = new_state();

    assert!(original.add_terminal(Some(project.clone())));
    let first_id = original.active_id.unwrap();
    original.split_terminal(original.panes.focused, Direction::Right, Some(project.clone())).unwrap();
    assert!(original.new_group());
    assert!(original.ensure_default_terminal(Some(project.clone())));
    let second_group_tab = original.active_id.unwrap();
    original.save_workspace(&snapshot).unwrap();
    drop(original);

    let mut restored = new_state();
    assert!(restored.restore_workspace(&snapshot, &project).unwrap());
    assert_eq!(restored.groups().collect::<Vec<_>>(), [(1, false), (2, true)]);
    assert_eq!(restored.active_id, Some(second_group_tab));
    assert_eq!(restored.panes.grid.len(), 1);
    assert!(restored.select_group(1));
    assert_eq!(restored.panes.grid.len(), 2);
    assert!(restored.select_terminal(first_id));
    assert!(restored.close_group(2));
    assert_eq!(restored.tabs.len(), 2);
    assert_eq!(restored.groups().collect::<Vec<_>>(), [(1, true)]);
}
