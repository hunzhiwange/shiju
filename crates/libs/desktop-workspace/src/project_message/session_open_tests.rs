fn session_info(id: &str, additions: i64, deletions: i64) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: "/tmp/project".to_string(),
        parent_id: None,
        summary: Some(session_info::Summary {
            additions,
            deletions,
            files: 1,
            diffs: None,
        }),
        share: None,
        title: id.to_string(),
        version: "1".to_string(),
        time: session_info::TimeInfo {
            created: 1,
            updated: 1,
            compacting: None,
            archived: None,
        },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn merge_project_sessions_returns_incoming_when_no_existing_list() {
    let incoming = vec![session_info("incoming", 1, 2)];

    let merged = super::merge_project_sessions(None, incoming.clone());

    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].id, incoming[0].id);
}

#[test]
fn merge_project_sessions_replaces_existing_summary() {
    let existing = vec![session_info("task-board-1", 0, 0), session_info("local-only", 2, 1)];
    let incoming = vec![session_info("task-board-1", 7, 3)];

    let merged = super::merge_project_sessions(Some(existing), incoming);

    assert_eq!(merged.len(), 2);
    let summary = merged[0].summary.as_ref().expect("incoming summary should be kept");
    assert_eq!(merged[0].id, "task-board-1");
    assert_eq!(summary.additions, 7);
    assert_eq!(summary.deletions, 3);
    assert_eq!(merged[1].id, "local-only");
}

#[test]
fn merge_project_sessions_keeps_incoming_order_and_unique_existing_sessions() {
    let incoming =
        (0..2_000).map(|index| session_info(&format!("incoming-{index}"), 0, 0)).collect();
    let mut existing = vec![session_info("incoming-0", 1, 1)];
    existing.extend((0..2_000).map(|index| session_info(&format!("existing-{index}"), 0, 0)));
    existing.push(session_info("existing-0", 2, 2));

    let merged = super::merge_project_sessions(Some(existing), incoming);

    assert_eq!(merged.len(), 4_000);
    assert_eq!(merged[0].id, "incoming-0");
    assert_eq!(merged[1_999].id, "incoming-1999");
    assert_eq!(merged[2_000].id, "existing-0");
    assert_eq!(merged[3_999].id, "existing-1999");
}

#[derive(Default)]
struct ListState {
    project_path: Option<String>,
    sessions: Vec<session_info::Info>,
    session_previews: std::collections::HashMap<String, String>,
    archived_session_ids: std::collections::HashSet<String>,
    project_sessions: std::collections::HashMap<String, Vec<session_info::Info>>,
    project_session_load_counts: std::collections::HashMap<String, usize>,
    project_sessions_loading: std::collections::HashSet<String>,
    project_session_has_vertical_scrollbar: std::collections::HashMap<String, bool>,
}
fn update(state: &mut ListState, event: super::ListEvent) -> Vec<super::ListEffect> {
    super::update_lists(&mut super::SessionLists {
        sessions: &mut state.sessions, previews: &mut state.session_previews, archived: &mut state.archived_session_ids,
        projects: &mut state.project_sessions, counts: &mut state.project_session_load_counts,
        loading: &mut state.project_sessions_loading, scrollbars: &mut state.project_session_has_vertical_scrollbar,
    }, &state.project_path, event)
}
#[test]
fn session_bootstrap_loaded_sets_previews_archived_and_sessions() {
    let mut state = ListState::default();
    let project_path = "/tmp/project".to_string();
    state.project_path = Some(project_path.clone());
    state.project_sessions_loading.insert(project_path.clone());
    let sessions = vec![session_info("s1", 1, 1)];
    let mut previews = std::collections::HashMap::new();
    previews.insert("s1".to_string(), "preview text".to_string());
    let archived = std::collections::HashSet::from(["old".to_string()]);

    let _ = update(
        &mut state,
        super::ListEvent::Bootstrap {
            project_path: Some(project_path.clone()),
            result: Ok(sessions),
            previews: previews.clone(),
            archived: archived.clone(),
        },
    );

    assert_eq!(state.session_previews, previews);
    assert_eq!(state.archived_session_ids, archived);
    assert_eq!(state.sessions[0].id, "s1");
    assert_eq!(state.project_sessions[&project_path][0].id, "s1");
    assert!(!state.project_sessions_loading.contains(&project_path));
}
#[test]
fn stale_session_bootstrap_result_is_ignored() {
    let mut state = ListState::default();
    let stale_path = "/tmp/stale".to_string();
    state.project_path = Some("/tmp/current".to_string());
    state.project_sessions_loading.insert(stale_path.clone());

    let _ = update(
        &mut state,
        super::ListEvent::Bootstrap {
            project_path: Some(stale_path.clone()),
            result: Ok(vec![session_info("stale", 1, 1)]),
            previews: std::collections::HashMap::new(),
            archived: std::collections::HashSet::new(),
        },
    );

    assert!(state.sessions.is_empty());
    assert!(!state.project_sessions.contains_key(&stale_path));
    assert!(!state.project_sessions_loading.contains(&stale_path));
}
#[test]
fn project_sessions_loaded_merges_and_clears_loading_state() {
    let mut state = ListState::default();
    let path = "/tmp/project".to_string();
    state.project_sessions_loading.insert(path.clone());
    state.project_sessions.insert(path.clone(), vec![session_info("local", 0, 0)]);

    let _ = update(
        &mut state,
        super::ListEvent::ProjectLoaded(
            path.clone(),
            Ok(vec![session_info("remote", 3, 1)]),
        ),
    );

    assert!(!state.project_sessions_loading.contains(&path));
    let sessions = state.project_sessions.get(&path).expect("sessions should be stored");
    assert_eq!(sessions.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["remote", "local"]);
    assert_eq!(state.project_session_load_counts.get(&path), Some(&10));
}
#[test]
fn project_sessions_loaded_prefers_incoming_duplicate_and_keeps_unique_existing_sessions() {
    let mut state = ListState::default();
    let path = "/tmp/project".to_string();
    state.project_sessions.insert(
        path.clone(),
        vec![session_info("duplicate", 1, 1), session_info("existing", 2, 2)],
    );

    let _ = update(
        &mut state,
        super::ListEvent::ProjectLoaded(
            path.clone(),
            Ok(vec![session_info("duplicate", 9, 8), session_info("incoming", 3, 4)]),
        ),
    );

    let sessions = state.project_sessions.get(&path).expect("sessions should be stored");
    assert_eq!(
        sessions.iter().map(|session| session.id.as_str()).collect::<Vec<_>>(),
        vec!["duplicate", "incoming", "existing"]
    );
    let summary = sessions[0].summary.as_ref().expect("duplicate should retain incoming data");
    assert_eq!((summary.additions, summary.deletions), (9, 8));
}
#[test]
fn project_session_scroll_and_load_more_update_per_project_state() {
    let mut state = ListState::default();
    let path = "/tmp/project".to_string();

    let _ = update(
        &mut state,
        super::ListEvent::Scrollbar(path.clone(), true),
    );
    assert_eq!(state.project_session_has_vertical_scrollbar.get(&path), Some(&true));

    let _ = update(
        &mut state,
        super::ListEvent::More(path.clone()),
    );
    assert_eq!(state.project_session_load_counts.get(&path), Some(&15));

    let _ = update(
        &mut state,
        super::ListEvent::More(path.clone()),
    );
    assert_eq!(state.project_session_load_counts.get(&path), Some(&20));
}
