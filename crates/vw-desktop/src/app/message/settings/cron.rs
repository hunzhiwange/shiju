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

fn start_butler_chat(app: &mut App) -> Task<Message> {
    app.cron_settings.list_menu = None;
    if app.workspace_state.new_session_creating {
        return Task::none();
    }
    if let Some(path) = app
        .navigation_tabs
        .project_path
        .as_ref()
        .filter(|path| !path.trim().is_empty())
        .cloned()
    {
        return open_butler_chat(app, path, None);
    }

    let gateway_user_generation = app.session_state.gateway_user_generation;
    let navigation_generation = app.session_state.session_view_cache.navigation_generation;
    let client = crate::app::config::gateway_client();
    Task::perform(
        async move {
            let client = client?;
            let response = client
                .project_resolve_workspace(client.endpoint().tenant_scope())
                .await?;
            Ok((response.project.directory, response.project.id.0))
        },
        move |result| {
            Message::Settings(SettingsMessage::CronWorkspaceResolved {
                gateway_user_generation,
                navigation_generation,
                result,
            })
        },
    )
}

fn open_butler_chat(app: &mut App, path: String, project_id: Option<String>) -> Task<Message> {
    use crate::app::message::{ChatMessage, ViewMessage, chat, project, view};
    use crate::app::state::ToastKind;
    use config_agent::SHIJU_BUTLER_AGENT_KEY;

    if !app.agents_settings.entries.iter().any(|entry| {
        entry.key == SHIJU_BUTLER_AGENT_KEY && entry.can_start_session()
    }) {
        return app.show_localized_toast(ToastKind::Warning, "智能体未启用或不支持当前入口", vec![]);
    }
    app.stash_active_session_view();
    let (tab_id, _) = desktop_workspace::projects::project_tab(&path);
    let navigate = if app.navigation_tabs.open_tabs.iter().any(|tab| tab.id == tab_id) {
        view::update(app, ViewMessage::TabSelected(tab_id))
    } else {
        app.open_project(path.clone())
    };
    if let Some(project_id) = project_id {
        app.workspace_state.project_id = Some(project_id);
    }
    app.navigation_panels.focus_chat();
    let open = project::open_session_draft(app, path.clone(), path);
    let select = chat::session::update(
        app,
        ChatMessage::SessionAgentSelected(Some(SHIJU_BUTLER_AGENT_KEY.into())),
    );
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    let prompt = i18n.__("我想创建一个定时任务。请帮我确认任务内容、执行时间或频率，以及是否需要通知。");
    let mut draft = app.current_session_runtime_mut().input_editor.text();
    if !draft.is_empty() && !draft.ends_with('\n') {
        draft.push('\n');
    }
    draft.push_str(prompt);
    let fill = chat::input::fill_new_chat_draft(app, draft);
    Task::batch([navigate, open, select, fill])
}

pub fn update(app: &mut App, event: SettingsMessage) -> Task<Message> {
    use desktop_settings_cron::state::{CronEditorPicker, CronEditorPickerAction, CronEditorTab};

    #[cfg(not(target_arch = "wasm32"))]
    if let SettingsMessage::CronEditorPickerChanged {
        editing,
        action: CronEditorPickerAction::BrowseProject,
    } = &event
    {
        let editing = *editing;
        let state = &app.cron_settings;
        let draft = if editing && state.editing_job_id.is_some() {
            &state.edit_draft
        } else if !editing && state.editing_job_id.is_none() && state.show_add_modal {
            &state.add_draft
        } else {
            return Task::none();
        };
        if draft.submitting
            || draft.editor_tab != CronEditorTab::Basic
            || draft.editor_picker != Some(CronEditorPicker::Project)
        {
            return Task::none();
        }
        let endpoint = crate::app::config::gateway_client_endpoint();
        if !desktop_settings_workspace::environment::can_add(&endpoint) {
            return app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "远程网关不支持选择本机目录。",
                vec![],
            );
        }
        let Some(path) = desktop_workspace::basic::pick_folder() else {
            return Task::none();
        };
        let selected = SettingsMessage::CronEditorPickerChanged {
            editing,
            action: CronEditorPickerAction::ProjectSelected(path.clone()),
        };
        return Task::batch([
            update(app, selected),
            crate::app::workbench_projects::register_directory(app, path),
        ]);
    }

    if let SettingsMessage::CronWorkspaceResolved {
        gateway_user_generation,
        navigation_generation,
        result,
    } = event {
        if gateway_user_generation != app.session_state.gateway_user_generation
            || navigation_generation != app.session_state.session_view_cache.navigation_generation
            || !crate::app::components::system_settings_cron::visible(app)
            || app.workspace_state.new_session_creating
        {
            return Task::none();
        }
        return match result {
            Ok((path, project_id)) => open_butler_chat(app, path, Some(project_id)),
            Err(_) => app.show_localized_toast(
                crate::app::state::ToastKind::Error,
                "无法打开默认工作区项目",
                vec![],
            ),
        };
    }
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
        return start_butler_chat(app);
    }
    if let SettingsMessage::CronList(desktop_settings_cron::message::ListMessage::OpenTask(id)) = &event {
        app.session_state.session_view_cache.navigation_generation += 1;
        if app.cron_settings.jobs.iter().any(|job| {
            job.id == *id
                && !job.task_pool
                && desktop_settings_cron::system_settings_cron::job_is_agent(job)
        }) {
            app.cron_settings.list_menu = None;
            let navigation_generation = app.session_state.session_view_cache.navigation_generation;
            let gateway_user_generation = app.session_state.gateway_user_generation;
            let job_id = id.clone();
            let session_id = format!("cron-{id}");
            let client = crate::app::gateway_client();
            return Task::perform(
                async move {
                    let client = client.ok()?;
                    desktop_workspace::project_message::session_open::resolve_navigation(
                        &client, session_id, None, String::new(),
                    ).await
                },
                move |destination| Message::Project(
                    crate::app::message::ProjectMessage::CronSessionNavigationResolved {
                        job_id: job_id.clone(),
                        gateway_user_generation,
                        navigation_generation,
                        destination,
                    },
                ),
            );
        }
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
        SettingsMessage::CronEditorPickerChanged {
            editing,
            action: CronEditorPickerAction::ProjectSelected(path),
        } => {
            let state = &app.cron_settings;
            let draft = if *editing { &state.edit_draft } else { &state.add_draft };
            state.editing_job_id.is_some() == *editing
                && (*editing || state.show_add_modal)
                && !draft.submitting
                && draft.editor_tab == CronEditorTab::Basic
                && draft.editor_picker == Some(CronEditorPicker::Project)
                && path.trim() != draft.project_path.trim()
        }
        SettingsMessage::CronJobEditProjectPathChanged(path) => {
            path.trim() != app.cron_settings.edit_draft.project_path.trim()
        }
        SettingsMessage::CronAddProjectPathChanged(path) => {
            path.trim() != app.cron_settings.add_draft.project_path.trim()
        }
        _ => false,
    };
    let check_wecom_binding = matches!(
        &event,
        SettingsMessage::CronEditorTabSelected { tab: CronEditorTab::Delivery, .. }
            | SettingsMessage::CronJobEditDeliveryWeComInstanceSelected(_)
            | SettingsMessage::CronAddDeliveryWeComInstanceSelected(_)
            | SettingsMessage::CronJobEditDeliveryEnabledToggled(true)
            | SettingsMessage::CronAddDeliveryEnabledToggled(true)
            | SettingsMessage::CronJobEditDeliveryWeComSelected(true)
            | SettingsMessage::CronAddDeliveryWeComSelected(true)
    );
    let task = update_editor(app, event);
    let state = &mut app.cron_settings;
    if check_wecom_binding {
        let draft = if state.editing_job_id.is_some() { &state.edit_draft } else { &state.add_draft };
        if (state.editing_job_id.is_some() || state.show_add_modal)
            && draft.editor_tab == CronEditorTab::Delivery
            && draft.delivery_enabled
            && draft.delivery_via_wecom
            && !draft.submitting
            && let Some(instance) = draft.delivery_wecom_instance.clone()
        {
            return Task::batch([task, super::channels::update(
                app,
                SettingsMessage::Channels(super::messages::ChannelsMessage::WeComCronBindingRequested {
                    instance,
                    start: false,
                }),
            )]);
        }
    }
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
    let mut notices = Vec::new();
    let config = crate::app::config::agent_config_access();
    let task = domain::update(
        &mut domain::Inputs {
            cron_settings: &mut app.cron_settings,
            config: &config,
            desktop_locale: app.general_settings.desktop_locale,
            system_timezone: app.general_settings.chat_timezone.parse().expect("validated system timezone"),
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
    domain::refresh_run_history_locale(
        &mut app.cron_settings,
        app.general_settings.desktop_locale,
        app.general_settings.chat_timezone.parse().expect("validated system timezone"),
    );
}
