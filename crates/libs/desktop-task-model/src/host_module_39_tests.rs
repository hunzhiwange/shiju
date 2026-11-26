use config_ui::DesktopLocale;
use desktop_i18n::I18n;


#[test]
fn desktop_gettext_area_12_status_log_keeps_semantics_and_raw_output() {
    use crate::models::{TaskLogEntry, TaskStatus};
    let entry = TaskLogEntry::new_status_change(TaskStatus::Pending, TaskStatus::Running);
    assert_eq!(
        crate::i18n::log_message(I18n::new(DesktopLocale::English), &entry),
        "Status changed: Pending → Running"
    );
    assert_eq!(
        crate::i18n::log_message(I18n::new(DesktopLocale::TraditionalChinese), &entry),
        "狀態變更: 待執行 → 執行中"
    );
    let raw = TaskLogEntry::new_message("用户输出 %s %n 待执行".into());
    assert_eq!(crate::i18n::log_message(I18n::new(DesktopLocale::English), &raw), raw.message);
}

#[test]
fn desktop_gettext_area_12_cached_message_preserves_arguments() {
    use crate::models::Task;
    let mut task = Task::default();
    task.add_localized_log("审核结果: %s", vec!["用户/%s/%n".into()]);
    let cached = task.logs.clone();
    assert_eq!(
        crate::i18n::log_message(I18n::new(DesktopLocale::English), &cached[0]),
        "Review result: 用户/%s/%n"
    );
    assert_eq!(
        crate::i18n::log_message(I18n::new(DesktopLocale::TraditionalChinese), &cached[0]),
        "審查結果: 用户/%s/%n"
    );
}
