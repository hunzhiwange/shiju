//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use desktop_session::session_gateway::{
    gateway_plugin_delete_async, gateway_plugin_set_enabled_async, gateway_plugins_async,
    gateway_skill_create_async, gateway_skill_delete_async, gateway_skill_detail_async,
    gateway_skill_install_builtin_async, gateway_skill_set_enabled_async, gateway_skills_async,
};
use crate::app::{App, Message};
use iced::Task;


use super::messages::SettingsMessage;

mod local_import;

use desktop_settings_skills::update::catalog::{self as domain, Event, Effect};
fn refresh_task(project_path: Option<String>) -> Task<Message> {
    Task::perform(async move { gateway_skills_async(crate::app::config::gateway_client_endpoint(), project_path.as_deref()).await }, |result| {
        Message::Settings(SettingsMessage::SkillsLoaded(result))
    })
}

fn plugins_refresh_task() -> Task<Message> {
    Task::perform(async move { gateway_plugins_async(crate::app::config::gateway_client_endpoint()).await }, |result| {
        Message::Settings(SettingsMessage::PluginsLoaded(result))
    })
}

fn detail_task(project_path: Option<String>, skill_id: String) -> Task<Message> {
    let response_skill_id = skill_id.clone();
    Task::perform(
        async move { gateway_skill_detail_async(crate::app::config::gateway_client_endpoint(), project_path.as_deref(), &skill_id).await },
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
            gateway_skill_set_enabled_async(crate::app::config::gateway_client_endpoint(), project_path.as_deref(), &skill_id, enabled).await
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
        async move { gateway_skill_delete_async(crate::app::config::gateway_client_endpoint(), project_path.as_deref(), &skill_id).await },
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
        async move { gateway_plugin_set_enabled_async(crate::app::config::gateway_client_endpoint(), &plugin_id, enabled).await },
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
    Task::perform(async move { gateway_plugin_delete_async(crate::app::config::gateway_client_endpoint(), &plugin_id).await }, move |result| {
        Message::Settings(SettingsMessage::PluginsDeleteCompleted {
            plugin_id: response_plugin_id.clone(),
            result,
        })
    })
}

pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::SkillsSync(event) => {
            return crate::app::components::system_settings_skills::sync::update(app, event);
        }
        SettingsMessage::SkillsImportZip | SettingsMessage::SkillsImportFolder => {
            return local_import::pick(app, matches!(message, SettingsMessage::SkillsImportZip));
        }
        SettingsMessage::SkillsLocalImportCompleted { project, result } => {
            return local_import::complete(app, project, result);
        }
        _ => {}
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
        SettingsMessage::SkillsProjectAutoLoadToggled(value) => Event::SkillsProjectAutoLoadToggled(value),
        SettingsMessage::SkillsPromptInjectionModeChanged(value) => Event::SkillsPromptInjectionModeChanged(value),
        SettingsMessage::SkillsSave => Event::SkillsSave,
        SettingsMessage::SkillsHelpOpen => Event::SkillsHelpOpen,
        SettingsMessage::SkillsHelpClose => Event::SkillsHelpClose,
        _ => return Task::none(),
    };
    if matches!(event, Event::SkillsTabChanged(_) | Event::SkillsProjectSelected(_)) {
        app.skills_settings.sync.menu = None;
        app.skills_settings.sync.form = None;
    }
    if matches!(event, Event::SkillsQueryChanged(_) | Event::SkillsDirectoryScopeChanged(_)
        | Event::SkillsDetailRequested(_))
    {
        app.skills_settings.sync.menu = None;
    }
    let effect = domain::update(&mut app.skills_settings, &app.navigation_tabs.project_path, event);
    execute(app, effect)
}
fn execute(app: &mut App, effect: Effect) -> Task<Message> {
    match effect {
        Effect::None => Task::none(),
        Effect::Batch(effects) => Task::batch(
            effects.into_iter().map(|effect| execute(app, effect)).collect::<Vec<_>>(),
        ),
        Effect::RefreshProject(path) => crate::app::projects::project_settings_refresh_task(path),
        Effect::OpenProject(path) => {
            use crate::app::components::system_settings_skills as local_skills;

            let keep_local_skills_open = local_skills::is_visible(app);
            if keep_local_skills_open {
                local_skills::invalidate(app);
                app.skills_settings.loading = true;
            }
            let open = app.open_project_and_index(path);
            if keep_local_skills_open {
                crate::app::message::skill_hub::open_expert_hub_tab(app);
            }
            open
        }
        Effect::Copy(value) => Task::done(Message::CopyCode(value)),
        Effect::Refresh(path) => refresh_task(path),
        Effect::PluginsRefresh => plugins_refresh_task(),
        Effect::Detail(path, id) => detail_task(path, id),
        Effect::SetEnabled(path, id, enabled) => set_enabled_task(path, id, enabled),
        Effect::Delete(path, id) => delete_task(path, id),
        Effect::PluginSetEnabled(id, enabled) => plugin_set_enabled_task(id, enabled),
        Effect::PluginDelete(id) => plugin_delete_task(id),
        Effect::Create(path) => Task::perform(
            async move { gateway_skill_create_async(crate::app::config::gateway_client_endpoint(), &path).await },
            |result| Message::Settings(SettingsMessage::SkillsCreateNewCompleted(result)),
        ),
        Effect::InstallBuiltin(path, id) => Task::perform(
            async move { gateway_skill_install_builtin_async(crate::app::config::gateway_client_endpoint(), &path, &id).await },
            |result| Message::Settings(SettingsMessage::SkillsInstallBuiltInCompleted(result)),
        ),
        Effect::Save(auto_load_project_skills, prompt_injection_mode) => {
            crate::app::update_skills_config_async(move |skills| {
                skills.auto_load_project_skills = auto_load_project_skills;
                skills.prompt_injection_mode = prompt_injection_mode;
            })
        }
    }
}

#[cfg(test)]
#[path = "skills_tests.rs"]
mod skills_tests;
