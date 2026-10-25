//! TUI 设置面板的已保存值、待提交值与子选择器状态。
//!
//! 面板始终分离 `original` 和 `pending`：预览只修改待提交值，取消时可完整回滚。
//! 能力矩阵负责标记不可用项，界面不伪造 Gateway 尚未提供的持久化能力。

use super::session_mode::{GoalMode, PermissionMode, ReasoningEffort, WorkflowMode};
use crate::cli::tui::vim::VimMode;

/// 稳定标识一个设置行，用于选择、过滤与值渲染。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingKey {
    Model,
    DelegateAgent,
    AcpBackend,
    FullAccess,
    SubagentMode,
    WorkflowMode,
    GoalMode,
    Effort,
    Fast,
    Theme,
    OutputStyle,
    Permissions,
    Vim,
}

/// 设置行的标签与不可用原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingRow {
    pub(crate) key: SettingKey,
    pub(crate) label: &'static str,
    pub(crate) unavailable_reason: Option<String>,
}

/// 一份可编辑的会话设置快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsValues {
    pub(crate) model: Option<String>,
    pub(crate) delegate_agent: Option<String>,
    pub(crate) acp_backend: Option<String>,
    pub(crate) full_access_enabled: bool,
    pub(crate) subagent_mode_enabled: bool,
    pub(crate) workflow_mode: WorkflowMode,
    pub(crate) goal_mode: GoalMode,
    pub(crate) effort: ReasoningEffort,
    pub(crate) fast: bool,
    pub(crate) theme: String,
    pub(crate) permission_mode: PermissionMode,
    pub(crate) vim_mode: VimMode,
}

impl Default for SettingsValues {
    fn default() -> Self {
        Self {
            model: None,
            delegate_agent: None,
            acp_backend: None,
            full_access_enabled: false,
            subagent_mode_enabled: false,
            workflow_mode: WorkflowMode::Disabled,
            goal_mode: GoalMode::Enabled,
            effort: ReasoningEffort::Auto,
            fast: false,
            theme: "Dark".to_string(),
            permission_mode: PermissionMode::Default,
            vim_mode: VimMode::Disabled,
        }
    }
}

/// 模型选择器的临时选中位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsModelPicker {
    pub(crate) selected: usize,
}

/// 委托智能体选择器的临时选中位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsDelegateAgentPicker {
    pub(crate) selected: usize,
}

/// ACP 后端选择器的临时选中位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsAcpBackendPicker {
    pub(crate) selected: usize,
}

/// 设置面板的行、过滤、原值、待提交值及可选能力。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsPanel {
    pub(crate) rows: Vec<SettingRow>,
    pub(crate) query: String,
    pub(crate) selected: usize,
    pub(crate) original: Box<SettingsValues>,
    pub(crate) pending: Box<SettingsValues>,
    pub(crate) model_options: Vec<String>,
    pub(crate) model_picker: Option<SettingsModelPicker>,
    pub(crate) delegate_agent_options: Vec<super::session_mode::DelegateAgentOption>,
    pub(crate) delegate_agent_picker: Option<SettingsDelegateAgentPicker>,
    pub(crate) acp_backend_options: Vec<super::session_mode::AcpBackendOption>,
    pub(crate) acp_backend_picker: Option<SettingsAcpBackendPicker>,
}

impl SettingsPanel {
    /// 从已保存值创建面板；目标模式固定开启，不向用户提供伪关闭入口。
    pub(crate) fn new(mut values: SettingsValues) -> Self {
        values.goal_mode = GoalMode::Enabled;
        Self {
            rows: vec![
                SettingRow { key: SettingKey::Model, label: "模型", unavailable_reason: None },
                SettingRow {
                    key: SettingKey::DelegateAgent,
                    label: "委托代理",
                    unavailable_reason: None,
                },
                SettingRow {
                    key: SettingKey::AcpBackend,
                    label: "ACP 后端",
                    unavailable_reason: None,
                },
                SettingRow {
                    key: SettingKey::FullAccess,
                    label: "完全访问",
                    unavailable_reason: None,
                },
                SettingRow {
                    key: SettingKey::SubagentMode,
                    label: "子智能体模式",
                    unavailable_reason: None,
                },
                SettingRow {
                    key: SettingKey::WorkflowMode,
                    label: "工作流模式",
                    unavailable_reason: None,
                },
                SettingRow {
                    key: SettingKey::Effort, label: "推理强度", unavailable_reason: None
                },
                SettingRow {
                    key: SettingKey::Fast, label: "快速模式", unavailable_reason: None
                },
                SettingRow { key: SettingKey::Theme, label: "主题", unavailable_reason: None },
                SettingRow {
                    key: SettingKey::OutputStyle,
                    label: "输出样式",
                    unavailable_reason: Some("当前 Gateway 未提供输出样式持久化能力".to_string()),
                },
                SettingRow {
                    key: SettingKey::Permissions,
                    label: "权限模式",
                    unavailable_reason: None,
                },
                SettingRow { key: SettingKey::Vim, label: "Vim 模式", unavailable_reason: None },
            ],
            query: String::new(),
            selected: 0,
            original: Box::new(values.clone()),
            pending: Box::new(values),
            model_options: Vec::new(),
            model_picker: None,
            delegate_agent_options: Vec::new(),
            delegate_agent_picker: None,
            acp_backend_options: Vec::new(),
            acp_backend_picker: None,
        }
    }

    /// 结合当前会话能力创建面板，对不支持的行附加明确原因。
    pub(crate) fn for_capabilities(
        values: SettingsValues,
        capabilities: &super::session_mode::SessionCapabilityMatrix,
    ) -> Self {
        let mut panel = Self::new(values);
        panel.model_options.clone_from(&capabilities.models);
        panel.delegate_agent_options.clone_from(&capabilities.delegate_agents);
        panel.acp_backend_options.clone_from(&capabilities.acp_backends);
        for row in &mut panel.rows {
            row.unavailable_reason = match row.key {
                SettingKey::Model if capabilities.models.is_empty() => {
                    Some("当前没有发现可选模型".to_string())
                }
                SettingKey::DelegateAgent if capabilities.delegate_agents.len() <= 1 => {
                    Some("当前配置未声明可委托 Agent".to_string())
                }
                SettingKey::AcpBackend if capabilities.acp_backends.len() <= 1 => {
                    Some("当前没有发现已启用 ACP 后端".to_string())
                }
                SettingKey::FullAccess
                    if !capabilities.permission_modes.contains(&PermissionMode::Bypass) =>
                {
                    Some("Gateway 安全配置未声明完整访问能力".to_string())
                }
                SettingKey::Effort
                    if capabilities
                        .efforts_for_model(panel.pending.model.as_deref())
                        .is_empty() =>
                {
                    Some("当前 provider 未声明推理强度能力".to_string())
                }
                SettingKey::Fast if !capabilities.supports_fast => {
                    Some("当前 Gateway stream 契约未提供快速模式".to_string())
                }
                SettingKey::OutputStyle => {
                    Some("当前 Gateway 未提供输出样式持久化能力".to_string())
                }
                SettingKey::Permissions if capabilities.permission_modes.is_empty() => {
                    Some("当前 Gateway 未声明可用权限模式".to_string())
                }
                _ => None,
            };
        }
        panel
    }

    /// 在存在可选模型时打开选择器，并对齐当前待提交值。
    pub(crate) fn open_model_picker(&mut self) -> bool {
        if self.model_options.is_empty() {
            return false;
        }
        let selected = self
            .pending
            .model
            .as_ref()
            .and_then(|model| self.model_options.iter().position(|candidate| candidate == model))
            .unwrap_or_default();
        self.model_picker = Some(SettingsModelPicker { selected });
        true
    }

    pub(crate) fn move_model_picker_selection(&mut self, delta: isize) {
        let Some(picker) = self.model_picker.as_mut() else {
            return;
        };
        if delta < 0 {
            picker.selected = picker.selected.saturating_sub(1);
        } else {
            picker.selected =
                picker.selected.saturating_add(1).min(self.model_options.len().saturating_sub(1));
        }
    }

    /// 确认模型选择；选项已失效时不修改待提交值。
    pub(crate) fn confirm_model_picker(&mut self) -> bool {
        let Some(picker) = self.model_picker.take() else {
            return false;
        };
        let Some(model) = self.model_options.get(picker.selected).cloned() else {
            return false;
        };
        self.pending.model = Some(model);
        true
    }

    pub(crate) fn cancel_model_picker(&mut self) -> bool {
        self.model_picker.take().is_some()
    }

    /// 在有可用委托智能体时打开对应选择器。
    pub(crate) fn open_delegate_agent_picker(&mut self) -> bool {
        if self.delegate_agent_options.is_empty() {
            return false;
        }
        let selected = self
            .delegate_agent_options
            .iter()
            .position(|candidate| candidate.key == self.pending.delegate_agent)
            .unwrap_or_default();
        self.delegate_agent_picker = Some(SettingsDelegateAgentPicker { selected });
        true
    }

    pub(crate) fn move_delegate_agent_picker_selection(&mut self, delta: isize) {
        let Some(picker) = self.delegate_agent_picker.as_mut() else {
            return;
        };
        if delta < 0 {
            picker.selected = picker.selected.saturating_sub(1);
        } else {
            picker.selected = picker
                .selected
                .saturating_add(1)
                .min(self.delegate_agent_options.len().saturating_sub(1));
        }
    }

    /// 确认委托智能体选择，仅更新待提交快照。
    pub(crate) fn confirm_delegate_agent_picker(&mut self) -> bool {
        let Some(picker) = self.delegate_agent_picker.take() else {
            return false;
        };
        let Some(agent) = self.delegate_agent_options.get(picker.selected) else {
            return false;
        };
        self.pending.delegate_agent.clone_from(&agent.key);
        true
    }

    pub(crate) fn cancel_delegate_agent_picker(&mut self) -> bool {
        self.delegate_agent_picker.take().is_some()
    }

    /// 仅在存在多个 ACP 后端时打开选择器。
    pub(crate) fn open_acp_backend_picker(&mut self) -> bool {
        if self.acp_backend_options.len() <= 1 {
            return false;
        }
        let selected = self
            .acp_backend_options
            .iter()
            .position(|candidate| candidate.key == self.pending.acp_backend)
            .unwrap_or_default();
        self.acp_backend_picker = Some(SettingsAcpBackendPicker { selected });
        true
    }

    pub(crate) fn move_acp_backend_picker_selection(&mut self, delta: isize) {
        let Some(picker) = self.acp_backend_picker.as_mut() else {
            return;
        };
        if delta < 0 {
            picker.selected = picker.selected.saturating_sub(1);
        } else {
            picker.selected = picker
                .selected
                .saturating_add(1)
                .min(self.acp_backend_options.len().saturating_sub(1));
        }
    }

    /// 确认 ACP 后端选择，不在此处执行会话切换。
    pub(crate) fn confirm_acp_backend_picker(&mut self) -> bool {
        let Some(picker) = self.acp_backend_picker.take() else {
            return false;
        };
        let Some(backend) = self.acp_backend_options.get(picker.selected) else {
            return false;
        };
        self.pending.acp_backend.clone_from(&backend.key);
        true
    }

    pub(crate) fn cancel_acp_backend_picker(&mut self) -> bool {
        self.acp_backend_picker.take().is_some()
    }

    /// 返回经过查询过滤后的当前选中行。
    pub(crate) fn selected_row(&self) -> Option<&SettingRow> {
        self.visible_rows().get(self.selected).copied()
    }

    /// 在可见行中循环移动选中位置，空列表时回到零。
    pub(crate) fn move_selection(&mut self, delta: isize) {
        let len = self.visible_rows().len();
        if len == 0 {
            self.selected = 0;
        } else if delta < 0 {
            self.selected = self.selected.checked_sub(1).unwrap_or(len.saturating_sub(1));
        } else {
            self.selected = (self.selected + 1) % len;
        }
    }

    /// 将指定待提交值转为面板使用的简短标签。
    pub(crate) fn value_label(&self, key: SettingKey) -> String {
        match key {
            SettingKey::Model => self.pending.model.clone().unwrap_or_else(|| "自动".to_string()),
            SettingKey::DelegateAgent => self
                .delegate_agent_options
                .iter()
                .find(|agent| agent.key == self.pending.delegate_agent)
                .map(|agent| agent.label.clone())
                .unwrap_or_else(|| {
                    self.pending.delegate_agent.clone().unwrap_or_else(|| "main".to_string())
                }),
            SettingKey::AcpBackend => self
                .acp_backend_options
                .iter()
                .find(|backend| backend.key == self.pending.acp_backend)
                .map(|backend| backend.label.clone())
                .unwrap_or_else(|| {
                    self.pending.acp_backend.clone().unwrap_or_else(|| "默认 ACP".to_string())
                }),
            SettingKey::FullAccess => {
                if self.pending.full_access_enabled { "已启用（危险）" } else { "关闭" }.to_string()
            }
            SettingKey::SubagentMode => {
                if self.pending.subagent_mode_enabled { "开" } else { "关" }.to_string()
            }
            SettingKey::WorkflowMode => {
                if self.pending.workflow_mode == WorkflowMode::Enabled { "开" } else { "关" }
                    .to_string()
            }
            SettingKey::GoalMode => {
                if self.pending.goal_mode == GoalMode::Enabled { "开" } else { "关" }.to_string()
            }
            SettingKey::Effort => format!("{:?}", self.pending.effort).to_ascii_lowercase(),
            SettingKey::Fast => if self.pending.fast { "开" } else { "关" }.to_string(),
            SettingKey::Theme => self.pending.theme.clone(),
            SettingKey::OutputStyle => "默认".to_string(),
            SettingKey::Permissions => {
                format!("{:?}", self.pending.permission_mode).to_ascii_lowercase()
            }
            SettingKey::Vim => format!("{:?}", self.pending.vim_mode).to_ascii_lowercase(),
        }
    }

    /// 按当前查询返回可见行，保留原始稳定顺序。
    pub(crate) fn visible_rows(&self) -> Vec<&SettingRow> {
        let query = self.query.trim().to_lowercase();
        self.rows
            .iter()
            .filter(|row| query.is_empty() || row.label.to_lowercase().contains(&query))
            .collect()
    }

    /// 校验主题名后更新预览；不支持的值会显式失败。
    pub(crate) fn preview_theme(&mut self, theme: &str) -> Result<(), String> {
        if !matches!(
            theme,
            "Dark" | "Light" | "DarkDaltonized" | "LightDaltonized" | "DarkAnsi" | "LightAnsi"
        ) {
            return Err(format!("unsupported TUI theme: {theme}"));
        }
        self.pending.theme = theme.to_string();
        Ok(())
    }

    /// 放弃所有未提交编辑，将待提交值恢复为打开面板时的快照。
    pub(crate) fn cancel(&mut self) {
        self.pending = self.original.clone();
    }
}
