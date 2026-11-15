//! 桌面壳：投影 Cron 视图输入并映射局部事件，保留跨领域组合。
use crate::app::{App, Message};
use crate::app::message::SettingsMessage;
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::state::CronSettingsTab;
use desktop_settings_cron::system_settings_cron as domain;
use desktop_settings_cron::message::{Message as LocalMessage, SettingsMessage as Event, DeliveryMode};
use iced::Element;
#[cfg(test)] use iced::{Theme, Length};
#[cfg(test)] use iced::widget::{text, container};
#[cfg(test)] use gateway_cron::CronJobDto;
#[cfg(test)]
use crate::app::state::{CronAddJobType, CronAddScheduleKind};
pub(crate) mod date_time;
#[cfg(test)] mod delivery;
#[cfg(test)] mod history;
#[cfg(test)] mod list;
#[cfg(test)] pub(crate) mod templates;
#[cfg(test)] use list::{job_row, jobs_tab};
#[cfg(test)] use domain::job_is_agent;

const fn tab(index: usize) -> SettingsTabDescriptor<CronSettingsTab> {
    let item = domain::CRON_TABS[index];
    SettingsTabDescriptor::new(item.0, item.1, item.2)
}
pub(super) const CRON_TABS: &[SettingsTabDescriptor<CronSettingsTab>] = &[tab(0), tab(1), tab(2), tab(3), tab(4)];

pub(crate) fn availability(app: &App) -> domain::DeliveryAvailability {
    let c = &app.channels_settings;
    domain::DeliveryAvailability {
        bot: c.wecom.as_ref().is_some_and(|c| c.has_required_credentials()),
        webhook: c.wecom_webhook.is_some(), telegram: c.telegram.is_some(), discord: c.discord.is_some(),
        slack: c.slack.is_some(), mattermost: c.mattermost.is_some(), qq: c.qq.is_some(),
        #[cfg(not(target_arch = "wasm32"))] email: c.email.is_some(),
    }
}
fn help_content<'a>(title: &'a str, body: std::borrow::Cow<'a,str>, color: Option<iced::Color>, i18n: desktop_i18n::I18n) -> Element<'a,LocalMessage> {
    crate::app::components::chat_panel::message_view::selectable_titled_text(title, body, color, i18n).map(|event| match event {
        Message::None => LocalMessage::None,
        Message::CopyCode(value) => LocalMessage::CopyCode(value),
        _ => unreachable!("selectable help without a chat only emits copy or no-op events"),
    })
}
fn data(app: &App) -> domain::ViewData<'_> {
    domain::ViewData {
        cron_settings: &app.cron_settings, desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme, project_path: app.project_path.as_deref(), recent_projects: &app.recent_projects,
        agents: app.agents_settings.entries.iter().filter(|e| e.enabled).map(|e| e.key.as_str()).collect(),
        providers: app.agents_settings.provider_models.iter().map(|p| domain::ProviderOptions { id: &p.id, models: p.models.iter().map(|m| m.id.as_str()).collect() }).collect(),
        delivery: availability(app), scheduler_error: app.scheduler_settings.save_error.as_deref(),
        last_copied_code_hash: app.last_copied_code_hash, help_content,
    }
}
fn map(event: LocalMessage) -> Message {
    match event {
        LocalMessage::None => Message::None,
        LocalMessage::CopyCode(value) => Message::CopyCode(value),
        LocalMessage::Settings(event) => Message::Settings(match event {
            Event::CronPickerOpen { editing } => SettingsMessage::CronPickerOpen { editing },
            Event::CronFullAccessConfirm { editing } => SettingsMessage::CronFullAccessConfirm { editing },
            Event::CronFullAccessCancel { editing } => SettingsMessage::CronFullAccessCancel { editing },
            Event::CronPickerCancel => SettingsMessage::CronPickerCancel,
            Event::CronPickerNavigate(v0) => SettingsMessage::CronPickerNavigate(v0),
            Event::CronPickerDate(v0) => SettingsMessage::CronPickerDate(v0),
            Event::CronPickerTime { hour, minute } => SettingsMessage::CronPickerTime { hour, minute },
            Event::CronPickerConfirm => SettingsMessage::CronPickerConfirm,
            Event::CronScheduleDraftChanged { editing, draft } => SettingsMessage::CronScheduleDraftChanged { editing, draft },
            Event::CronSchedulePreview { editing } => SettingsMessage::CronSchedulePreview { editing },
            Event::CronEnabledToggled(v0) => SettingsMessage::CronEnabledToggled(v0),
            Event::CronMaxRunHistoryChanged(v0) => SettingsMessage::CronMaxRunHistoryChanged(v0),
            Event::CronTabSelected(v0) => SettingsMessage::CronTabSelected(v0),
            Event::CronTemplatesToggled(v0) => SettingsMessage::CronTemplatesToggled(v0),
            Event::CronTemplateSelected(v0) => SettingsMessage::CronTemplateSelected(v0),
            Event::CronHistory(v0) => SettingsMessage::CronHistory(v0),
            Event::CronListFilterChanged(v0) => SettingsMessage::CronListFilterChanged(v0),
            Event::CronBatchModeToggled(v0) => SettingsMessage::CronBatchModeToggled(v0),
            Event::CronJobsRefresh => SettingsMessage::CronJobsRefresh,
            Event::CronJobSelectionToggled(v0, v1) => SettingsMessage::CronJobSelectionToggled(v0, v1),
            Event::CronJobDetailsToggled(v0, v1) => SettingsMessage::CronJobDetailsToggled(v0, v1),
            Event::CronJobsSelectAllToggled(v0) => SettingsMessage::CronJobsSelectAllToggled(v0),
            Event::CronJobRunsOpen(v0) => SettingsMessage::CronJobRunsOpen(v0),
            Event::CronJobRunsEditorAction(v0) => SettingsMessage::CronJobRunsEditorAction(v0),
            Event::CronJobRunsClose => SettingsMessage::CronJobRunsClose,
            Event::CronJobEditStarted(v0) => SettingsMessage::CronJobEditStarted(v0),
            Event::CronJobEditCanceled => SettingsMessage::CronJobEditCanceled,
            Event::CronJobEditNameChanged(v0) => SettingsMessage::CronJobEditNameChanged(v0),
            Event::CronJobEditJobTypeChanged(v0) => SettingsMessage::CronJobEditJobTypeChanged(v0),
            Event::CronJobEditCommandEditorAction(v0) => SettingsMessage::CronJobEditCommandEditorAction(v0),
            Event::CronJobEditPromptEditorAction(v0) => SettingsMessage::CronJobEditPromptEditorAction(v0),
            Event::CronJobEditAgentChanged(v0) => SettingsMessage::CronJobEditAgentChanged(v0),
            Event::CronJobEditAcpAgentChanged(v0) => SettingsMessage::CronJobEditAcpAgentChanged(v0),
            Event::CronJobEditProjectPathChanged(v0) => SettingsMessage::CronJobEditProjectPathChanged(v0),
            Event::CronJobEditModelProviderChanged(v0) => SettingsMessage::CronJobEditModelProviderChanged(v0),
            Event::CronJobEditModelChanged(v0) => SettingsMessage::CronJobEditModelChanged(v0),
            Event::CronJobEditWakeToggled(v0) => SettingsMessage::CronJobEditWakeToggled(v0),
            Event::CronJobEditFallbacksChanged(v0) => SettingsMessage::CronJobEditFallbacksChanged(v0),
            Event::CronJobEditFullAccessToggled(v0) => SettingsMessage::CronJobEditFullAccessToggled(v0),
            Event::CronJobEditTaskPoolToggled(v0) => SettingsMessage::CronJobEditTaskPoolToggled(v0),
            Event::CronJobEditDeliveryEnabledToggled(v0) => SettingsMessage::CronJobEditDeliveryEnabledToggled(v0),
            Event::CronJobEditDeliveryChannelChanged(v0) => SettingsMessage::CronJobEditDeliveryChannelChanged(v0),
            Event::CronJobEditDeliveryToChanged(v0) => SettingsMessage::CronJobEditDeliveryToChanged(v0),
            Event::CronJobEditDeliveryBestEffortToggled(v0) => SettingsMessage::CronJobEditDeliveryBestEffortToggled(v0),
            Event::CronJobEditDeleteAfterRunToggled(v0) => SettingsMessage::CronJobEditDeleteAfterRunToggled(v0),
            Event::CronJobEditSave => SettingsMessage::CronJobEditSave,
            Event::CronJobEnabledChanged(v0, v1) => SettingsMessage::CronJobEnabledChanged(v0, v1),
            Event::CronJobDelete(v0) => SettingsMessage::CronJobDelete(v0),
            Event::CronSelectedJobsEnable => SettingsMessage::CronSelectedJobsEnable,
            Event::CronSelectedJobsDisable => SettingsMessage::CronSelectedJobsDisable,
            Event::CronSelectedJobsDelete => SettingsMessage::CronSelectedJobsDelete,
            Event::CronAddNameChanged(v0) => SettingsMessage::CronAddNameChanged(v0),
            Event::CronAddJobTypeChanged(v0) => SettingsMessage::CronAddJobTypeChanged(v0),
            Event::CronAddCommandEditorAction(v0) => SettingsMessage::CronAddCommandEditorAction(v0),
            Event::CronAddPromptEditorAction(v0) => SettingsMessage::CronAddPromptEditorAction(v0),
            Event::CronAddAgentChanged(v0) => SettingsMessage::CronAddAgentChanged(v0),
            Event::CronAddAcpAgentChanged(v0) => SettingsMessage::CronAddAcpAgentChanged(v0),
            Event::CronAddProjectPathChanged(v0) => SettingsMessage::CronAddProjectPathChanged(v0),
            Event::CronAddModelProviderChanged(v0) => SettingsMessage::CronAddModelProviderChanged(v0),
            Event::CronAddModelChanged(v0) => SettingsMessage::CronAddModelChanged(v0),
            Event::CronAddWakeToggled(v0) => SettingsMessage::CronAddWakeToggled(v0),
            Event::CronAddFallbacksChanged(v0) => SettingsMessage::CronAddFallbacksChanged(v0),
            Event::CronAddFullAccessToggled(v0) => SettingsMessage::CronAddFullAccessToggled(v0),
            Event::CronAddTaskPoolToggled(v0) => SettingsMessage::CronAddTaskPoolToggled(v0),
            Event::CronAddDeliveryEnabledToggled(v0) => SettingsMessage::CronAddDeliveryEnabledToggled(v0),
            Event::CronAddDeliveryChannelChanged(v0) => SettingsMessage::CronAddDeliveryChannelChanged(v0),
            Event::CronAddDeliveryToChanged(v0) => SettingsMessage::CronAddDeliveryToChanged(v0),
            Event::CronAddDeliveryBestEffortToggled(v0) => SettingsMessage::CronAddDeliveryBestEffortToggled(v0),
            Event::CronAddDeleteAfterRunToggled(v0) => SettingsMessage::CronAddDeleteAfterRunToggled(v0),
            Event::CronAddSubmit => SettingsMessage::CronAddSubmit,
            Event::CronHelpOpen => SettingsMessage::CronHelpOpen,
            Event::CronHelpClose => SettingsMessage::CronHelpClose,
            Event::OpenDelivery(mode) => SettingsMessage::Channels(crate::app::message::settings::ChannelsMessage::WeComDeliveryOpened(match mode {
                DeliveryMode::Qr => crate::app::state::WeComDeliveryMode::Qr,
                DeliveryMode::Webhook => crate::app::state::WeComDeliveryMode::Webhook,
            })),
        }),
    }
}
pub fn view(app: &App) -> Element<'_, Message> {
    let scheduler = if app.cron_settings.active_tab == CronSettingsTab::Scheduler {
        crate::app::components::system_settings_scheduler::embedded_view(app)
    } else { iced::widget::Space::new().into() };
    domain::view(&data(app), scheduler, map)
}
pub fn view_overlays<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    domain::view_overlays(&data(app), base, map)
}
#[cfg(test)]
fn edit_form(app: &App) -> Element<'_,Message> { domain::edit_form(&data(app)).map(map) }
#[cfg(test)]
fn add_tab(app: &App) -> Element<'_,Message> { domain::add_tab(&data(app)).map(map) }
#[cfg(test)]
fn config_tab(app: &App) -> Element<'_,Message> { domain::config_tab(&data(app)).map(map) }
#[cfg(test)]
#[path = "system_settings_cron_tests.rs"] mod system_settings_cron_tests;
#[cfg(test)]
#[path = "system_settings_cron_ui_refresh_27_tests.rs"] mod ui_refresh_27_tests;

#[cfg(test)] fn with_edit_modal<'a>(app: &'a App, base: Element<'a,Message>) -> Element<'a,Message> { domain::with_edit_modal(&data(app), base, map) }
