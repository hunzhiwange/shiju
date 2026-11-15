//! Server-owned session identity. UI snapshots never supply this record.
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use config_agent_profile::{AgentDefaults, AgentRuntime};
use config_agent::{AgentDefinitionConfig, DelegateCapabilityPolicy};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentBinding {
    pub agent: String,
    pub runtime: AgentRuntime,
    pub label: Option<String>,
    pub defaults: AgentDefaults,
    pub system_prompt: Option<String>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub allowed_tools: Vec<String>,
    pub allowed_skills: Vec<String>,
    pub skill_policy: DelegateCapabilityPolicy,
    pub provider: String,
    pub model: String,
    pub force_model_selection: bool,
}

impl AgentBinding {
    pub fn from_definition(agent: String, definition: &AgentDefinitionConfig) -> Self {
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
    pub fn apply(&self, definition: &mut AgentDefinitionConfig) {
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

    pub fn identity(&self) -> vw_api_chat_identity::SessionAgentIdentity {
        vw_api_chat_identity::SessionAgentIdentity {
            agent: self.agent.clone(),
            label: self.label.clone(),
            acp_agent: match &self.runtime {
                AgentRuntime::Native => None,
                AgentRuntime::Acp { backend } => Some(backend.clone()),
            },
        }
    }

    pub fn validate_selection(
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
