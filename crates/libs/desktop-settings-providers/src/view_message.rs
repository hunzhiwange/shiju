use crate::state::{CustomProviderModelTab};
#[derive(Debug, Clone)]
pub enum ModelRoutesMessage { AddRoute, RemoveRoute(usize), PatternChanged(usize,String), ProviderChanged(usize,String), ModelChanged(usize,String), PriorityChanged(usize,String) }
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    EmbeddingRoutes(crate::embedding_view_message::EmbeddingRoutesMessage),
    CustomProviderModelClose,
    CustomProviderModelModalAttachmentChanged(bool),
    CustomProviderModelModalContextLimitChanged(String),
    CustomProviderModelModalFamilyChanged(String),
    CustomProviderModelModalIdChanged(String),
    CustomProviderModelModalInputLimitChanged(String),
    CustomProviderModelModalInputModalityToggled(String, bool),
    CustomProviderModelModalInterleavedChanged(bool),
    CustomProviderModelModalNameChanged(String),
    CustomProviderModelModalOutputLimitChanged(String),
    CustomProviderModelModalOutputModalityToggled(String, bool),
    CustomProviderModelModalReasoningChanged(bool),
    CustomProviderModelModalSave,
    CustomProviderModelModalTemperatureChanged(bool),
    CustomProviderModelModalToolCallChanged(bool),
    CustomProviderModelTabSelected(CustomProviderModelTab),
    ModelDetailClose,
    ModelDetailOpen(String, String),
    ModelDetailToggleRaw,
    ModelQueryChanged(String),
    ModelRoutes(ModelRoutesMessage),
    ModelToggle(String, String, bool),
    ModelsRefresh,
    ModelsSetAll(bool),
    ProviderModelsExpansionToggled(String),
    ProviderModelsReset(String),
    ProviderModelsSetAll(String, bool),
}
#[derive(Debug, Clone)]
pub enum Message { Settings(SettingsMessage), CopyCode(String), None }
