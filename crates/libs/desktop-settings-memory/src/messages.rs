use crate::state::MemorySettingsTab;
#[derive(Debug, Clone)]
pub enum MemoryMessage {
    Refresh,
    TabSelected(MemorySettingsTab),
    BackendChanged(String),
    AutoSaveToggled(bool),
    HygieneEnabledToggled(bool),
    ArchiveAfterDaysChanged(u32),
    PurgeAfterDaysChanged(u32),
    ConversationRetentionDaysChanged(u32),
    EmbeddingProviderChanged(String),
    EmbeddingModelChanged(String),
    EmbeddingDimensionsChanged(u32),
    VectorWeightChanged(f32),
    KeywordWeightChanged(f32),
    MinRelevanceScoreChanged(f32),
    EmbeddingCacheSizeChanged(u32),
    ChunkMaxTokensChanged(u32),
    ResponseCacheEnabledToggled(bool),
    ResponseCacheTtlMinutesChanged(u32),
    ResponseCacheMaxEntriesChanged(u32),
    SnapshotEnabledToggled(bool),
    SnapshotOnHygieneToggled(bool),
    AutoHydrateToggled(bool),
    SqliteOpenTimeoutSecsChanged(u32),
    QdrantUrlChanged(String),
    QdrantCollectionChanged(String),
    QdrantApiKeyChanged(String),
}
#[derive(Debug, Clone)]
pub enum StorageMessage {
    HelpCopy(String),
    HelpAppend(String),
    HelpOpenUrl(String),
    HelpSelection(String, usize, usize, Option<String>),
    None,
    ProviderChanged(String),
    DbUrlChanged(String),
    SchemaChanged(String),
    TableChanged(String),
    ConnectTimeoutSecsChanged(String),
    TlsToggled(bool),
    Save,
}
#[derive(Debug, Clone)]
pub enum PageMessage<E> {
    Memory(MemoryMessage),
    Storage(StorageMessage),
    External(E),
    None,
}

use desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage;
impl<E: SelectionMessage> SelectionMessage for PageMessage<E> {
    fn none() -> Self { Self::None }
    fn copy(text: String) -> Self { Self::External(E::copy(text)) }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self { Self::External(E::context(effect)) }
    fn selection(session: String, message: usize, text_index: usize, text: Option<String>) -> Self { Self::External(E::selection(session, message, text_index, text)) }
}
impl SelectionMessage for StorageMessage {
    fn none() -> Self { Self::None }
    fn copy(text: String) -> Self { Self::HelpCopy(text) }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self { match effect { desktop_ui_controls::text_editor_context_menu::TextContextEffect::Copy(text) => Self::HelpCopy(text), desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(text) => Self::HelpAppend(text), desktop_ui_controls::text_editor_context_menu::TextContextEffect::OpenUrl(url) => Self::HelpOpenUrl(url) } }
    fn selection(session: String, message: usize, text_index: usize, text: Option<String>) -> Self { Self::HelpSelection(session, message, text_index, text) }
}
