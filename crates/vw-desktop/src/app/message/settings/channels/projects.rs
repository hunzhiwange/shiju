//! 通过当前 Gateway 租户加载项目，不接受目录或跨实例异步结果。
use super::persist::{build_channels_config_from_settings, persist_channels_settings};
use crate::app::message::settings::{ChannelsMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::{GatewayClient, vw_api_types::project::ListProjectsRequest};

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
    Task::perform(
        async move {
            let client = client?;
            let mut request = ListProjectsRequest { limit: Some(100), ..Default::default() };
            let mut projects = Vec::new();
            loop {
                let page = client.project_list(client.endpoint().tenant_scope(), &request).await?;
                projects.extend(page.items.into_iter().map(|project| (project.id.0, project.name)));
                match page.next_cursor {
                    Some(cursor) if request.cursor.as_ref() == Some(&cursor) => {
                        return Err("项目分页游标未前进".into());
                    }
                    Some(cursor) => request.cursor = Some(cursor),
                    None => break,
                }
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
