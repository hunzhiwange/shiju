#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegrationKind {
    Mcp,
    Skill,
    Plugin,
    Hook,
    Memory,
    Doctor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IntegrationLoadState {
    Loading,
    Empty,
    Ready,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrationAction {
    pub(crate) label: String,
    pub(crate) enabled: bool,
    pub(crate) disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrationItem {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) detail: String,
    pub(crate) enabled: bool,
    pub(crate) actions: Vec<IntegrationAction>,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillProviderPicker {
    pub(crate) selected: usize,
}

impl IntegrationPanel {
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

    pub(crate) fn begin_refresh(&mut self) {
        self.state = IntegrationLoadState::Loading;
    }

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

    pub(crate) fn set_skill_provider(&mut self, provider: SkillsDirectoryProvider) {
        self.skill_provider = provider;
        self.can_toggle = provider == SkillsDirectoryProvider::Vibewindow;
        self.skill_platform_focused = true;
        self.skill_provider_picker = None;
        self.selected = 0;
    }

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
