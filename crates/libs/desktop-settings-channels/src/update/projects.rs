//! 从项目管理列表加载工作项目，并在当前 Gateway 租户解析会话归属。
use super::persist::{build_channels_config_from_settings, persist_channels_settings};
use super::{Effect as Message, Services};
use crate::messages::ChannelsMessage;
use crate::state::ChannelsSettingsState;
use iced::Task;
use vw_api_project::ResolveProjectRequest;
use vw_gateway_client::GatewayClient;

fn tenant(client: &GatewayClient) -> Result<String, String> {
    serde_json::to_string(client.endpoint().tenant_scope()).map_err(|err| err.to_string())
}

fn configured(app: &ChannelsSettingsState, instance: &str) -> Result<(), String> {
    let config = build_channels_config_from_settings(&app)?;
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

pub fn load_active(app: &mut ChannelsSettingsState, services: &Services) -> Task<Message> {
    let instance = build_channels_config_from_settings(&app)
        .ok()
        .and_then(|config| config.resolve_channel_instance_profiles().ok())
        .and_then(|profiles| {
            profiles.into_iter().find(|profile| {
                crate::state::ChannelsSettingsTab::from_channel_id(profile.channel_type)
                    == Some(app.active_tab)
            })
        });
    match instance {
        Some(profile) => load(app, services, profile.profile_ref),
        None => Task::none(),
    }
}

pub fn load(
    app: &mut ChannelsSettingsState,
    services: &Services,
    instance: String,
) -> Task<Message> {
    if let Err(error) = configured(app, &instance) {
        return super::error_toast(error);
    }
    let client = services.client.clone();
    let scope = client.as_ref().map_err(Clone::clone).and_then(tenant);
    let scope = match scope {
        Ok(scope) => scope,
        Err(error) => return super::error_toast(error),
    };
    let generation = app.begin_project_choices(&scope, &instance);
    let choices = services.project_choices.clone();
    Task::perform(
        async move {
            let client = client?;
            let mut projects = Vec::with_capacity(choices.len());
            for (directory, name) in choices {
                let response =
                    gateway_project::GatewayProjectClient::new(client.transport().clone())
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
            Message::Channels(ChannelsMessage::ProjectChoicesLoaded {
                tenant: scope.clone(),
                instance: instance.clone(),
                generation,
                result,
            })
        },
    )
}

pub fn loaded(
    app: &mut ChannelsSettingsState,
    services: &Services,
    scope: String,
    instance: String,
    generation: u64,
    result: Result<Vec<(String, String)>, String>,
) -> Task<Message> {
    let current = services.client.clone().and_then(|client| tenant(&client));
    if current.as_ref() != Ok(&scope) || configured(app, &instance).is_err() {
        return Task::none();
    }
    let error = result.as_ref().err().cloned();
    if app.finish_project_choices(&scope, &instance, generation, result)
        && let Some(error) = error
    {
        return super::localized_toast(
            super::ToastKind::Error,
            "项目加载失败：%s",
            vec![error.into()],
        );
    }
    Task::none()
}

pub fn select(
    app: &mut ChannelsSettingsState,
    services: &Services,
    instance: String,
    project: Option<String>,
) -> Task<Message> {
    let context = configured(app, &instance).and_then(|()| {
        let client = services.client.clone()?;
        tenant(&client)
    });
    let scope = match context {
        Ok(scope) => scope,
        Err(error) => return super::error_toast(error),
    };
    match app.select_default_project(&scope, &instance, project.as_deref()) {
        Ok(()) => {
            let save = persist_channels_settings(app, services).unwrap_or_else(Task::none);
            Task::batch([save, super::execution::load(app, services, instance)])
        }
        Err(source) => super::localized_toast(super::ToastKind::Error, source, vec![]),
    }
}

/// 删除或重命名实例后，仅保留仍然配置的真实实例键。
pub fn retain_configured_bindings(app: &mut ChannelsSettingsState) {
    if let Ok(config) = build_channels_config_from_settings(&app)
        && let Ok(profiles) = config.resolve_channel_instance_profiles()
    {
        app.instance_execution
            .retain(|key, _| profiles.iter().any(|profile| &profile.profile_ref == key));
        app.instance_default_projects
            .retain(|key, _| profiles.iter().any(|profile| &profile.profile_ref == key));
    }
}

/// Keep managed order and prefer edited names, then stored names, then directory names.
pub fn project_choices<'a>(
    projects: &[String],
    edits: &[String],
    metadata: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<(String, String)> {
    let metadata: Vec<_> = metadata.into_iter().collect();
    projects
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let name = edits
                .get(index)
                .filter(|name| !name.trim().is_empty())
                .cloned()
                .or_else(|| {
                    metadata
                        .iter()
                        .find(|(meta_path, name)| *meta_path == path.as_str() && !name.trim().is_empty())
                        .map(|(_, name)| (*name).to_owned())
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
