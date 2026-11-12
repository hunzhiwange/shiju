use super::*;
use vw_gateway_client::CronHistoryRunDto;

fn run(id: &str, name: Option<&str>, status: &str) -> CronHistoryRunDto {
    CronHistoryRunDto {
        id: Some(1),
        execution_id: None,
        job_id: id.into(),
        name: name.map(str::to_string),
        status: status.into(),
        started_at: "start".into(),
        finished_at: None,
        duration_ms: None,
        output: None,
    }
}

#[test]
fn cron_history_filters_intersect() {
    let mut state = CronHistoryState::default();
    state.runs = vec![
        run("a", Some("  Backup  "), "ok"),
        run("b", Some("Backup"), "error"),
        run("c", None, "running"),
    ];
    state.search = " BACK ".into();
    state.status_filter = CronHistoryStatus::Success;
    assert_eq!(state.visible_runs().count(), 1);
    state.selected_job_id = Some("b".into());
    assert_eq!(state.visible_runs().count(), 0);
    state.search.clear();
    state.selected_job_id = None;
    state.status_filter = CronHistoryStatus::Running;
    assert_eq!(state.visible_runs().next().unwrap().job_id, "c");
    assert_eq!(CronHistoryState::display_name(&state.runs[2]), "c");
    assert_eq!(CronHistoryState::display_name(&run("empty", Some("  "), "ok")), "empty");
}

#[test]
fn cron_history_duplicate_names_use_ids() {
    let mut state = CronHistoryState::default();
    state.runs = vec![run("a", Some("same"), "ok"), run("b", Some("same"), "ok")];
    state.selected_job_id = Some("b".into());
    assert_eq!(state.visible_runs().map(|run| run.job_id.as_str()).collect::<Vec<_>>(), vec!["b"]);
}

#[test]
fn cron_history_unknown_status_is_not_success() {
    let mut state = CronHistoryState::default();
    state.runs = vec![run("a", None, "unknown"), run("b", None, "ok"), run("c", None, "error")];
    state.status_filter = CronHistoryStatus::Success;
    assert_eq!(state.visible_runs().count(), 1);
    state.status_filter = CronHistoryStatus::Failed;
    assert_eq!(state.visible_runs().next().unwrap().job_id, "c");
    state.status_filter = CronHistoryStatus::All;
    assert_eq!(state.visible_runs().count(), 3);
}

#[test]
fn cron_history_context_invalidates_inflight_and_keeps_filters() {
    let mut state = CronHistoryState::default();
    state.search = "backup".into();
    assert!(state.sync_context(true, Some("one".into())));
    let first = state.begin_request().unwrap();
    assert!(state.accepts(first));
    assert_eq!(state.begin_request(), None);
    assert!(!state.sync_context(false, Some("one".into())));
    assert!(!state.accepts(first));
    assert!(state.sync_context(true, Some("two".into())));
    let second = state.begin_request().unwrap();
    assert_ne!(first, second);
    assert!(!state.accepts(first));
    assert!(state.accepts(second));
    assert_eq!(state.search, "backup");
}

#[test]
fn cron_history_refresh_prunes_deleted_selection_and_preserves_filters() {
    let mut state = CronHistoryState::default();
    state.search = "search".into();
    state.status_filter = CronHistoryStatus::Failed;
    state.location_filter = CronHistoryLocation::Local;
    state.selected_job_id = Some("deleted".into());
    state.apply_response(
        vw_gateway_client::CronHistoryResponse {
            runs: vec![run("a", Some("search"), "error")],
            finished_limit: 200,
            truncated: true,
        },
        &[],
    );
    assert_eq!(state.selected_job_id, None);
    assert_eq!(state.search, "search");
    assert_eq!(state.visible_runs().count(), 1);
    assert_eq!(state.finished_limit, 200);
    assert!(state.truncated);
}

#[test]
fn cron_history_candidates_search_all_jobs() {
    let job: vw_gateway_client::CronJobDto = serde_json::from_value(serde_json::json!({
        "id": "unrun", "name": " DAILY ", "job_type": "shell", "schedule_kind": "cron",
        "expression": "* * * * *", "command": "echo", "next_run": "later", "enabled": true
    }))
    .unwrap();
    let mut state = CronHistoryState::default();
    state.task_search = " DaI ".into();
    let jobs = vec![job];
    assert_eq!(state.task_candidates(&jobs), vec![("unrun", "DAILY".into())]);
    state.task_picker_open = true;
    state.sync_context(true, None);
    assert!(!state.task_picker_open);
    assert!(state.error.is_none());
}
