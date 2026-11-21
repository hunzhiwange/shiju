//! MCP、技能、插件等集成页面的视图模型。
//!
//! 本模块统一加载状态、选中项和技能平台选择，但不直接调用安装或启停 API。
//! 进入渲染的外部错误会经过最小脱敏，避免直接显示常见凭据字段。

/// 集成面板当前展示的资源类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntegrationKind {
    Mcp,
    Skill,
    Plugin,
    Hook,
    Memory,
    Doctor,
}

/// 集成列表的异步加载结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationLoadState {
    Loading,
    Empty,
    Ready,
    Error(String),
}

/// 某条集成可执行的界面操作及其可用性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationAction {
    pub label: String,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

/// 一条集成资源的可显示快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationItem {
    pub kind: IntegrationKind,
    pub id: String,
    pub title: String,
    pub detail: String,
    pub source: String,
    pub enabled: bool,
    pub can_toggle: bool,
    pub can_delete: bool,
    pub prompt_prefix: Option<String>,
    pub actions: Vec<IntegrationAction>,
}

impl IntegrationItem {
    pub fn stable_key(&self) -> String {
        format!("{}:{}", self.kind.command_name(), self.id)
    }
}

impl IntegrationKind {
    pub fn command_name(self) -> &'static str {
        match self {
            Self::Mcp => "mcp",
            Self::Skill => "skill",
            Self::Plugin => "plugin",
            Self::Hook => "hook",
            Self::Memory => "memory",
            Self::Doctor => "doctor",
        }
    }
}

/// 集成页面的列表和选中位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationPanel {
    pub kind: IntegrationKind,
    pub state: IntegrationLoadState,
    pub items: Vec<IntegrationItem>,
    pub selected: usize,
}

impl IntegrationPanel {
    /// 以指定类别创建面板，初始状态为正在加载。
    pub fn new(kind: IntegrationKind) -> Self {
        Self {
            kind,
            state: IntegrationLoadState::Loading,
            items: Vec::new(),
            selected: 0,
        }
    }

    /// 将已有内容标记为正在刷新，便于界面保留上一次列表。
    #[cfg(test)]
    pub(crate) fn begin_refresh(&mut self) {
        self.state = IntegrationLoadState::Loading;
    }

    /// 应用刷新结果，成功时按稳定 ID 保留选中项，失败时仅保存脱敏错误。
    #[cfg(test)]
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
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![format!("{:?} · {:?}", self.kind, self.state)];
        lines.extend(self.items.iter().enumerate().map(|(index, item)| {
            let status = if item.enabled {
                "enabled"
            } else {
                "disabled"
            };
            format!(
                "{} {} · {} · {}",
                if index == self.selected { ">" } else { " " },
                item.title,
                status,
                sanitize_integration_text(&item.detail)
            )
        }));
        lines
    }
}

/// 折叠空白并遮蔽常见凭据形式，供集成详情与错误文本入界面前使用。
pub fn sanitize_integration_text(text: &str) -> String {
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
