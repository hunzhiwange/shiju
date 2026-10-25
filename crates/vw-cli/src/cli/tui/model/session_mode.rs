#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ReasoningEffort {
    #[default]
    Auto,
    None,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PermissionMode {
    #[default]
    Default,
    Plan,
    AcceptEdits,
    Bypass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegateAgentOption {
    pub(crate) key: Option<String>,
    pub(crate) label: String,
}

impl DelegateAgentOption {
    pub(crate) fn main() -> Self {
        Self { key: None, label: "main".to_string() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AcpBackendOption {
    pub(crate) key: Option<String>,
    pub(crate) label: String,
}

impl AcpBackendOption {
    pub(crate) fn default_backend() -> Self {
        Self { key: None, label: "默认 ACP".to_string() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SessionExecutionSnapshot {
    pub(crate) effort: ReasoningEffort,
    pub(crate) permission_mode: PermissionMode,
    pub(crate) delegate_agent: Option<String>,
    pub(crate) acp_backend: Option<String>,
    pub(crate) acp_force_new_session: bool,
    pub(crate) full_access_enabled: bool,
    pub(crate) subagent_mode_enabled: bool,
    pub(crate) workflow_mode: WorkflowMode,
    pub(crate) goal_mode: GoalMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum AcpSessionMode {
    #[default]
    Reuse,
    ForceNew,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum AccessMode {
    #[default]
    Standard,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SubagentMode {
    #[default]
    Disabled,
    Enabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WorkflowMode {
    #[default]
    Disabled,
    Enabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum GoalMode {
    Disabled,
    #[default]
    Enabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionCapabilityMatrix {
    pub(crate) models: Vec<String>,
    pub(crate) efforts: Vec<ReasoningEffort>,
    pub(crate) model_efforts: std::collections::HashMap<String, Vec<ReasoningEffort>>,
    pub(crate) supports_thinking: bool,
    pub(crate) supports_fast: bool,
    pub(crate) permission_modes: Vec<PermissionMode>,
    pub(crate) delegate_agents: Vec<DelegateAgentOption>,
    pub(crate) acp_backends: Vec<AcpBackendOption>,
}

impl SessionCapabilityMatrix {
    pub(crate) fn efforts_for_model(&self, model: Option<&str>) -> &[ReasoningEffort] {
        model
            .and_then(|model| self.model_efforts.get(model))
            .map(Vec::as_slice)
            .unwrap_or(&self.efforts)
    }
}

impl Default for SessionCapabilityMatrix {
    fn default() -> Self {
        Self {
            models: Vec::new(),
            efforts: Vec::new(),
            model_efforts: std::collections::HashMap::new(),
            supports_thinking: false,
            supports_fast: false,
            permission_modes: vec![PermissionMode::Default],
            delegate_agents: vec![DelegateAgentOption::main()],
            acp_backends: vec![AcpBackendOption::default_backend()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SessionOptions {
    pub(crate) model: Option<String>,
    pub(crate) effort: ReasoningEffort,
    pub(crate) thinking: bool,
    pub(crate) fast: bool,
    pub(crate) permission_mode: PermissionMode,
    pub(crate) bypass_confirmed: bool,
    pub(crate) delegate_agent: Option<String>,
    pub(crate) acp_backend: Option<String>,
    pub(crate) acp_session_mode: AcpSessionMode,
    pub(crate) access_mode: AccessMode,
    pub(crate) subagent_mode: SubagentMode,
    pub(crate) workflow_mode: WorkflowMode,
    pub(crate) goal_mode: GoalMode,
}

impl SessionOptions {
    pub(crate) fn cycle_permission_mode(
        &self,
        capabilities: &SessionCapabilityMatrix,
    ) -> PermissionMode {
        let available = &capabilities.permission_modes;
        if available.is_empty() {
            return self.permission_mode;
        }
        let current =
            available.iter().position(|mode| *mode == self.permission_mode).unwrap_or_default();
        available[(current + 1) % available.len()]
    }

    pub(crate) fn execution_snapshot(&self) -> SessionExecutionSnapshot {
        SessionExecutionSnapshot {
            effort: self.effort,
            permission_mode: self.permission_mode,
            delegate_agent: self.delegate_agent.clone(),
            acp_backend: self.acp_backend.clone(),
            acp_force_new_session: self.acp_session_mode == AcpSessionMode::ForceNew,
            full_access_enabled: self.access_mode == AccessMode::Full
                || (self.permission_mode == PermissionMode::Bypass && self.bypass_confirmed),
            subagent_mode_enabled: self.subagent_mode == SubagentMode::Enabled,
            workflow_mode: self.workflow_mode,
            goal_mode: self.goal_mode,
        }
    }

    pub(crate) fn select_acp_backend(&mut self, backend: Option<String>) {
        if self.acp_backend == backend {
            return;
        }
        self.acp_session_mode =
            if backend.is_some() { AcpSessionMode::ForceNew } else { AcpSessionMode::Reuse };
        self.acp_backend = backend;
    }

    pub(crate) fn finish_acp_session_start(
        &mut self,
        execution: &SessionExecutionSnapshot,
        succeeded: bool,
    ) {
        if succeeded && execution.acp_force_new_session && self.acp_backend == execution.acp_backend
        {
            self.acp_session_mode = AcpSessionMode::Reuse;
        }
    }

    pub(crate) fn acp_force_new_session(&self) -> bool {
        self.acp_session_mode == AcpSessionMode::ForceNew
    }

    pub(crate) fn full_access_enabled(&self) -> bool {
        self.access_mode == AccessMode::Full
            || (self.permission_mode == PermissionMode::Bypass && self.bypass_confirmed)
    }

    pub(crate) fn explicit_full_access_enabled(&self) -> bool {
        self.access_mode == AccessMode::Full
    }

    pub(crate) fn set_permission_mode(&mut self, mode: PermissionMode, dangerous_confirmed: bool) {
        self.permission_mode = mode;
        self.bypass_confirmed = mode == PermissionMode::Bypass && dangerous_confirmed;
    }

    pub(crate) fn set_full_access(&mut self, enabled: bool) {
        let next = if enabled { AccessMode::Full } else { AccessMode::Standard };
        if self.access_mode == next {
            return;
        }
        self.access_mode = next;
        if self.acp_backend.is_some() {
            self.acp_session_mode = AcpSessionMode::ForceNew;
        }
    }

    pub(crate) fn subagent_mode_enabled(&self) -> bool {
        self.subagent_mode == SubagentMode::Enabled
    }

    pub(crate) fn set_subagent_mode(&mut self, enabled: bool) {
        self.subagent_mode = if enabled { SubagentMode::Enabled } else { SubagentMode::Disabled };
    }

    pub(crate) fn workflow_mode_enabled(&self) -> bool {
        self.workflow_mode == WorkflowMode::Enabled
    }

    pub(crate) fn goal_mode_enabled(&self) -> bool {
        true
    }

    pub(crate) fn set_workflow_mode(&mut self, enabled: bool) {
        self.workflow_mode = if enabled { WorkflowMode::Enabled } else { WorkflowMode::Disabled };
    }

    pub(crate) fn set_goal_mode(&mut self, _enabled: bool) {
        // 目标模式底层固定开启，不可关闭。
        self.goal_mode = GoalMode::Enabled;
    }
}
