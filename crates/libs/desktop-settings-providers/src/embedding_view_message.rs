#[derive(Debug, Clone)]
pub enum EmbeddingRoutesMessage {
    AddRoute,
    InsertAlibabaTemplate,
    InsertDoubaoTemplate,
    InsertShijuTemplate,
    RemoveRoute(usize),
    PatternChanged(usize, String),
    ProviderChanged(usize, String),
    ModelChanged(usize, String),
    DimensionsChanged(usize, String),
    ApiKeyChanged(usize, String),
    Save,
}
