//! 按通道实例与工作项目加载执行候选，拒绝过期下拉结果。
use super::persist::{build_channels_config_from_settings, persist_channels_settings};
use crate::app::message::settings::{ChannelsMessage, SettingsMessage};
use crate::app::state::ChannelExecutionChoices;
use crate::app::{App, Message};
use iced::Task;
use vw_config_types::agent::AgentModelReference;

fn configured(app: &App, instance: &str) -> Result<(), String> {
    let config = build_channels_config_from_settings(&app.channels_settings)?;
    if !config
        .resolve_channel_instance_profiles()
        .map_err(|e| e.to_string())?
        .iter()
        .any(|profile| profile.profile_ref == instance)
    {
        return Err("通道实例已删除，请重新选择实例".into());
    }
    Ok(())
}

fn tenant() -> Result<String, String> {
    serde_json::to_string(crate::app::config::gateway_client()?.endpoint().tenant_scope())
        .map_err(|e| e.to_string())
}

/// 页面展示前预加载各实例，候选就绪后才创建可交互的选择框。
pub(super) fn load_active(app: &mut App) -> Task<Message> {
    let profiles = build_channels_config_from_settings(&app.channels_settings)
        .ok()
        .and_then(|config| config.resolve_channel_instance_profiles().ok())
        .unwrap_or_default();
    let instances = profiles
        .into_iter()
        .filter(|profile| {
            crate::app::state::ChannelsSettingsTab::from_channel_id(profile.channel_type)
                == Some(app.channels_settings.active_tab)
        })
        .map(|profile| profile.profile_ref)
        .collect::<Vec<_>>();
    Task::batch(instances.into_iter().map(|instance| load(app, instance)))
}

pub(super) fn load(app: &mut App, instance: String) -> Task<Message> {
    let scope = match configured(app, &instance).and_then(|()| tenant()) {
        Ok(scope) => scope,
        Err(error) => {
            app.channels_settings.execution_choices.remove(&instance);
            app.channels_settings.execution_request.remove(&instance);
            app.channels_settings.execution_error.insert(instance, error.clone());
            return app.show_error_toast(error);
        }
    };
    let state = &mut app.channels_settings;
    let generation = state.begin_execution_choices(&scope, &instance);
    let project = state.instance_default_projects.get(&instance).cloned();
    let fallback = state.project_dir_input.trim().to_string();
    let client = crate::app::config::gateway_client();
    Task::perform(
        async move {
            let client = client?;
            let directory = match project {
                Some(project) => {
                    let scope = vw_gateway_client::vw_api_types::id::ProjectScope::new(
                        client.endpoint().tenant_scope().clone(),
                        project.into(),
                    )
                    .map_err(|e| e.to_string())?;
                    Some(client.project_get(&scope).await?.project.directory)
                }
                None => (!fallback.is_empty()).then_some(fallback),
            };
            #[derive(serde::Deserialize)]
            struct Agent {
                key: String,
                #[serde(flatten)]
                definition: vw_config_types::agent::AgentDefinitionConfig,
            }
            let agents: Vec<Agent> =
                serde_json::from_value(client.agent_profiles(directory.as_deref(), false).await?)
                    .map_err(|e| e.to_string())?;
            let mut agents = agents
                .into_iter()
                .filter(|agent| agent.definition.enabled && !agent.definition.hidden)
                .map(|agent| {
                    let name = agent
                        .definition
                        .label
                        .filter(|label| !label.is_empty())
                        .unwrap_or_else(|| agent.key.clone());
                    (agent.key, name)
                })
                .collect::<Vec<_>>();
            agents.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
            let providers = client.provider_list(directory.as_deref()).await?;
            let mut models = providers
                .all
                .into_iter()
                .flat_map(|provider| {
                    provider
                        .models
                        .into_values()
                        .filter(|model| model.status == "active" && model.provider_id != "acp")
                        .map(|model| AgentModelReference {
                            provider: model.provider_id,
                            model: model.id,
                        })
                })
                .collect::<Vec<_>>();
            models.sort_by(|a, b| (&a.provider, &a.model).cmp(&(&b.provider, &b.model)));
            models.dedup();
            Ok(ChannelExecutionChoices { agents, models })
        },
        move |result| {
            Message::Settings(SettingsMessage::Channels(ChannelsMessage::ExecutionChoicesLoaded {
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
    result: Result<ChannelExecutionChoices, String>,
) -> Task<Message> {
    if tenant().as_ref() != Ok(&scope) || configured(app, &instance).is_err() {
        return Task::none();
    }
    let error = result.as_ref().err().cloned();
    if app.channels_settings.finish_execution_choices(&scope, &instance, generation, result)
        && let Some(error) = error
    {
        return app.show_error_toast(error);
    }
    Task::none()
}

fn choices<'a>(app: &'a App, instance: &str) -> Result<&'a ChannelExecutionChoices, String> {
    configured(app, instance)?;
    let scope = tenant()?;
    if !app
        .channels_settings
        .execution_request
        .get(instance)
        .is_some_and(|(tenant, _)| tenant == &scope)
    {
        return Err("执行候选已过期，请重新加载".into());
    }
    app.channels_settings
        .execution_choices
        .get(instance)
        .ok_or_else(|| "请等待执行候选加载完成".into())
}

pub(super) fn select_agent(
    app: &mut App,
    instance: String,
    agent: Option<String>,
) -> Task<Message> {
    let valid = choices(app, &instance).and_then(|choices| {
        if agent.as_ref().is_some_and(|key| !choices.agents.iter().any(|(id, _)| id == key)) {
            Err("智能体不可用，请重新加载".into())
        } else {
            Ok(())
        }
    });
    if let Err(error) = valid {
        return app.show_error_toast(error);
    }
    app.channels_settings.instance_execution.entry(instance.clone()).or_default().agent = agent;
    save(app, instance)
}

/// 与定时任务一致，切换提供商先清空模型，完整 provider/model 在选模型后保存。
pub(super) fn select_provider(
    app: &mut App,
    instance: String,
    provider: Option<String>,
) -> Task<Message> {
    let valid = choices(app, &instance).and_then(|choices| {
        if provider
            .as_ref()
            .is_some_and(|provider| !choices.models.iter().any(|model| &model.provider == provider))
        {
            Err("模型提供商不可用，请重新加载".into())
        } else {
            Ok(())
        }
    });
    if let Err(error) = valid {
        return app.show_error_toast(error);
    }
    let current =
        app.channels_settings.execution_provider.get(&instance).map(String::as_str).or_else(|| {
            app.channels_settings
                .instance_execution
                .get(&instance)
                .and_then(|binding| binding.model.as_ref())
                .map(|model| model.provider.as_str())
        });
    if current == provider.as_deref() {
        return Task::none();
    }
    match provider {
        Some(provider) => {
            app.channels_settings.execution_provider.insert(instance.clone(), provider);
        }
        None => {
            app.channels_settings.execution_provider.remove(&instance);
        }
    }
    select_model(app, instance, None)
}

pub(super) fn select_model(
    app: &mut App,
    instance: String,
    model: Option<AgentModelReference>,
) -> Task<Message> {
    let valid = choices(app, &instance).and_then(|choices| {
        if model.as_ref().is_some_and(|model| !choices.models.contains(model)) {
            Err("模型不可用，请重新加载".into())
        } else {
            Ok(())
        }
    });
    if let Err(error) = valid {
        return app.show_error_toast(error);
    }
    if let Some(model) = &model {
        app.channels_settings.execution_provider.insert(instance.clone(), model.provider.clone());
    }
    app.channels_settings.instance_execution.entry(instance.clone()).or_default().model = model;
    save(app, instance)
}

fn save(app: &mut App, instance: String) -> Task<Message> {
    if app
        .channels_settings
        .instance_execution
        .get(&instance)
        .is_some_and(|binding| binding.agent.is_none() && binding.model.is_none())
    {
        app.channels_settings.instance_execution.remove(&instance);
    }
    persist_channels_settings(app).unwrap_or_else(Task::none)
}

#[cfg(test)]
#[path = "execution_tests.rs"]
mod tests;
