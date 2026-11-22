//! Server-owned session identity. UI snapshots never supply this record.
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use config_agent_profile::{AgentDefaults, AgentRuntime};
use config_agent::{AgentDefinitionConfig, DelegateCapabilityPolicy};

/// Non-sensitive product identity tied to a server-owned session binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmployeeTarget {
    #[serde(deserialize_with = "deserialize_employee_id")]
    employee_id: String,
    pub kind: EmployeeKind,
}

impl EmployeeTarget {
    pub fn employee_id(&self) -> &str {
        &self.employee_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployeeKind {
    Private,
    Shared,
}

fn deserialize_employee_id<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if !(1..=128).contains(&value.len())
        || !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(serde::de::Error::custom("invalid employee_id"));
    }
    Ok(value)
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    employee_target: Option<EmployeeTarget>,
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
            employee_target: None,
        }
    }

    pub fn is_employee(&self) -> bool {
        matches!(&self.runtime, AgentRuntime::Acp { backend } if backend == "employee-acp")
    }

    pub fn employee_target(&self) -> Option<&EmployeeTarget> {
        self.employee_target.as_ref()
    }

    pub(crate) fn set_employee_target(&mut self, target: EmployeeTarget) {
        self.employee_target = Some(target);
    }

    pub(crate) fn validate_employee_selection(
        &self,
        requested: Option<&EmployeeTarget>,
    ) -> Result<()> {
        if self.is_employee() {
            let bound = self.employee_target()
                .ok_or_else(|| anyhow::anyhow!("missing employee session binding"))?;
            if requested.is_some_and(|target| target != bound) {
                bail!("changing the employee target requires a new session");
            }
        } else if requested.is_some() {
            bail!("employee_target requires the employee ACP backend");
        }
        Ok(())
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
            label: self.employee_target()
                .map(|target| format!("数字员工 · {}", target.employee_id()))
                .or_else(|| self.label.clone()),
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
