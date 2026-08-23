//! MCP、技能、插件等集成页面的视图模型。
//!
//! 本模块统一加载状态、选中项和技能平台选择，但不直接调用安装或启停 API。
//! 进入渲染的外部错误会经过最小脱敏，避免直接显示常见凭据字段。

/// 集成面板当前展示的资源类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegrationKind {
    Mcp,
    Skill,
    Plugin,
    Hook,
    Memory,
    Doctor,
}

/// 集成列表的异步加载结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IntegrationLoadState {
    Loading,
    Empty,
    Ready,
    Error(String),
}

/// 某条集成可执行的界面操作及其可用性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrationAction {
    pub(crate) label: String,
    pub(crate) enabled: bool,
    pub(crate) disabled_reason: Option<String>,
}

/// 一条集成资源的可显示快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrationItem {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) enabled: bool,
    pub(crate) actions: Vec<IntegrationAction>,
}

/// 集成页面的列表、选中位置与技能平台子选择器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrationPanel {
    pub(crate) kind: IntegrationKind,
    pub(crate) state: IntegrationLoadState,
    pub(crate) items: Vec<IntegrationItem>,
    pub(crate) selected: usize,
    pub(crate) skill_provider: SkillsDirectoryProvider,
    pub(crate) can_toggle: bool,
    pub(crate) skill_platform_focused: bool,
    pub(crate) skill_provider_picker: Option<SkillProviderPicker>,
}

/// 技能目录平台选择器的临时状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillProviderPicker {
    pub(crate) selected: usize,
}

impl IntegrationPanel {
    /// 以指定类别创建面板，初始状态为正在加载。
    pub(crate) fn new(kind: IntegrationKind) -> Self {
        Self {
            kind,
            state: IntegrationLoadState::Loading,
            items: Vec::new(),
            selected: 0,
            skill_provider: SkillsDirectoryProvider::default(),
            can_toggle: true,
            skill_platform_focused: kind == IntegrationKind::Skill,
            skill_provider_picker: None,
        }
    }

    /// 将已有内容标记为正在刷新，便于界面保留上一次列表。
    pub(crate) fn begin_refresh(&mut self) {
        self.state = IntegrationLoadState::Loading;
    }

    /// 应用刷新结果，成功时按稳定 ID 保留选中项，失败时仅保存脱敏错误。
    pub(crate) fn finish_refresh(&mut self, result: Result<Vec<IntegrationItem>, String>) {
        let selected_id = self.items.get(self.selected).map(|item| item.id.clone());
        match result {
            Ok(items) => {
                self.items = items;
                self.selected = selected_id
                    .and_then(|id| self.items.iter().position(|item| item.id == id))
                    .unwrap_or_default();
                self.state = if self.items.is_empty() {
                    IntegrationLoadState::Empty
                } else {
                    IntegrationLoadState::Ready
                };
            }
            Err(error) => {
                self.state = IntegrationLoadState::Error(sanitize_integration_text(&error));
            }
        }
    }

    /// 生成终端列表文本，所有条目详情在插值前会经过脱敏。
    pub(crate) fn lines(&self) -> Vec<String> {
        let mut lines = vec![format!("{:?} · {:?}", self.kind, self.state)];
        if self.kind == IntegrationKind::Skill {
            lines.push(format!(
                "{} 平台 · {}",
                if self.skill_platform_focused { ">" } else { " " },
                self.skill_provider.label()
            ));
        }
        lines.extend(self.items.iter().enumerate().map(|(index, item)| {
            let status = if self.kind == IntegrationKind::Skill && !self.can_toggle {
                "installed"
            } else if item.enabled {
                "enabled"
            } else {
                "disabled"
            };
            format!(
                "{} {} · {} · {}",
                if !self.skill_platform_focused && index == self.selected { ">" } else { " " },
                item.title,
                status,
                sanitize_integration_text(&item.detail)
            )
        }));
        lines
    }

    /// 切换技能目录平台，并重置子选择器和条目选中位置。
    pub(crate) fn set_skill_provider(&mut self, provider: SkillsDirectoryProvider) {
        self.skill_provider = provider;
        self.can_toggle = provider == SkillsDirectoryProvider::Vibewindow;
        self.skill_platform_focused = true;
        self.skill_provider_picker = None;
        self.selected = 0;
    }

    /// 仅在技能面板中打开平台选择器。
    pub(crate) fn open_skill_provider_picker(&mut self) -> bool {
        if self.kind != IntegrationKind::Skill {
            return false;
        }
        let selected = SkillsDirectoryProvider::ALL
            .iter()
            .position(|provider| *provider == self.skill_provider)
            .unwrap_or_default();
        self.skill_provider_picker = Some(SkillProviderPicker { selected });
        true
    }

    pub(crate) fn cancel_skill_provider_picker(&mut self) -> bool {
        self.skill_provider_picker.take().is_some()
    }

    pub(crate) fn move_skill_provider_picker_selection(&mut self, delta: isize) {
        let Some(picker) = self.skill_provider_picker.as_mut() else {
            return;
        };
        let len = SkillsDirectoryProvider::ALL.len();
        picker.selected = if delta < 0 {
            picker.selected.checked_sub(1).unwrap_or(len.saturating_sub(1))
        } else {
            (picker.selected + 1) % len
        };
    }

    pub(crate) fn selected_skill_provider(&self) -> Option<SkillsDirectoryProvider> {
        self.skill_provider_picker
            .as_ref()
            .and_then(|picker| SkillsDirectoryProvider::ALL.get(picker.selected))
            .copied()
    }
}

/// 折叠空白并遮蔽常见凭据形式，供集成详情与错误文本入界面前使用。
pub(crate) fn sanitize_integration_text(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            let normalized = word.to_ascii_lowercase();
            if normalized.contains("token=")
                || normalized.contains("secret=")
                || normalized.contains("authorization=")
            {
                "[redacted]"
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
use vw_config_types::skills::SkillsDirectoryProvider;
