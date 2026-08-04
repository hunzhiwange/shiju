//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::session_gateway::{
    gateway_plugin_delete_async, gateway_plugin_set_enabled_async, gateway_plugins_async,
    gateway_skill_create_async, gateway_skill_delete_async, gateway_skill_detail_async,
    gateway_skill_install_builtin_async, gateway_skill_set_enabled_async, gateway_skills_async,
};
use crate::app::state::{
    PluginCatalogItem, SkillsCatalogItem, SkillsCatalogKind, SkillsSelectedDetail,
};
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::{
    DesktopPluginCatalogEntryDto, DesktopSkillCatalogEntryDto, DesktopSkillDetailDto,
};

use super::messages::SettingsMessage;

const INSTALL_COMMAND_TEMPLATE: &str = "vibewindow skills install <source>";

fn clear_selected_detail(app: &mut App) {
    app.skills_settings.selected_skill_id = None;
    app.skills_settings.selected_skill_detail = None;
    app.skills_settings.detail_loading = false;
    app.skills_settings.detail_error = None;
}

fn clear_selected_plugin(app: &mut App) {
    app.skills_settings.selected_plugin_id = None;
}

fn set_status(app: &mut App, message: impl Into<String>, is_error: bool) {
    app.skills_settings.status_message = Some(message.into());
    app.skills_settings.status_is_error = is_error;
}

fn set_plugin_status(app: &mut App, message: impl Into<String>, is_error: bool) {
    app.skills_settings.plugin_status_message = Some(message.into());
    app.skills_settings.plugin_status_is_error = is_error;
}

fn clear_catalog_load_error(app: &mut App) {
    let is_load_error = app.skills_settings.status_is_error
        && app
            .skills_settings
            .status_message
            .as_deref()
            .is_some_and(|message| message.starts_with("读取技能目录失败:"));
    if is_load_error {
        app.skills_settings.status_message = None;
        app.skills_settings.status_is_error = false;
    }
}

fn clear_plugin_catalog_load_error(app: &mut App) {
    let is_load_error = app.skills_settings.plugin_status_is_error
        && app
            .skills_settings
            .plugin_status_message
            .as_deref()
            .is_some_and(|message| message.starts_with("读取插件目录失败:"));
    if is_load_error {
        app.skills_settings.plugin_status_message = None;
        app.skills_settings.plugin_status_is_error = false;
    }
}

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

fn map_skill_kind(kind: &str) -> SkillsCatalogKind {
    match kind {
        "recommended" => SkillsCatalogKind::Recommended,
        "personal" => SkillsCatalogKind::Personal,
        _ => SkillsCatalogKind::System,
    }
}

fn map_catalog_items(items: Vec<DesktopSkillCatalogEntryDto>) -> Vec<SkillsCatalogItem> {
    items
        .into_iter()
        .map(|item| SkillsCatalogItem {
            id: item.id,
            title: item.title,
            description: item.description,
            kind: map_skill_kind(&item.kind),
            resource_count: item.resource_count,
            installed: item.installed,
            enabled: item.enabled,
            source: item.source,
            source_path: item.source_path,
        })
        .collect()
}

fn map_plugin_catalog_items(items: Vec<DesktopPluginCatalogEntryDto>) -> Vec<PluginCatalogItem> {
    items
        .into_iter()
        .map(|item| PluginCatalogItem {
            id: item.id,
            display_name: item.display_name,
            version: item.version,
            description: item.description,
            enabled: item.enabled,
            component_count: item.component_count,
            source_path: item.source_path,
        })
        .collect()
}

fn map_skill_detail(detail: DesktopSkillDetailDto) -> SkillsSelectedDetail {
    SkillsSelectedDetail {
        id: detail.id,
        title: detail.title,
        description: detail.description,
        kind: map_skill_kind(&detail.kind),
        installed: detail.installed,
        enabled: detail.enabled,
        source: detail.source,
        source_path: detail.source_path,
        document_name: detail.document_name,
        document_content: detail.document_content,
        can_install: detail.can_install,
        can_toggle: detail.can_toggle,
        can_delete: detail.can_delete,
    }
}

fn persist_skills_settings(app: &mut App) -> Task<Message> {
    let s = &app.skills_settings;
    let directory_provider = s.directory_provider;
    let auto_load_project_skills = s.auto_load_project_skills;
    let open_skills_enabled = s.open_skills_enabled;
    let prompt_injection_mode = s.prompt_injection_mode;
    let open_skills_dir = s.open_skills_dir_input.trim().to_string();

    crate::app::update_skills_config_async(move |skills| {
        skills.directory_provider = directory_provider;
        skills.auto_load_project_skills = auto_load_project_skills;
        skills.open_skills_enabled = open_skills_enabled;
        skills.open_skills_dir =
            if open_skills_dir.is_empty() { None } else { Some(open_skills_dir) };
        skills.prompt_injection_mode = prompt_injection_mode;
    })
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
        SettingsMessage::SkillsProjectSelected(project_path) => {
            if app.project_path.as_deref() == Some(project_path.as_str()) {
                return crate::app::projects::project_settings_refresh_task(project_path);
            }
            app.open_project_and_index(project_path)
        }
        SettingsMessage::SkillsRefresh => {
            app.skills_settings.loading = true;
            refresh_task(app.project_path.clone())
        }
        SettingsMessage::PluginsRefresh => {
            app.skills_settings.plugin_loading = true;
            plugins_refresh_task()
        }
        SettingsMessage::PluginsLoaded(result) => {
            app.skills_settings.plugin_loading = false;
            match result {
                Ok(items) => {
                    app.skills_settings.plugin_catalog = map_plugin_catalog_items(items);
                    if app.skills_settings.selected_plugin_id.as_deref().is_some_and(|selected| {
                        !app.skills_settings
                            .plugin_catalog
                            .iter()
                            .any(|plugin| plugin.id == selected)
                    }) {
                        clear_selected_plugin(app);
                    }
                    clear_plugin_catalog_load_error(app);
                }
                Err(err) => {
                    app.skills_settings.plugin_catalog.clear();
                    set_plugin_status(app, format!("读取插件目录失败: {err}"), true);
                }
            }
            Task::none()
        }
        SettingsMessage::PluginsSelected(plugin_id) => {
            app.skills_settings.selected_plugin_id = Some(plugin_id);
            Task::none()
        }
        SettingsMessage::PluginsSetEnabledRequested { plugin_id, enabled } => {
            app.skills_settings.plugin_loading = true;
            plugin_set_enabled_task(plugin_id, enabled)
        }
        SettingsMessage::PluginsSetEnabledCompleted { plugin_id, enabled, result } => {
            match result {
                Ok(path) => {
                    let action = if enabled { "已启用插件" } else { "已禁用插件" };
                    set_plugin_status(app, format!("{action}: {path}"), false);
                    app.skills_settings.plugin_loading = true;
                    return plugins_refresh_task();
                }
                Err(err) => {
                    app.skills_settings.plugin_loading = false;
                    set_plugin_status(app, err, true);
                    if app.skills_settings.selected_plugin_id.as_deref() == Some(plugin_id.as_str())
                    {
                        app.skills_settings.selected_plugin_id = Some(plugin_id);
                    }
                }
            }
            Task::none()
        }
        SettingsMessage::PluginsDeleteRequested(plugin_id) => {
            app.skills_settings.plugin_loading = true;
            plugin_delete_task(plugin_id)
        }
        SettingsMessage::PluginsDeleteCompleted { plugin_id, result } => {
            match result {
                Ok(path) => {
                    set_plugin_status(app, format!("已删除插件: {path}"), false);
                    if app.skills_settings.selected_plugin_id.as_deref() == Some(plugin_id.as_str())
                    {
                        clear_selected_plugin(app);
                    }
                    app.skills_settings.plugin_loading = true;
                    return plugins_refresh_task();
                }
                Err(err) => {
                    app.skills_settings.plugin_loading = false;
                    set_plugin_status(app, err, true);
                }
            }
            Task::none()
        }
        SettingsMessage::SkillsTabChanged(tab) => {
            app.skills_settings.active_tab = tab;
            clear_selected_detail(app);
            if matches!(tab, crate::app::state::SkillsSettingsTab::Plugins)
                && app.skills_settings.plugin_catalog.is_empty()
                && !app.skills_settings.plugin_loading
            {
                app.skills_settings.plugin_loading = true;
                return plugins_refresh_task();
            }
            Task::none()
        }
        SettingsMessage::SkillsDetailClosed => {
            clear_selected_detail(app);
            Task::none()
        }
        SettingsMessage::SkillsDetailRequested(skill_id) => {
            app.skills_settings.selected_skill_id = Some(skill_id.clone());
            app.skills_settings.selected_skill_detail = None;
            app.skills_settings.detail_loading = true;
            app.skills_settings.detail_error = None;
            detail_task(app.project_path.clone(), skill_id)
        }
        SettingsMessage::SkillsLoaded(result) => {
            app.skills_settings.loading = false;
            match result {
                Ok(items) => {
                    app.skills_settings.catalog = map_catalog_items(items);
                    if app.skills_settings.selected_skill_id.as_deref().is_some_and(|selected| {
                        !app.skills_settings.catalog.iter().any(|skill| skill.id == selected)
                    }) {
                        clear_selected_detail(app);
                    }
                    clear_catalog_load_error(app);
                }
                Err(err) => {
                    app.skills_settings.catalog.clear();
                    set_status(app, format!("读取技能目录失败: {err}"), true);
                }
            }
            Task::none()
        }
        SettingsMessage::SkillsDetailLoaded { skill_id, result } => {
            if app.skills_settings.selected_skill_id.as_deref() != Some(skill_id.as_str()) {
                return Task::none();
            }

            app.skills_settings.detail_loading = false;
            match result {
                Ok(detail) => {
                    app.skills_settings.selected_skill_detail = Some(map_skill_detail(detail));
                    app.skills_settings.detail_error = None;
                }
                Err(err) => {
                    app.skills_settings.selected_skill_detail = None;
                    app.skills_settings.detail_error = Some(err);
                }
            }
            Task::none()
        }
        SettingsMessage::SkillsQueryChanged(value) => {
            app.skills_settings.query = value;
            clear_selected_detail(app);
            Task::none()
        }
        SettingsMessage::SkillsDirectoryScopeChanged(scope) => {
            app.skills_settings.directory_scope = scope;
            clear_selected_detail(app);
            Task::none()
        }
        SettingsMessage::SkillsCreateNewRequested => {
            let Some(project_path) = app.project_path.clone() else {
                set_status(app, "请先打开一个项目，再创建技能。", true);
                return Task::none();
            };
            app.skills_settings.loading = true;

            Task::perform(
                async move { gateway_skill_create_async(&project_path).await },
                |result| Message::Settings(SettingsMessage::SkillsCreateNewCompleted(result)),
            )
        }
        SettingsMessage::SkillsCreateNewCompleted(result) => {
            match result {
                Ok(path) => {
                    set_status(app, format!("已创建新技能: {path}"), false);
                    app.skills_settings.loading = true;
                    return refresh_task(app.project_path.clone());
                }
                Err(err) => {
                    app.skills_settings.loading = false;
                    set_status(app, err, true);
                }
            }
            Task::none()
        }
        SettingsMessage::SkillsCopyInstallCommand => {
            Task::done(Message::CopyCode(INSTALL_COMMAND_TEMPLATE.to_string()))
        }
        SettingsMessage::SkillsInstallBuiltInRequested(skill_id) => {
            let Some(project_path) = app.project_path.clone() else {
                set_status(app, "请先打开一个项目，再安装内置技能。", true);
                return Task::none();
            };

            app.skills_settings.loading = true;
            Task::perform(
                async move { gateway_skill_install_builtin_async(&project_path, &skill_id).await },
                |result| Message::Settings(SettingsMessage::SkillsInstallBuiltInCompleted(result)),
            )
        }
        SettingsMessage::SkillsInstallBuiltInCompleted(result) => {
            match result {
                Ok(path) => {
                    set_status(app, format!("已安装到: {path}"), false);
                    app.skills_settings.loading = true;
                    if let Some(skill_id) = app.skills_settings.selected_skill_id.clone() {
                        app.skills_settings.detail_loading = true;
                        app.skills_settings.detail_error = None;
                        return Task::batch(vec![
                            refresh_task(app.project_path.clone()),
                            detail_task(app.project_path.clone(), skill_id),
                        ]);
                    }
                    return refresh_task(app.project_path.clone());
                }
                Err(err) => {
                    app.skills_settings.loading = false;
                    set_status(app, err, true);
                }
            }
            Task::none()
        }
        SettingsMessage::SkillsSetEnabledRequested { skill_id, enabled } => {
            app.skills_settings.loading = true;
            if app.skills_settings.selected_skill_id.as_deref() == Some(skill_id.as_str()) {
                app.skills_settings.detail_loading = true;
                app.skills_settings.detail_error = None;
            }
            set_enabled_task(app.project_path.clone(), skill_id, enabled)
        }
        SettingsMessage::SkillsSetEnabledCompleted { skill_id, enabled, result } => {
            match result {
                Ok(path) => {
                    let action = if enabled { "已启用技能" } else { "已禁用技能" };
                    set_status(app, format!("{action}: {path}"), false);
                    app.skills_settings.loading = true;

                    let mut tasks = vec![refresh_task(app.project_path.clone())];
                    if app.skills_settings.selected_skill_id.as_deref() == Some(skill_id.as_str()) {
                        app.skills_settings.detail_loading = true;
                        app.skills_settings.detail_error = None;
                        tasks.push(detail_task(app.project_path.clone(), skill_id));
                    }
                    return Task::batch(tasks);
                }
                Err(err) => {
                    app.skills_settings.loading = false;
                    app.skills_settings.detail_loading = false;
                    set_status(app, err, true);
                }
            }
            Task::none()
        }
        SettingsMessage::SkillsDeleteRequested(skill_id) => {
            app.skills_settings.loading = true;
            delete_task(app.project_path.clone(), skill_id)
        }
        SettingsMessage::SkillsDeleteCompleted { skill_id, result } => {
            match result {
                Ok(path) => {
                    set_status(app, format!("已删除技能: {path}"), false);
                    if app.skills_settings.selected_skill_id.as_deref() == Some(skill_id.as_str()) {
                        clear_selected_detail(app);
                    }
                    app.skills_settings.loading = true;
                    return refresh_task(app.project_path.clone());
                }
                Err(err) => {
                    app.skills_settings.loading = false;
                    set_status(app, err, true);
                }
            }
            Task::none()
        }
        SettingsMessage::SkillsDirectoryProviderChanged(provider) => {
            app.skills_settings.directory_provider = provider;
            app.skills_settings.save_error = None;
            app.skills_settings.loading = true;
            clear_selected_detail(app);
            Task::batch(vec![persist_skills_settings(app), refresh_task(app.project_path.clone())])
        }
        SettingsMessage::SkillsProjectAutoLoadToggled(v) => {
            app.skills_settings.auto_load_project_skills = v;
            app.skills_settings.save_error = None;
            persist_skills_settings(app)
        }
        SettingsMessage::SkillsOpenEnabledToggled(v) => {
            app.skills_settings.open_skills_enabled = v;
            app.skills_settings.save_error = None;
            persist_skills_settings(app)
        }
        SettingsMessage::SkillsOpenDirChanged(v) => {
            app.skills_settings.open_skills_dir_input = v;
            app.skills_settings.save_error = None;
            persist_skills_settings(app)
        }
        SettingsMessage::SkillsPromptInjectionModeChanged(v) => {
            app.skills_settings.prompt_injection_mode = v;
            app.skills_settings.save_error = None;
            persist_skills_settings(app)
        }
        SettingsMessage::SkillsSave => {
            app.skills_settings.save_error = None;
            persist_skills_settings(app)
        }
        SettingsMessage::SkillsHelpOpen => {
            app.skills_settings.show_help_modal = true;
            Task::none()
        }
        SettingsMessage::SkillsHelpClose => {
            app.skills_settings.show_help_modal = false;
            Task::none()
        }
        _ => Task::none(),
    }
}
#[cfg(test)]
#[path = "skills_tests.rs"]
mod skills_tests;
