// 验证任务输出的元数据拆分与提示文本处理。

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
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
fn build_task_prompt_trims_raw_input_and_ignores_advisory_arguments() {

    assert_eq!(
        super::build_task_prompt_from_input(" \n  implement review tests  \t", Some(7)),
        "implement review tests"
    );
    assert_eq!(super::build_task_prompt_from_input(" \n\t ", None), "");
}
