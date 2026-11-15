#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutonomyListKind {
    AllowedCommands,
    ForbiddenPaths,
    ShellEnvPassthrough,
    AutoApprove,
    AlwaysAsk,
    AllowedRoots,
    NonCliExcludedTools,
}

impl AutonomyListKind {
    pub const fn index(self) -> usize {
        match self {
            Self::AllowedCommands => 0,
            Self::ForbiddenPaths => 1,
            Self::ShellEnvPassthrough => 2,
            Self::AutoApprove => 3,
            Self::AlwaysAsk => 4,
            Self::AllowedRoots => 5,
            Self::NonCliExcludedTools => 6,
        }
    }

    pub const fn is_tool_list(self) -> bool {
        matches!(self, Self::AutoApprove | Self::AlwaysAsk | Self::NonCliExcludedTools)
    }
}

#[derive(Debug, Clone, Default)]
pub struct AutonomyListEditorState {
    drafts: [String; 7],
    pending_remove: Option<(AutonomyListKind, usize)>,
}

impl AutonomyListEditorState {
    pub fn draft(&self, kind: AutonomyListKind) -> &str {
        &self.drafts[kind.index()]
    }

    pub fn set_draft(&mut self, kind: AutonomyListKind, value: String) {
        self.drafts[kind.index()] = value;
    }

    pub fn request_remove(&mut self, kind: AutonomyListKind, index: usize) {
        self.pending_remove = Some((kind, index));
    }

    pub fn is_remove_pending(&self, kind: AutonomyListKind, index: usize) -> bool {
        self.pending_remove == Some((kind, index))
    }

    pub fn pending_remove_index(&self, kind: AutonomyListKind) -> Option<usize> {
        self.pending_remove
            .and_then(|(pending_kind, index)| (pending_kind == kind).then_some(index))
    }

    pub fn cancel_remove(&mut self) {
        self.pending_remove = None;
    }
}

/// 自主性设置面板状态
///
/// 管理 Agent 自主性级别的配置，包括权限控制、
/// 风险管理和审批策略等。
#[derive(Debug, Clone)]
pub struct AutonomySettingsState {
    /// 当前显示的自治配置页签
    pub active_tab: AutonomySettingsTab,
    /// 自主性级别
    pub level: config_security::AutonomyLevel,
    /// 是否限制在工作区
    pub workspace_only: bool,
    /// 允许的命令列表
    pub allowed_commands: Vec<String>,
    /// 禁止的路径列表
    pub forbidden_paths: Vec<String>,
    /// 每小时最大操作数
    pub max_actions_per_hour: u32,
    /// 每天最大成本（美分）
    pub max_cost_per_day_cents: u32,
    /// 中等风险操作是否需要审批
    pub require_approval_for_medium_risk: bool,
    /// 是否阻止高风险命令
    pub block_high_risk_commands: bool,
    /// Shell 重定向策略
    pub shell_redirect_policy: config_security::ShellRedirectPolicy,
    /// Shell 环境变量透传
    pub shell_env_passthrough: Vec<String>,
    /// 自动审批的操作
    pub auto_approve: Vec<String>,
    /// 始终询问的操作
    pub always_ask: Vec<String>,
    /// 允许的根目录
    pub allowed_roots: Vec<String>,
    /// 非 CLI 排除的工具
    pub non_cli_excluded_tools: Vec<String>,
    /// 七个逐项列表的草稿输入
    pub list_editor: AutonomyListEditorState,
    /// 非 CLI 审批人输入
    pub non_cli_approval_approvers_input: String,
    /// 非 CLI 自然语言审批模式
    pub non_cli_natural_language_approval_mode:
        config_security::NonCliNaturalLanguageApprovalMode,
    /// 按 channel 的非 CLI 自然语言审批模式输入
    pub non_cli_natural_language_approval_mode_by_channel_input: String,
    /// 是否显示帮助对话框
    pub show_help_modal: bool,
    /// 保存错误信息
    pub save_error: Option<String>,
    pub save_error_translation: Option<desktop_i18n::format::Message>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutonomySettingsTab {
    Permissions,
    Budget,
    ToolsAndPaths,
    NonCli,
}

impl Default for AutonomySettingsState {
    fn default() -> Self {
        Self::from_config(&config_security::AutonomyConfig::default())
    }
}

impl AutonomySettingsState {
    pub fn from_config(config: &config_security::AutonomyConfig) -> Self {
        Self {
            active_tab: AutonomySettingsTab::Permissions,
            level: config.level,
            workspace_only: config.workspace_only,
            allowed_commands: config.allowed_commands.clone(),
            forbidden_paths: config.forbidden_paths.clone(),
            max_actions_per_hour: config.max_actions_per_hour,
            max_cost_per_day_cents: config.max_cost_per_day_cents,
            require_approval_for_medium_risk: config.require_approval_for_medium_risk,
            block_high_risk_commands: config.block_high_risk_commands,
            shell_redirect_policy: config.shell_redirect_policy,
            shell_env_passthrough: config.shell_env_passthrough.clone(),
            auto_approve: config.auto_approve.clone(),
            always_ask: config.always_ask.clone(),
            allowed_roots: config.allowed_roots.clone(),
            non_cli_excluded_tools: config.non_cli_excluded_tools.clone(),
            list_editor: AutonomyListEditorState::default(),
            non_cli_approval_approvers_input: config.non_cli_approval_approvers.join(", "),
            non_cli_natural_language_approval_mode: config.non_cli_natural_language_approval_mode,
            non_cli_natural_language_approval_mode_by_channel_input: config
                .non_cli_natural_language_approval_mode_by_channel
                .iter()
                .map(|(channel, mode)| {
                    let mode = match mode {
                        config_security::NonCliNaturalLanguageApprovalMode::Disabled => {
                            "disabled"
                        }
                        config_security::NonCliNaturalLanguageApprovalMode::RequestConfirm => {
                            "request_confirm"
                        }
                        config_security::NonCliNaturalLanguageApprovalMode::Direct => {
                            "direct"
                        }
                    };
                    format!("{channel}:{mode}")
                })
                .collect::<Vec<_>>()
                .join(", "),
            show_help_modal: false,
            save_error: None,
            save_error_translation: None,
        }
    }

    pub fn list(&self, kind: AutonomyListKind) -> &[String] {
        match kind {
            AutonomyListKind::AllowedCommands => &self.allowed_commands,
            AutonomyListKind::ForbiddenPaths => &self.forbidden_paths,
            AutonomyListKind::ShellEnvPassthrough => &self.shell_env_passthrough,
            AutonomyListKind::AutoApprove => &self.auto_approve,
            AutonomyListKind::AlwaysAsk => &self.always_ask,
            AutonomyListKind::AllowedRoots => &self.allowed_roots,
            AutonomyListKind::NonCliExcludedTools => &self.non_cli_excluded_tools,
        }
    }

    pub fn list_mut(&mut self, kind: AutonomyListKind) -> &mut Vec<String> {
        match kind {
            AutonomyListKind::AllowedCommands => &mut self.allowed_commands,
            AutonomyListKind::ForbiddenPaths => &mut self.forbidden_paths,
            AutonomyListKind::ShellEnvPassthrough => &mut self.shell_env_passthrough,
            AutonomyListKind::AutoApprove => &mut self.auto_approve,
            AutonomyListKind::AlwaysAsk => &mut self.always_ask,
            AutonomyListKind::AllowedRoots => &mut self.allowed_roots,
            AutonomyListKind::NonCliExcludedTools => &mut self.non_cli_excluded_tools,
        }
    }
}
