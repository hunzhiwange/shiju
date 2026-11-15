use crate::state::*;
use iced::widget::text_editor;
use gateway_cron::{CronHistoryResponse, CronJobDto};
#[derive(Debug, Clone)]
pub enum HistoryMessage {
    Refresh,
    SearchToggled,
    DetailsOpened(String),
    DetailsClosed,
    CopyOutput(String),
    Tick(u64),
    Loaded { generation: u64, result: Result<(CronHistoryResponse, Vec<CronJobDto>), String> },
    SearchChanged(String),
    StatusChanged(CronHistoryStatus),
    LocationChanged(CronHistoryLocation),
    TaskSelected(Option<String>),
    TaskSearchChanged(String),
    TaskPickerToggled(bool),
}


#[derive(Debug, Clone)]
pub enum SettingsMessage {
    CronPickerOpen {
        editing: bool,
    },
    CronFullAccessConfirm {
        editing: bool,
    },
    CronFullAccessCancel {
        editing: bool,
    },
    CronPickerCancel,
    CronPickerNavigate(i32),
    CronPickerDate(chrono::NaiveDate),
    CronPickerTime {
        hour: u32,
        minute: u32,
    },
    CronPickerConfirm,
    CronScheduleDraftChanged {
        editing: bool,
        draft: crate::state::CronScheduleDraft,
    },
    CronSchedulePreview {
        editing: bool,
    },
    CronEnabledToggled(bool),
    CronMaxRunHistoryChanged(u32),
    CronTabSelected(CronSettingsTab),
    CronTemplatesToggled(bool),
    CronTemplateSelected(crate::state::CronTemplateId),
    CronHistory(HistoryMessage),
    CronListFilterChanged(CronListFilter),
    CronBatchModeToggled(bool),
    CronJobsRefresh,
    CronJobSelectionToggled(String, bool),
    CronJobDetailsToggled(String, bool),
    CronJobsSelectAllToggled(bool),
    CronJobRunsOpen(String),
    CronJobRunsEditorAction(text_editor::Action),
    CronJobRunsClose,
    CronJobEditStarted(String),
    CronJobEditCanceled,
    CronJobEditNameChanged(String),
    CronJobEditJobTypeChanged(CronAddJobType),
    CronJobEditCommandEditorAction(text_editor::Action),
    CronJobEditPromptEditorAction(text_editor::Action),
    CronJobEditAgentChanged(String),
    CronJobEditAcpAgentChanged(String),
    CronJobEditProjectPathChanged(String),
    CronJobEditModelProviderChanged(String),
    CronJobEditModelChanged(String),
    CronJobEditWakeToggled(bool),
    CronJobEditFallbacksChanged(String),
    CronJobEditFullAccessToggled(bool),
    CronJobEditTaskPoolToggled(bool),
    CronJobEditDeliveryEnabledToggled(bool),
    CronJobEditDeliveryChannelChanged(String),
    CronJobEditDeliveryToChanged(String),
    CronJobEditDeliveryBestEffortToggled(bool),
    CronJobEditDeleteAfterRunToggled(bool),
    CronJobEditSave,
    CronJobEnabledChanged(String, bool),
    CronJobDelete(String),
    CronSelectedJobsEnable,
    CronSelectedJobsDisable,
    CronSelectedJobsDelete,
    CronAddNameChanged(String),
    CronAddJobTypeChanged(CronAddJobType),
    CronAddCommandEditorAction(text_editor::Action),
    CronAddPromptEditorAction(text_editor::Action),
    CronAddAgentChanged(String),
    CronAddAcpAgentChanged(String),
    CronAddProjectPathChanged(String),
    CronAddModelProviderChanged(String),
    CronAddModelChanged(String),
    CronAddWakeToggled(bool),
    CronAddFallbacksChanged(String),
    CronAddFullAccessToggled(bool),
    CronAddTaskPoolToggled(bool),
    CronAddDeliveryEnabledToggled(bool),
    CronAddDeliveryChannelChanged(String),
    CronAddDeliveryToChanged(String),
    CronAddDeliveryBestEffortToggled(bool),
    CronAddDeleteAfterRunToggled(bool),
    CronAddSubmit,
    CronHelpOpen,
    CronHelpClose,
    OpenDelivery(DeliveryMode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryMode {
    Qr,
    Webhook,
}
#[derive(Debug, Clone)]
pub enum Message {
    None,
    Settings(SettingsMessage),
    CopyCode(String),
}
