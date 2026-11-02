use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_12_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "统一管理调度、工作池与批量任务动作。"),
        (DesktopLocale::TraditionalChinese, "統一管理排程、工作池與批次任務動作。"),
        (DesktopLocale::English, "Manage scheduling, work pools, and bulk task actions."),
    ] {
        assert_eq!(I18n::new(locale).__("统一管理调度、工作池与批量任务动作。"), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_12_feedback_switches_language() {
    use crate::app::message::TaskBoardMessage;
    let (mut app, _) = crate::app::App::new();
    let _ = crate::app::message::task_board::update(
        &mut app,
        TaskBoardMessage::DeleteAllWorktreesCompleted(Ok(3)),
    );
    let message = app.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "已删除 3 个 worktree"),
        (DesktopLocale::TraditionalChinese, "已刪除 3 個 worktree"),
        (DesktopLocale::English, "Deleted 3 worktrees"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
    }
}

#[test]
fn desktop_gettext_area_12_status_log_keeps_semantics_and_raw_output() {
    use crate::app::task::{TaskLogEntry, TaskStatus};
    let entry = TaskLogEntry::new_status_change(TaskStatus::Pending, TaskStatus::Running);
    assert_eq!(
        super::tasks::log_message(I18n::new(DesktopLocale::English), &entry),
        "Status changed: Pending → Running"
    );
    assert_eq!(
        super::tasks::log_message(I18n::new(DesktopLocale::TraditionalChinese), &entry),
        "狀態變更: 待執行 → 執行中"
    );
    let raw = TaskLogEntry::new_message("用户输出 %s %n 待执行".into());
    assert_eq!(super::tasks::log_message(I18n::new(DesktopLocale::English), &raw), raw.message);
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_12_locale_selection_refreshes_cached_logs() {
    use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
    use crate::app::task::{Task, TaskLogEntry, TaskStatus};
    let (mut app, _) = crate::app::App::new();
    app.desktop_locale_saving = true;
    let mut task = Task::default();
    task.logs.push(TaskLogEntry::new_status_change(TaskStatus::Pending, TaskStatus::Running));
    task.logs.push(TaskLogEntry::new_message("raw 中文 %s %n".into()));
    app.task_board_viewing_logs = Some(task);
    for (locale, expected) in [
        (DesktopLocale::English, "Status changed: Pending → Running"),
        (DesktopLocale::TraditionalChinese, "狀態變更: 待執行 → 執行中"),
        (DesktopLocale::SimplifiedChinese, "状态变更: 待执行 → 执行中"),
    ] {
        let _ = crate::app::message::settings::update(
            &mut app,
            SettingsMessage::General(GeneralSettingsMessage::LocaleSelected(locale)),
        );
        let text = app.task_board_logs_editor.text();
        assert!(text.contains(expected), "{text}");
        assert!(text.contains("raw 中文 %s %n"));
    }
}

#[test]
fn desktop_gettext_area_12_cached_message_preserves_arguments() {
    use crate::app::task::Task;
    let mut task = Task::default();
    task.add_localized_log("审核结果: %s", vec!["用户/%s/%n".into()]);
    let cached = task.logs.clone();
    assert_eq!(
        super::tasks::log_message(I18n::new(DesktopLocale::English), &cached[0]),
        "Review result: 用户/%s/%n"
    );
    assert_eq!(
        super::tasks::log_message(I18n::new(DesktopLocale::TraditionalChinese), &cached[0]),
        "審查結果: 用户/%s/%n"
    );
}

#[test]
fn desktop_gettext_area_12_catalog_covers_display_calls() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    let literal =
        regex::Regex::new(r#"(?:\.__|\.format|\.add_localized_log|\.push_localized_notification)\(\s*("(?:[^"\\]|\\.)*")"#)
            .unwrap();
    for source in [
        include_str!("../views/task_board/drag.rs"),
        include_str!("../views/task_board/control.rs"),
        include_str!("../views/task_board/board.rs"),
        include_str!("../views/task_board/mod.rs"),
        include_str!("../views/task_board/modals.rs"),
        include_str!("../views/task_board/common.rs"),
        include_str!("../views/task_board/panel/subtask_editor.rs"),
        include_str!("../views/task_board/panel/reasoning_selector.rs"),
        include_str!("../views/task_board/panel/mod.rs"),
        include_str!("../views/task_board/panel/model_selector.rs"),
        include_str!("../views/task_board/panel/styles.rs"),
        include_str!("../views/task_board/panel/logs_modal.rs"),
        include_str!("../views/task_board/panel/executor_selector.rs"),
        include_str!("../views/task_board/control/worktree_office.rs"),
        include_str!("../views/task_board/control/helpers.rs"),
        include_str!("../views/task_board/control/worktree_office/floor.rs"),
        include_str!("../views/task_board/control/worktree_office/room_scene.rs"),
        include_str!("../views/task_board/control/worktree_office/summary.rs"),
        include_str!("../message/task_board/message.rs"),
        include_str!("../message/task_board/update.rs"),
        include_str!("../message/task_board/helpers.rs"),
        include_str!("../message/task_board/update/execution.rs"),
        include_str!("../message/task_board/update/execution_results.rs"),
        include_str!("../message/task_board/helpers/review.rs"),
        include_str!("../message/task_board/helpers/bulk.rs"),
        include_str!("../message/task_board/helpers/execution.rs"),
        include_str!("../message/task_board/helpers/draft.rs"),
        include_str!("../message/task_board/helpers/settings.rs"),
        include_str!("../message/task_board/helpers/logs.rs"),
    ] {
        for capture in literal.captures_iter(source) {
            let key: String = serde_json::from_str(&capture[1]).unwrap();
            assert!(en.contains_key(&key), "missing English: {key}");
            assert!(tw.contains_key(&key), "missing Traditional Chinese: {key}");
        }
    }
}
