use super::session_mode::{PermissionMode, PlanRunMode, ReasoningEffort};
use crate::cli::tui_v2::vim::VimMode;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingRow {
    pub(crate) key: SettingKey,
    pub(crate) label: &'static str,
    pub(crate) unavailable_reason: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsValues {
    pub(crate) model: Option<String>,
    pub(crate) delegate_agent: Option<String>,
    pub(crate) acp_backend: Option<String>,
    pub(crate) full_access_enabled: bool,
    pub(crate) subagent_mode_enabled: bool,
    pub(crate) plan_run_mode: PlanRunMode,
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
            plan_run_mode: PlanRunMode::Disabled,
            effort: ReasoningEffort::Medium,
            fast: false,
            theme: "Dark".to_string(),
            permission_mode: PermissionMode::Default,
            vim_mode: VimMode::Disabled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsModelPicker {
    pub(crate) selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsDelegateAgentPicker {
    pub(crate) selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsAcpBackendPicker {
    pub(crate) selected: usize,
}

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
    pub(crate) fn new(values: SettingsValues) -> Self {
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
                    key: SettingKey::GoalMode,
                    label: "目标模式",
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
                    unavailable_reason: Some("当前 Gateway 未提供输出样式持久化能力"),
                },
                SettingRow {
                    key: SettingKey::Permissions,
                    label: "权限模式",
                    unavailable_reason: Some("危险权限模式必须通过专用确认流程"),
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
                SettingKey::Model if capabilities.models.is_empty() => Some("当前没有发现可选模型"),
                SettingKey::AcpBackend if capabilities.acp_backends.len() <= 1 => {
                    Some("当前没有发现已启用 ACP 后端")
                }
                SettingKey::Effort if capabilities.efforts.is_empty() => {
                    Some("当前 provider 未声明推理强度能力")
                }
                SettingKey::Fast if !capabilities.supports_fast => {
                    Some("当前 Gateway stream 契约未提供快速模式")
                }
                SettingKey::OutputStyle => Some("当前 Gateway 未提供输出样式持久化能力"),
                SettingKey::Permissions => {
                    Some("这里只切换会话模式；待审批请求请使用 /permissions")
                }
                _ => None,
            };
        }
        panel
    }

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

    pub(crate) fn selected_row(&self) -> Option<&SettingRow> {
        self.visible_rows().get(self.selected).copied()
    }

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
                if self.pending.full_access_enabled {
                    "已启用（危险）"
                } else {
                    "关闭"
                }
                .to_string()
            }
            SettingKey::SubagentMode => {
                if self.pending.subagent_mode_enabled { "开" } else { "关" }.to_string()
            }
            SettingKey::WorkflowMode => {
                if self.pending.plan_run_mode == PlanRunMode::Workflow { "开" } else { "关" }
                    .to_string()
            }
            SettingKey::GoalMode => {
                if self.pending.plan_run_mode == PlanRunMode::Goal { "开" } else { "关" }
                    .to_string()
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

    pub(crate) fn visible_rows(&self) -> Vec<&SettingRow> {
        let query = self.query.trim().to_lowercase();
        self.rows
            .iter()
            .filter(|row| query.is_empty() || row.label.to_lowercase().contains(&query))
            .collect()
    }

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

    pub(crate) fn cancel(&mut self) {
        self.pending = self.original.clone();
    }
}
