
#[test]
fn desktop_gettext_area_12_catalog_covers_display_calls() {
    let en = crate::catalog::parse_catalog(include_str!("../locales/en.json")).unwrap();
    let tw = crate::catalog::parse_catalog(include_str!("../locales/zh-TW.json")).unwrap();
    let literal =
        regex::Regex::new(r#"(?:\.__|\.format|\.add_localized_log|\.push_localized_notification)\(\s*("(?:[^"\\]|\\.)*")"#)
            .unwrap();
    for source in [
        include_str!("../../../../vw-desktop/src/app/views/task_board/drag.rs"),
        include_str!("../../../../vw-desktop/src/app/views/task_board/control.rs"),
        include_str!("../../../../vw-desktop/src/app/views/task_board/board.rs"),
        include_str!("../../../../vw-desktop/src/app/views/task_board/mod.rs"),
        include_str!("../../../../vw-desktop/src/app/views/task_board/modals.rs"),
        include_str!("../../../../vw-desktop/src/app/views/task_board/common.rs"),
        include_str!("../../../desktop-task-board/src/panel/subtask_editor.rs"),
        include_str!("../../../../vw-desktop/src/app/views/task_board/panel/reasoning_selector.rs"),
        include_str!("../../../../vw-desktop/src/app/views/task_board/panel/mod.rs"),
        include_str!("../../../../vw-desktop/src/app/views/task_board/panel/model_selector.rs"),
        include_str!("../../../../vw-desktop/src/app/views/task_board/panel/styles.rs"),
        include_str!("../../../../vw-desktop/src/app/views/task_board/panel/logs_modal.rs"),
        include_str!("../../../../vw-desktop/src/app/views/task_board/panel/executor_selector.rs"),
        include_str!("../../../desktop-task-board/src/control/worktree_office.rs"),
        include_str!("../../../desktop-task-board/src/control/helpers.rs"),
        include_str!("../../../desktop-task-board/src/control/worktree_office/floor.rs"),
        include_str!("../../../desktop-task-board/src/control/worktree_office/room_scene.rs"),
        include_str!("../../../desktop-task-board/src/control/worktree_office/summary.rs"),
        include_str!("../../../../vw-desktop/src/app/message/task_board/message.rs"),
        include_str!("../../../../vw-desktop/src/app/message/task_board/update.rs"),
        include_str!("../../../../vw-desktop/src/app/message/task_board/helpers.rs"),
        include_str!("../../../../vw-desktop/src/app/message/task_board/update/execution.rs"),
        include_str!("../../../../vw-desktop/src/app/message/task_board/update/execution_results.rs"),
        include_str!("../../../../vw-desktop/src/app/message/task_board/helpers/review.rs"),
        include_str!("../../../../vw-desktop/src/app/message/task_board/helpers/bulk.rs"),
        include_str!("../../../../vw-desktop/src/app/message/task_board/helpers/execution.rs"),
        include_str!("../../../../vw-desktop/src/app/message/task_board/helpers/draft.rs"),
        include_str!("../../../../vw-desktop/src/app/message/task_board/helpers/settings.rs"),
        include_str!("../../../../vw-desktop/src/app/message/task_board/helpers/logs.rs"),
    ] {
        for capture in literal.captures_iter(source) {
            let key: String = serde_json::from_str(&capture[1]).unwrap();
            assert!(en.contains_key(&key), "missing English: {key}");
            assert!(tw.contains_key(&key), "missing Traditional Chinese: {key}");
        }
    }
}
