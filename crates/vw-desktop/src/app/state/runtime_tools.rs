pub use desktop_chat_tools::state::*;
pub use desktop_session::message::chat::input::slash_palette::SlashPaletteKind;

#[derive(Debug, Clone, Default)]
/// 表示 SessionToolInventory 相关的应用状态或派生数据。
pub(crate) struct SessionToolInventory {
    pub(crate) base_tools: Vec<String>,
    pub(crate) restricts_tools: bool,
}
