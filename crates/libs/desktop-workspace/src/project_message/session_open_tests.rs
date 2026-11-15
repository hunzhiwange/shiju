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
