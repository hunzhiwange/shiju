use crate::app::{App, Message, message::SettingsMessage};
use desktop_settings_skills::types::SkillsDirectoryScope;
use iced::Task;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn pick(app: &mut App, zip: bool) -> Task<Message> {
    if app.skills_settings.local_import_busy {
        return Task::none();
    }
    let endpoint = crate::app::config::gateway_client_endpoint();
    if !endpoint.is_loopback() {
        return app.show_message_toast(
            crate::app::state::ToastKind::Error,
            desktop_i18n::settings_extensions::feedback_message(&("本地技能导入需要连接本机网关")),
        );
    }
    app.skills_settings.local_import_busy = true;
    let project = app.navigation_tabs.project_path.clone();
    let destination_project = project.clone();

    Task::perform(
        async move {
            let dialog = rfd::AsyncFileDialog::new();
            let source = if zip {
                dialog.add_filter("ZIP", &["zip"]).pick_file().await
            } else {
                dialog.pick_folder().await
            };
            let Some(source) = source else {
                return Ok(None);
            };
            let source = source.path().to_owned();
            let config = desktop_config::ConfigAccess::new(endpoint, None)
                .load_full_agent_config_async()
                .await?;
            tokio::task::spawn_blocking(move || {
                let skills_root = match destination_project {
                    Some(path) => {
                        let path = std::path::Path::new(&path)
                            .canonicalize()
                            .map_err(|error| format!("无法打开项目目录：{error}"))?;
                        if !path.is_dir() {
                            return Err("项目目录不存在".to_owned());
                        }
                        agent_skills::local_sources::workspace_skills_dir(&path)
                    }
                    None => {
                        let home = std::env::var_os("HOME")
                            .filter(|value| !value.is_empty())
                            .or_else(|| {
                                std::env::var_os("USERPROFILE").filter(|value| !value.is_empty())
                            })
                            .ok_or_else(|| "无法定位全局技能目录".to_owned())?;
                        config_runtime_paths::home_config_dir(std::path::PathBuf::from(home))
                            .join("skills")
                    }
                };
                let security = &config.skills.security;
                let result = if zip {
                    agent_skills::import::import_skill_zip(&source, &skills_root, security)
                } else {
                    agent_skills::import::import_skill_folder(&source, &skills_root, security)
                        .map(|()| (1, Vec::new()))
                };
                result.map(Some).map_err(|error| format!("{error:#}"))
            })
            .await
            .map_err(|error| error.to_string())?
        },
        move |result| {
            Message::Settings(SettingsMessage::SkillsLocalImportCompleted {
                project: project.clone(),
                result,
            })
        },
    )
}

#[cfg(target_arch = "wasm32")]
pub(super) fn pick(_app: &mut App, _zip: bool) -> Task<Message> {
    Task::none()
}

pub(super) fn complete(
    app: &mut App,
    project: Option<String>,
    result: Result<Option<(usize, Vec<String>)>, String>,
) -> Task<Message> {
    app.skills_settings.local_import_busy = false;
    let (count, errors) = match result {
        Ok(Some(outcome)) => outcome,
        Ok(None) => return Task::none(),
        Err(error) => {
            return app.show_message_toast(
                crate::app::state::ToastKind::Error,
                desktop_i18n::settings_extensions::feedback_message(
                    &(format!("技能导入失败：{error}")),
                ),
            );
        }
    };
    let toast = if let Some(error) = errors.first() {
        let message = format!("已导入 {count} 项技能，{} 项失败：{error}", errors.len());
        if count == 0 {
            app.show_message_toast(
                crate::app::state::ToastKind::Error,
                desktop_i18n::settings_extensions::feedback_message(&(message)),
            )
        } else {
            app.show_message_toast(
                crate::app::state::ToastKind::Warning,
                desktop_i18n::settings_extensions::feedback_message(&(message)),
            )
        }
    } else {
        app.show_message_toast(
            crate::app::state::ToastKind::Success,
            desktop_i18n::settings_extensions::feedback_message(
                &(format!("已导入 {count} 项技能")),
            ),
        )
    };
    if count == 0 {
        return toast;
    }
    if project == app.navigation_tabs.project_path
        && app.skills_settings.directory_scope != SkillsDirectoryScope::All
    {
        app.skills_settings.directory_scope = if project.is_some() {
            SkillsDirectoryScope::Project
        } else {
            SkillsDirectoryScope::Global
        };
    }
    Task::batch([toast, super::update(app, SettingsMessage::SkillsRefresh)])
}
