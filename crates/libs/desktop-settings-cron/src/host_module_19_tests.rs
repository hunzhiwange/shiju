use crate::message::{Message as LocalMessage, SettingsMessage as Event};
use crate::state::*;
use crate::system_settings_cron as domain;
use crate::update::messages::SettingsMessage;
use gateway_cron::CronJobDto;
use iced::widget::{container, text, text_editor};
use iced::{Element, Length, Theme};
#[derive(Debug, Clone)]
enum ViewMessage {
    OpenUrlExternal(String),
}
#[derive(Debug, Clone)]
enum Message {
    None,
    CopyCode(String),
    View(ViewMessage),
    Settings(SettingsMessage),
}
struct Fixture {
    cron_settings: CronSettingsState,
    heartbeat_settings: crate::app_init::state::HeartbeatSettingsState,
    scheduler_settings: crate::app_init::state::SchedulerSettingsState,
    desktop_locale: config_ui::DesktopLocale,
    app_theme: Theme,
    project_path: Option<String>,
    recent_projects: Vec<String>,
    acp_agents: Vec<String>,
    providers: Vec<(String, Vec<String>)>,
    delivery: domain::DeliveryAvailability,
}
fn fixture() -> Fixture {
    Fixture {
        cron_settings: Default::default(),
        heartbeat_settings: Default::default(),
        scheduler_settings: Default::default(),
        desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
        app_theme: Theme::Dark,
        project_path: None,
        recent_projects: Vec::new(),
        acp_agents: vec!["main".into()],
        providers: Vec::new(),
        delivery: Default::default(),
    }
}
fn update(
    app: &mut Fixture,
    event: SettingsMessage,
) -> iced::Task<crate::update::messages::Message> {
    let config = desktop_config::ConfigAccess::new(
        gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 9),
        None,
    );
    let mut inputs = crate::update::cron::Inputs {
            cron_settings: &mut app.cron_settings,
            desktop_locale: app.desktop_locale,
            delivery: app.delivery,
            client: Err("fixture gateway unavailable".into()),
            config: &config,
            notices: &mut Vec::new(),
        };
    match event {
        SettingsMessage::CronHistory(event) => crate::update::history::update(&mut inputs, event),
        event => crate::update::cron::update(&mut inputs, event),
    }
}
fn jobs_tab(app: &Fixture) -> Element<'_, Message> {
    domain::list::jobs_tab(&data(app)).map(map)
}
fn job_row<'a>(app: &'a Fixture, job: &'a CronJobDto) -> Element<'a, Message> {
    domain::list::job_row(&data(app), job).map(map)
}
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(value: String) -> Self {
        Self::CopyCode(value)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        use desktop_ui_controls::text_editor_context_menu::TextContextEffect;
        match effect {
            TextContextEffect::Copy(value) => Self::CopyCode(value),
            TextContextEffect::OpenUrl(url) => Self::View(ViewMessage::OpenUrlExternal(url)),
            _ => Self::None,
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
fn help_cron<'a>(
    title: &'a str,
    body: std::borrow::Cow<'a, str>,
    color: Option<iced::Color>,
    i18n: desktop_i18n::I18n,
) -> Element<'a, LocalMessage> {
    desktop_chat_view::message_view::markdown_selection::selectable_titled_text::<Message>(
        title, body, color, i18n,
    )
    .map(|m| match m {
        Message::None => LocalMessage::None,
        Message::CopyCode(v) => LocalMessage::CopyCode(v),
        _ => unreachable!(),
    })
}
fn help_heartbeat<'a>(
    title: &'a str,
    body: std::borrow::Cow<'a, str>,
    color: Option<iced::Color>,
    i18n: desktop_i18n::I18n,
) -> Element<'a, crate::system_settings_heartbeat::Message> {
    desktop_chat_view::message_view::markdown_selection::selectable_titled_text::<Message>(
        title, body, color, i18n,
    )
    .map(|m| match m {
        Message::None => crate::system_settings_heartbeat::Message::None,
        Message::CopyCode(v) => crate::system_settings_heartbeat::Message::CopyCode(v),
        Message::View(ViewMessage::OpenUrlExternal(url)) => crate::system_settings_heartbeat::Message::OpenUrl(url),
        _ => unreachable!(),
    })
}
fn help_scheduler<'a>(
    title: &'a str,
    body: std::borrow::Cow<'a, str>,
    color: Option<iced::Color>,
    i18n: desktop_i18n::I18n,
) -> Element<'a, crate::system_settings_scheduler::Message> {
    desktop_chat_view::message_view::markdown_selection::selectable_titled_text::<Message>(
        title, body, color, i18n,
    )
    .map(|m| match m {
        Message::None => crate::system_settings_scheduler::Message::None,
        Message::CopyCode(v) => crate::system_settings_scheduler::Message::CopyCode(v),
        _ => unreachable!(),
    })
}
mod heartbeat {
    use super::*;
    use crate::system_settings_heartbeat as domain;
    use domain::{Message as Local, SettingsMessage as Event};
    fn data(app: &Fixture) -> domain::ViewData<'_> {
        domain::ViewData {
            heartbeat_settings: &app.heartbeat_settings,
            desktop_locale: app.desktop_locale,
            app_theme: &app.app_theme,
            last_copied_code_hash: None,
            help_content: help_heartbeat,
        }
    }
    fn map(event: Local) -> Message {
        match event {
            Local::None => Message::None,
            Local::CopyCode(v) => Message::CopyCode(v),
            Local::OpenUrl(url) => Message::View(ViewMessage::OpenUrlExternal(url)),
            Local::Settings(event) => Message::Settings(match event {
                Event::HeartbeatHelpOpen => SettingsMessage::HeartbeatHelpOpen,
                Event::HeartbeatHelpClose => SettingsMessage::HeartbeatHelpClose,
                Event::HeartbeatEnabledToggled(v) => SettingsMessage::HeartbeatEnabledToggled(v),
                Event::HeartbeatIntervalChanged(v) => SettingsMessage::HeartbeatIntervalChanged(v),
                Event::HeartbeatMessageChanged(v) => SettingsMessage::HeartbeatMessageChanged(v),
                Event::HeartbeatTargetChanged(v) => SettingsMessage::HeartbeatTargetChanged(v),
                Event::HeartbeatToChanged(v) => SettingsMessage::HeartbeatToChanged(v),
            }),
        }
    }
    pub fn view(app: &Fixture) -> Element<'_, Message> {
        domain::view(&data(app)).map(map)
    }
    pub fn view_overlays<'a>(app: &'a Fixture, base: Element<'a, Message>) -> Element<'a, Message> {
        domain::view_overlays(&data(app), base, map)
    }
}
mod scheduler {
    use super::*;
    use crate::system_settings_scheduler as domain;
    use domain::{Message as Local, SettingsMessage as Event};
    fn data(app: &Fixture) -> domain::ViewData<'_> {
        domain::ViewData {
            scheduler_settings: &app.scheduler_settings,
            desktop_locale: app.desktop_locale,
            app_theme: &app.app_theme,
            last_copied_code_hash: None,
            help_content: help_scheduler,
        }
    }
    fn map(event: Local) -> Message {
        match event {
            Local::None => Message::None,
            Local::CopyCode(v) => Message::CopyCode(v),
            Local::Settings(event) => Message::Settings(match event {
                Event::SchedulerHelpOpen => SettingsMessage::SchedulerHelpOpen,
                Event::SchedulerHelpClose => SettingsMessage::SchedulerHelpClose,
                Event::SchedulerEnabledToggled(v) => SettingsMessage::SchedulerEnabledToggled(v),
                Event::SchedulerMaxTasksChanged(v) => SettingsMessage::SchedulerMaxTasksChanged(v),
                Event::SchedulerMaxConcurrentChanged(v) => {
                    SettingsMessage::SchedulerMaxConcurrentChanged(v)
                }
            }),
        }
    }
    pub fn view(app: &Fixture) -> Element<'_, Message> {
        domain::view(&data(app)).map(map)
    }
    pub(crate) fn embedded_view(app: &Fixture) -> Element<'_, Message> {
        domain::embedded_view(&data(app)).map(map)
    }
}
fn data(app: &Fixture) -> domain::ViewData<'_> {
    domain::ViewData {
        cron_settings: &app.cron_settings,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
        project_path: app.project_path.as_deref(),
        recent_projects: &app.recent_projects,
        agents: app.acp_agents.iter().map(String::as_str).collect(),
        providers: app
            .providers
            .iter()
            .map(|(id, models)| domain::ProviderOptions {
                id,
                models: models.iter().map(String::as_str).collect(),
            })
            .collect(),
        delivery: app.delivery,
        scheduler_error: app.scheduler_settings.save_error.as_deref(),
        last_copied_code_hash: None,
        help_content: help_cron,
    }
}
fn map(event: LocalMessage) -> Message {
    match event {
        LocalMessage::None => Message::None,
        LocalMessage::CopyCode(value) => Message::CopyCode(value),
        LocalMessage::Settings(event) => Message::Settings(match event {
            Event::CronPickerOpen { editing } => SettingsMessage::CronPickerOpen { editing },
            Event::CronFullAccessConfirm { editing } => {
                SettingsMessage::CronFullAccessConfirm { editing }
            }
            Event::CronFullAccessCancel { editing } => {
                SettingsMessage::CronFullAccessCancel { editing }
            }
            Event::CronPickerCancel => SettingsMessage::CronPickerCancel,
            Event::CronPickerNavigate(v0) => SettingsMessage::CronPickerNavigate(v0),
            Event::CronPickerDate(v0) => SettingsMessage::CronPickerDate(v0),
            Event::CronPickerTime { hour, minute } => {
                SettingsMessage::CronPickerTime { hour, minute }
            }
            Event::CronPickerConfirm => SettingsMessage::CronPickerConfirm,
            Event::CronScheduleDraftChanged { editing, draft } => {
                SettingsMessage::CronScheduleDraftChanged { editing, draft }
            }
            Event::CronSchedulePreview { editing } => {
                SettingsMessage::CronSchedulePreview { editing }
            }
            Event::CronEnabledToggled(v0) => SettingsMessage::CronEnabledToggled(v0),
            Event::CronMaxRunHistoryChanged(v0) => SettingsMessage::CronMaxRunHistoryChanged(v0),
            Event::CronTabSelected(v0) => SettingsMessage::CronTabSelected(v0),
            Event::CronTemplatesToggled(v0) => SettingsMessage::CronTemplatesToggled(v0),
            Event::CronTemplateSelected(v0) => SettingsMessage::CronTemplateSelected(v0),
            Event::CronHistory(v0) => SettingsMessage::CronHistory(v0),
            Event::CronListFilterChanged(v0) => SettingsMessage::CronListFilterChanged(v0),
            Event::CronBatchModeToggled(v0) => SettingsMessage::CronBatchModeToggled(v0),
            Event::CronJobsRefresh => SettingsMessage::CronJobsRefresh,
            Event::CronJobSelectionToggled(v0, v1) => {
                SettingsMessage::CronJobSelectionToggled(v0, v1)
            }
            Event::CronJobDetailsToggled(v0, v1) => SettingsMessage::CronJobDetailsToggled(v0, v1),
            Event::CronJobsSelectAllToggled(v0) => SettingsMessage::CronJobsSelectAllToggled(v0),
            Event::CronJobRunsOpen(v0) => SettingsMessage::CronJobRunsOpen(v0),
            Event::CronJobRunsEditorAction(v0) => SettingsMessage::CronJobRunsEditorAction(v0),
            Event::CronJobRunsClose => SettingsMessage::CronJobRunsClose,
            Event::CronJobEditStarted(v0) => SettingsMessage::CronJobEditStarted(v0),
            Event::CronJobEditCanceled => SettingsMessage::CronJobEditCanceled,
            Event::CronJobEditNameChanged(v0) => SettingsMessage::CronJobEditNameChanged(v0),
            Event::CronJobEditJobTypeChanged(v0) => SettingsMessage::CronJobEditJobTypeChanged(v0),
            Event::CronJobEditCommandEditorAction(v0) => {
                SettingsMessage::CronJobEditCommandEditorAction(v0)
            }
            Event::CronJobEditPromptEditorAction(v0) => {
                SettingsMessage::CronJobEditPromptEditorAction(v0)
            }
            Event::CronJobEditAgentChanged(v0) => SettingsMessage::CronJobEditAgentChanged(v0),
            Event::CronJobEditAcpAgentChanged(v0) => {
                SettingsMessage::CronJobEditAcpAgentChanged(v0)
            }
            Event::CronJobEditProjectPathChanged(v0) => {
                SettingsMessage::CronJobEditProjectPathChanged(v0)
            }
            Event::CronJobEditModelProviderChanged(v0) => {
                SettingsMessage::CronJobEditModelProviderChanged(v0)
            }
            Event::CronJobEditModelChanged(v0) => SettingsMessage::CronJobEditModelChanged(v0),
            Event::CronJobEditWakeToggled(v0) => SettingsMessage::CronJobEditWakeToggled(v0),
            Event::CronJobEditFallbacksChanged(v0) => {
                SettingsMessage::CronJobEditFallbacksChanged(v0)
            }
            Event::CronJobEditFullAccessToggled(v0) => {
                SettingsMessage::CronJobEditFullAccessToggled(v0)
            }
            Event::CronJobEditTaskPoolToggled(v0) => {
                SettingsMessage::CronJobEditTaskPoolToggled(v0)
            }
            Event::CronJobEditDeliveryEnabledToggled(v0) => {
                SettingsMessage::CronJobEditDeliveryEnabledToggled(v0)
            }
            Event::CronJobEditDeliveryChannelChanged(v0) => {
                SettingsMessage::CronJobEditDeliveryChannelChanged(v0)
            }
            Event::CronJobEditDeliveryToChanged(v0) => {
                SettingsMessage::CronJobEditDeliveryToChanged(v0)
            }
            Event::CronJobEditDeliveryBestEffortToggled(v0) => {
                SettingsMessage::CronJobEditDeliveryBestEffortToggled(v0)
            }
            Event::CronJobEditDeleteAfterRunToggled(v0) => {
                SettingsMessage::CronJobEditDeleteAfterRunToggled(v0)
            }
            Event::CronJobEditSave => SettingsMessage::CronJobEditSave,
            Event::CronJobEnabledChanged(v0, v1) => SettingsMessage::CronJobEnabledChanged(v0, v1),
            Event::CronJobDelete(v0) => SettingsMessage::CronJobDelete(v0),
            Event::CronSelectedJobsEnable => SettingsMessage::CronSelectedJobsEnable,
            Event::CronSelectedJobsDisable => SettingsMessage::CronSelectedJobsDisable,
            Event::CronSelectedJobsDelete => SettingsMessage::CronSelectedJobsDelete,
            Event::CronAddNameChanged(v0) => SettingsMessage::CronAddNameChanged(v0),
            Event::CronAddJobTypeChanged(v0) => SettingsMessage::CronAddJobTypeChanged(v0),
            Event::CronAddCommandEditorAction(v0) => {
                SettingsMessage::CronAddCommandEditorAction(v0)
            }
            Event::CronAddPromptEditorAction(v0) => SettingsMessage::CronAddPromptEditorAction(v0),
            Event::CronAddAgentChanged(v0) => SettingsMessage::CronAddAgentChanged(v0),
            Event::CronAddAcpAgentChanged(v0) => SettingsMessage::CronAddAcpAgentChanged(v0),
            Event::CronAddProjectPathChanged(v0) => SettingsMessage::CronAddProjectPathChanged(v0),
            Event::CronAddModelProviderChanged(v0) => {
                SettingsMessage::CronAddModelProviderChanged(v0)
            }
            Event::CronAddModelChanged(v0) => SettingsMessage::CronAddModelChanged(v0),
            Event::CronAddWakeToggled(v0) => SettingsMessage::CronAddWakeToggled(v0),
            Event::CronAddFallbacksChanged(v0) => SettingsMessage::CronAddFallbacksChanged(v0),
            Event::CronAddFullAccessToggled(v0) => SettingsMessage::CronAddFullAccessToggled(v0),
            Event::CronAddTaskPoolToggled(v0) => SettingsMessage::CronAddTaskPoolToggled(v0),
            Event::CronAddDeliveryEnabledToggled(v0) => {
                SettingsMessage::CronAddDeliveryEnabledToggled(v0)
            }
            Event::CronAddDeliveryChannelChanged(v0) => {
                SettingsMessage::CronAddDeliveryChannelChanged(v0)
            }
            Event::CronAddDeliveryToChanged(v0) => SettingsMessage::CronAddDeliveryToChanged(v0),
            Event::CronAddDeliveryBestEffortToggled(v0) => {
                SettingsMessage::CronAddDeliveryBestEffortToggled(v0)
            }
            Event::CronAddDeleteAfterRunToggled(v0) => {
                SettingsMessage::CronAddDeleteAfterRunToggled(v0)
            }
            Event::CronAddSubmit => SettingsMessage::CronAddSubmit,
            Event::CronHelpOpen => SettingsMessage::CronHelpOpen,
            Event::CronHelpClose => SettingsMessage::CronHelpClose,
            Event::OpenDelivery(_) => SettingsMessage::Ignore,
        }),
    }
}
pub fn view(app: &Fixture) -> Element<'_, Message> {
    let scheduler = if app.cron_settings.active_tab == CronSettingsTab::Scheduler {
        scheduler::embedded_view(app)
    } else {
        iced::widget::Space::new().into()
    };
    domain::view(&data(app), scheduler, map)
}
pub fn view_overlays<'a>(app: &'a Fixture, base: Element<'a, Message>) -> Element<'a, Message> {
    domain::view_overlays(&data(app), base, map)
}
#[cfg(test)]
fn edit_form(app: &Fixture) -> Element<'_, Message> {
    domain::edit_form(&data(app)).map(map)
}
#[cfg(test)]
fn add_tab(app: &Fixture) -> Element<'_, Message> {
    domain::add_tab(&data(app)).map(map)
}
#[cfg(test)]
fn config_tab(app: &Fixture) -> Element<'_, Message> {
    domain::config_tab(&data(app)).map(map)
}

mod rendering;
mod views;

mod controller;

mod templates {
    use super::*;
    pub fn view() -> Element<'static, Message> {
        domain::templates::view().map(map)
    }
}
