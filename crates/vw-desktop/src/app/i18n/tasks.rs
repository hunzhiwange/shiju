//! Task presentation reads semantic status changes without interpreting raw logs.
use super::I18n;
use crate::app::task::{TaskLogEntry, TaskStatus};

pub(crate) fn status_label(i18n: I18n, status: TaskStatus) -> String {
    let source = status.label_for_workflow_id("");
    if matches!(status, TaskStatus::Custom(_)) { source } else { i18n.__(&source).to_owned() }
}

pub(crate) fn log_message(i18n: I18n, entry: &TaskLogEntry) -> String {
    if let Some(message) = &entry.localized_message {
        return message.render(i18n).expect("valid task log template");
    }
    match (entry.status_from, entry.status_to) {
        (Some(from), Some(to)) => i18n
            .format(
                "状态变更: %s → %s",
                &[status_label(i18n, from).into(), status_label(i18n, to).into()],
            )
            .expect("valid task status template"),
        _ => entry.message.clone(),
    }
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
