//! 桌面壳：注入配置入口并映射领域结果。
use crate::app::{App, Message};
use super::messages::SettingsMessage;
use desktop_settings_cron::update::cron as domain;
use iced::Task;
#[path = "cron_mapping.rs"] pub(super) mod mapping;

pub(crate) fn open_page(app: &mut App) -> Task<Message> {
    use crate::app::{AppTab, Screen};

    const TAB_ID: &str = "cron";
    if !app.navigation_tabs.open_tabs.iter().any(|tab| tab.id == TAB_ID) {
        let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
        app.navigation_tabs.open_tabs.push(AppTab {
            id: TAB_ID.into(),
            title: i18n.__("定时任务").into(),
            screen: Screen::Cron,
            project_path: None,
        });
    }
    let was_visible = crate::app::components::system_settings_cron::visible(app);
    app.navigation_tabs.active_tab_id = Some(TAB_ID.into());
    app.navigation_tabs.screen = Screen::Cron;
    app.navigation_panels.show_system_settings = false;
    app.mobile_layout.drawer_open = false;
    app.mobile_layout.mine_open = false;
    if was_visible {
        return Task::none();
    }
    app.agents_settings.loading = true;
    #[cfg(not(target_arch = "wasm32"))]
    let employee_schedules = if app.cron_settings.active_tab == crate::app::state::CronSettingsTab::Employee {
        crate::app::digital_employees::cron_schedules::refresh(app)
    } else {
        Task::none()
    };
    #[cfg(target_arch = "wasm32")]
    let employee_schedules = Task::none();
    Task::batch([
        update(app, SettingsMessage::CronJobsRefresh),
        employee_schedules,
        Task::done(Message::Settings(SettingsMessage::Agents(
            super::messages::AgentsMessage::Refresh,
        ))),
        Task::done(Message::Settings(SettingsMessage::ModelsRefresh)),
    ])
}

pub fn update(app: &mut App, event: SettingsMessage) -> Task<Message> {
    if let SettingsMessage::CronSkillCatalogLoaded { revision, result } = event {
        let state = &mut app.cron_settings;
        if state.skill_catalog_revision == revision && state.skill_catalog_request.take().is_some()
        {
            state.skill_catalog_failed = result.is_err();
            state.skill_catalog = result.unwrap_or_default();
        }
        return Task::none();
    }
    if matches!(event, SettingsMessage::CronAddFromConversation) {
        let Some(prompt) = crate::app::components::system_settings_cron::conversation_prompt(app)
            .map(str::to_owned)
        else {
            return Task::none();
        };
        let session = app.current_session_runtime();
        let project = app
            .session_state
            .active_session_id
            .as_deref()
            .and_then(|id| {
                crate::app::message::project::helpers::find_session_project_path(app, id)
            })
            .unwrap_or_default();
        if app.cron_settings.add_draft.submitting || app.cron_settings.edit_draft.submitting {
            return Task::none();
        }
        let task = update_editor(
            app,
            SettingsMessage::CronTabSelected(crate::app::state::CronSettingsTab::Add),
        );
        let draft = &mut app.cron_settings.add_draft;
        draft.load_prompt(&prompt);
        draft.project_path = project;
        if !session.auto_model {
            draft.model = session.model;
        }
        if let Some(agent) = session.agent {
            draft.agent = agent;
        }
        for id in session.tool_selector.selected_manual_skills() {
            let choice = app
                .skills_settings
                .catalog
                .iter()
                .find(|entry| entry.id == id)
                .and_then(crate::app::components::system_settings_cron::skill_choice);
            if let Some(choice) = choice
                && !draft.skills.contains(&choice.id)
            {
                draft.skills.push(choice.id);
            }
        }
        return Task::batch([task, refresh_skill_catalog(app, false)]);
    }
    use crate::app::state::CronSettingsTab;
    #[cfg(not(target_arch = "wasm32"))]
    if matches!(&event, SettingsMessage::CronTabSelected(CronSettingsTab::Employee)) {
        let task = update_editor(app, event);
        return Task::batch([task, crate::app::digital_employees::cron_schedules::refresh(app)]);
    }
    let opened = matches!(
        &event,
        SettingsMessage::CronTabSelected(CronSettingsTab::Add)
            | SettingsMessage::CronJobEditStarted(_)
            | SettingsMessage::CronTemplateSelected(_)
    );
    let workspace_changed = match &event {
        SettingsMessage::CronJobEditProjectPathChanged(path) => {
            path.trim() != app.cron_settings.edit_draft.project_path.trim()
        }
        SettingsMessage::CronAddProjectPathChanged(path) => {
            path.trim() != app.cron_settings.add_draft.project_path.trim()
        }
        _ => false,
    };
    let task = update_editor(app, event);
    let state = &mut app.cron_settings;
    if state.editing_job_id.is_none()
        && !state.show_add_modal
        && let Some(pending) = state.skill_catalog_request.take()
    {
        pending.abort();
    }
    if opened || workspace_changed {
        Task::batch([task, refresh_skill_catalog(app, workspace_changed)])
    } else {
        task
    }
}

fn refresh_skill_catalog(app: &mut App, debounce: bool) -> Task<Message> {
    let state = &mut app.cron_settings;
    let draft = if state.editing_job_id.is_some() {
        &state.edit_draft
    } else if state.show_add_modal {
        &state.add_draft
    } else {
        return Task::none();
    };
    if draft.submitting {
        return Task::none();
    }
    let project = draft.project_path.trim().to_owned();
    if let Some(pending) = state.skill_catalog_request.take() {
        pending.abort();
    }
    state.skill_catalog.clear();
    state.skill_catalog_failed = false;
    state.skill_catalog_revision += 1;
    let revision = state.skill_catalog_revision;
    let (task, handle) = Task::perform(
        async move {
            if debounce {
                crate::app::message::sleep(std::time::Duration::from_millis(250)).await;
            }
            let items = desktop_session::session_gateway::gateway_skills_async(
                crate::app::config::gateway_client_endpoint(),
                (!project.is_empty()).then_some(project.as_str()),
            )
            .await?;
            Ok(desktop_settings_skills::update::catalog::map_catalog_items(items)
                .iter()
                .filter_map(crate::app::components::system_settings_cron::skill_choice)
                .collect())
        },
        move |result| {
            Message::Settings(SettingsMessage::CronSkillCatalogLoaded { revision, result })
        },
    )
    .abortable();
    state.skill_catalog_request = Some(handle);
    task
}

fn update_editor(app: &mut App, event: SettingsMessage) -> Task<Message> {
    let delivery = crate::app::components::system_settings_cron::availability(app);
    let mut notices = Vec::new();
    let config = crate::app::config::agent_config_access();
    let task = domain::update(
        &mut domain::Inputs {
            cron_settings: &mut app.cron_settings,
            config: &config,
            desktop_locale: app.general_settings.desktop_locale,
            delivery,
            client: crate::app::config::gateway_client(),
            notices: &mut notices,
        },
        mapping::event(event),
    )
    .map(mapping::message);
    let mut tasks = vec![task];
    for notice in notices {
        tasks.push(match notice {
            domain::Notice::Error(text) => app.show_error_toast(text),
            domain::Notice::Success(text) => app.show_success_toast(text),
        });
    }
    Task::batch(tasks)
}
#[cfg(test)]
#[path = "cron_tests.rs"] mod cron_tests;
pub(crate) fn refresh_run_history_locale(app: &mut App) {
 domain::refresh_run_history_locale(&mut app.cron_settings, app.general_settings.desktop_locale);
}
#[cfg(test)]
pub(crate) fn validate_delivery(app: &App, draft: &crate::app::state::CronJobDraft) -> Result<(), String> {
 desktop_settings_cron::delivery::validate(draft, crate::app::components::system_settings_cron::availability(app))
}
