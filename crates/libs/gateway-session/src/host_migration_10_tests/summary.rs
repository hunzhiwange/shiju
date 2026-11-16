use super::{task_session_summary_needs_refresh, refresh_task_session_summaries};
use session_info as session;

fn summary(files: i64, additions: i64, deletions: i64) -> session::Summary {
    session::Summary { additions, deletions, files, diffs: None }
}

fn info_with_summary(
    id: &str,
    summary: Option<session::Summary>,
) -> session::Info {
    session::Info {
        id: id.to_string(),
        slug: "fixture-slug".to_string(),
        project_id: "fixture-project".to_string(),
        directory: "/tmp/fixture".to_string(),
        parent_id: None,
        summary,
        share: None,
        title: "Fixture".to_string(),
        version: "test".to_string(),
        time: session::TimeInfo {
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
fn task_session_summary_needs_refresh_only_for_empty_task_board_summaries() {
    let non_task = info_with_summary("ses-fixture", None);
    assert!(!task_session_summary_needs_refresh(&non_task));

    let missing = info_with_summary("task-board-missing", None);
    assert!(task_session_summary_needs_refresh(&missing));

    let zero = info_with_summary("task-board-zero", Some(summary(0, 0, 0)));
    assert!(task_session_summary_needs_refresh(&zero));

    let with_files = info_with_summary("task-board-files", Some(summary(1, 0, 0)));
    assert!(!task_session_summary_needs_refresh(&with_files));

    let with_additions = info_with_summary("task-board-additions", Some(summary(0, 1, 0)));
    assert!(!task_session_summary_needs_refresh(&with_additions));

    let with_deletions = info_with_summary("task-board-deletions", Some(summary(0, 0, 1)));
    assert!(!task_session_summary_needs_refresh(&with_deletions));
}

#[tokio::test]
async fn refresh_task_session_summaries_preserves_summary_when_refresh_fails() {

    let original = summary(0, 0, 0);
    let mut sessions =
        vec![info_with_summary("task-board-missing-refresh-fixture", Some(original.clone()))];

    refresh_task_session_summaries(&mut sessions, async |_| Err("fixture refresh failed".to_string())).await;

    let refreshed = sessions[0].summary.as_ref().expect("summary should be preserved");
    assert_eq!(refreshed.files, original.files);
    assert_eq!(refreshed.additions, original.additions);
    assert_eq!(refreshed.deletions, original.deletions);
}
