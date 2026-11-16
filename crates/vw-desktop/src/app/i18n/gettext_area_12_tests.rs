use super::I18n;
use config_ui::DesktopLocale;

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_12_feedback_switches_language() {
    use crate::app::message::TaskBoardMessage;
    let (mut app, _) = crate::app::App::new();
    let _ = crate::app::message::task_board::update(
        &mut app,
        TaskBoardMessage::DeleteAllWorktreesCompleted(Ok(3)),
    );
    let message =
        app.toast_state.active_toast.as_ref().unwrap().localized_message.as_ref().unwrap();
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "已删除 3 个 worktree"),
        (DesktopLocale::TraditionalChinese, "已刪除 3 個 worktree"),
        (DesktopLocale::English, "Deleted 3 worktrees"),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), expected);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_12_locale_selection_refreshes_cached_logs() {
    use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
    use crate::app::task::{Task, TaskLogEntry, TaskStatus};
    let (mut app, _) = crate::app::App::new();
    app.general_settings.desktop_locale_saving = true;
    let mut task = Task::default();
    task.logs.push(TaskLogEntry::new_status_change(TaskStatus::Pending, TaskStatus::Running));
    task.logs.push(TaskLogEntry::new_message("raw 中文 %s %n".into()));
    app.task_board.task_board_viewing_logs = Some(task);
    for (locale, expected) in [
        (DesktopLocale::English, "Status changed: Pending → Running"),
        (DesktopLocale::TraditionalChinese, "狀態變更: 待執行 → 執行中"),
        (DesktopLocale::SimplifiedChinese, "状态变更: 待执行 → 执行中"),
    ] {
        let _ = crate::app::message::settings::update(
            &mut app,
            SettingsMessage::General(GeneralSettingsMessage::LocaleSelected(locale)),
        );
        let text = app.task_board.task_board_logs_editor.text();
        assert!(text.contains(expected), "{text}");
        assert!(text.contains("raw 中文 %s %n"));
    }
}
