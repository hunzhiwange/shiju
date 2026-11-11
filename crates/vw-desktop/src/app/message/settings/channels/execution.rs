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

pub(super) fn load(app: &mut App, instance: String) -> Task<Message> {
    let scope = match configured(app, &instance).and_then(|()| tenant()) {
        Ok(scope) => scope,
        Err(error) => return app.show_error_toast(error),
    };
    let state = &mut app.channels_settings;
    state.execution_generation = state.execution_generation.wrapping_add(1);
    let generation = state.execution_generation;
    state.execution_request = Some((scope.clone(), instance.clone(), generation));
    state.execution_choices = None;
    state.execution_error = None;
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
                    let label = match agent.definition.runtime {
                        vw_config_types::agent::AgentRuntime::Native => name,
                        vw_config_types::agent::AgentRuntime::Acp { backend } => {
                            format!("{name} ({backend})")
                        }
                    };
                    (agent.key, label)
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
    if tenant().as_ref() != Ok(&scope)
        || configured(app, &instance).is_err()
        || app.channels_settings.execution_request.as_ref()
            != Some(&(scope, instance.clone(), generation))
    {
        return Task::none();
    }
    match result {
        Ok(choices) => app.channels_settings.execution_choices = Some((instance, choices)),
        Err(error) => {
            app.channels_settings.execution_error = Some(error.clone());
            return app.show_error_toast(error);
        }
    }
    Task::none()
}

fn choices<'a>(app: &'a App, instance: &str) -> Result<&'a ChannelExecutionChoices, String> {
    configured(app, instance)?;
    let scope = tenant()?;
    if !app
        .channels_settings
        .execution_request
        .as_ref()
        .is_some_and(|(tenant, key, _)| tenant == &scope && key == instance)
    {
        return Err("执行候选已过期，请重新打开下拉框".into());
    }
    app.channels_settings
        .execution_choices
        .as_ref()
        .filter(|(key, _)| key == instance)
        .map(|(_, choices)| choices)
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
