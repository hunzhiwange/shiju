//! 从项目管理列表加载工作项目，并在当前 Gateway 租户解析会话归属。
use super::persist::{build_channels_config_from_settings, persist_channels_settings};
use crate::app::message::settings::{ChannelsMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::{GatewayClient, vw_api_types::project::ResolveProjectRequest};

fn tenant(client: &GatewayClient) -> Result<String, String> {
    serde_json::to_string(client.endpoint().tenant_scope()).map_err(|err| err.to_string())
}

fn configured(app: &App, instance: &str) -> Result<(), String> {
    let config = build_channels_config_from_settings(&app.channels_settings)?;
    if !config
        .resolve_channel_instance_profiles()
        .map_err(|err| err.to_string())?
        .iter()
        .any(|profile| profile.profile_ref == instance)
    {
        return Err("通道实例已删除或 ID 无效，请重新选择实例".into());
    }
    Ok(())
}

/// 使用项目管理中的名称和顺序，不枚举 Gateway 的工作区登记记录。
fn project_choices(app: &App) -> Vec<(String, String)> {
    app.recent_projects
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let name = app
                .recent_projects_edits
                .get(index)
                .filter(|name| !name.trim().is_empty())
                .cloned()
                .or_else(|| {
                    app.recent_projects_meta
                        .iter()
                        .find(|meta| meta.path == *path && !meta.name.trim().is_empty())
                        .map(|meta| meta.name.clone())
                })
                .unwrap_or_else(|| {
                    std::path::Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(path)
                        .to_owned()
                });
            (path.clone(), name)
        })
        .collect()
}

pub(super) fn load_active(app: &mut App) -> Task<Message> {
    let instance = build_channels_config_from_settings(&app.channels_settings)
        .ok()
        .and_then(|config| config.resolve_channel_instance_profiles().ok())
        .and_then(|profiles| {
            profiles.into_iter().find(|profile| {
                crate::app::state::ChannelsSettingsTab::from_channel_id(profile.channel_type)
                    == Some(app.channels_settings.active_tab)
            })
        });
    match instance {
        Some(profile) => load(app, profile.profile_ref),
        None => Task::none(),
    }
}

pub(super) fn load(app: &mut App, instance: String) -> Task<Message> {
    if let Err(error) = configured(app, &instance) {
        return app.show_error_toast(error);
    }
    let client = crate::app::config::gateway_client();
    let scope = client.as_ref().map_err(Clone::clone).and_then(tenant);
    let scope = match scope {
        Ok(scope) => scope,
        Err(error) => return app.show_error_toast(error),
    };
    let generation = app.channels_settings.begin_project_choices(&scope, &instance);
    let choices = project_choices(app);
    Task::perform(
        async move {
            let client = client?;
            let mut projects = Vec::with_capacity(choices.len());
            for (directory, name) in choices {
                let response = client
                    .project_resolve(
                        client.endpoint().tenant_scope(),
                        &ResolveProjectRequest { directory, create_if_missing: true },
                    )
                    .await?;
                projects.push((response.project.id.0, name));
            }
            Ok(projects)
        },
        move |result| {
            Message::Settings(SettingsMessage::Channels(ChannelsMessage::ProjectChoicesLoaded {
                tenant: scope.clone(),
                instance: instance.clone(),
                generation,
                result,
            }))
        },
    )
}

pub(super) fn loaded(
    app: &mut App,
    scope: String,
    instance: String,
    generation: u64,
    result: Result<Vec<(String, String)>, String>,
) -> Task<Message> {
    let current = crate::app::config::gateway_client().and_then(|client| tenant(&client));
    if current.as_ref() != Ok(&scope) || configured(app, &instance).is_err() {
        return Task::none();
    }
    let error = result.as_ref().err().cloned();
    if app.channels_settings.finish_project_choices(&scope, &instance, generation, result)
        && let Some(error) = error
    {
        return app.show_error_toast(format!("项目加载失败：{error}"));
    }
    Task::none()
}

pub(super) fn select(app: &mut App, instance: String, project: Option<String>) -> Task<Message> {
    let result = configured(app, &instance).and_then(|()| {
        let client = crate::app::config::gateway_client()?;
        app.channels_settings.select_default_project(
            &tenant(&client)?,
            &instance,
            project.as_deref(),
        )
    });
    match result {
        Ok(()) => persist_channels_settings(app).unwrap_or_else(Task::none),
        Err(error) => app.show_error_toast(error),
    }
}

/// 删除或重命名实例后，仅保留仍然配置的真实实例键。
pub(super) fn retain_configured_bindings(app: &mut App) {
    if let Ok(config) = build_channels_config_from_settings(&app.channels_settings)
        && let Ok(profiles) = config.resolve_channel_instance_profiles()
    {
        app.channels_settings
            .instance_default_projects
            .retain(|key, _| profiles.iter().any(|profile| &profile.profile_ref == key));
    }
}

#[cfg(test)]
#[path = "projects_tests.rs"]
mod tests;
