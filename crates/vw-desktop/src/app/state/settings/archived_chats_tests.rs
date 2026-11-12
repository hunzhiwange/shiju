use super::*;
use vw_shared::session::info::TimeInfo;

fn session(id: &str, project: &str, title: &str, created: u64, updated: u64) -> Info {
    Info {
        id: id.into(),
        slug: id.into(),
        project_id: project.into(),
        directory: format!("/workspace/{project}"),
        parent_id: None,
        summary: None,
        share: None,
        title: title.into(),
        version: "1".into(),
        time: TimeInfo { created, updated, compacting: None, archived: Some(10) },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

fn ids(state: &ArchivedChatsState) -> Vec<String> {
    state.groups().iter().flat_map(|group| &group.rows).map(|row| row.id.clone()).collect()
}

#[test]
fn archived_chats_query_excludes_unarchived_and_internal_sessions() {
    let mut active = session("active", "p", "Active", 1, 1);
    active.time.archived = None;
    let mut agent = session("agent", "p", "Agent", 1, 1);
    agent.mark_agent_tool();
    let mut plan = session("plan", "p", "Plan", 1, 1);
    plan.slug = Info::plan_run_slug(&plan.slug);
    let mut pending = session("pending", "p", "Pending", 1, 1);
    pending.mark_first_response_pending();
    let state = ArchivedChatsState {
        sessions: vec![active, agent, plan, pending, session("visible", "p", "Visible", 1, 1)],
        ..Default::default()
    };
    assert_eq!(ids(&state), ["visible"]);
    assert_eq!(state.project_options().len(), 1);
}

#[test]
fn archived_chats_query_includes_archived_children_and_groups_worktrees_by_project_id() {
    let root = session("root", "p", "Root", 1, 2);
    let mut child = session("child", "p", "Child", 1, 3);
    child.parent_id = Some("root".into());
    child.directory = "/worktrees/branch".into();
    let state = ArchivedChatsState { sessions: vec![root, child], ..Default::default() };
    let groups = state.groups();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].rows.len(), 2);
    assert_eq!(ids(&state), ["child", "root"]);
}

#[test]
fn archived_chats_query_search_trims_and_matches_chinese_and_case_insensitively() {
    let mut state = ArchivedChatsState {
        sessions: vec![session("a", "p", "归档 Rust 聊天", 1, 1), session("b", "p", "Other", 1, 1)],
        search: "  归档  ".into(),
        ..Default::default()
    };
    assert_eq!(ids(&state), ["a"]);
    state.search = " RUST ".into();
    assert_eq!(ids(&state), ["a"]);
}

#[test]
fn archived_chats_query_same_name_projects_remain_separate_and_filter_by_id() {
    let mut state = ArchivedChatsState {
        sessions: vec![session("a", "p1", "Chat", 1, 1), session("b", "p2", "Chat", 1, 1)],
        project_names: HashMap::from([("p1".into(), "Same".into()), ("p2".into(), "Same".into())]),
        ..Default::default()
    };
    assert_eq!(state.groups().len(), 2);
    assert_eq!(state.project_options().len(), 2);
    state.selected_project_id = Some("p2".into());
    assert_eq!(ids(&state), ["b"]);
    assert_eq!(state.project_options().len(), 2);
}

#[test]
fn archived_chats_query_three_sorts_use_id_for_equal_keys() {
    let mut state = ArchivedChatsState {
        sessions: vec![
            session("b", "p", "ÄBC", 3, 1),
            session("a", "p", "äbc", 3, 1),
            session("c", "p", "Alpha", 1, 5),
        ],
        ..Default::default()
    };
    assert_eq!(ids(&state), ["c", "a", "b"]);
    state.sort = ArchivedChatsSort::Created;
    assert_eq!(ids(&state), ["a", "b", "c"]);
    state.sort = ArchivedChatsSort::Title;
    assert_eq!(ids(&state), ["c", "a", "b"]);
    state.sessions.reverse();
    assert_eq!(ids(&state), ["c", "a", "b"]);
}

#[test]
fn archived_chats_query_deduplicates_ids_before_visible_counts() {
    let row = session("a", "p", "Chat", 1, 1);
    let state = ArchivedChatsState { sessions: vec![row.clone(), row], ..Default::default() };
    assert_eq!(ids(&state), ["a"]);
    assert_eq!(state.groups()[0].rows.len(), 1);
}

#[test]
fn archived_chats_query_empty_results_preserve_complete_project_options_and_fallback_names() {
    let mut unnamed = session("b", "unknown", "Other", 1, 1);
    unnamed.directory = "/".into();
    let state = ArchivedChatsState {
        sessions: vec![session("a", "historical", "Chat", 1, 1), unnamed],
        search: "missing".into(),
        ..Default::default()
    };
    assert!(state.groups().is_empty());
    let options = state.project_options();
    assert_eq!(options.len(), 2);
    assert!(options.iter().any(|p| p.id == "historical" && p.name == "historical"));
    assert!(options.iter().any(|p| p.id == "unknown" && p.name == "unknown"));
    assert!(ArchivedChatsState::default().groups().is_empty());
}

#[test]
fn archived_chats_query_search_counts_only_visible_rows() {
    let state = ArchivedChatsState {
        sessions: vec![session("a", "p", "Match", 1, 1), session("b", "p", "Other", 1, 1)],
        search: "match".into(),
        ..Default::default()
    };
    assert_eq!(state.groups()[0].rows.len(), 1);
    assert_eq!(state.project_options().len(), 1);
}
