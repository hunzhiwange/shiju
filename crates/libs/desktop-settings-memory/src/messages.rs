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
