//! Server-owned session identity. UI snapshots never supply this record.
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use vw_config_types::agent::{
    AgentDefaults, AgentDefinitionConfig, AgentRuntime, DelegateCapabilityPolicy,
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentBinding {
    pub(crate) agent: String,
    pub(crate) runtime: AgentRuntime,
    pub(crate) label: Option<String>,
    pub(crate) defaults: AgentDefaults,
    pub(crate) system_prompt: Option<String>,
    pub(crate) temperature: Option<f64>,
    pub(crate) top_p: Option<f64>,
    pub(crate) allowed_tools: Vec<String>,
    pub(crate) allowed_skills: Vec<String>,
    pub(crate) skill_policy: DelegateCapabilityPolicy,
    pub(crate) provider: String,
    pub(crate) model: String,
    pub(crate) force_model_selection: bool,
}

impl AgentBinding {
    pub(crate) fn from_definition(agent: String, definition: &AgentDefinitionConfig) -> Self {
        Self {
            agent,
            runtime: definition.runtime.clone(),
            label: definition.label.clone(),
            defaults: definition.defaults.clone(),
            system_prompt: definition.system_prompt.clone(),
            temperature: definition.temperature,
            top_p: definition.top_p,
            allowed_tools: definition.allowed_tools.clone(),
            allowed_skills: definition.allowed_skills.clone(),
            skill_policy: definition.skill_policy,
            provider: definition.provider.clone(),
            model: definition.model.clone(),
            force_model_selection: definition.force_model_selection,
        }
    }

    /// Keep credentials and live security rules in current configuration only.
    pub(crate) fn apply(&self, definition: &mut AgentDefinitionConfig) {
        definition.runtime = self.runtime.clone();
        definition.defaults = self.defaults.clone();
        definition.system_prompt = self.system_prompt.clone();
        definition.temperature = self.temperature;
        definition.top_p = self.top_p;
        definition.allowed_tools = self.allowed_tools.clone();
        definition.allowed_skills = self.allowed_skills.clone();
        definition.skill_policy = self.skill_policy;
        definition.provider = self.provider.clone();
        definition.model = self.model.clone();
        definition.force_model_selection = self.force_model_selection;
        // Disabling a profile prevents new bindings, not continuation of an existing one.
        definition.enabled = true;
    }

    pub(crate) fn identity(&self) -> vw_api_types::chat::SessionAgentIdentity {
        vw_api_types::chat::SessionAgentIdentity {
            agent: self.agent.clone(),
            label: self.label.clone(),
            acp_agent: match &self.runtime {
                AgentRuntime::Native => None,
                AgentRuntime::Acp { backend } => Some(backend.clone()),
            },
        }
    }

    pub(crate) fn validate_selection(
        &self,
        agent: Option<&str>,
        backend: Option<&str>,
    ) -> Result<()> {
        if agent.is_some_and(|agent| !agent.trim().is_empty() && agent.trim() != self.agent) {
            bail!("changing a session agent requires a new session");
        }
        if let Some(backend) = backend {
            match &self.runtime {
                AgentRuntime::Acp { backend: selected } if selected == backend => {}
                _ => {
                    bail!("agent and ACP backend conflict; changing engines requires a new session")
                }
            }
        }
        Ok(())
    }
}

fn component(value: &str) -> Result<String> {
    if value.trim().is_empty() {
        bail!("session binding scope must be non-empty");
    }
    Ok(value.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect())
}

fn storage_key(project: &str, session: &str) -> Result<String> {
    Ok(format!("{}:{}", component(project)?, component(session)?))
}

fn store() -> Result<crate::gateway::settings_store::GatewaySettingsStore> {
    let database = crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home)?;
    crate::gateway::settings_store::GatewaySettingsStore::new(database)
}

pub(crate) async fn load_scoped(project: &str, session: &str) -> Result<Option<AgentBinding>> {
    let key = storage_key(project, session)?;
    let tenant = crate::session::ui_store::current_tenant_scope();
    crate::global_database::run_isolated(move || {
        store()?
            .get(&tenant, "session.agent-binding", &key)?
            .map(|payload| serde_json::from_str(&payload).map_err(Into::into))
            .transpose()
    })
    .await?
}

pub(crate) async fn save_scoped(
    project: &str,
    session: &str,
    binding: &AgentBinding,
) -> Result<()> {
    let key = storage_key(project, session)?;
    let tenant = crate::session::ui_store::current_tenant_scope();
    let payload = serde_json::to_string(binding)?;
    crate::global_database::run_isolated(move || {
        store()?.upsert(&tenant, "session.agent-binding", &key, &payload)
    })
    .await?
}

// Persist the session choice separately from the immutable agent defaults snapshot.
static FULL_ACCESS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub(crate) async fn resolve_full_access_scoped(
    project: &str,
    session: &str,
    explicit: Option<bool>,
    initial: bool,
) -> Result<bool> {
    let _guard = FULL_ACCESS.lock().await;
    let key = storage_key(project, session)?;
    let tenant = crate::session::ui_store::current_tenant_scope();
    crate::global_database::run_isolated(move || {
        let store = store()?;
        let previous = store.get(&tenant, "session.full-access", &key)?;
        let enabled = match explicit {
            Some(enabled) => enabled,
            None => previous
                .as_deref()
                .map(serde_json::from_str::<bool>)
                .transpose()?
                .unwrap_or(initial),
        };
        store.upsert(&tenant, "session.full-access", &key, &serde_json::to_string(&enabled)?)?;
        Ok(enabled)
    })
    .await?
}

pub(crate) async fn resolve_full_access(
    session: &str,
    explicit: Option<bool>,
    initial: bool,
) -> Result<bool> {
    if session.is_empty() {
        return Ok(explicit.unwrap_or(initial));
    }
    let project = crate::project::instance::project()
        .ok_or_else(|| anyhow::anyhow!("session permissions require a project"))?
        .id;
    resolve_full_access_scoped(&project, session, explicit, initial).await
}

pub(crate) async fn load(session: &str) -> Result<Option<AgentBinding>> {
    let project = crate::project::instance::project()
        .ok_or_else(|| anyhow::anyhow!("session binding requires a project"))?
        .id;
    load_scoped(&project, session).await
}

/// Reload from server storage at the execution boundary, never from request options.
pub(crate) async fn apply_to_config(
    session: &str,
    config: &mut crate::config::Config,
) -> Result<()> {
    if session.is_empty() || crate::project::instance::project().is_none() {
        return Ok(());
    }
    if let Some(binding) = load(session).await? {
        let mut definition = crate::agent::catalog::merged_agent_catalog(&config.agents)
            .remove(&binding.agent)
            .unwrap_or_default();
        binding.apply(&mut definition);
        config.agents.insert(binding.agent, definition);
    }
    Ok(())
}

#[cfg(test)]
#[path = "agent_binding_tests.rs"]
mod tests;

static INITIALIZE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Freeze defaults before the first model/ACP request, while retaining live safety checks.
pub(crate) async fn prepare(
    session: &str,
    selected_agent: Option<&str>,
    backend: Option<&str>,
    explicit_model: Option<&str>,
    workspace: &std::path::Path,
) -> Result<(AgentBinding, crate::config::Config)> {
    use crate::agent::defaults::{DefaultSelection, DefaultsContext, resolve};
    use vw_config_types::agent::{AgentDefault, AgentModelReference};
    let _guard = INITIALIZE.lock().await;
    let mut config = crate::config::get_current().await?;
    if !session.is_empty()
        && let Some(binding) = load(session).await?
    {
        binding.validate_selection(selected_agent, backend)?;
        let mut definition = crate::agent::catalog::merged_agent_catalog(&config.agents)
            .remove(&binding.agent)
            .unwrap_or_default();
        binding.apply(&mut definition);
        config.agents.insert(binding.agent.clone(), definition);
        return Ok((binding, config));
    }
    let agent = selected_agent
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            backend.map(|backend| format!("acp-{backend}")).unwrap_or_else(|| "main".into())
        });
    let mut definition = crate::agent::get(&agent)
        .await
        .ok_or_else(|| anyhow::anyhow!("unknown or disabled agent: {agent}"))?;
    let provisional = AgentBinding::from_definition(agent.clone(), &definition);
    provisional.validate_selection(Some(&agent), backend)?;
    let mcp = crate::tools::mcp_common::load_resolved_acp_config_from(workspace).await?;
    let context = DefaultsContext {
        skills: crate::skills::load_skills_with_config(workspace, &config)
            .iter()
            .map(|skill| (skill.invocation_name().to_string(), true))
            .collect(),
        mcp: mcp
            .mcp_servers
            .iter()
            .map(|server| {
                (
                    crate::tools::mcp_common::server_name(server).to_string(),
                    crate::tools::mcp_common::management_for_server(&mcp, server).enabled,
                )
            })
            .collect(),
        permission_ceiling: crate::agent::native_permission_ceiling(&agent)
            .await
            .map_err(anyhow::Error::msg)?,
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
        let model = crate::provider::provider::default_model().await.map_err(anyhow::Error::msg)?;
        Some(AgentModelReference { provider: model.provider_id, model: model.model_id })
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
    let binding = AgentBinding::from_definition(agent.clone(), &definition);
    let project = crate::project::instance::project()
        .ok_or_else(|| anyhow::anyhow!("session binding requires a project"))?
        .id;
    if !session.is_empty() {
        save_scoped(&project, session, &binding).await?;
    }
    config.agents.insert(agent, definition);
    Ok((binding, config))
}
