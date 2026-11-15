//! Trusted session admission orchestration; host methods retain current request scope.
use anyhow::Result;
use agent_config::runtime::Config;
use config_agent_profile::AgentRuntime;
use crate::agent_binding::AgentBinding;
use crate::admission_host::AdmissionHost;

static INITIALIZE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Freeze defaults before the first model/ACP request, while retaining live safety checks.
pub async fn prepare<H: AdmissionHost>(
    session: &str,
    selected_agent: Option<&str>,
    backend: Option<&str>,
    explicit_model: Option<&str>,
    workspace: &std::path::Path,
) -> Result<(AgentBinding, Config)> {
    use agent_defaults::{DefaultSelection, DefaultsContext, resolve};
    use config_agent_profile::{AgentDefault, AgentModelReference};
    let _guard = INITIALIZE.lock().await;
    let mut config = H::config().await?;
    let fork_selectable = if !session.is_empty() {
        let project = H::project()
            .ok_or_else(|| anyhow::anyhow!("session binding requires a project"))?;
        H::fork_selectable(&project, session).await?
    } else {
        false
    };
    let previous = if session.is_empty() { None } else { H::load_binding(session).await? };
    if !fork_selectable && let Some(binding) = previous.as_ref() {
        binding.validate_selection(selected_agent, backend)?;
        let mut definition = agent_config::catalog::merged_agent_catalog(&config.agents)
            .remove(&binding.agent)
            .unwrap_or_default();
        binding.apply(&mut definition);
        config.agents.insert(binding.agent.clone(), definition);
        return Ok((binding.clone(), config));
    }
    let history = if session.is_empty() {
        Vec::new()
    } else {
        H::history_agents(session).await?
    };
    let recorded_agent = history.first().filter(|_| !fork_selectable)
        .filter(|agent| !agent.is_empty() && agent.as_str() != "gateway").map(String::as_str);
    let fork_preference = if fork_selectable {
        selected_agent
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| backend.map(|backend| format!("acp-{backend}")))
            .or_else(|| previous.as_ref().map(|binding| binding.agent.clone()))
    } else {
        None
    };
    let agent = recorded_agent
        .or(fork_preference.as_deref())
        .or_else(|| selected_agent.map(str::trim).filter(|key| !key.is_empty()))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            backend.map(|backend| format!("acp-{backend}")).unwrap_or_else(|| "main".into())
        });
    let mut definition = H::agent(&agent)
        .await
        .ok_or_else(|| anyhow::anyhow!("unknown or disabled agent: {agent}"))?;
    if fork_selectable
        && let Some(binding) = previous.as_ref().filter(|binding| binding.agent == agent)
    {
        binding.apply(&mut definition);
    }
    let provisional = AgentBinding::from_definition(agent.clone(), &definition);
    provisional.validate_selection(selected_agent, backend)?;
    let mcp = H::mcp(workspace, &provisional.runtime).await?;
    let context = DefaultsContext {
        skills: H::skills(workspace, &config),
        mcp,
        permission_ceiling: H::permission_ceiling(&agent).await?,
    };
    let model = explicit_model
        .filter(|model| !model.trim().is_empty() && !model.trim().eq_ignore_ascii_case("auto"))
        .map(|model| {
            let (provider, model) =
                model.split_once('/').ok_or_else(|| anyhow::anyhow!("invalid model reference"))?;
            Ok::<_, anyhow::Error>(AgentModelReference {
                provider: provider.into(),
                model: model.into(),
            })
        })
        .transpose()?;
    let inherited_model = if definition.has_complete_model_binding() {
        Some(AgentModelReference {
            provider: definition.provider.clone(),
            model: definition.model.clone(),
        })
    } else if matches!(definition.runtime, AgentRuntime::Native) {
        Some(H::default_model().await?)
    } else {
        None
    };
    let effective = resolve(
        &definition,
        &DefaultSelection { model, ..Default::default() },
        &DefaultSelection {
            model: inherited_model,
            skills: Some(definition.allowed_skills.clone()),
            mcp: Some(
                context
                    .mcp
                    .iter()
                    .filter(|(_, enabled)| **enabled)
                    .map(|(id, _)| id.clone())
                    .collect(),
            ),
            ..Default::default()
        },
        &context,
    )?;
    if let Some(model) = effective.model {
        definition.defaults.model = AgentDefault::Fixed(model);
    }
    definition.defaults.skills = AgentDefault::Fixed(effective.skills);
    definition.defaults.mcp = AgentDefault::Fixed(effective.mcp);
    if matches!(definition.runtime, AgentRuntime::Native)
        && matches!(definition.defaults.permission, AgentDefault::Inherit)
    {
        // An inherited default adds no grant of its own; the live ceiling remains
        // authoritative at every tool invocation, including disabled profiles.
        definition.defaults.permission = AgentDefault::Fixed(serde_json::json!({"*":"allow"}));
    }
    if agent == "main" && !history.is_empty() {
        // Legacy sessions without a binding did not opt in to the new default.
        definition.defaults.full_access = false;
    }
    let binding = AgentBinding::from_definition(agent.clone(), &definition);
    let project = H::project()
        .ok_or_else(|| anyhow::anyhow!("session binding requires a project"))?;
    if !session.is_empty() {
        H::save_binding(&project, session, &binding).await?;
    }
    config.agents.insert(agent, definition);
    Ok((binding, config))
}

/// Reload immutable server-owned defaults at the execution boundary.
pub async fn apply_to_config<H: AdmissionHost>(session: &str, config: &mut Config) -> Result<()> {
    if session.is_empty() || H::project().is_none() {
        return Ok(());
    }
    if let Some(binding) = H::load_binding(session).await? {
        let mut definition = agent_config::catalog::merged_agent_catalog(&config.agents)
            .remove(&binding.agent).unwrap_or_default();
        binding.apply(&mut definition);
        config.agents.insert(binding.agent, definition);
    }
    Ok(())
}
