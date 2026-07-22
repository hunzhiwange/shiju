use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

const DEFAULT_DELEGATE_AGENT: &str = "main";
const DEFAULT_MAX_ITERATIONS: usize = 10;
const STAFF_CHANNEL_BINDING_TYPES: &[&str] = &["matrix", "feishu", "dingtalk"];
const ADVANCE_STATUS_CAPABILITY_ID: &str = "advance_status";
const EXECUTE_TASK_CAPABILITY_ID: &str = "execute_task";
const CODE_REVIEW_CAPABILITY_ID: &str = "code_review";
const GIT_MERGE_CAPABILITY_ID: &str = "git_merge";
const SAVED_WORKFLOW_CAPABILITY_ID: &str = "saved_workflow";
const MANUAL_CAPABILITY_ID: &str = "manual";
const BUILT_IN_STAGE_CAPABILITY_IDS: &[&str] = &[
    ADVANCE_STATUS_CAPABILITY_ID,
    EXECUTE_TASK_CAPABILITY_ID,
    CODE_REVIEW_CAPABILITY_ID,
    GIT_MERGE_CAPABILITY_ID,
    SAVED_WORKFLOW_CAPABILITY_ID,
    MANUAL_CAPABILITY_ID,
];

/// Gateway-backed staff roster, workflow, and project assignment configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffConfig {
    /// Staff members available to the task board staff mode.
    #[serde(default = "default_staff_members")]
    pub members: Vec<StaffMemberConfig>,
    /// Stable string workflow stages used when projecting into desktop task statuses.
    #[serde(default = "default_workflow_stages")]
    pub workflow_stages: Vec<StaffWorkflowStageConfig>,
    /// System-level workflow orchestration templates keyed by workflow_id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub workflow_orchestrations: BTreeMap<String, StaffWorkflowOrchestrationConfig>,
    /// Project-scoped staff state persisted through the gateway config.
    #[serde(default)]
    pub project_state: StaffProjectStateConfig,
}

impl StaffConfig {
    /// Returns a normalized copy with defaults filled, whitespace removed, and lists deduplicated.
    pub fn normalize(mut self) -> Result<Self, String> {
        if self.members.is_empty() {
            self.members = default_staff_members();
        }
        if self.workflow_stages.is_empty() {
            self.workflow_stages = default_workflow_stages();
        }

        normalize_members(&mut self.members)?;
        normalize_workflow_stages(&mut self.workflow_stages)?;

        let member_ids =
            self.members.iter().map(|member| member.id.clone()).collect::<HashSet<_>>();
        let stage_ids =
            self.workflow_stages.iter().map(|stage| stage.id.clone()).collect::<HashSet<_>>();
        let status_keys = self
            .workflow_stages
            .iter()
            .map(|stage| stage.task_status.clone())
            .collect::<HashSet<_>>();
        normalize_workflow_orchestrations(&mut self.workflow_orchestrations, &member_ids)?;
        normalize_project_state(&mut self.project_state, &member_ids, &stage_ids, &status_keys)?;

        Ok(self)
    }

    /// Validates the config without mutating it.
    pub fn validate(&self) -> Result<(), String> {
        validate_members(&self.members)?;
        validate_workflow_stages(&self.workflow_stages)?;

        let member_ids =
            self.members.iter().map(|member| member.id.as_str()).collect::<HashSet<_>>();
        let stage_ids =
            self.workflow_stages.iter().map(|stage| stage.id.as_str()).collect::<HashSet<_>>();
        let status_keys = self
            .workflow_stages
            .iter()
            .map(|stage| stage.task_status.as_str())
            .collect::<HashSet<_>>();
        validate_workflow_orchestrations(&self.workflow_orchestrations, &member_ids)?;
        validate_project_state(&self.project_state, &member_ids, &stage_ids, &status_keys)
    }
}

impl Default for StaffConfig {
    fn default() -> Self {
        Self {
            members: default_staff_members(),
            workflow_stages: default_workflow_stages(),
            workflow_orchestrations: BTreeMap::new(),
            project_state: StaffProjectStateConfig::default(),
        }
    }
}

/// One configured staff member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffMemberConfig {
    /// Stable staff member id.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Stable role key such as `orchestrator`, `planner`, or `executor`.
    pub role: String,
    /// Stable level key such as `principal`, `manager`, `executor`, or `member`.
    pub level: String,
    /// Stable status key. Desktop maps this to its local staff status type.
    #[serde(default = "default_staff_status")]
    pub status: String,
    /// Whether this member can receive staff-mode work.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Maximum concurrent task capacity for this member.
    #[serde(default = "default_capacity")]
    pub capacity: u32,
    /// Bound delegate agent key, usually `main` by default.
    #[serde(default)]
    pub delegate_agent: Option<String>,
    /// Optional bound ACP agent key.
    #[serde(default)]
    pub acp_agent: Option<String>,
    /// Optional IPC agent id used as the staff message target.
    #[serde(default)]
    pub ipc_agent_id: Option<String>,
    /// External channel instance bindings keyed by channel type.
    #[serde(default)]
    pub channel_bindings: Vec<StaffChannelBindingConfig>,
    /// Staff-owned capability and runtime limits.
    #[serde(default)]
    pub capabilities: StaffCapabilitiesConfig,
}

impl StaffMemberConfig {
    pub fn new(id: &str, name: &str, role: &str, level: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            role: role.to_string(),
            level: level.to_string(),
            status: default_staff_status(),
            enabled: default_enabled(),
            capacity: default_capacity(),
            delegate_agent: Some(DEFAULT_DELEGATE_AGENT.to_string()),
            acp_agent: None,
            ipc_agent_id: None,
            channel_bindings: Vec::new(),
            capabilities: StaffCapabilitiesConfig::default(),
        }
    }
}

/// Binding from one staff member to one named external channel instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffChannelBindingConfig {
    /// Supported channel type such as `matrix`, `feishu`, or `dingtalk`.
    pub channel_type: String,
    /// Stable channel instance id.
    pub instance_id: String,
}

/// Staff-owned tool, skill, and runtime capability configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffCapabilitiesConfig {
    /// Whether this staff member can run an agentic tool loop.
    #[serde(default)]
    pub agentic: bool,
    /// Staff-specific tool allowlist.
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Staff-specific skill allowlist.
    #[serde(default)]
    pub allowed_skills: Vec<String>,
    /// Maximum agentic tool iterations for this staff member.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// Optional cap for simultaneously active tasks.
    #[serde(default)]
    pub max_parallel_tasks: Option<u32>,
    /// Optional cap for active work duration.
    #[serde(default)]
    pub max_active_minutes: Option<u32>,
}

impl StaffCapabilitiesConfig {
    pub fn normalize(&mut self) -> Result<(), String> {
        normalize_string_list(&mut self.allowed_tools);
        normalize_string_list(&mut self.allowed_skills);
        validate_capabilities(self)
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_capabilities(self)
    }
}

impl Default for StaffCapabilitiesConfig {
    fn default() -> Self {
        Self {
            agentic: false,
            allowed_tools: Vec::new(),
            allowed_skills: Vec::new(),
            max_iterations: default_max_iterations(),
            max_parallel_tasks: None,
            max_active_minutes: None,
        }
    }
}

/// Shared workflow stage definition using stable task status strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffWorkflowStageConfig {
    /// Stable stage id.
    pub id: String,
    /// Human-readable stage name.
    pub name: String,
    /// Stable desktop task status key such as `running` or `code_review`.
    pub task_status: String,
    /// Optional staff role key expected to own this stage.
    #[serde(default)]
    pub role: Option<String>,
    /// Whether this lifecycle stage can participate in staff orchestration.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Display-only stages cannot own work.
    #[serde(default)]
    pub display_only: bool,
}

impl StaffWorkflowStageConfig {
    pub fn new(id: &str, name: &str, task_status: &str, role: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            task_status: task_status.to_string(),
            role: Some(role.to_string()),
            enabled: true,
            display_only: false,
        }
    }

    pub fn display_only(id: &str, name: &str, task_status: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            task_status: task_status.to_string(),
            role: None,
            enabled: true,
            display_only: true,
        }
    }
}

/// System-level orchestration template for one workflow_id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffWorkflowOrchestrationConfig {
    /// Workflow-scoped custom capability catalog entries.
    #[serde(default)]
    pub capabilities: Vec<StaffCapabilityConfig>,
    /// Stage ownership assignments for tasks using this workflow.
    #[serde(default)]
    pub stage_assignments: Vec<StaffStageAssignmentConfig>,
    /// Workflow UUID bindings for stages in this workflow.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stage_workflow_bindings: Vec<StaffStageWorkflowBindingConfig>,
    /// Stage execution actions for this workflow.
    #[serde(default)]
    pub stage_actions: Vec<StaffStageActionConfig>,
}

/// Project-scoped staff state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffProjectStateConfig {
    /// Project-scoped custom capability catalog entries.
    #[serde(default)]
    pub capabilities: Vec<StaffCapabilityConfig>,
    /// Stage ownership assignments for the active project.
    #[serde(default)]
    pub stage_assignments: Vec<StaffStageAssignmentConfig>,
    /// Workflow UUID bindings for stages in the active project.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stage_workflow_bindings: Vec<StaffStageWorkflowBindingConfig>,
    /// Stage execution actions for the active project.
    #[serde(default)]
    pub stage_actions: Vec<StaffStageActionConfig>,
    /// Current task ownership assignments for the active project.
    #[serde(default)]
    pub member_task_assignments: Vec<StaffMemberTaskAssignmentConfig>,
}

/// Project capability catalog entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffCapabilityConfig {
    /// Stable capability id referenced by stage actions.
    pub id: String,
    /// Human-readable capability label.
    pub label: String,
    /// Capability execution family.
    #[serde(rename = "type")]
    pub capability_type: StaffCapabilityKindConfig,
    /// Whether this entry describes a built-in capability.
    #[serde(default)]
    pub built_in: bool,
    /// Inputs expected by this capability.
    #[serde(default)]
    pub input_summary: StaffCapabilityIoSummaryConfig,
    /// Results produced by this capability.
    #[serde(default)]
    pub result_summary: StaffCapabilityIoSummaryConfig,
}

/// Capability execution family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StaffCapabilityKindConfig {
    BuiltIn,
    SavedWorkflow,
}

/// Human-readable IO summary for a capability catalog entry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffCapabilityIoSummaryConfig {
    /// Short description of the input or result contract.
    #[serde(default)]
    pub description: String,
    /// Named fields or result keys in the contract.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
}

/// Assignment of a workflow stage to one or more staff members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffStageAssignmentConfig {
    /// Workflow stage id.
    pub stage_id: String,
    /// Primary staff member id.
    #[serde(default)]
    pub primary_member_id: Option<String>,
    /// Backup staff member ids.
    #[serde(default)]
    pub backup_member_ids: Vec<String>,
    /// Manager staff member ids.
    #[serde(default)]
    pub manager_member_ids: Vec<String>,
    /// Whether this stage can be scheduled autonomously.
    #[serde(default)]
    pub autonomous: bool,
}

/// Binding of a workflow stage to one persisted workflow UUID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffStageWorkflowBindingConfig {
    /// Workflow stage id.
    pub stage_id: String,
    /// Persisted workflow UUID for this stage.
    pub workflow_uuid: String,
}

/// Configured action for one workflow stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffStageActionConfig {
    /// Workflow stage id.
    pub stage_id: String,
    /// Stable stage capability id.
    pub capability_id: String,
    /// Persisted workflow UUID, required for `saved_workflow`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_uuid: Option<String>,
    /// Structured stage action inputs.
    #[serde(default, skip_serializing_if = "StaffStageActionInputsConfig::is_empty")]
    pub inputs: StaffStageActionInputsConfig,
}

/// Structured inputs for a stage action binding.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffStageActionInputsConfig {
    /// Saved workflow UUID for `saved_workflow` capabilities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_uuid: Option<String>,
    /// Generic capability parameters.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, Value>,
}

impl StaffStageActionInputsConfig {
    pub fn is_empty(&self) -> bool {
        self.workflow_uuid.is_none() && self.parameters.is_empty()
    }
}

/// Assignment of active task ids to a staff member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffMemberTaskAssignmentConfig {
    /// Staff member id.
    pub member_id: String,
    /// Task ids currently owned by this staff member.
    #[serde(default)]
    pub task_ids: Vec<String>,
}

fn default_staff_members() -> Vec<StaffMemberConfig> {
    vec![
        StaffMemberConfig::new("staff-orchestrator", "Orchestrator", "orchestrator", "principal"),
        StaffMemberConfig::new("staff-planner", "Planner", "planner", "manager"),
        StaffMemberConfig::new("staff-executor", "Executor", "executor", "executor"),
        StaffMemberConfig::new("staff-reviewer", "Reviewer", "reviewer", "manager"),
        StaffMemberConfig::new("staff-merger", "Merger", "merger", "manager"),
    ]
}

fn default_workflow_stages() -> Vec<StaffWorkflowStageConfig> {
    vec![
        StaffWorkflowStageConfig::new("pool", "Pool", "pool", "orchestrator"),
        StaffWorkflowStageConfig::new("pending", "Pending", "pending", "orchestrator"),
        StaffWorkflowStageConfig::new("planning", "Planning", "planning", "planner"),
        StaffWorkflowStageConfig::new("running", "Running", "running", "executor"),
        StaffWorkflowStageConfig::new("failed", "Failed", "failed", "orchestrator"),
        StaffWorkflowStageConfig::new("paused", "Paused", "paused", "orchestrator"),
        StaffWorkflowStageConfig::new(
            "code_complete",
            "Code Complete",
            "code_complete",
            "reviewer",
        ),
        StaffWorkflowStageConfig::new("code_review", "Code Review", "code_review", "reviewer"),
        StaffWorkflowStageConfig::new("pr_submitted", "PR Submitted", "pr_submitted", "merger"),
        StaffWorkflowStageConfig::display_only("completed", "Completed", "completed"),
        StaffWorkflowStageConfig::display_only("archived", "Archived", "archived"),
    ]
}

fn default_staff_status() -> String {
    "online".to_string()
}

fn default_enabled() -> bool {
    true
}

fn default_capacity() -> u32 {
    1
}

fn default_max_iterations() -> usize {
    DEFAULT_MAX_ITERATIONS
}

fn normalize_members(members: &mut [StaffMemberConfig]) -> Result<(), String> {
    let mut seen_ids = HashSet::new();
    for member in members {
        member.id = member.id.trim().to_string();
        if member.id.is_empty() {
            return Err("staff member id cannot be empty".to_string());
        }
        if !seen_ids.insert(member.id.clone()) {
            return Err(format!("duplicate staff member id: {}", member.id));
        }

        member.name = member.name.trim().to_string();
        if member.name.is_empty() {
            member.name = member.id.clone();
        }
        member.role = trim_or_default(&member.role, "member");
        member.level = trim_or_default(&member.level, "member");
        member.status = trim_or_default(&member.status, "online");
        member.delegate_agent = normalize_optional_string(member.delegate_agent.as_deref());
        member.acp_agent = normalize_optional_string(member.acp_agent.as_deref());
        member.ipc_agent_id = normalize_optional_string(member.ipc_agent_id.as_deref());
        normalize_channel_bindings(&mut member.channel_bindings)?;
        member.capabilities.normalize()?;

        validate_member(member)?;
    }
    Ok(())
}

fn validate_members(members: &[StaffMemberConfig]) -> Result<(), String> {
    if members.is_empty() {
        return Err("staff config must contain at least one member".to_string());
    }

    let mut seen_ids = HashSet::new();
    for member in members {
        if member.id.trim().is_empty() {
            return Err("staff member id cannot be empty".to_string());
        }
        if !seen_ids.insert(member.id.as_str()) {
            return Err(format!("duplicate staff member id: {}", member.id));
        }
        validate_member(member)?;
        validate_string_list(&member.capabilities.allowed_tools, "staff allowed tool")?;
        validate_string_list(&member.capabilities.allowed_skills, "staff allowed skill")?;
    }
    Ok(())
}

fn validate_member(member: &StaffMemberConfig) -> Result<(), String> {
    if member.enabled && member.capacity == 0 {
        return Err(format!("staff member {} capacity cannot be 0", member.id));
    }
    let has_delegate_agent =
        member.delegate_agent.as_deref().is_some_and(|value| !value.trim().is_empty());
    let has_acp_agent = member.acp_agent.as_deref().is_some_and(|value| !value.trim().is_empty());
    if member.enabled && !has_delegate_agent && !has_acp_agent {
        return Err(format!(
            "enabled staff member {} must bind a delegate_agent or acp_agent",
            member.id
        ));
    }
    validate_channel_bindings(&member.channel_bindings)?;
    member.capabilities.validate()
}

fn normalize_workflow_stages(stages: &mut [StaffWorkflowStageConfig]) -> Result<(), String> {
    let mut seen_ids = HashSet::new();
    let mut seen_statuses = HashSet::new();

    for stage in stages {
        stage.id = stage.id.trim().to_string();
        if stage.id.is_empty() {
            return Err("staff workflow stage id cannot be empty".to_string());
        }
        if !seen_ids.insert(stage.id.clone()) {
            return Err(format!("duplicate staff workflow stage id: {}", stage.id));
        }
        stage.name = stage.name.trim().to_string();
        if stage.name.is_empty() {
            stage.name = stage.id.clone();
        }
        stage.task_status = stage.task_status.trim().to_string();
        if stage.task_status.is_empty() {
            return Err(format!("staff workflow stage {} task_status cannot be empty", stage.id));
        }
        if !seen_statuses.insert(stage.task_status.clone()) {
            return Err(format!("duplicate staff workflow task_status: {}", stage.task_status));
        }
        stage.role = normalize_optional_string(stage.role.as_deref());
        if !stage.enabled {
            stage.role = None;
            stage.display_only = true;
        }
        validate_workflow_stage(stage)?;
    }

    Ok(())
}

fn validate_workflow_stages(stages: &[StaffWorkflowStageConfig]) -> Result<(), String> {
    if stages.is_empty() {
        return Err("staff config must contain at least one workflow stage".to_string());
    }

    let mut seen_ids = HashSet::new();
    let mut seen_statuses = HashSet::new();
    for stage in stages {
        if stage.id.trim().is_empty() {
            return Err("staff workflow stage id cannot be empty".to_string());
        }
        if !seen_ids.insert(stage.id.as_str()) {
            return Err(format!("duplicate staff workflow stage id: {}", stage.id));
        }
        if stage.task_status.trim().is_empty() {
            return Err(format!("staff workflow stage {} task_status cannot be empty", stage.id));
        }
        if !seen_statuses.insert(stage.task_status.as_str()) {
            return Err(format!("duplicate staff workflow task_status: {}", stage.task_status));
        }
        validate_workflow_stage(stage)?;
    }
    Ok(())
}

fn validate_workflow_stage(stage: &StaffWorkflowStageConfig) -> Result<(), String> {
    let has_role = stage.role.as_deref().is_some_and(|role| !role.trim().is_empty());
    if !stage.enabled {
        if has_role {
            return Err(format!("disabled staff workflow stage cannot own role: {}", stage.id));
        }
        return Ok(());
    }
    if stage.display_only && has_role {
        return Err(format!("display-only staff workflow stage cannot own role: {}", stage.id));
    }
    if !stage.display_only && !has_role {
        return Err(format!("staff workflow stage requires a role: {}", stage.id));
    }
    Ok(())
}

fn normalize_project_state(
    project_state: &mut StaffProjectStateConfig,
    member_ids: &HashSet<String>,
    stage_ids: &HashSet<String>,
    status_keys: &HashSet<String>,
) -> Result<(), String> {
    normalize_capability_catalog(&mut project_state.capabilities)?;
    normalize_stage_assignments(&mut project_state.stage_assignments, member_ids, stage_ids)?;
    project_state.stage_workflow_bindings.clear();
    normalize_stage_actions(
        &mut project_state.stage_actions,
        stage_ids,
        status_keys,
        &project_state.capabilities,
    )?;
    normalize_member_task_assignments(&mut project_state.member_task_assignments, member_ids)?;
    Ok(())
}

fn normalize_workflow_orchestrations(
    orchestrations: &mut BTreeMap<String, StaffWorkflowOrchestrationConfig>,
    member_ids: &HashSet<String>,
) -> Result<(), String> {
    let original = std::mem::take(orchestrations);
    for (workflow_id, mut orchestration) in original {
        let workflow_id = workflow_id.trim().to_string();
        if workflow_id.is_empty() {
            return Err("staff workflow orchestration workflow_id cannot be empty".to_string());
        }
        if orchestrations.contains_key(&workflow_id) {
            return Err(format!(
                "duplicate staff workflow orchestration workflow_id: {workflow_id}"
            ));
        }

        normalize_workflow_orchestration(&mut orchestration, member_ids)?;
        orchestrations.insert(workflow_id, orchestration);
    }
    Ok(())
}

fn normalize_workflow_orchestration(
    orchestration: &mut StaffWorkflowOrchestrationConfig,
    member_ids: &HashSet<String>,
) -> Result<(), String> {
    normalize_capability_catalog(&mut orchestration.capabilities)?;
    normalize_workflow_stage_assignments(&mut orchestration.stage_assignments, member_ids)?;
    orchestration.stage_workflow_bindings.clear();
    normalize_workflow_stage_actions(&mut orchestration.stage_actions, &orchestration.capabilities)
}

fn validate_project_state(
    project_state: &StaffProjectStateConfig,
    member_ids: &HashSet<&str>,
    stage_ids: &HashSet<&str>,
    status_keys: &HashSet<&str>,
) -> Result<(), String> {
    validate_capability_catalog(&project_state.capabilities)?;
    validate_stage_assignments(&project_state.stage_assignments, member_ids, stage_ids)?;
    validate_stage_actions(
        &project_state.stage_actions,
        stage_ids,
        status_keys,
        &project_state.capabilities,
    )?;
    validate_member_task_assignments(&project_state.member_task_assignments, member_ids)?;
    Ok(())
}

fn validate_workflow_orchestrations(
    orchestrations: &BTreeMap<String, StaffWorkflowOrchestrationConfig>,
    member_ids: &HashSet<&str>,
) -> Result<(), String> {
    for (workflow_id, orchestration) in orchestrations {
        let workflow_id = workflow_id.trim();
        if workflow_id.is_empty() {
            return Err("staff workflow orchestration workflow_id cannot be empty".to_string());
        }
        validate_workflow_orchestration(orchestration, member_ids)?;
    }
    Ok(())
}

fn validate_workflow_orchestration(
    orchestration: &StaffWorkflowOrchestrationConfig,
    member_ids: &HashSet<&str>,
) -> Result<(), String> {
    validate_capability_catalog(&orchestration.capabilities)?;
    validate_workflow_stage_assignments(&orchestration.stage_assignments, member_ids)?;
    validate_workflow_stage_actions(&orchestration.stage_actions, &orchestration.capabilities)
}

fn normalize_stage_assignments(
    assignments: &mut [StaffStageAssignmentConfig],
    member_ids: &HashSet<String>,
    stage_ids: &HashSet<String>,
) -> Result<(), String> {
    let mut seen_stages = HashSet::new();
    for assignment in assignments {
        assignment.stage_id = assignment.stage_id.trim().to_string();
        if assignment.stage_id.is_empty() {
            return Err("staff stage assignment stage_id cannot be empty".to_string());
        }
        if !stage_ids.contains(&assignment.stage_id) {
            return Err(format!(
                "unknown staff stage assignment stage_id: {}",
                assignment.stage_id
            ));
        }
        if !seen_stages.insert(assignment.stage_id.clone()) {
            return Err(format!(
                "duplicate staff stage assignment stage_id: {}",
                assignment.stage_id
            ));
        }

        assignment.primary_member_id =
            normalize_optional_string(assignment.primary_member_id.as_deref());
        normalize_string_list(&mut assignment.backup_member_ids);
        normalize_string_list(&mut assignment.manager_member_ids);
        validate_stage_assignment(assignment, member_ids)?;
    }
    Ok(())
}

fn validate_stage_assignments(
    assignments: &[StaffStageAssignmentConfig],
    member_ids: &HashSet<&str>,
    stage_ids: &HashSet<&str>,
) -> Result<(), String> {
    let mut seen_stages = HashSet::new();
    let member_ids = member_ids.iter().copied().collect::<HashSet<_>>();
    for assignment in assignments {
        if assignment.stage_id.trim().is_empty() {
            return Err("staff stage assignment stage_id cannot be empty".to_string());
        }
        if !stage_ids.contains(assignment.stage_id.as_str()) {
            return Err(format!(
                "unknown staff stage assignment stage_id: {}",
                assignment.stage_id
            ));
        }
        if !seen_stages.insert(assignment.stage_id.as_str()) {
            return Err(format!(
                "duplicate staff stage assignment stage_id: {}",
                assignment.stage_id
            ));
        }

        validate_stage_assignment_readonly(assignment, &member_ids)?;
    }
    Ok(())
}

fn normalize_workflow_stage_assignments(
    assignments: &mut [StaffStageAssignmentConfig],
    member_ids: &HashSet<String>,
) -> Result<(), String> {
    let mut seen_stages = HashSet::new();
    for assignment in assignments {
        assignment.stage_id = assignment.stage_id.trim().to_string();
        if assignment.stage_id.is_empty() {
            return Err("staff stage assignment stage_id cannot be empty".to_string());
        }
        if !seen_stages.insert(assignment.stage_id.clone()) {
            return Err(format!(
                "duplicate staff stage assignment stage_id: {}",
                assignment.stage_id
            ));
        }

        assignment.primary_member_id =
            normalize_optional_string(assignment.primary_member_id.as_deref());
        normalize_string_list(&mut assignment.backup_member_ids);
        normalize_string_list(&mut assignment.manager_member_ids);
        validate_stage_assignment(assignment, member_ids)?;
    }
    Ok(())
}

fn validate_workflow_stage_assignments(
    assignments: &[StaffStageAssignmentConfig],
    member_ids: &HashSet<&str>,
) -> Result<(), String> {
    let mut seen_stages = HashSet::new();
    let member_ids = member_ids.iter().copied().collect::<HashSet<_>>();
    for assignment in assignments {
        let stage_id = assignment.stage_id.trim();
        if stage_id.is_empty() {
            return Err("staff stage assignment stage_id cannot be empty".to_string());
        }
        if !seen_stages.insert(stage_id) {
            return Err(format!(
                "duplicate staff stage assignment stage_id: {}",
                assignment.stage_id
            ));
        }

        validate_stage_assignment_readonly(assignment, &member_ids)?;
    }
    Ok(())
}

fn normalize_capability_catalog(capabilities: &mut [StaffCapabilityConfig]) -> Result<(), String> {
    let mut seen_ids = HashSet::new();
    for capability in capabilities {
        capability.id = capability.id.trim().to_string();
        if capability.id.is_empty() {
            return Err("staff capability id cannot be empty".to_string());
        }
        if !seen_ids.insert(capability.id.clone()) {
            return Err(format!("duplicate staff capability id: {}", capability.id));
        }
        if is_built_in_stage_capability_id(&capability.id) && !capability.built_in {
            return Err(format!(
                "staff capability id cannot override built-in capability: {}",
                capability.id
            ));
        }
        if capability.built_in && !is_built_in_stage_capability_id(&capability.id) {
            return Err(format!(
                "staff built-in capability id is not registered: {}",
                capability.id
            ));
        }
        capability.label = capability.label.trim().to_string();
        if capability.label.is_empty() {
            capability.label = capability.id.clone();
        }
        normalize_capability_io_summary(&mut capability.input_summary)?;
        normalize_capability_io_summary(&mut capability.result_summary)?;
    }
    Ok(())
}

fn validate_capability_catalog(capabilities: &[StaffCapabilityConfig]) -> Result<(), String> {
    let mut seen_ids = HashSet::new();
    for capability in capabilities {
        let id = capability.id.trim();
        if id.is_empty() {
            return Err("staff capability id cannot be empty".to_string());
        }
        if !seen_ids.insert(id) {
            return Err(format!("duplicate staff capability id: {}", capability.id));
        }
        if is_built_in_stage_capability_id(id) && !capability.built_in {
            return Err(format!("staff capability id cannot override built-in capability: {id}"));
        }
        if capability.built_in && !is_built_in_stage_capability_id(id) {
            return Err(format!("staff built-in capability id is not registered: {id}"));
        }
        if capability.label.trim().is_empty() {
            return Err(format!("staff capability label for {id} cannot be empty"));
        }
        validate_capability_io_summary(&capability.input_summary, "input_summary", id)?;
        validate_capability_io_summary(&capability.result_summary, "result_summary", id)?;
    }
    Ok(())
}

fn normalize_capability_io_summary(
    summary: &mut StaffCapabilityIoSummaryConfig,
) -> Result<(), String> {
    summary.description = summary.description.trim().to_string();
    normalize_string_list(&mut summary.fields);
    Ok(())
}

fn validate_capability_io_summary(
    summary: &StaffCapabilityIoSummaryConfig,
    field: &str,
    capability_id: &str,
) -> Result<(), String> {
    validate_string_list(
        &summary.fields,
        &format!("staff capability {field} field for {capability_id}"),
    )
}

fn normalize_stage_actions(
    actions: &mut [StaffStageActionConfig],
    stage_ids: &HashSet<String>,
    _status_keys: &HashSet<String>,
    capabilities: &[StaffCapabilityConfig],
) -> Result<(), String> {
    let mut seen_stages = HashSet::new();
    for action in actions {
        action.stage_id = action.stage_id.trim().to_string();
        if action.stage_id.is_empty() {
            return Err("staff stage action stage_id cannot be empty".to_string());
        }
        if !stage_ids.contains(&action.stage_id) {
            return Err(format!("unknown staff stage action stage_id: {}", action.stage_id));
        }
        if !seen_stages.insert(action.stage_id.clone()) {
            return Err(format!("duplicate staff stage action stage_id: {}", action.stage_id));
        }

        action.capability_id = action.capability_id.trim().to_string();
        action.workflow_uuid = normalize_optional_string(action.workflow_uuid.as_deref());
        normalize_stage_action_inputs(action)?;
        validate_stage_action(action, &action.stage_id, capabilities)?;
    }
    Ok(())
}

fn validate_stage_actions(
    actions: &[StaffStageActionConfig],
    stage_ids: &HashSet<&str>,
    _status_keys: &HashSet<&str>,
    capabilities: &[StaffCapabilityConfig],
) -> Result<(), String> {
    let mut seen_stages = HashSet::new();
    for action in actions {
        let stage_id = action.stage_id.trim();
        if stage_id.is_empty() {
            return Err("staff stage action stage_id cannot be empty".to_string());
        }
        if !stage_ids.contains(stage_id) {
            return Err(format!("unknown staff stage action stage_id: {}", action.stage_id));
        }
        if !seen_stages.insert(stage_id) {
            return Err(format!("duplicate staff stage action stage_id: {stage_id}"));
        }
        validate_stage_action_readonly(action, stage_id, capabilities)?;
    }
    Ok(())
}

fn normalize_workflow_stage_actions(
    actions: &mut [StaffStageActionConfig],
    capabilities: &[StaffCapabilityConfig],
) -> Result<(), String> {
    let mut seen_stages = HashSet::new();
    for action in actions {
        action.stage_id = action.stage_id.trim().to_string();
        if action.stage_id.is_empty() {
            return Err("staff stage action stage_id cannot be empty".to_string());
        }
        if !seen_stages.insert(action.stage_id.clone()) {
            return Err(format!("duplicate staff stage action stage_id: {}", action.stage_id));
        }

        action.capability_id = action.capability_id.trim().to_string();
        action.workflow_uuid = normalize_optional_string(action.workflow_uuid.as_deref());
        normalize_stage_action_inputs(action)?;
        validate_stage_action(action, &action.stage_id, capabilities)?;
    }
    Ok(())
}

fn validate_workflow_stage_actions(
    actions: &[StaffStageActionConfig],
    capabilities: &[StaffCapabilityConfig],
) -> Result<(), String> {
    let mut seen_stages = HashSet::new();
    for action in actions {
        let stage_id = action.stage_id.trim();
        if stage_id.is_empty() {
            return Err("staff stage action stage_id cannot be empty".to_string());
        }
        if !seen_stages.insert(stage_id) {
            return Err(format!("duplicate staff stage action stage_id: {stage_id}"));
        }
        validate_stage_action_readonly(action, stage_id, capabilities)?;
    }
    Ok(())
}

fn normalize_stage_action_inputs(action: &mut StaffStageActionConfig) -> Result<(), String> {
    action.inputs.workflow_uuid = normalize_optional_string(action.inputs.workflow_uuid.as_deref());
    normalize_value_map(
        &mut action.inputs.parameters,
        &format!("staff stage action input parameter for {}", action.stage_id),
    )?;
    if let Some(workflow_uuid) = action.workflow_uuid.clone() {
        match action.inputs.workflow_uuid.as_deref() {
            Some(input_workflow_uuid) if input_workflow_uuid != workflow_uuid => {
                return Err(format!(
                    "staff stage action workflow_uuid for {} conflicts with inputs.workflow_uuid",
                    action.stage_id
                ));
            }
            Some(_) => {}
            None => action.inputs.workflow_uuid = Some(workflow_uuid),
        }
    }
    Ok(())
}

fn validate_stage_action(
    action: &StaffStageActionConfig,
    stage_id: &str,
    capabilities: &[StaffCapabilityConfig],
) -> Result<(), String> {
    validate_stage_action_readonly(action, stage_id, capabilities)
}

fn validate_stage_action_readonly(
    action: &StaffStageActionConfig,
    stage_id: &str,
    capabilities: &[StaffCapabilityConfig],
) -> Result<(), String> {
    let capability_id = action.capability_id.trim();
    if capability_id.is_empty() {
        return Err(format!("staff stage action capability_id for {stage_id} cannot be empty"));
    }
    if !stage_action_capability_exists(capability_id, capabilities) {
        return Err(format!(
            "unknown staff stage action capability_id for {stage_id}: {capability_id}"
        ));
    }
    if action.workflow_uuid.as_deref().is_some_and(|value| value.trim().is_empty()) {
        return Err(format!(
            "staff stage action workflow_uuid for {stage_id} cannot be empty when provided"
        ));
    }
    validate_stage_action_inputs_readonly(&action.inputs, stage_id)?;
    Ok(())
}

fn validate_stage_action_inputs_readonly(
    inputs: &StaffStageActionInputsConfig,
    stage_id: &str,
) -> Result<(), String> {
    if inputs.workflow_uuid.as_deref().is_some_and(|value| value.trim().is_empty()) {
        return Err(format!(
            "staff stage action inputs.workflow_uuid for {stage_id} cannot be empty when provided"
        ));
    }
    validate_value_map(
        &inputs.parameters,
        &format!("staff stage action input parameter for {stage_id}"),
    )
}

fn stage_action_capability_exists(
    capability_id: &str,
    capabilities: &[StaffCapabilityConfig],
) -> bool {
    is_built_in_stage_capability_id(capability_id)
        || capabilities.iter().any(|capability| capability.id.trim() == capability_id)
}

fn normalize_member_task_assignments(
    assignments: &mut Vec<StaffMemberTaskAssignmentConfig>,
    member_ids: &HashSet<String>,
) -> Result<(), String> {
    let mut seen_members = HashSet::new();
    let mut seen_task_ids = HashSet::new();
    for assignment in assignments.iter_mut() {
        assignment.member_id = assignment.member_id.trim().to_string();
        if assignment.member_id.is_empty() {
            return Err("staff member task assignment member_id cannot be empty".to_string());
        }
        if !member_ids.contains(&assignment.member_id) {
            return Err(format!(
                "unknown staff member task assignment member_id: {}",
                assignment.member_id
            ));
        }
        if !seen_members.insert(assignment.member_id.clone()) {
            return Err(format!(
                "duplicate staff member task assignment member_id: {}",
                assignment.member_id
            ));
        }

        normalize_string_list(&mut assignment.task_ids);
        for task_id in &assignment.task_ids {
            if !seen_task_ids.insert(task_id.clone()) {
                return Err(format!("duplicate staff member task assignment task id: {task_id}"));
            }
        }
    }
    assignments.retain(|assignment| !assignment.task_ids.is_empty());
    Ok(())
}

fn validate_member_task_assignments(
    assignments: &[StaffMemberTaskAssignmentConfig],
    member_ids: &HashSet<&str>,
) -> Result<(), String> {
    let mut seen_members = HashSet::new();
    let mut seen_task_ids = HashSet::new();
    for assignment in assignments {
        let member_id = assignment.member_id.trim();
        if member_id.is_empty() {
            return Err("staff member task assignment member_id cannot be empty".to_string());
        }
        if !member_ids.contains(member_id) {
            return Err(format!(
                "unknown staff member task assignment member_id: {}",
                assignment.member_id
            ));
        }
        if !seen_members.insert(member_id) {
            return Err(format!(
                "duplicate staff member task assignment member_id: {}",
                assignment.member_id
            ));
        }
        validate_string_list(&assignment.task_ids, "staff member task assignment task id")?;
        for task_id in &assignment.task_ids {
            if !seen_task_ids.insert(task_id.as_str()) {
                return Err(format!("duplicate staff member task assignment task id: {task_id}"));
            }
        }
    }
    Ok(())
}

fn validate_stage_assignment(
    assignment: &StaffStageAssignmentConfig,
    member_ids: &HashSet<String>,
) -> Result<(), String> {
    let member_ids = member_ids.iter().map(String::as_str).collect::<HashSet<_>>();
    validate_stage_assignment_readonly(assignment, &member_ids)
}

fn validate_stage_assignment_readonly(
    assignment: &StaffStageAssignmentConfig,
    member_ids: &HashSet<&str>,
) -> Result<(), String> {
    let mut seen_member_ids = HashSet::new();
    if let Some(member_id) = assignment.primary_member_id.as_deref() {
        validate_stage_assignment_member_id(member_id, member_ids, &mut seen_member_ids)?;
    }
    for member_id in &assignment.backup_member_ids {
        validate_stage_assignment_member_id(member_id, member_ids, &mut seen_member_ids)?;
    }
    for member_id in &assignment.manager_member_ids {
        validate_stage_assignment_member_id(member_id, member_ids, &mut seen_member_ids)?;
    }
    Ok(())
}

fn validate_stage_assignment_member_id<'a>(
    member_id: &'a str,
    member_ids: &HashSet<&str>,
    seen_member_ids: &mut HashSet<&'a str>,
) -> Result<(), String> {
    if member_id.trim().is_empty() {
        return Err("staff stage assignment member id cannot be empty".to_string());
    }
    if !member_ids.contains(member_id) {
        return Err(format!("unknown staff stage assignment member id: {member_id}"));
    }
    if !seen_member_ids.insert(member_id) {
        return Err(format!("duplicate staff stage assignment member id: {member_id}"));
    }
    Ok(())
}

fn is_built_in_stage_capability_id(capability_id: &str) -> bool {
    BUILT_IN_STAGE_CAPABILITY_IDS.contains(&capability_id)
}

fn normalize_optional_string(value: Option<&str>) -> Option<String> {
    value.map(str::trim).filter(|value| !value.is_empty()).map(ToString::to_string)
}

fn normalize_channel_bindings(bindings: &mut [StaffChannelBindingConfig]) -> Result<(), String> {
    let mut seen_channel_types = HashSet::new();
    for binding in bindings {
        binding.channel_type = binding.channel_type.trim().to_ascii_lowercase();
        binding.instance_id = binding.instance_id.trim().to_string();
        validate_channel_binding(binding)?;
        if !seen_channel_types.insert(binding.channel_type.clone()) {
            return Err(format!("duplicate staff channel binding type: {}", binding.channel_type));
        }
    }
    Ok(())
}

fn validate_channel_bindings(bindings: &[StaffChannelBindingConfig]) -> Result<(), String> {
    let mut seen_channel_types = HashSet::new();
    for binding in bindings {
        validate_channel_binding(binding)?;
        let channel_type = binding.channel_type.trim();
        if !seen_channel_types.insert(channel_type) {
            return Err(format!("duplicate staff channel binding type: {channel_type}"));
        }
    }
    Ok(())
}

fn validate_channel_binding(binding: &StaffChannelBindingConfig) -> Result<(), String> {
    let channel_type = binding.channel_type.trim();
    if channel_type.is_empty() {
        return Err("staff channel binding type cannot be empty".to_string());
    }
    if !STAFF_CHANNEL_BINDING_TYPES.contains(&channel_type) {
        return Err(format!("unsupported staff channel binding type: {channel_type}"));
    }

    let instance_id = binding.instance_id.trim();
    if instance_id.is_empty() {
        return Err(format!(
            "staff channel binding instance_id for {channel_type} cannot be empty"
        ));
    }
    if instance_id.contains([':', '/', '\\']) {
        return Err(format!(
            "staff channel binding instance_id for {channel_type} must not contain ':', '/' or '\\'"
        ));
    }
    Ok(())
}

fn trim_or_default(value: &str, default_value: &str) -> String {
    let value = value.trim();
    if value.is_empty() { default_value.to_string() } else { value.to_string() }
}

fn normalize_string_list(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain_mut(|value| {
        *value = value.trim().to_string();
        !value.is_empty() && seen.insert(value.clone())
    });
}

fn validate_string_list(values: &[String], label: &str) -> Result<(), String> {
    let mut seen = HashSet::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() {
            return Err(format!("{label} cannot be empty"));
        }
        if !seen.insert(value) {
            return Err(format!("duplicate {label}: {value}"));
        }
    }
    Ok(())
}

fn normalize_value_map(values: &mut BTreeMap<String, Value>, label: &str) -> Result<(), String> {
    let mut normalized = BTreeMap::new();
    for (key, value) in std::mem::take(values) {
        let key = key.trim().to_string();
        if key.is_empty() {
            return Err(format!("{label} key cannot be empty"));
        }
        if normalized.insert(key.clone(), value).is_some() {
            return Err(format!("duplicate {label} key: {key}"));
        }
    }
    *values = normalized;
    Ok(())
}

fn validate_value_map(values: &BTreeMap<String, Value>, label: &str) -> Result<(), String> {
    let mut seen_keys = HashSet::new();
    for key in values.keys() {
        let key = key.trim();
        if key.is_empty() {
            return Err(format!("{label} key cannot be empty"));
        }
        if !seen_keys.insert(key) {
            return Err(format!("duplicate {label} key: {key}"));
        }
    }
    Ok(())
}

fn validate_capabilities(capabilities: &StaffCapabilitiesConfig) -> Result<(), String> {
    if capabilities.max_iterations == 0 {
        return Err("staff capabilities max_iterations cannot be 0".to_string());
    }
    if capabilities.max_parallel_tasks == Some(0) {
        return Err("staff capabilities max_parallel_tasks cannot be 0".to_string());
    }
    if capabilities.max_active_minutes == Some(0) {
        return Err("staff capabilities max_active_minutes cannot be 0".to_string());
    }
    validate_string_list(&capabilities.allowed_tools, "staff allowed tool")?;
    validate_string_list(&capabilities.allowed_skills, "staff allowed skill")
}

#[cfg(test)]
#[path = "staff_tests.rs"]
mod staff_tests;
