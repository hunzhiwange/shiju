pub use desktop_chat_tools::state::*;
/// Slash 面板条目类别，顺序固定为 Skills → Tools → MCP → Models → ACP。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SlashPaletteKind {
    Skill,
    Tool,
    Mcp,
    Model,
    Acp,
}

impl SlashPaletteKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Skill => "Skills",
            Self::Tool => "Tools",
            Self::Mcp => "MCP",
            Self::Model => "Models",
            Self::Acp => "智能体",
        }
    }

    pub(crate) fn all() -> [Self; 5] {
        [Self::Skill, Self::Tool, Self::Mcp, Self::Model, Self::Acp]
    }
}

#[derive(Debug, Clone, Default)]
/// 表示 SessionToolInventory 相关的应用状态或派生数据。
pub(crate) struct SessionToolInventory {
    pub(crate) base_tools: Vec<String>,
    pub(crate) restricts_tools: bool,
}
