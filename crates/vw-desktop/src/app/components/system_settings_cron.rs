//! 桌面壳：投影 Cron 视图输入并映射局部事件，保留跨领域组合。
use crate::app::message::SettingsMessage;
use crate::app::state::CronSettingsTab;
use crate::app::{App, Message};
use desktop_settings_cron::message::{
    Message as LocalMessage, SettingsMessage as Event,
};
use desktop_settings_cron::system_settings_cron as domain;
use iced::Element;
pub(crate) mod date_time;
fn help_content<'a>(
    title: &'a str,
    body: std::borrow::Cow<'a, str>,
    color: Option<iced::Color>,
    i18n: desktop_i18n::I18n,
) -> Element<'a, LocalMessage> {
    crate::app::components::chat_panel::message_view::selectable_titled_text(
        title, body, color, i18n,
    )
    .map(|event| match event {
        Message::None => LocalMessage::None,
        Message::CopyCode(value) => LocalMessage::CopyCode(value),
        _ => unreachable!("selectable help without a chat only emits copy or no-op events"),
    })
}
pub(crate) fn skill_choice(
    skill: &desktop_settings_skills::types::SkillsCatalogItem,
) -> Option<domain::SkillChoice> {
    if !skill.installed || !skill.enabled {
        return None;
    }
    let id = if skill.source == "plugin" {
        let (plugin, _): (String, String) =
            serde_json::from_str(skill.id.strip_prefix("plugin:")?).ok()?;
        format!("{plugin}:{}", skill.title)
    } else {
        skill.id.clone()
    };
    desktop_settings_cron::state::valid_skill_reference(&id).then(|| domain::SkillChoice {
        id,
        label: skill.title.clone(),
        description: skill.description.clone(),
    })
}

fn data(app: &App) -> domain::ViewData<'_> {
    use desktop_settings_agents::acp::{EMPLOYEE_ACP_BACKEND, EMPLOYEE_ACP_PROFILE};

    let locale = app.general_settings.desktop_locale;
    let mut agents: Vec<_> = app
        .agents_settings
        .entries
        .iter()
        .filter(|entry| entry.can_start_session())
        .map(|entry| {
            let backend = match &entry.runtime {
                config_agent_profile::AgentRuntime::Native => None,
                config_agent_profile::AgentRuntime::Acp { backend } => Some(backend.clone()),
            };
            domain::AgentChoice {
                id: entry.key.clone(),
                label: entry.label.clone(),
                model: entry.model.clone(),
                installed: backend.as_ref().and_then(|backend| app.acp_settings.installed.get(backend).copied()),
                requires_employee_target: backend.as_deref() == Some(EMPLOYEE_ACP_BACKEND),
                backend,
            }
        })
        .collect();
    if !agents.iter().any(|agent| agent.requires_employee_target) {
        agents.push(domain::AgentChoice {
            id: EMPLOYEE_ACP_PROFILE.into(),
            label: desktop_i18n::I18n::new(locale).__("识矩数字员工").into(),
            model: String::new(),
            backend: Some(EMPLOYEE_ACP_BACKEND.into()),
            installed: app.acp_settings.installed.get(EMPLOYEE_ACP_BACKEND).copied(),
            requires_employee_target: true,
        });
    }
    domain::ViewData {
        cron_settings: &app.cron_settings,
        system_timezone: app.general_settings.chat_timezone.parse().expect("validated system timezone"),
        desktop_locale: locale,
        app_theme: &app.appearance.app_theme,
        project_path: app.navigation_tabs.project_path.as_deref(),
        recent_projects: &app.workspace_state.recent_projects,
        recent_project_labels: &app.workspace_state.recent_projects_edits,
        agents,
        models: app
            .model_settings
            .providers
            .iter()
            .filter_map(|provider| {
                let mut models: Vec<_> =
                    provider.models.iter().filter(|model| model.enabled).collect();
                if models.is_empty() {
                    return None;
                }
                models.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
                Some(domain::ModelGroup {
                    id: provider.id.clone(),
                    label: provider.name.clone(),
                    models: models.into_iter().map(|model| domain::ModelChoice {
                        id: format!("{}/{}", provider.id, model.id),
                        label: model.name.clone(),
                    }).collect(),
                })
            })
            .collect(),
        skills: app.cron_settings.skill_catalog.clone(),
        wecom_notifications: app.channels_settings.wecom_instances.iter()
            .filter(|instance| config_channel_services::WeComConfig::is_cron_instance(&instance.id)
                && instance.config.has_required_credentials())
            .map(|instance| {
                let status = app.channels_settings.wecom_cron_bindings.get(&instance.id);
                domain::WeComNotification {
                    id: instance.id.clone(),
                    label: instance.config.bot_id.clone(),
                    bound: status.and_then(|status| status.bound),
                    connected: status.and_then(|status| status.connected),
                    pairing_code: status.and_then(|status| status.pairing_code.clone()),
                    loading: app.channels_settings.wecom_cron_binding_requests.contains_key(&instance.id),
                }
            })
            .collect(),
        scheduler_error: app.scheduler_settings.save_error.as_deref(),
        last_copied_code_hash: app.clipboard_state.last_copied_code_hash,
        help_content,
    }
}
fn map(event: LocalMessage) -> Message {
    match event {
        LocalMessage::None => Message::None,
        LocalMessage::CopyCode(value) => Message::CopyCode(value),
        LocalMessage::WeComBindingRequested { instance, start } => Message::Settings(
            SettingsMessage::Channels(crate::app::message::settings::ChannelsMessage::WeComCronBindingRequested {
                instance,
                start,
            }),
        ),
        LocalMessage::Settings(event) => Message::Settings(match event {
            Event::CronOpenEmployeeSchedule => {
                #[cfg(not(target_arch = "wasm32"))]
                return Message::DigitalEmployees(crate::app::digital_employees::Event::CronSchedules(
                    crate::app::digital_employees::cron_schedules::Event::New { from_cron_form: true },
                ));
                #[cfg(target_arch = "wasm32")]
                return Message::None;
            }
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
            Event::CronEditorTabSelected { editing, tab } => {
                SettingsMessage::CronEditorTabSelected { editing, tab }
            }
            Event::CronEditorPickerChanged { editing, action } => {
                SettingsMessage::CronEditorPickerChanged { editing, action }
            }
            Event::CronEditorSkillToggled { editing, id } => {
                SettingsMessage::CronEditorSkillToggled { editing, id }
            }
            Event::CronAddFromConversation => SettingsMessage::CronAddFromConversation,
            Event::CronTemplateSelected(v0) => SettingsMessage::CronTemplateSelected(v0),
            Event::CronHistory(v0) => SettingsMessage::CronHistory(v0),
            Event::CronList(event) => SettingsMessage::CronList(event),
            Event::CronListFilterChanged(v0) => SettingsMessage::CronListFilterChanged(v0),
            Event::CronBatchModeToggled(v0) => SettingsMessage::CronBatchModeToggled(v0),
            Event::CronJobsRefresh => SettingsMessage::CronJobsRefresh,
            Event::CronJobSelectionToggled(v0, v1) => {
                SettingsMessage::CronJobSelectionToggled(v0, v1)
            }
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
            Event::CronJobEditProjectPathChanged(v0) => {
                SettingsMessage::CronJobEditProjectPathChanged(v0)
            }
            Event::CronJobEditModelProviderChanged(v0) => {
                SettingsMessage::CronJobEditModelProviderChanged(v0)
            }
            Event::CronJobEditModelChanged(v0) => SettingsMessage::CronJobEditModelChanged(v0),
            Event::CronJobEditWakeToggled(v0) => SettingsMessage::CronJobEditWakeToggled(v0),
            Event::CronJobEditFullAccessToggled(v0) => {
                SettingsMessage::CronJobEditFullAccessToggled(v0)
            }
            Event::CronJobEditTaskPoolToggled(v0) => {
                SettingsMessage::CronJobEditTaskPoolToggled(v0)
            }
            Event::CronJobEditDeliveryEnabledToggled(v0) => {
                SettingsMessage::CronJobEditDeliveryEnabledToggled(v0)
            }
            Event::CronJobEditDeliveryWeComInstanceSelected(id) => {
                SettingsMessage::CronJobEditDeliveryWeComInstanceSelected(id)
            }
            Event::CronJobEditDeliveryWeComSelected(v0) => {
                SettingsMessage::CronJobEditDeliveryWeComSelected(v0)
            }
            Event::CronJobEditDeliveryWebhookChanged(v0) => {
                SettingsMessage::CronJobEditDeliveryWebhookChanged(v0)
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
            Event::CronAddProjectPathChanged(v0) => SettingsMessage::CronAddProjectPathChanged(v0),
            Event::CronAddModelProviderChanged(v0) => {
                SettingsMessage::CronAddModelProviderChanged(v0)
            }
            Event::CronAddModelChanged(v0) => SettingsMessage::CronAddModelChanged(v0),
            Event::CronAddWakeToggled(v0) => SettingsMessage::CronAddWakeToggled(v0),
            Event::CronAddFullAccessToggled(v0) => SettingsMessage::CronAddFullAccessToggled(v0),
            Event::CronAddTaskPoolToggled(v0) => SettingsMessage::CronAddTaskPoolToggled(v0),
            Event::CronAddDeliveryEnabledToggled(v0) => {
                SettingsMessage::CronAddDeliveryEnabledToggled(v0)
            }
            Event::CronAddDeliveryWeComInstanceSelected(id) => {
                SettingsMessage::CronAddDeliveryWeComInstanceSelected(id)
            }
            Event::CronAddDeliveryWeComSelected(v0) => {
                SettingsMessage::CronAddDeliveryWeComSelected(v0)
            }
            Event::CronAddDeliveryWebhookChanged(v0) => {
                SettingsMessage::CronAddDeliveryWebhookChanged(v0)
            }
            Event::CronAddDeliveryBestEffortToggled(v0) => {
                SettingsMessage::CronAddDeliveryBestEffortToggled(v0)
            }
            Event::CronAddDeleteAfterRunToggled(v0) => {
                SettingsMessage::CronAddDeleteAfterRunToggled(v0)
            }
            Event::CronAddSubmit => SettingsMessage::CronAddSubmit,
            Event::CronHelpOpen => SettingsMessage::CronHelpOpen,
            Event::CronHelpClose => SettingsMessage::CronHelpClose,
        }),
    }
}
pub(crate) fn visible(app: &App) -> bool {
    app.navigation_tabs.screen == crate::app::Screen::Cron
        && !app.navigation_panels.show_system_settings
        && (app.mobile_layout.mode() == crate::app::state::LayoutMode::Desktop
            || !(app.mobile_layout.drawer_open || app.mobile_layout.mine_open))
}

pub(crate) fn page(app: &App) -> Element<'_, Message> {
    let owns_scroll = matches!(
        app.cron_settings.active_tab,
        CronSettingsTab::Jobs | CronSettingsTab::History | CronSettingsTab::Employee | CronSettingsTab::Add
    );
    let content = desktop_settings_shell::content_page(
        iced::widget::column![view(app)],
        true,
        owns_scroll,
    );
    // 与识矩技能保持 28px 横向边距，设置页外壳已预留 7px。
    iced::widget::container(content)
        .padding([24, 28 - desktop_settings_shell::SETTINGS_SCROLL_GUTTER])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

fn view(app: &App) -> Element<'_, Message> {
    let host_content = match app.cron_settings.active_tab {
        CronSettingsTab::Scheduler => crate::app::components::system_settings_scheduler::embedded_view(app),
        #[cfg(not(target_arch = "wasm32"))]
        CronSettingsTab::Employee => crate::app::digital_employees::cron_schedules::view(app),
        _ => iced::widget::Space::new().into(),
    };
    domain::view(&data(app), host_content, map)
}
pub fn view_overlays<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let base = domain::view_overlays(&data(app), base, map);
    #[cfg(not(target_arch = "wasm32"))]
    let base = crate::app::digital_employees::cron_schedules::with_overlay(app, base);
    base
}
#[cfg(test)]
#[path = "system_settings_cron_ui_refresh_27_tests.rs"]
mod ui_refresh_27_tests;
