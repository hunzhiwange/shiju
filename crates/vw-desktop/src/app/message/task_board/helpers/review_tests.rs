use crate::app::task::Task;

fn merge_task(source: Option<&str>, target: Option<&str>, worktree: Option<&str>) -> Task {
    let mut task = Task::new(1);
    task.merge_source_branch = source.map(str::to_string);
    task.merge_target_branch = target.map(str::to_string);
    task.selected_worktree_path = worktree.map(str::to_string);
    task
}

#[test]
fn split_output_without_metadata_trims_and_preserves_body_lines() {
    let parsed = super::split_output_and_git_metadata("  first line  \nsecond line\n");

    assert_eq!(parsed, ("first line  \nsecond line".to_string(), None, None, None, None));
}

#[test]
fn split_output_extracts_all_git_metadata_and_removes_marker_lines() {
    let parsed = super::split_output_and_git_metadata(
        "before\n__VW_GIT_SUMMARY__:  2 files changed  \n__VW_GIT_SOURCE_BRANCH__: feature/review\n__VW_GIT_TARGET_BRANCH__: main\n__VW_GIT_WORKTREE_PATH__: /tmp/worktree\nafter",
    );

    assert_eq!(
        parsed,
        (
            "before\nafter".to_string(),
            Some("2 files changed".to_string()),
            Some("feature/review".to_string()),
            Some("main".to_string()),
            Some("/tmp/worktree".to_string()),
        )
    );
}

#[test]
fn split_output_ignores_empty_metadata_and_keeps_last_non_empty_value() {
    let parsed = super::split_output_and_git_metadata(
        "__VW_GIT_SUMMARY__: first\n__VW_GIT_SUMMARY__:   \n__VW_GIT_SUMMARY__: final\n__VW_GIT_SOURCE_BRANCH__:\nbody",
    );

    assert_eq!(parsed.0, "body");
    assert_eq!(parsed.1.as_deref(), Some("final"));
    assert_eq!(parsed.2, None);
}

#[test]
fn split_output_keeps_unknown_and_non_prefix_markers_in_body() {
    let parsed = super::split_output_and_git_metadata(
        "prefix __VW_GIT_SUMMARY__: inline\n__VW_GIT_UNKNOWN__: value",
    );

    assert_eq!(parsed.0, "prefix __VW_GIT_SUMMARY__: inline\n__VW_GIT_UNKNOWN__: value");
    assert_eq!((parsed.1, parsed.2, parsed.3, parsed.4), (None, None, None, None));
}

#[test]
fn validate_ready_for_merge_rejects_missing_project_path() {
    let task = merge_task(Some("feature/review"), Some("main"), Some("/tmp/worktree"));

    assert_eq!(
        super::validate_ready_for_merge(&task, None),
        Err("缺少项目路径，无法执行合并".to_string())
    );
}

#[test]
fn validate_ready_for_merge_rejects_missing_blank_and_head_source() {
    for source in [None, Some("  "), Some(" HEAD ")] {
        let task = merge_task(source, Some("main"), Some("/tmp/worktree"));

        assert_eq!(
            super::validate_ready_for_merge(&task, Some("/project")),
            Err("缺少有效 source branch，无法自动合并".to_string())
        );
    }
}

#[test]
fn validate_ready_for_merge_rejects_head_target() {
    let task = merge_task(Some("feature/review"), Some(" HEAD "), Some("/tmp/worktree"));

    assert_eq!(
        super::validate_ready_for_merge(&task, Some("/project")),
        Err("无效 target branch，无法自动合并: HEAD".to_string())
    );
}

#[test]
fn validate_ready_for_merge_rejects_equal_trimmed_branches() {
    let task = merge_task(Some(" feature/review "), Some("feature/review"), Some("/tmp/worktree"));

    assert_eq!(
        super::validate_ready_for_merge(&task, Some("/project")),
        Err("source branch 与 target branch 相同，无法自动合并: feature/review".to_string())
    );
}

#[test]
fn validate_ready_for_merge_rejects_missing_or_blank_worktree() {
    for worktree in [None, Some(" \t ")] {
        let task = merge_task(Some("feature/review"), Some("main"), worktree);

        assert_eq!(
            super::validate_ready_for_merge(&task, Some("/project")),
            Err("缺少任务 worktree，无法自动合并".to_string())
        );
    }
}

#[test]
fn validate_ready_for_merge_accepts_distinct_or_omitted_target() {
    for target in [None, Some("  "), Some(" main ")] {
        let task = merge_task(Some(" feature/review "), target, Some(" /tmp/worktree "));

        assert_eq!(super::validate_ready_for_merge(&task, Some("/project")), Ok(()));
    }
}

#[test]
fn build_task_prompt_trims_raw_input_and_ignores_advisory_arguments() {
    let subtasks = vec!["first".to_string(), "second".to_string()];

    assert_eq!(
        super::build_task_prompt_from_input(" \n  implement review tests  \t", Some(7), &subtasks),
        "implement review tests"
    );
    assert_eq!(super::build_task_prompt_from_input(" \n\t ", None, &[]), "");
}

#[test]
fn parse_review_decision_accepts_direct_ok_json() {
    assert_eq!(super::parse_review_decision(r#"{"status":"ok"}"#), Ok(()));
}

#[test]
fn parse_review_decision_returns_trimmed_error_reason_or_default() {
    assert_eq!(
        super::parse_review_decision(r#"{"status":"error","error":"  unsafe merge  "}"#),
        Err("unsafe merge".to_string())
    );
    for output in [
        r#"{"status":"error"}"#,
        r#"{"status":"error","error":"   "}"#,
        r#"{"status":"error","error":42}"#,
    ] {
        assert_eq!(super::parse_review_decision(output), Err("风险太高".to_string()));
    }
}

#[test]
fn parse_review_decision_finds_nested_object_and_json_string() {
    assert_eq!(super::parse_review_decision(r#"{"result":{"payload":{"status":"ok"}}}"#), Ok(()));
    assert_eq!(
        super::parse_review_decision(
            r#"{"payload":"{\"status\":\"error\",\"error\":\"blocked\"}"}"#
        ),
        Err("blocked".to_string())
    );
}

#[test]
fn parse_review_decision_uses_last_array_or_line_decision() {
    assert_eq!(
        super::parse_review_decision(r#"[{"status":"error","error":"stale"},{"status":"ok"}]"#),
        Ok(())
    );
    assert_eq!(
        super::parse_review_decision(
            "review log\n{\"status\":\"ok\"}\n{\"status\":\"error\",\"error\":\"latest\"}"
        ),
        Err("latest".to_string())
    );
}

#[test]
fn parse_review_decision_finds_last_embedded_balanced_object() {
    let output = concat!(
        "prefix {\"status\":\"error\",\"error\":\"old\"} middle ",
        "{\"wrapper\":{\"status\":\"ok\"}} suffix"
    );

    assert_eq!(super::parse_review_decision(output), Ok(()));
}

#[test]
fn parse_review_decision_ignores_unmatched_closing_braces_before_json() {
    assert_eq!(super::parse_review_decision("noise } } then {\"status\":\"ok\"}"), Ok(()));
}

#[test]
fn parse_review_decision_unescapes_json_candidate() {
    assert_eq!(super::parse_review_decision(r#"{\"status\":\"ok\"}"#), Ok(()));
}

#[test]
fn parse_review_decision_uses_legacy_error_marker_fallback() {
    assert_eq!(
        super::parse_review_decision(r#"malformed prefix "status":"error" suffix"#),
        Err("风险太高".to_string())
    );
}

#[test]
fn parse_review_decision_rejects_empty_unknown_and_incomplete_outputs() {
    for output in ["", "   \n", r#"{"status":"pending"}"#, "prefix {\"status\":\"ok\""] {
        assert_eq!(super::parse_review_decision(output), Err("审核结果缺少可解析结论".to_string()));
    }
}
