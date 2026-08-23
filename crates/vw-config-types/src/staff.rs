//! 定义员工编排、工作流与项目分配配置，并集中执行归一化和一致性校验。

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

/// 由网关持久化的员工名册、工作流及项目分配配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffConfig {
    /// 任务看板员工模式可调度的成员。
    #[serde(default = "default_staff_members")]
    pub members: Vec<StaffMemberConfig>,
    /// 映射到桌面任务状态时使用的稳定工作流阶段。
    #[serde(default = "default_workflow_stages")]
    pub workflow_stages: Vec<StaffWorkflowStageConfig>,
    /// 以工作流标识为键的系统级编排模板。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub workflow_orchestrations: BTreeMap<String, StaffWorkflowOrchestrationConfig>,
    /// 通过网关配置持久化的项目级员工状态。
    #[serde(default)]
    pub project_state: StaffProjectStateConfig,
}

impl StaffConfig {
    /// 返回归一化后的配置副本，补齐默认项、清理空白并对列表去重。
    ///
    /// 归一化同时验证成员、阶段及其交叉引用，失败时返回带字段上下文的错误。
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

    /// 在不修改配置的前提下验证字段约束与所有交叉引用。
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

/// 一名可参与员工模式调度的成员。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffMemberConfig {
    /// 员工的稳定标识。
    pub id: String,
    /// 面向用户的显示名称。
    pub name: String,
    /// 稳定角色键，例如 `orchestrator`、`planner` 或 `executor`。
    pub role: String,
    /// 稳定级别键，例如 `principal`、`manager`、`executor` 或 `member`。
    pub level: String,
    /// 稳定状态键；桌面端会将其映射为本地员工状态类型。
    #[serde(default = "default_staff_status")]
    pub status: String,
    /// 此成员是否可以接收员工模式任务。
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 此成员可同时承担的最大任务数。
    #[serde(default = "default_capacity")]
    pub capacity: u32,
    /// 绑定的委派代理键，通常默认为 `main`。
    #[serde(default)]
    pub delegate_agent: Option<String>,
    /// 可选的已绑定 ACP 代理键。
    #[serde(default)]
    pub acp_agent: Option<String>,
    /// 作为员工消息目标的可选 IPC 代理标识。
    #[serde(default)]
    pub ipc_agent_id: Option<String>,
    /// 按通道类型区分的外部通道实例绑定。
    #[serde(default)]
    pub channel_bindings: Vec<StaffChannelBindingConfig>,
    /// 此成员专属的能力与运行时限制。
    #[serde(default)]
    pub capabilities: StaffCapabilitiesConfig,
}

impl StaffMemberConfig {
    /// 使用必需身份字段创建启用的成员，并绑定默认委派代理。
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

/// 一名员工到一个具名外部通道实例的绑定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffChannelBindingConfig {
    /// 支持的通道类型，例如 `matrix`、`feishu` 或 `dingtalk`。
    pub channel_type: String,
    /// 通道实例的稳定标识。
    pub instance_id: String,
}

/// 员工专属的工具、技能及运行时能力配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffCapabilitiesConfig {
    /// 此成员是否可以运行代理式工具循环。
    #[serde(default)]
    pub agentic: bool,
    /// 此成员专属的工具允许列表。
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// 此成员专属的技能允许列表。
    #[serde(default)]
    pub allowed_skills: Vec<String>,
    /// 此成员运行代理式工具循环的最大迭代次数。
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// 可选的同时活跃任务数上限。
    #[serde(default)]
    pub max_parallel_tasks: Option<u32>,
    /// 可选的活跃工作时长上限，单位为分钟。
    #[serde(default)]
    pub max_active_minutes: Option<u32>,
}

impl StaffCapabilitiesConfig {
    /// 清理工具和技能列表后验证运行时限制。
    pub fn normalize(&mut self) -> Result<(), String> {
        normalize_string_list(&mut self.allowed_tools);
        normalize_string_list(&mut self.allowed_skills);
        validate_capabilities(self)
    }

    /// 在不修改配置的前提下验证能力与运行时限制。
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

/// 使用稳定任务状态字符串描述的共享工作流阶段。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffWorkflowStageConfig {
    /// 阶段的稳定标识。
    pub id: String,
    /// 面向用户的阶段名称。
    pub name: String,
    /// 桌面端稳定任务状态键，例如 `running` 或 `code_review`。
    pub task_status: String,
    /// 预期负责此阶段的可选员工角色键。
    #[serde(default)]
    pub role: Option<String>,
    /// 此生命周期阶段是否参与员工编排。
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 是否仅用于展示；展示阶段不能持有任务。
    #[serde(default)]
    pub display_only: bool,
}

impl StaffWorkflowStageConfig {
    /// 创建可持有任务并绑定角色的工作流阶段。
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

    /// 创建不绑定角色、不能持有任务的展示阶段。
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

/// 一个工作流标识对应的系统级编排模板。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffWorkflowOrchestrationConfig {
    /// 仅在此工作流内生效的自定义能力目录项。
    #[serde(default)]
    pub capabilities: Vec<StaffCapabilityConfig>,
    /// 使用此工作流的任务阶段归属关系。
    #[serde(default)]
    pub stage_assignments: Vec<StaffStageAssignmentConfig>,
    /// 此工作流中各阶段绑定的持久化工作流 UUID。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stage_workflow_bindings: Vec<StaffStageWorkflowBindingConfig>,
    /// 此工作流各阶段的执行动作。
    #[serde(default)]
    pub stage_actions: Vec<StaffStageActionConfig>,
}

/// 项目范围内持久化的员工状态。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffProjectStateConfig {
    /// 项目范围内的自定义能力目录项。
    #[serde(default)]
    pub capabilities: Vec<StaffCapabilityConfig>,
    /// 当前项目的阶段归属关系。
    #[serde(default)]
    pub stage_assignments: Vec<StaffStageAssignmentConfig>,
    /// 当前项目各阶段绑定的持久化工作流 UUID。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stage_workflow_bindings: Vec<StaffStageWorkflowBindingConfig>,
    /// 当前项目各阶段的执行动作。
    #[serde(default)]
    pub stage_actions: Vec<StaffStageActionConfig>,
    /// 当前项目内生效的任务归属关系。
    #[serde(default)]
    pub member_task_assignments: Vec<StaffMemberTaskAssignmentConfig>,
}

/// 项目或工作流能力目录中的一项定义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffCapabilityConfig {
    /// 供阶段动作引用的稳定能力标识。
    pub id: String,
    /// 面向用户的能力标签。
    pub label: String,
    /// 能力所属的执行类别。
    #[serde(rename = "type")]
    pub capability_type: StaffCapabilityKindConfig,
    /// 此目录项是否描述内置能力。
    #[serde(default)]
    pub built_in: bool,
    /// 此能力所需输入的摘要。
    #[serde(default)]
    pub input_summary: StaffCapabilityIoSummaryConfig,
    /// 此能力产出结果的摘要。
    #[serde(default)]
    pub result_summary: StaffCapabilityIoSummaryConfig,
}

/// 能力的执行类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StaffCapabilityKindConfig {
    BuiltIn,
    SavedWorkflow,
}

/// 能力目录项中面向用户的输入或输出摘要。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffCapabilityIoSummaryConfig {
    /// 输入或结果契约的简短描述。
    #[serde(default)]
    pub description: String,
    /// 契约中约定的字段名或结果键。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
}

/// 一个工作流阶段到一名或多名员工的归属配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffStageAssignmentConfig {
    /// 工作流阶段标识。
    pub stage_id: String,
    /// 主要负责人标识。
    #[serde(default)]
    pub primary_member_id: Option<String>,
    /// 备用负责人标识列表。
    #[serde(default)]
    pub backup_member_ids: Vec<String>,
    /// 管理者标识列表。
    #[serde(default)]
    pub manager_member_ids: Vec<String>,
    /// 此阶段是否可以自主调度。
    #[serde(default)]
    pub autonomous: bool,
}

/// 工作流阶段到一个持久化工作流 UUID 的绑定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffStageWorkflowBindingConfig {
    /// 工作流阶段标识。
    pub stage_id: String,
    /// 此阶段对应的持久化工作流 UUID。
    pub workflow_uuid: String,
}

/// 为一个工作流阶段配置的执行动作。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffStageActionConfig {
    /// 工作流阶段标识。
    pub stage_id: String,
    /// 阶段所调用的稳定能力标识。
    pub capability_id: String,
    /// 持久化工作流 UUID；`saved_workflow` 能力必须提供。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_uuid: Option<String>,
    /// 阶段动作的结构化输入。
    #[serde(default, skip_serializing_if = "StaffStageActionInputsConfig::is_empty")]
    pub inputs: StaffStageActionInputsConfig,
}

/// 阶段动作绑定使用的结构化输入。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffStageActionInputsConfig {
    /// `saved_workflow` 能力使用的已保存工作流 UUID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_uuid: Option<String>,
    /// 传递给能力的通用参数。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, Value>,
}

impl StaffStageActionInputsConfig {
    /// 判断此输入是否未提供工作流 UUID 和通用参数。
    pub fn is_empty(&self) -> bool {
        self.workflow_uuid.is_none() && self.parameters.is_empty()
    }
}

/// 一名员工当前负责的活跃任务标识集合。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StaffMemberTaskAssignmentConfig {
    /// 员工标识。
    pub member_id: String,
    /// 此员工当前负责的任务标识列表。
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
