use super::I18n;
use crate::app::task::{TaskLogEntry, TaskStatus};

pub(crate) fn status_label(i18n: I18n, status: TaskStatus) -> String {
    desktop_i18n::tasks::status_label(i18n, &status.label_for_workflow_id(""), matches!(status, TaskStatus::Custom(_)))
}

pub(crate) fn log_message(i18n: I18n, entry: &TaskLogEntry) -> String {
    let transition = match (entry.status_from, entry.status_to) {
        (Some(from), Some(to)) => Some((status_label(i18n, from), status_label(i18n, to))),
        _ => None,
    };
    desktop_i18n::tasks::log_message(i18n, entry.localized_message.as_ref(), transition, &entry.message)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SplitOption(
    pub crate::app::task::TaskSplitMode,
    pub vw_config_types::ui::DesktopLocale,
);
impl std::fmt::Display for SplitOption {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(I18n::new(self.1).__(self.0.label()))
    }
}
