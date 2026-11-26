use crate::models::{TaskLogEntry, TaskStatus};
use desktop_i18n::I18n;

pub fn status_label(i18n: I18n, status: TaskStatus) -> String {
    desktop_i18n::tasks::status_label(
        i18n,
        &status.label_for_workflow_id(""),
        matches!(status, TaskStatus::Custom(_)),
    )
}

pub fn log_message(i18n: I18n, entry: &TaskLogEntry) -> String {
    let transition = match (entry.status_from, entry.status_to) {
        (Some(from), Some(to)) => Some((status_label(i18n, from), status_label(i18n, to))),
        _ => None,
    };
    desktop_i18n::tasks::log_message(
        i18n,
        entry.localized_message.as_ref(),
        transition,
        &entry.message,
    )
}
