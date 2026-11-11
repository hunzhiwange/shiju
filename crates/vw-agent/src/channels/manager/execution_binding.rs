//! 将实例执行配置解析为模型路由与服务端会话绑定。
use super::*;
use vw_config_types::agent::{AgentDefault, AgentDefinitionConfig, AgentRuntime};
use vw_config_types::channels::ChannelExecutionConfig;

pub(super) fn execution_route(
    execution: &ChannelExecutionConfig,
    agent: &AgentDefinitionConfig,
    mut fallback: ChannelRouteSelection,
) -> Result<(ChannelRouteSelection, bool)> {
    anyhow::ensure!(agent.enabled, "channel agent is disabled");
    let configured_model = execution.model.clone().or_else(|| match &agent.defaults.model {
        AgentDefault::Fixed(model) => Some(model.clone()),
        AgentDefault::Inherit if agent.has_complete_model_binding() => {
            Some(vw_config_types::agent::AgentModelReference {
                provider: agent.provider.clone(),
                model: agent.model.clone(),
            })
        }
        AgentDefault::Inherit => None,
    });
    if agent.force_model_selection
        && execution
            .model
            .as_ref()
            .is_some_and(|model| model.provider != agent.provider || model.model != agent.model)
    {
        anyhow::bail!("channel model conflicts with forced agent model binding");
    }
    let acp = matches!(agent.runtime, AgentRuntime::Acp { .. });
    match configured_model {
        Some(model) => {
            fallback.provider = model.provider;
            fallback.model = model.model;
        }
        None => {
            if let AgentRuntime::Acp { backend } = &agent.runtime {
                // No session-model selection means use this ACP's own local default.
                fallback.provider = "acp".into();
                fallback.model = backend.clone();
            }
        }
    }
    Ok((fallback, acp))
}

pub(super) async fn resolve_execution_route(
    snapshot: &ChannelMessageSnapshot,
    fallback: ChannelRouteSelection,
) -> Result<(ChannelRouteSelection, bool)> {
    let session = sender_session_store()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&snapshot.scope)
        .cloned();
    let binding = match session {
        Some(session) => {
            crate::session::agent_binding::load_scoped(&snapshot.scope.project, &session).await?
        }
        None => None,
    };
    let Some(execution) = &snapshot.execution else {
        anyhow::ensure!(binding.is_none(), "通道执行配置已更改，请使用 /new 新建会话");
        return Ok((fallback, false));
    };
    let key = execution.agent.as_deref().unwrap_or("main");
    let config = crate::config::get_current().await?;
    let mut agent = crate::agent::catalog::merged_agent_catalog(&config.agents)
        .remove(key)
        .or_else(|| binding.as_ref().map(|_| AgentDefinitionConfig::default()))
        .ok_or_else(|| anyhow::anyhow!("unknown channel agent: {key}"))?;
    if let Some(binding) = binding {
        binding
            .validate_selection(Some(key), None)
            .context("更换通道智能体后请使用 /new 新建会话")?;
        binding.apply(&mut agent);
    }
    execution_route(execution, &agent, fallback)
}

pub(super) async fn execution_options(
    snapshot: &ChannelMessageSnapshot,
    session: &str,
    route: &ChannelRouteSelection,
    mut options: serde_json::Value,
) -> Result<serde_json::Value> {
    let Some(execution) = &snapshot.execution else {
        anyhow::ensure!(
            crate::session::agent_binding::load_scoped(&snapshot.scope.project, session)
                .await?
                .is_none(),
            "通道执行配置已更改，请使用 /new 新建会话"
        );
        return Ok(options);
    };
    let model = (route.provider != "acp")
        .then(|| channel_model_request_model(&route.provider, &route.model));
    let (binding, _) = crate::session::agent_binding::prepare(
        session,
        Some(execution.agent.as_deref().unwrap_or("main")),
        None,
        model.as_deref(),
        &snapshot.directory,
    )
    .await
    .context("通道智能体绑定不可用；更换智能体后请使用 /new 新建会话")?;
    if let AgentDefault::Fixed(model) = &binding.defaults.model
        && route.model != "auto"
        && route.provider != "acp"
    {
        anyhow::ensure!(
            model.provider == route.provider && model.model == route.model,
            "更换通道模型后请使用 /new 新建会话"
        );
    }
    if matches!(binding.runtime, AgentRuntime::Acp { .. })
        && let Some(locked) =
            crate::session::llm::locked_acp_model(session, &snapshot.directory.to_string_lossy())
                .map_err(anyhow::Error::msg)?
    {
        let local = locked.source == vw_api_types::acp_model_source::AcpModelSource::CliLocal;
        anyhow::ensure!(
            local == (route.provider == "acp"),
            "更换通道模型来源后请使用 /new 新建会话"
        );
        if !local {
            anyhow::ensure!(
                locked.provider_id.as_deref() == Some(route.provider.as_str())
                    && locked.model.as_deref() == Some(route.model.as_str()),
                "更换通道模型后请使用 /new 新建会话"
            );
        }
    }
    options["agent"] = serde_json::json!(binding.agent);
    if let Some(prompt) = binding.system_prompt {
        options["chat_system_prompt"] = serde_json::json!(prompt);
    }
    if let Some(temperature) = binding.temperature {
        options["temperature"] = serde_json::json!(temperature);
    }
    if let Some(top_p) = binding.top_p {
        options["top_p"] = serde_json::json!(top_p);
    }
    options["delegate_default_allowed_skills"] = serde_json::json!(binding.allowed_skills);
    options["delegate_skill_policy"] = serde_json::to_value(binding.skill_policy)?;
    if binding.agent != "main" {
        options["allowed_tools"] = serde_json::json!(binding.allowed_tools);
    }
    if let AgentRuntime::Acp { backend } = binding.runtime {
        options["acp_agent"] = serde_json::json!(backend);
        options["acp_test"] = serde_json::json!(true);
        options["acp_model_source"] =
            serde_json::json!(if route.provider == "acp" { "cli_local" } else { "session_model" });
        options["cwd"] = serde_json::json!(snapshot.directory);
        options["acp_non_interactive_permissions"] = serde_json::json!("deny");
        options["acp_close_client_after_request"] = serde_json::json!(true);
    }
    Ok(options)
}

#[cfg(test)]
#[path = "execution_binding_tests.rs"]
mod tests;
