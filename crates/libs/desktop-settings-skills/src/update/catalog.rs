use super::state::State;
use crate::types::*;
use gateway_desktop_settings::{DesktopPluginCatalogEntryDto, DesktopSkillCatalogEntryDto, DesktopSkillDetailDto};
#[derive(Clone, Debug)]
pub enum Event {
SkillsProjectSelected(String),
SkillsRefresh,
PluginsRefresh,
PluginsLoaded(Result<Vec<DesktopPluginCatalogEntryDto>, String>),
PluginsSelected(String),
PluginsSetEnabledRequested {
        plugin_id: String,
        enabled: bool,
    },
PluginsSetEnabledCompleted {
        plugin_id: String,
        enabled: bool,
        result: Result<String, String>,
    },
PluginsDeleteRequested(String),
PluginsDeleteCompleted {
        plugin_id: String,
        result: Result<String, String>,
    },
SkillsTabChanged(SkillsSettingsTab),
SkillsDetailClosed,
SkillsDetailRequested(String),
SkillsLoaded(Result<Vec<DesktopSkillCatalogEntryDto>, String>),
SkillsDetailLoaded {
        skill_id: String,
        result: Result<DesktopSkillDetailDto, String>,
    },
SkillsQueryChanged(String),
SkillsDirectoryScopeChanged(SkillsDirectoryScope),
SkillsCreateNewRequested,
SkillsCreateNewCompleted(Result<String, String>),
SkillsCopyInstallCommand,
SkillsInstallBuiltInRequested(String),
SkillsInstallBuiltInCompleted(Result<String, String>),
SkillsSetEnabledRequested {
        skill_id: String,
        enabled: bool,
    },
SkillsSetEnabledCompleted {
        skill_id: String,
        enabled: bool,
        result: Result<String, String>,
    },
SkillsDeleteRequested(String),
SkillsDeleteCompleted {
        skill_id: String,
        result: Result<String, String>,
    },
SkillsDirectoryProviderChanged(config_skills::SkillsDirectoryProvider),
SkillsProjectAutoLoadToggled(bool),
SkillsPromptInjectionModeChanged(config_skills::SkillsPromptInjectionMode),
SkillsSave,
SkillsHelpOpen,
SkillsHelpClose,
}
pub enum Effect {
 None, Batch(Vec<Effect>), RefreshProject(String), OpenProject(String), Copy(String),
 Refresh(Option<String>), PluginsRefresh, Detail(Option<String>, String),
 SetEnabled(Option<String>, String, bool), Delete(Option<String>, String),
 PluginSetEnabled(String, bool), PluginDelete(String), Create(String), InstallBuiltin(String, String),
 Save(config_skills::SkillsDirectoryProvider, bool, config_skills::SkillsPromptInjectionMode),
}
fn refresh_task(p: Option<String>) -> Effect { Effect::Refresh(p) }
fn plugins_refresh_task() -> Effect { Effect::PluginsRefresh }
fn detail_task(p: Option<String>, id: String) -> Effect { Effect::Detail(p,id) }
fn set_enabled_task(p: Option<String>, id: String, enabled: bool) -> Effect { Effect::SetEnabled(p,id,enabled) }
fn delete_task(p: Option<String>, id: String) -> Effect { Effect::Delete(p,id) }
fn plugin_set_enabled_task(id: String, enabled: bool) -> Effect { Effect::PluginSetEnabled(id,enabled) }
fn plugin_delete_task(id: String) -> Effect { Effect::PluginDelete(id) }
fn persist_skills_settings(s: &State) -> Effect { Effect::Save(s.directory_provider,s.auto_load_project_skills,s.prompt_injection_mode) }
pub const INSTALL_COMMAND_TEMPLATE: &str = "shiju skills install <source>";

fn clear_selected_detail(app: &mut State) {
    app.selected_skill_id = None;
    app.selected_skill_detail = None;
    app.detail_loading = false;
    app.detail_error = None;
}

fn clear_selected_plugin(app: &mut State) {
    app.selected_plugin_id = None;
}

fn set_status(app: &mut State, message: impl Into<String>, is_error: bool) {
    app.status_message_translation = None;
    app.status_message = Some(message.into());
    app.status_is_error = is_error;
}

fn set_plugin_status(app: &mut State, message: impl Into<String>, is_error: bool) {
    app.plugin_status_message_translation = None;
    app.plugin_status_message = Some(message.into());
    app.plugin_status_is_error = is_error;
}

fn clear_catalog_load_error(app: &mut State) {
    let is_load_error = app.status_is_error
        && app
            .status_message
            .as_deref()
            .is_some_and(|message| message.starts_with("读取技能目录失败:"));
    if is_load_error {
        app.status_message = None;
        app.status_is_error = false;
    }
}

fn clear_plugin_catalog_load_error(app: &mut State) {
    let is_load_error = app.plugin_status_is_error
        && app
            .plugin_status_message
            .as_deref()
            .is_some_and(|message| message.starts_with("读取插件目录失败:"));
    if is_load_error {
        app.plugin_status_message = None;
        app.plugin_status_is_error = false;
    }
}

pub use super::catalog_mapping::map_catalog_items;
use super::catalog_mapping::{map_plugin_catalog_items, map_skill_detail};
#[cfg(test)]
use super::catalog_mapping::map_skill_kind;

pub fn update(app: &mut State, current_project: &Option<String>, message: Event) -> Effect {
    match message {
        Event::SkillsProjectSelected(project_path) => {
            if current_project.as_deref() == Some(project_path.as_str()) {
                return Effect::RefreshProject(project_path);
            }
            Effect::OpenProject(project_path)
        }
        Event::SkillsRefresh => {
            app.loading = true;
            refresh_task(current_project.clone())
        }
        Event::PluginsRefresh => {
            app.plugin_loading = true;
            plugins_refresh_task()
        }
        Event::PluginsLoaded(result) => {
            app.plugin_loading = false;
            match result {
                Ok(items) => {
                    app.plugin_catalog = map_plugin_catalog_items(items);
                    if app.selected_plugin_id.as_deref().is_some_and(|selected| {
                        !app
                            .plugin_catalog
                            .iter()
                            .any(|plugin| plugin.id == selected)
                    }) {
                        clear_selected_plugin(app);
                    }
                    clear_plugin_catalog_load_error(app);
                }
                Err(err) => {
                    app.plugin_catalog.clear();
                    set_plugin_status_localized(
                        app,
                        "读取插件目录失败: %s",
                        vec![(err).into()],
                        true,
                    );
                }
            }
            Effect::None
        }
        Event::PluginsSelected(plugin_id) => {
            app.selected_plugin_id = Some(plugin_id);
            Effect::None
        }
        Event::PluginsSetEnabledRequested { plugin_id, enabled } => {
            app.plugin_loading = true;
            plugin_set_enabled_task(plugin_id, enabled)
        }
        Event::PluginsSetEnabledCompleted { plugin_id, enabled, result } => {
            match result {
                Ok(path) => {
                    set_plugin_status_localized(
                        app,
                        if enabled { "已启用插件: %s" } else { "已禁用插件: %s" },
                        vec![path.into()],
                        false,
                    );
                    app.plugin_loading = true;
                    return plugins_refresh_task();
                }
                Err(err) => {
                    app.plugin_loading = false;
                    set_plugin_status(app, err, true);
                    if app.selected_plugin_id.as_deref() == Some(plugin_id.as_str())
                    {
                        app.selected_plugin_id = Some(plugin_id);
                    }
                }
            }
            Effect::None
        }
        Event::PluginsDeleteRequested(plugin_id) => {
            app.plugin_loading = true;
            plugin_delete_task(plugin_id)
        }
        Event::PluginsDeleteCompleted { plugin_id, result } => {
            match result {
                Ok(path) => {
                    set_plugin_status_localized(app, "已删除插件: %s", vec![(path).into()], false);
                    if app.selected_plugin_id.as_deref() == Some(plugin_id.as_str())
                    {
                        clear_selected_plugin(app);
                    }
                    app.plugin_loading = true;
                    return plugins_refresh_task();
                }
                Err(err) => {
                    app.plugin_loading = false;
                    set_plugin_status(app, err, true);
                }
            }
            Effect::None
        }
        Event::SkillsTabChanged(tab) => {
            app.active_tab = tab;
            clear_selected_detail(app);
            Effect::None
        }
        Event::SkillsDetailClosed => {
            clear_selected_detail(app);
            Effect::None
        }
        Event::SkillsDetailRequested(skill_id) => {
            app.selected_skill_id = Some(skill_id.clone());
            app.selected_skill_detail = None;
            app.detail_loading = true;
            app.detail_error = None;
            detail_task(current_project.clone(), skill_id)
        }
        Event::SkillsLoaded(result) => {
            app.loading = false;
            match result {
                Ok(items) => {
                    app.catalog = map_catalog_items(items);
                    if app.selected_skill_id.as_deref().is_some_and(|selected| {
                        !app.catalog.iter().any(|skill| skill.id == selected)
                    }) {
                        clear_selected_detail(app);
                    }
                    clear_catalog_load_error(app);
                }
                Err(err) => {
                    app.catalog.clear();
                    set_status_localized(app, "读取技能目录失败: %s", vec![(err).into()], true);
                }
            }
            Effect::None
        }
        Event::SkillsDetailLoaded { skill_id, result } => {
            if app.selected_skill_id.as_deref() != Some(skill_id.as_str()) {
                return Effect::None;
            }

            app.detail_loading = false;
            match result {
                Ok(detail) => {
                    app.selected_skill_detail = Some(map_skill_detail(detail));
                    app.detail_error = None;
                }
                Err(err) => {
                    app.selected_skill_detail = None;
                    app.detail_error = Some(err);
                }
            }
            Effect::None
        }
        Event::SkillsQueryChanged(value) => {
            app.query = value;
            clear_selected_detail(app);
            Effect::None
        }
        Event::SkillsDirectoryScopeChanged(scope) => {
            app.directory_scope = scope;
            clear_selected_detail(app);
            Effect::None
        }
        Event::SkillsCreateNewRequested => {
            let Some(project_path) = current_project.clone() else {
                set_status_localized(app, "请先打开一个项目，再创建技能。", vec![], true);
                return Effect::None;
            };
            app.loading = true;

            Effect::Create(project_path)
        }
        Event::SkillsCreateNewCompleted(result) => {
            match result {
                Ok(path) => {
                    set_status_localized(app, "已创建新技能: %s", vec![(path).into()], false);
                    app.loading = true;
                    return refresh_task(current_project.clone());
                }
                Err(err) => {
                    app.loading = false;
                    set_status(app, err, true);
                }
            }
            Effect::None
        }
        Event::SkillsCopyInstallCommand => {
            Effect::Copy(INSTALL_COMMAND_TEMPLATE.to_string())
        }
        Event::SkillsInstallBuiltInRequested(skill_id) => {
            let Some(project_path) = current_project.clone() else {
                set_status_localized(app, "请先打开一个项目，再安装内置技能。", vec![], true);
                return Effect::None;
            };

            app.loading = true;
            Effect::InstallBuiltin(project_path, skill_id)
        }
        Event::SkillsInstallBuiltInCompleted(result) => {
            match result {
                Ok(path) => {
                    set_status_localized(app, "已安装到: %s", vec![(path).into()], false);
                    app.loading = true;
                    if let Some(skill_id) = app.selected_skill_id.clone() {
                        app.detail_loading = true;
                        app.detail_error = None;
                        return Effect::Batch(vec![
                            refresh_task(current_project.clone()),
                            detail_task(current_project.clone(), skill_id),
                        ]);
                    }
                    return refresh_task(current_project.clone());
                }
                Err(err) => {
                    app.loading = false;
                    set_status(app, err, true);
                }
            }
            Effect::None
        }
        Event::SkillsSetEnabledRequested { skill_id, enabled } => {
            app.loading = true;
            if app.selected_skill_id.as_deref() == Some(skill_id.as_str()) {
                app.detail_loading = true;
                app.detail_error = None;
            }
            set_enabled_task(current_project.clone(), skill_id, enabled)
        }
        Event::SkillsSetEnabledCompleted { skill_id, enabled, result } => {
            match result {
                Ok(path) => {
                    set_status_localized(
                        app,
                        if enabled { "已启用技能: %s" } else { "已禁用技能: %s" },
                        vec![path.into()],
                        false,
                    );
                    app.loading = true;

                    let mut tasks = vec![refresh_task(current_project.clone())];
                    if app.selected_skill_id.as_deref() == Some(skill_id.as_str()) {
                        app.detail_loading = true;
                        app.detail_error = None;
                        tasks.push(detail_task(current_project.clone(), skill_id));
                    }
                    return Effect::Batch(tasks);
                }
                Err(err) => {
                    app.loading = false;
                    app.detail_loading = false;
                    set_status(app, err, true);
                }
            }
            Effect::None
        }
        Event::SkillsDeleteRequested(skill_id) => {
            app.loading = true;
            delete_task(current_project.clone(), skill_id)
        }
        Event::SkillsDeleteCompleted { skill_id, result } => {
            match result {
                Ok(path) => {
                    set_status_localized(app, "已删除技能: %s", vec![(path).into()], false);
                    if app.selected_skill_id.as_deref() == Some(skill_id.as_str()) {
                        clear_selected_detail(app);
                    }
                    app.loading = true;
                    return refresh_task(current_project.clone());
                }
                Err(err) => {
                    app.loading = false;
                    set_status(app, err, true);
                }
            }
            Effect::None
        }
        Event::SkillsDirectoryProviderChanged(provider) => {
            app.directory_provider = provider;
            app.save_error = None;
            app.loading = true;
            clear_selected_detail(app);
            Effect::Batch(vec![persist_skills_settings(app), refresh_task(current_project.clone())])
        }
        Event::SkillsProjectAutoLoadToggled(v) => {
            app.auto_load_project_skills = v;
            app.save_error = None;
            persist_skills_settings(app)
        }
        Event::SkillsPromptInjectionModeChanged(v) => {
            app.prompt_injection_mode = v;
            app.save_error = None;
            persist_skills_settings(app)
        }
        Event::SkillsSave => {
            app.save_error = None;
            persist_skills_settings(app)
        }
        Event::SkillsHelpOpen => {
            app.show_help_modal = true;
            Effect::None
        }
        Event::SkillsHelpClose => {
            app.show_help_modal = false;
            Effect::None
        }
    }
}

fn set_status_localized(
    app: &mut State,
    source: &str,
    arguments: Vec<desktop_i18n::format::Argument>,
    is_error: bool,
) {
    let message = desktop_i18n::format::Message::new(source, arguments);
    let chinese = message
        .render(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese))
        .expect("valid skills status template");
    set_status(app, chinese, is_error);
    app.status_message_translation = Some(message);
}

fn set_plugin_status_localized(
    app: &mut State,
    source: &str,
    arguments: Vec<desktop_i18n::format::Argument>,
    is_error: bool,
) {
    let message = desktop_i18n::format::Message::new(source, arguments);
    let chinese = message
        .render(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese))
        .expect("valid skills status template");
    set_plugin_status(app, chinese, is_error);
    app.plugin_status_message_translation = Some(message);
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
