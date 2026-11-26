//! Cross-page legacy-control source contracts; serialized settings assertions live in desktop-task-board.
#[test]
fn legacy_task_board_switches_are_ignored_after_capability_cutover() {
    let control_source = include_str!("../../../views/task_board/control.rs");
    assert!(!control_source.contains("自动执行任务池 ·"));
    assert!(!control_source.contains("代码审核 ·"));
    assert!(!control_source.contains("ToggleAutoPromotePoolTasks"));
    assert!(!control_source.contains("ToggleCodeReview"));

    let modal_source = include_str!("../../../views/task_board/modals.rs");

    let project_refresh_source = include_str!(
        "../../../../../../libs/desktop-workspace/src/project_edit_view/refresh_tab.rs"
    );
    assert!(!project_refresh_source.contains("自动拉起任务池"));
    assert!(!project_refresh_source.contains("执行代码审查"));
    assert!(!project_refresh_source.contains("ProjectEditAutoPromotePoolTasksToggled"));
    assert!(!project_refresh_source.contains("ProjectEditCodeReviewToggled"));
}
