use serde_json::Value;

/// 工具权限决策结果。
#[derive(Debug, Clone)]
pub enum PermissionDecision {
    /// 允许执行，并可能附带归一化后的输入。
    Allow { reason: Option<String>, updated_input: Value, warning: Option<String> },
    /// 需要继续走审批确认。
    Ask { reason: String, updated_input: Value, warning: Option<String> },
    /// 拒绝执行。
    Deny { reason: String, updated_input: Option<Value>, warning: Option<String> },
}

impl PermissionDecision {
    /// 构造允许决策。
    pub fn allow(updated_input: Value) -> Self {
        Self::Allow { reason: None, updated_input, warning: None }
    }

    /// 构造需要审批的决策。
    pub fn ask(reason: impl Into<String>, updated_input: Value) -> Self {
        Self::Ask { reason: reason.into(), updated_input, warning: None }
    }

    /// 构造拒绝决策。
    pub fn deny(reason: impl Into<String>) -> Self {
        Self::Deny { reason: reason.into(), updated_input: None, warning: None }
    }

    /// 为决策补充 warning。
    pub fn with_warning(mut self, warning: Option<String>) -> Self {
        match &mut self {
            Self::Allow { warning: slot, .. }
            | Self::Ask { warning: slot, .. }
            | Self::Deny { warning: slot, .. } => *slot = warning,
        }
        self
    }

    /// 为拒绝决策补充输入快照。
    pub fn with_updated_input(mut self, updated_input: Value) -> Self {
        if let Self::Deny { updated_input: slot, .. } = &mut self {
            *slot = Some(updated_input);
        }
        self
    }

    /// 读取决策原因。
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Allow { reason, .. } => reason.as_deref(),
            Self::Ask { reason, .. } | Self::Deny { reason, .. } => Some(reason.as_str()),
        }
    }

    /// 读取 warning。
    pub fn warning(&self) -> Option<&str> {
        match self {
            Self::Allow { warning, .. }
            | Self::Ask { warning, .. }
            | Self::Deny { warning, .. } => warning.as_deref(),
        }
    }

    /// 读取更新后的输入。
    pub fn updated_input(&self) -> Option<&Value> {
        match self {
            Self::Allow { updated_input, .. } | Self::Ask { updated_input, .. } => {
                Some(updated_input)
            }
            Self::Deny { updated_input, .. } => updated_input.as_ref(),
        }
    }
}
