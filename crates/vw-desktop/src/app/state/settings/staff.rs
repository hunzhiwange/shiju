//! 维护员工配置系统设置页的 UI 草稿状态。

use vw_config_types::staff::{
    StaffCapabilitiesConfig, StaffChannelBindingConfig, StaffConfig, StaffMemberConfig,
};

const BUILTIN_STAFF_MEMBER_IDS: &[&str] =
    &["staff-orchestrator", "staff-planner", "staff-executor", "staff-reviewer", "staff-merger"];
pub(crate) fn is_builtin_staff_member_id(member_id: &str) -> bool {
    BUILTIN_STAFF_MEMBER_IDS.iter().any(|candidate| *candidate == member_id)
}

pub(crate) fn staff_member_needs_agent_binding(draft: &StaffSettingsMemberDraft) -> bool {
    draft.enabled
        && !optional_has_value(&draft.delegate_agent)
        && !optional_has_value(&draft.acp_agent)
}

#[derive(Debug, Clone)]
pub(crate) struct StaffSettingsState {
    pub(crate) loading: bool,
    pub(crate) config: StaffConfig,
    pub(crate) active_tab: StaffSettingsTab,
    pub(crate) selected_member_id: Option<String>,
    pub(crate) member_drafts: Vec<StaffSettingsMemberDraft>,
    pub(crate) capability_drafts: Vec<StaffSettingsCapabilitiesDraft>,
    pub(crate) save_error: Option<String>,
    pub(crate) save_success: bool,
    pub(crate) pending_config: Option<StaffConfig>,
    pub(crate) workers: super::StaffWorkerResourcesState,
    pub(crate) teams: super::StaffTeamResourcesState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaffSettingsTab {
    Roster,
    Capabilities,
    Teams,
    Workers,
}

impl StaffSettingsTab {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Roster => "roster",
            Self::Capabilities => "capabilities",
            Self::Teams => "teams",
            Self::Workers => "workers",
        }
    }

    pub(crate) fn from_key(value: &str) -> Option<Self> {
        match value {
            "roster" => Some(Self::Roster),
            "capabilities" => Some(Self::Capabilities),
            "teams" => Some(Self::Teams),
            "workers" => Some(Self::Workers),
            _ => None,
        }
    }
}

impl StaffSettingsState {
    pub(crate) fn from_config(config: StaffConfig) -> Self {
        let member_drafts =
            config.members.iter().map(StaffSettingsMemberDraft::from_member).collect::<Vec<_>>();
        let capability_drafts = config
            .members
            .iter()
            .map(StaffSettingsCapabilitiesDraft::from_member)
            .collect::<Vec<_>>();

        Self {
            loading: false,
            active_tab: StaffSettingsTab::Roster,
            selected_member_id: config.members.first().map(|member| member.id.clone()),
            config,
            member_drafts,
            capability_drafts,
            save_error: None,
            save_success: false,
            pending_config: None,
            workers: super::StaffWorkerResourcesState::default(),
            teams: super::StaffTeamResourcesState::default(),
        }
    }

    pub(crate) fn replace_config(&mut self, config: StaffConfig) {
        let replacement = Self::from_config(config);
        self.loading = replacement.loading;
        self.config = replacement.config;
        self.selected_member_id = replacement.selected_member_id;
        self.member_drafts = replacement.member_drafts;
        self.capability_drafts = replacement.capability_drafts;
        self.save_error = replacement.save_error;
        self.save_success = replacement.save_success;
        self.pending_config = replacement.pending_config;
    }

    pub(crate) fn has_unsaved_changes(&self) -> bool {
        match self.to_config() {
            Ok(config) => {
                comparable_staff_config(config) != comparable_staff_config(self.config.clone())
            }
            Err(_) => true,
        }
    }

    pub(crate) fn to_config(&self) -> Result<StaffConfig, String> {
        let mut members = Vec::with_capacity(self.member_drafts.len());
        for draft in &self.member_drafts {
            if staff_member_needs_agent_binding(draft) {
                return Err(format!(
                    "enabled staff member `{}` must bind a delegate agent or ACP agent",
                    draft.id.trim()
                ));
            }

            let capabilities = self
                .capability_drafts
                .iter()
                .find(|candidate| candidate.member_id == draft.id)
                .cloned()
                .unwrap_or_else(|| StaffSettingsCapabilitiesDraft::for_member(&draft.id))
                .to_config()?;

            members.push(StaffMemberConfig {
                id: draft.id.trim().to_string(),
                name: draft.name.trim().to_string(),
                role: draft.role.trim().to_string(),
                level: draft.level.trim().to_string(),
                status: draft.status.trim().to_string(),
                enabled: draft.enabled,
                capacity: parse_u32_input(&draft.capacity_input, "staff capacity")?,
                delegate_agent: optional_trimmed(&draft.delegate_agent),
                acp_agent: optional_trimmed(&draft.acp_agent),
                ipc_agent_id: optional_trimmed(&draft.ipc_agent_id),
                channel_bindings: draft
                    .channel_bindings
                    .iter()
                    .map(StaffSettingsChannelBindingDraft::to_config)
                    .collect(),
                capabilities,
            });
        }

        StaffConfig {
            members,
            workflow_stages: self.config.workflow_stages.clone(),
            workflow_orchestrations: self.config.workflow_orchestrations.clone(),
            project_state: self.config.project_state.clone(),
        }
        .normalize()
    }
}

fn comparable_staff_config(config: StaffConfig) -> StaffConfig {
    let mut config = config.clone().normalize().unwrap_or(config);
    for member in &mut config.members {
        member.capabilities.allowed_tools =
            normalized_capability_allowlist_for_save(&member.capabilities.allowed_tools);
        member.capabilities.allowed_skills =
            normalized_capability_allowlist_for_save(&member.capabilities.allowed_skills);
    }
    config
}

impl Default for StaffSettingsState {
    fn default() -> Self {
        Self::from_config(StaffConfig::default())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StaffSettingsMemberDraft {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) role: String,
    pub(crate) level: String,
    pub(crate) status: String,
    pub(crate) enabled: bool,
    pub(crate) capacity_input: String,
    pub(crate) delegate_agent: Option<String>,
    pub(crate) acp_agent: Option<String>,
    pub(crate) ipc_agent_id: Option<String>,
    pub(crate) channel_bindings: Vec<StaffSettingsChannelBindingDraft>,
}

impl StaffSettingsMemberDraft {
    pub(crate) fn from_member(member: &StaffMemberConfig) -> Self {
        Self {
            id: member.id.clone(),
            name: member.name.clone(),
            role: member.role.clone(),
            level: member.level.clone(),
            status: member.status.clone(),
            enabled: member.enabled,
            capacity_input: member.capacity.to_string(),
            delegate_agent: member.delegate_agent.clone(),
            acp_agent: member.acp_agent.clone(),
            ipc_agent_id: member.ipc_agent_id.clone(),
            channel_bindings: member
                .channel_bindings
                .iter()
                .map(StaffSettingsChannelBindingDraft::from_config)
                .collect(),
        }
    }

    pub(crate) fn new(member_id: String) -> Self {
        Self {
            id: member_id.clone(),
            name: member_id,
            role: "member".to_string(),
            level: "member".to_string(),
            status: "online".to_string(),
            enabled: true,
            capacity_input: "1".to_string(),
            delegate_agent: Some("main".to_string()),
            acp_agent: None,
            ipc_agent_id: None,
            channel_bindings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StaffSettingsChannelBindingDraft {
    pub(crate) channel_type: String,
    pub(crate) instance_id: String,
}

impl StaffSettingsChannelBindingDraft {
    pub(crate) fn new() -> Self {
        Self { channel_type: "matrix".to_string(), instance_id: "default".to_string() }
    }

    pub(crate) fn from_config(binding: &StaffChannelBindingConfig) -> Self {
        Self {
            channel_type: binding.channel_type.clone(),
            instance_id: binding.instance_id.clone(),
        }
    }

    fn to_config(&self) -> StaffChannelBindingConfig {
        StaffChannelBindingConfig {
            channel_type: self.channel_type.trim().to_string(),
            instance_id: self.instance_id.trim().to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StaffSettingsCapabilitiesDraft {
    pub(crate) member_id: String,
    pub(crate) agentic: bool,
    pub(crate) allowed_tools: Vec<String>,
    pub(crate) allowed_skills: Vec<String>,
    pub(crate) max_iterations_input: String,
    pub(crate) max_parallel_tasks_input: String,
    pub(crate) max_active_minutes_input: String,
}

impl StaffSettingsCapabilitiesDraft {
    pub(crate) fn from_member(member: &StaffMemberConfig) -> Self {
        Self::from_config(&member.id, &member.capabilities)
    }

    pub(crate) fn for_member(member_id: &str) -> Self {
        Self::from_config(member_id, &StaffCapabilitiesConfig::default())
    }

    fn from_config(member_id: &str, capabilities: &StaffCapabilitiesConfig) -> Self {
        Self {
            member_id: member_id.to_string(),
            agentic: capabilities.agentic,
            allowed_tools: capabilities.allowed_tools.clone(),
            allowed_skills: capabilities.allowed_skills.clone(),
            max_iterations_input: capabilities.max_iterations.to_string(),
            max_parallel_tasks_input: capabilities
                .max_parallel_tasks
                .map(|value| value.to_string())
                .unwrap_or_default(),
            max_active_minutes_input: capabilities
                .max_active_minutes
                .map(|value| value.to_string())
                .unwrap_or_default(),
        }
    }

    fn to_config(&self) -> Result<StaffCapabilitiesConfig, String> {
        let mut config = StaffCapabilitiesConfig {
            agentic: self.agentic,
            allowed_tools: normalized_capability_allowlist_for_save(&self.allowed_tools),
            allowed_skills: normalized_capability_allowlist_for_save(&self.allowed_skills),
            max_iterations: parse_usize_input(&self.max_iterations_input, "staff max iterations")?,
            max_parallel_tasks: parse_optional_u32_input(
                &self.max_parallel_tasks_input,
                "staff max parallel tasks",
            )?,
            max_active_minutes: parse_optional_u32_input(
                &self.max_active_minutes_input,
                "staff max active minutes",
            )?,
        };
        config.normalize()?;
        Ok(config)
    }
}

fn normalized_capability_allowlist_for_save(values: &[String]) -> Vec<String> {
    let mut values = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn optional_trimmed(value: &Option<String>) -> Option<String> {
    value.as_deref().map(str::trim).filter(|value| !value.is_empty()).map(str::to_string)
}

fn optional_has_value(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|value| !value.trim().is_empty())
}

fn parse_u32_input(value: &str, field: &str) -> Result<u32, String> {
    value.trim().parse::<u32>().map_err(|_| format!("{field} must be a positive integer"))
}

fn parse_usize_input(value: &str, field: &str) -> Result<usize, String> {
    value.trim().parse::<usize>().map_err(|_| format!("{field} must be a positive integer"))
}

fn parse_optional_u32_input(value: &str, field: &str) -> Result<Option<u32>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed.parse::<u32>().map(Some).map_err(|_| format!("{field} must be a positive integer"))
    }
}

#[cfg(test)]
#[path = "staff_tests.rs"]
mod staff_tests;
