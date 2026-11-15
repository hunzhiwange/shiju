use vw_api_transport::id::TenantScope;
use super::*;
fn identity(company: &str) -> Identity {
    Identity {
        gateway: "http://127.0.0.1:1".into(),
        tenant: TenantScope::new("10000", company).unwrap(),
        auth_fingerprint: [0; 32],
    }
}

fn restore_state() -> (ArchivedChatsState, Identity) {
    let current = identity("100000000");
    (
        ArchivedChatsState {
            identity: Some(current.clone()),
            sessions: vec![info("a", "p", "Archived", 10), info("child", "q", "Child", 9)],
            selected_project_id: Some("p".into()),
            request_generation: 0,
            ..Default::default()
        },
        current,
    )
}

fn restored(current: &Identity, result: Result<Info, String>) -> ArchivedChatsMessage {
    ArchivedChatsMessage::Restored {
        identity: current.clone(),
        generation: 1,
        id: "a".into(),
        result,
    }
}

#[test]
fn archived_chats_restore_waits_for_persistence_and_clears_empty_project() {
    let (mut state, current) = restore_state();
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    assert!(state.operating_ids.contains("a"));
    assert_eq!(state.sessions.len(), 2);
    let mut saved = state.sessions[0].clone();
    saved.time.archived = None;
    apply(&mut state, restored(&current, Ok(saved)), &current);
    assert_eq!(state.sessions.len(), 1);
    assert_eq!(state.sessions[0].id, "child");
    assert!(state.sessions[0].time.archived.is_some());
    assert!(state.selected_project_id.is_none());
    assert!(!state.operating_ids.contains("a"));
    assert_eq!(state.project_options().len(), 1);
}

#[test]
fn archived_chats_restore_failure_retains_row_and_allows_retry() {
    let (mut state, current) = restore_state();
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    apply(&mut state, restored(&current, Err("fixture unavailable".into())), &current);
    assert_eq!(state.sessions.len(), 2);
    assert!(state.sessions[0].time.archived.is_some());
    assert!(!state.operating_ids.contains("a"));
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    assert!(state.operating_ids.contains("a"));
}

#[test]
fn archived_chats_restore_rejects_other_endpoint_and_unsolicited_response() {
    let (mut state, current) = restore_state();
    let mut saved = state.sessions[0].clone();
    saved.time.archived = None;
    apply(&mut state, restored(&current, Ok(saved.clone())), &current);
    assert_eq!(state.sessions.len(), 2);
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    let mut other = current.clone();
    other.gateway = "http://127.0.0.1:2".into();
    apply(&mut state, restored(&other, Ok(saved)), &current);
    assert_eq!(state.sessions.len(), 2);
    assert!(state.operating_ids.contains("a"));
}

#[test]
fn archived_chats_restore_duplicate_and_conflicting_delete_do_not_start_requests() {
    let (mut state, current) = restore_state();
    state.delete_confirmation =
        Some(crate::state::ArchivedChatsDeleteConfirmation::Chat("a".into()));
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    assert!(state.operating_ids.is_empty());
    state.delete_confirmation =
        Some(crate::state::ArchivedChatsDeleteConfirmation::Project("p".into()));
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    assert!(state.operating_ids.is_empty());
    state.delete_confirmation = None;
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    let generation = state.request_generation;
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    apply(&mut state, ArchivedChatsMessage::Refresh, &current);
    assert_eq!(state.request_generation, generation);
    assert!(!state.loading);
    assert_eq!(state.restore_requests.len(), 1);
}

#[test]
fn archived_chats_restore_retry_and_same_endpoint_reset_reject_late_responses() {
    let (mut state, current) = restore_state();
    let mut saved = state.sessions[0].clone();
    saved.time.archived = None;
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    apply(&mut state, restored(&current, Err("failed".into())), &current);
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    apply(&mut state, restored(&current, Ok(saved.clone())), &current);
    assert_eq!(state.sessions.len(), 2);
    assert!(state.operating_ids.contains("a"));
    reset(&mut state);
    state.identity = Some(current.clone());
    state.sessions = vec![info("a", "p", "Archived again", 10)];
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    apply(&mut state, restored(&current, Ok(saved)), &current);
    assert_eq!(state.sessions.len(), 1);
    assert!(state.operating_ids.contains("a"));
}

#[test]
fn archived_chats_restore_success_invalidates_older_project_refreshes() {
    let (mut state, current) = restore_state();
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    let first_generation = state.restore_requests["a"];
    apply(&mut state, ArchivedChatsMessage::Restore("child".into()), &current);
    let second_generation = state.restore_requests["child"];
    let mut first = state.sessions[0].clone();
    first.time.archived = None;
    let mut second = state.sessions[1].clone();
    second.time.archived = None;
    apply(
        &mut state,
        ArchivedChatsMessage::Restored {
            identity: current.clone(),
            generation: first_generation,
            id: "a".into(),
            result: Ok(first),
        },
        &current,
    );
    let refresh_generation = state.request_generation;
    apply(
        &mut state,
        ArchivedChatsMessage::Restored {
            identity: current.clone(),
            generation: second_generation,
            id: "child".into(),
            result: Ok(second),
        },
        &current,
    );
    assert!(state.sessions.is_empty());
    assert!(state.operating_ids.is_empty());
    assert!(
        state.request_generation > refresh_generation,
        "older project snapshots must be invalidated by later successful restores"
    );
}

fn info(id: &str, project: &str, title: &str, updated: i64) -> Info {
    serde_json::from_value(serde_json::json!({
        "id": id, "slug": id, "projectID": project, "directory": format!("/fixture/{project}"),
        "title": title, "version": "1",
        "time": {"created": 100 - updated, "updated": updated, "archived": 1}
    }))
    .unwrap()
}

fn load(
    state: &mut ArchivedChatsState,
    current: &Identity,
    generation: u64,
    result: Result<Vec<Info>, String>,
) {
    apply(
        state,
        ArchivedChatsMessage::Loaded { identity: current.clone(), generation, result },
        current,
    );
}

#[test]
fn archived_chats_browse_loads_historical_projects() {
    let mut state =
        ArchivedChatsState { loading: true, request_generation: 1, ..Default::default() };
    load(
        &mut state,
        &identity("100000000"),
        1,
        Ok(vec![info("a", "old-project", "旧项目聊天", 10)]),
    );
    assert!(!state.loading);
    assert_eq!(state.groups()[0].project.name, "old-project");
    assert_eq!(state.groups()[0].rows[0].title, "旧项目聊天");
}

#[test]
fn archived_chats_browse_filters_and_all_three_sorts() {
    let current = identity("100000000");
    let mut state = ArchivedChatsState {
        sessions: vec![
            info("a", "p", "Zulu", 20),
            info("b", "p", "Alpha", 10),
            info("c", "q", "Other", 30),
        ],
        ..Default::default()
    };
    apply(&mut state, ArchivedChatsMessage::ProjectSelected(Some("p".into())), &current);
    assert_eq!(state.groups().len(), 1);
    for (sort, first) in [
        (ArchivedChatsSort::Updated, "a"),
        (ArchivedChatsSort::Created, "b"),
        (ArchivedChatsSort::Title, "b"),
    ] {
        apply(&mut state, ArchivedChatsMessage::SortSelected(sort), &current);
        assert_eq!(state.groups()[0].rows[0].id, first);
    }
    apply(&mut state, ArchivedChatsMessage::SearchChanged("  ZULU ".into()), &current);
    assert_eq!(state.groups()[0].rows.len(), 1);
    apply(&mut state, ArchivedChatsMessage::SearchChanged("missing".into()), &current);
    assert!(state.groups().is_empty());
}

#[test]
fn archived_chats_browse_retries_after_failure() {
    let mut state =
        ArchivedChatsState { loading: true, request_generation: 1, ..Default::default() };
    let current = identity("100000000");
    load(&mut state, &current, 1, Err("unavailable".into()));
    assert_eq!(state.error.as_deref(), Some("unavailable"));
    assert!(!state.loading);
    apply(&mut state, ArchivedChatsMessage::Refresh, &current);
    assert!(state.loading);
    assert_eq!(state.request_generation, 2);
    assert!(state.error.is_none());
    load(&mut state, &current, 2, Ok(vec![]));
    assert!(!state.loading);
    assert!(state.error.is_none());
}

#[test]
fn archived_chats_browse_rejects_out_of_order_results() {
    let mut state = ArchivedChatsState::default();
    let current = identity("100000000");
    apply(&mut state, ArchivedChatsMessage::Refresh, &current);
    apply(&mut state, ArchivedChatsMessage::Refresh, &current);
    assert_eq!(state.request_generation, 2);
    load(&mut state, &current, 2, Ok(vec![info("new", "p", "New", 2)]));
    load(&mut state, &current, 1, Ok(vec![info("old", "p", "Old", 1)]));
    assert_eq!(state.sessions[0].id, "new");
}

#[test]
fn archived_chats_browse_rejects_other_tenant_and_gateway() {
    let mut state =
        ArchivedChatsState { loading: true, request_generation: 1, ..Default::default() };
    let current = identity("100000001");
    apply(
        &mut state,
        ArchivedChatsMessage::Loaded {
            identity: identity("100000000"),
            generation: 1,
            result: Ok(vec![info("old", "p", "Old", 1)]),
        },
        &current,
    );
    assert!(state.sessions.is_empty());
    assert!(state.loading);
    let mut old_gateway = current.clone();
    old_gateway.gateway = "http://127.0.0.1:2".into();
    apply(
        &mut state,
        ArchivedChatsMessage::Loaded {
            identity: old_gateway,
            generation: 1,
            result: Err("old".into()),
        },
        &current,
    );
    assert!(state.error.is_none());
}

#[test]
fn archived_chats_browse_switch_clears_data_and_pending_actions() {
    let mut state = ArchivedChatsState {
        sessions: vec![info("old", "p", "Old", 1)],
        search: "old".into(),
        selected_project_id: Some("p".into()),
        request_generation: 8,
        loading: true,
        error: Some("old".into()),
        ..Default::default()
    };
    state.operating_ids.insert("old".into());
    state.delete_confirmation =
        Some(crate::state::ArchivedChatsDeleteConfirmation::Chat("old".into()));
    reset(&mut state);
    assert!(state.sessions.is_empty());
    assert!(state.selected_project_id.is_none());
    assert!(state.operating_ids.is_empty());
    assert!(state.delete_confirmation.is_none());
    assert!(state.error.is_none());
    assert!(!state.loading);
    assert!(state.request_generation > 8);
}

#[test]
fn archived_chats_browse_refresh_invalidates_previous_identity() {
    let old = identity("100000000");
    let mut state = ArchivedChatsState::default();
    apply(&mut state, ArchivedChatsMessage::Refresh, &old);
    state.sessions = vec![info("old", "p", "Old", 1)];
    let current = identity("100000001");
    apply(&mut state, ArchivedChatsMessage::Refresh, &current);
    assert!(state.sessions.is_empty());
    let generation = state.request_generation;
    apply(
        &mut state,
        ArchivedChatsMessage::Loaded {
            identity: old,
            generation,
            result: Ok(vec![info("old", "p", "Old", 1)]),
        },
        &current,
    );
    assert!(state.loading);
    load(&mut state, &current, generation, Ok(vec![info("new", "q", "New", 2)]));
    assert_eq!(state.sessions[0].id, "new");
    let mut other_auth = current.clone();
    other_auth.auth_fingerprint = [1; 32];
    apply(&mut state, ArchivedChatsMessage::Refresh, &other_auth);
    assert!(state.sessions.is_empty());
}

#[test]
fn archived_chats_delete_requires_confirmation_and_cancel_keeps_rows() {
    let (mut state, current) = restore_state();
    apply(&mut state, ArchivedChatsMessage::DeleteRequested("a".into()), &current);
    assert!(state.delete_confirmation.is_some());
    assert!(state.operating_ids.is_empty());
    assert_eq!(state.sessions.len(), 2);
    apply(&mut state, ArchivedChatsMessage::DeleteCancelled, &current);
    apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
    assert!(state.delete_confirmation.is_none());
    assert!(state.operating_ids.is_empty());
    assert_eq!(state.sessions.len(), 2);
}

#[test]
fn archived_chats_delete_confirm_once_and_reject_changed_target_or_service() {
    let (mut state, current) = restore_state();
    apply(&mut state, ArchivedChatsMessage::DeleteRequested("a".into()), &current);
    state.sessions[0].project_id = "changed".into();
    apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
    assert!(state.delete_confirmation.is_none());
    assert!(state.operating_ids.is_empty());
    apply(&mut state, ArchivedChatsMessage::DeleteRequested("a".into()), &current);
    let mut other = current.clone();
    other.auth_fingerprint = [1; 32];
    apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &other);
    assert!(state.operating_ids.is_empty());
    apply(&mut state, ArchivedChatsMessage::DeleteRequested("a".into()), &current);
    apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
    state.error = Some("previous refresh failure".into());
    let generation = state.delete_generation.unwrap();
    apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
    apply(&mut state, ArchivedChatsMessage::DeleteCancelled, &current);
    assert_eq!(state.delete_generation, Some(generation));
    assert_eq!(state.operating_ids.len(), 1);
    assert_eq!(state.sessions.len(), 2);
    apply(
        &mut state,
        ArchivedChatsMessage::Deleted {
            identity: other,
            generation,
            id: "a".into(),
            result: Ok(vec!["a".into()]),
        },
        &current,
    );
    assert_eq!(state.sessions.len(), 2);
    apply(
        &mut state,
        ArchivedChatsMessage::Deleted {
            identity: current.clone(),
            generation,
            id: "a".into(),
            result: Ok(vec!["child".into(), "a".into()]),
        },
        &current,
    );
    assert!(state.sessions.is_empty());
    assert!(state.error.is_none());
    assert!(state.selected_project_id.is_none());
    assert!(state.delete_confirmation.is_none());
    assert!(state.operating_ids.is_empty());
}

#[test]
fn archived_chats_delete_refresh_closes_confirmation_for_restored_or_missing_target() {
    for restored in [false, true] {
        let (mut state, current) = restore_state();
        apply(&mut state, ArchivedChatsMessage::DeleteRequested("a".into()), &current);
        apply(&mut state, ArchivedChatsMessage::Refresh, &current);
        let mut rows = state.sessions.clone();
        if restored {
            rows[0].time.archived = None;
        } else {
            rows.remove(0);
        }
        let generation = state.request_generation;
        load(&mut state, &current, generation, Ok(rows));
        assert!(state.delete_confirmation.is_none());
        apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
        assert!(state.operating_ids.is_empty());
    }
}

#[test]
fn archived_chats_project_delete_confirmation_starts_project_operation() {
    let (mut state, current) = restore_state();
    state.delete_confirmation =
        Some(crate::state::ArchivedChatsDeleteConfirmation::Project("p".into()));
    apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
    assert!(state.delete_generation.is_some(), "project confirmation must start deletion");
}

fn project_request(state: &mut ArchivedChatsState, current: &Identity) {
    apply(state, ArchivedChatsMessage::ProjectDeleteRequested("p".into()), current);
}

fn project_result(
    state: &mut ArchivedChatsState,
    current: &Identity,
    result: Result<Option<Vec<String>>, String>,
) {
    let batch = state.project_delete.as_ref().unwrap();
    let id = batch.targets[batch.next].id.clone();
    let generation = state.delete_generation.unwrap();
    apply(
        state,
        ArchivedChatsMessage::ProjectDeleted { identity: current.clone(), generation, id, result },
        current,
    );
}

#[test]
fn archived_chats_project_delete_freezes_hidden_rows_and_keeps_project_boundary() {
    let (mut state, current) = restore_state();
    let mut live = info("live", "p", "Live", 5);
    live.time.archived = None;
    state.sessions.extend([info("b", "p", "Hidden", 8), live]);
    state.project_names.insert("p".into(), "同名项目".into());
    state.project_names.insert("q".into(), "同名项目".into());
    state.search = "Archived".into();
    project_request(&mut state, &current);
    assert_eq!(
        state
            .project_delete
            .as_ref()
            .unwrap()
            .targets
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    state.sessions.push(info("new", "p", "New archive", 1));
    apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
    project_result(&mut state, &current, Ok(Some(vec!["a".into()])));
    assert!(state.delete_generation.is_some());
    project_result(&mut state, &current, Ok(Some(vec!["b".into()])));
    assert!(state.delete_generation.is_none());
    assert_eq!(
        state.sessions.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["child", "live", "new"]
    );
}

#[test]
fn archived_chats_project_delete_deduplicates_ancestor_roots() {
    let (mut state, current) = restore_state();
    let mut child = info("nested", "p", "Nested", 8);
    child.parent_id = Some("a".into());
    state.sessions.push(child);
    state.sessions.push(state.sessions[0].clone());
    project_request(&mut state, &current);
    assert_eq!(state.project_delete.as_ref().unwrap().targets.len(), 1);
    assert_eq!(state.project_delete.as_ref().unwrap().targets[0].id, "a");
}

#[test]
fn archived_chats_project_delete_cancel_sends_no_operation() {
    let (mut state, current) = restore_state();
    project_request(&mut state, &current);
    apply(&mut state, ArchivedChatsMessage::DeleteCancelled, &current);
    apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
    assert!(state.operating_ids.is_empty());
    assert!(state.project_delete.is_none());
}

#[test]
fn archived_chats_project_delete_partial_failure_retries_only_remaining_archived_failures() {
    let (mut state, current) = restore_state();
    state.sessions.extend([info("b", "p", "B", 2), info("c", "p", "C", 1)]);
    project_request(&mut state, &current);
    apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
    project_result(&mut state, &current, Ok(Some(vec!["a".into()])));
    project_result(&mut state, &current, Err("fixture failure".into()));
    project_result(&mut state, &current, Ok(None));
    let batch = state.project_delete.as_ref().unwrap();
    assert_eq!((batch.succeeded, batch.failed.len(), batch.skipped), (1, 1, 1));
    apply(&mut state, ArchivedChatsMessage::ProjectDeleteRetry, &current);
    assert_eq!(
        state
            .project_delete
            .as_ref()
            .unwrap()
            .targets
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        ["b"]
    );
    apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
    project_result(&mut state, &current, Err("fixture failure".into()));
    state.sessions.iter_mut().find(|r| r.id == "b").unwrap().time.archived = None;
    apply(&mut state, ArchivedChatsMessage::ProjectDeleteRetry, &current);
    assert!(state.delete_confirmation.is_none());
}

#[test]
fn archived_chats_project_delete_endpoint_change_rejects_old_result() {
    let (mut state, current) = restore_state();
    project_request(&mut state, &current);
    apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
    let generation = state.delete_generation.unwrap();
    let other = identity("100000001");
    apply(
        &mut state,
        ArchivedChatsMessage::ProjectDeleted {
            identity: current.clone(),
            generation,
            id: "a".into(),
            result: Ok(Some(vec!["a".into()])),
        },
        &other,
    );
    assert_eq!(state.project_delete.as_ref().unwrap().next, 0);
    reset(&mut state);
    assert!(state.project_delete.is_none());
    assert!(state.operating_ids.is_empty());
}

#[test]
fn archived_chats_project_delete_finishes_empty_project_and_rejects_duplicate_result() {
    let (mut state, current) = restore_state();
    project_request(&mut state, &current);
    apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
    let generation = state.delete_generation.unwrap();
    project_result(&mut state, &current, Ok(Some(vec!["a".into()])));
    assert!(state.selected_project_id.is_none());
    assert!(state.project_action_menu.is_none());
    assert!(state.delete_confirmation.is_none());
    assert!(state.operating_ids.is_empty());
    apply(
        &mut state,
        ArchivedChatsMessage::ProjectDeleted {
            identity: current.clone(),
            generation,
            id: "a".into(),
            result: Ok(Some(vec!["child".into()])),
        },
        &current,
    );
    assert_eq!(state.sessions[0].id, "child");
    assert_eq!(state.project_delete.as_ref().unwrap().succeeded, 1);
}