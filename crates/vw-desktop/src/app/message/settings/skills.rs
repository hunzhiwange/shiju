//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::session_gateway::{
    gateway_plugin_delete_async, gateway_plugin_set_enabled_async, gateway_plugins_async,
    gateway_skill_create_async, gateway_skill_delete_async, gateway_skill_detail_async,
    gateway_skill_install_builtin_async, gateway_skill_set_enabled_async, gateway_skills_async,
};
use crate::app::{App, Message};
use iced::Task;


use super::messages::SettingsMessage;

pub(crate) mod updates_view;
pub(crate) mod install_view;
pub(crate) mod jobs_view;
pub(crate) mod market_view;
pub(crate) mod repo_modal;
pub(crate) mod deeplink;
pub(crate) mod import_modal;
pub(crate) mod claude_view;
pub(crate) mod bindings_view;
pub(crate) mod storage_modal;
pub(crate) mod backup_modal;

use desktop_settings_skills::update::catalog::{self as domain, Event, Effect};
pub(super) use domain::map_catalog_items;
#[cfg(test)]
use domain::INSTALL_COMMAND_TEMPLATE;
fn refresh_task(project_path: Option<String>) -> Task<Message> {
    Task::perform(async move { gateway_skills_async(project_path.as_deref()).await }, |result| {
        Message::Settings(SettingsMessage::SkillsLoaded(result))
    })
}

fn plugins_refresh_task() -> Task<Message> {
    Task::perform(async move { gateway_plugins_async().await }, |result| {
        Message::Settings(SettingsMessage::PluginsLoaded(result))
    })
}

fn detail_task(project_path: Option<String>, skill_id: String) -> Task<Message> {
    let response_skill_id = skill_id.clone();
    Task::perform(
        async move { gateway_skill_detail_async(project_path.as_deref(), &skill_id).await },
        move |result| {
            Message::Settings(SettingsMessage::SkillsDetailLoaded {
                skill_id: response_skill_id.clone(),
                result,
            })
        },
    )
}

fn set_enabled_task(
    project_path: Option<String>,
    skill_id: String,
    enabled: bool,
) -> Task<Message> {
    let response_skill_id = skill_id.clone();
    Task::perform(
        async move {
            gateway_skill_set_enabled_async(project_path.as_deref(), &skill_id, enabled).await
        },
        move |result| {
            Message::Settings(SettingsMessage::SkillsSetEnabledCompleted {
                skill_id: response_skill_id.clone(),
                enabled,
                result,
            })
        },
    )
}

fn delete_task(project_path: Option<String>, skill_id: String) -> Task<Message> {
    let response_skill_id = skill_id.clone();
    Task::perform(
        async move { gateway_skill_delete_async(project_path.as_deref(), &skill_id).await },
        move |result| {
            Message::Settings(SettingsMessage::SkillsDeleteCompleted {
                skill_id: response_skill_id.clone(),
                result,
            })
        },
    )
}

fn plugin_set_enabled_task(plugin_id: String, enabled: bool) -> Task<Message> {
    let response_plugin_id = plugin_id.clone();
    Task::perform(
        async move { gateway_plugin_set_enabled_async(&plugin_id, enabled).await },
        move |result| {
            Message::Settings(SettingsMessage::PluginsSetEnabledCompleted {
                plugin_id: response_plugin_id.clone(),
                enabled,
                result,
            })
        },
    )
}

fn plugin_delete_task(plugin_id: String) -> Task<Message> {
    let response_plugin_id = plugin_id.clone();
    Task::perform(async move { gateway_plugin_delete_async(&plugin_id).await }, move |result| {
        Message::Settings(SettingsMessage::PluginsDeleteCompleted {
            plugin_id: response_plugin_id.clone(),
            result,
        })
    })
}

pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    if let SettingsMessage::SkillsBackups(event) = message {
        return backup_modal::update(app, event);
    }
    if let SettingsMessage::SkillsStorage(event) = message {
        return storage_modal::update(app, event);
    }
    if let SettingsMessage::SkillsBindings(event) = message {
        return bindings_view::update(app, event);
    }
    if let SettingsMessage::SkillsUpdates(event) = message {
        return updates_view::update(app, event);
    }
    if let SettingsMessage::SkillsClaudeConfig(event) = message {
        return claude_view::update(app, event);
    }
    if let SettingsMessage::SkillsJobs(event) = message {
        return jobs_view::update(app, event);
    }
    if let SettingsMessage::SkillsInstall(event) = message {
        return install_view::update(app, event);
    }
    if let SettingsMessage::SkillsMarket(event) = message {
        return market_view::update(app, event);
    }
    if let SettingsMessage::SkillRepositories(event) = message {
        return repo_modal::update(app, event);
    }
    if let SettingsMessage::SkillsImport(event) = message {
        return import_modal::update(app, event);
    }
    let event = match message {
        SettingsMessage::SkillsProjectSelected(value) => Event::SkillsProjectSelected(value),
        SettingsMessage::SkillsRefresh => Event::SkillsRefresh,
        SettingsMessage::PluginsRefresh => Event::PluginsRefresh,
        SettingsMessage::PluginsLoaded(value) => Event::PluginsLoaded(value),
        SettingsMessage::PluginsSelected(value) => Event::PluginsSelected(value),
        SettingsMessage::PluginsSetEnabledRequested { plugin_id, enabled } => Event::PluginsSetEnabledRequested { plugin_id, enabled },
        SettingsMessage::PluginsSetEnabledCompleted { plugin_id, enabled, result } => Event::PluginsSetEnabledCompleted { plugin_id, enabled, result },
        SettingsMessage::PluginsDeleteRequested(value) => Event::PluginsDeleteRequested(value),
        SettingsMessage::PluginsDeleteCompleted { plugin_id, result } => Event::PluginsDeleteCompleted { plugin_id, result },
        SettingsMessage::SkillsTabChanged(value) => Event::SkillsTabChanged(value),
        SettingsMessage::SkillsDetailClosed => Event::SkillsDetailClosed,
        SettingsMessage::SkillsDetailRequested(value) => Event::SkillsDetailRequested(value),
        SettingsMessage::SkillsLoaded(value) => Event::SkillsLoaded(value),
        SettingsMessage::SkillsDetailLoaded { skill_id, result } => Event::SkillsDetailLoaded { skill_id, result },
        SettingsMessage::SkillsQueryChanged(value) => Event::SkillsQueryChanged(value),
        SettingsMessage::SkillsDirectoryScopeChanged(value) => Event::SkillsDirectoryScopeChanged(value),
        SettingsMessage::SkillsCreateNewRequested => Event::SkillsCreateNewRequested,
        SettingsMessage::SkillsCreateNewCompleted(value) => Event::SkillsCreateNewCompleted(value),
        SettingsMessage::SkillsCopyInstallCommand => Event::SkillsCopyInstallCommand,
        SettingsMessage::SkillsInstallBuiltInRequested(value) => Event::SkillsInstallBuiltInRequested(value),
        SettingsMessage::SkillsInstallBuiltInCompleted(value) => Event::SkillsInstallBuiltInCompleted(value),
        SettingsMessage::SkillsSetEnabledRequested { skill_id, enabled } => Event::SkillsSetEnabledRequested { skill_id, enabled },
        SettingsMessage::SkillsSetEnabledCompleted { skill_id, enabled, result } => Event::SkillsSetEnabledCompleted { skill_id, enabled, result },
        SettingsMessage::SkillsDeleteRequested(value) => Event::SkillsDeleteRequested(value),
        SettingsMessage::SkillsDeleteCompleted { skill_id, result } => Event::SkillsDeleteCompleted { skill_id, result },
        SettingsMessage::SkillsDirectoryProviderChanged(value) => Event::SkillsDirectoryProviderChanged(value),
        SettingsMessage::SkillsProjectAutoLoadToggled(value) => Event::SkillsProjectAutoLoadToggled(value),
        SettingsMessage::SkillsPromptInjectionModeChanged(value) => Event::SkillsPromptInjectionModeChanged(value),
        SettingsMessage::SkillsSave => Event::SkillsSave,
        SettingsMessage::SkillsHelpOpen => Event::SkillsHelpOpen,
        SettingsMessage::SkillsHelpClose => Event::SkillsHelpClose,
        _ => return Task::none(),
    };
    let effect = domain::update(&mut app.skills_settings, &app.project_path, event);
    execute(app, effect)
}
fn execute(app: &mut App, effect: Effect) -> Task<Message> {
    match effect {
        Effect::None => Task::none(),
        Effect::Batch(effects) => Task::batch(
            effects.into_iter().map(|effect| execute(app, effect)).collect::<Vec<_>>(),
        ),
        Effect::RefreshProject(path) => crate::app::projects::project_settings_refresh_task(path),
        Effect::OpenProject(path) => app.open_project_and_index(path),
        Effect::Copy(value) => Task::done(Message::CopyCode(value)),
        Effect::Refresh(path) => refresh_task(path),
        Effect::PluginsRefresh => plugins_refresh_task(),
        Effect::Detail(path, id) => detail_task(path, id),
        Effect::SetEnabled(path, id, enabled) => set_enabled_task(path, id, enabled),
        Effect::Delete(path, id) => delete_task(path, id),
        Effect::PluginSetEnabled(id, enabled) => plugin_set_enabled_task(id, enabled),
        Effect::PluginDelete(id) => plugin_delete_task(id),
        Effect::Create(path) => Task::perform(
            async move { gateway_skill_create_async(&path).await },
            |result| Message::Settings(SettingsMessage::SkillsCreateNewCompleted(result)),
        ),
        Effect::InstallBuiltin(path, id) => Task::perform(
            async move { gateway_skill_install_builtin_async(&path, &id).await },
            |result| Message::Settings(SettingsMessage::SkillsInstallBuiltInCompleted(result)),
        ),
        Effect::Save(directory_provider, auto_load_project_skills, prompt_injection_mode) => {
            crate::app::update_skills_config_async(move |skills| {
                skills.directory_provider = directory_provider;
                skills.auto_load_project_skills = auto_load_project_skills;
                skills.prompt_injection_mode = prompt_injection_mode;
            })
        }
    }
}

#[cfg(test)]
#[path = "skills_tests.rs"]
mod skills_tests;

fn update_input(app: &App) -> desktop_settings_skills::update::Input {
    let client = crate::app::config::gateway_client();
    let context = client.as_ref().ok().map(|client| crate::app::state::AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    });
    desktop_settings_skills::update::Input {
        connection_error: client.as_ref().err().map(ToString::to_string).unwrap_or_default(),
        authorized: context.is_some() && context.as_ref() == app.apps_catalog.context.as_ref()
            && !app.apps_catalog.token.is_empty(),
        loopback: client.as_ref().is_ok_and(|client| client.endpoint().is_loopback()),
        context,
        project: app.project_id.clone(),
        directory: app.project_path.clone(),
        host: app.apps_catalog.host.clone(),
        catalog: app.apps_catalog.generation,
        targets: app.apps_catalog.rows.iter().filter_map(|row| row.observation.as_ref().ok())
            .flat_map(|row| row.installations.iter())
            .filter(|installation| installation.target_host_id == app.apps_catalog.host)
            .map(|installation| vw_api_app_skills::bindings_api::SkillBindingTarget {
                installation_id: installation.id.0.clone(), app_id: installation.app_id,
            }).collect(),
        repository_version: app.skills_settings.repositories.snapshot.as_ref()
            .filter(|snapshot| Some(snapshot.scope.project_id.as_ref()) == app.project_id.as_deref()
                && snapshot.scope.target_host_id == app.apps_catalog.host)
            .map_or(0, |snapshot| snapshot.version),
        job_active: app.skills_settings.jobs.task_id.is_some()
            && app.skills_settings.jobs.snapshot.as_ref().is_none_or(|job| !jobs_view::terminal(job.phase)),
    }
}
