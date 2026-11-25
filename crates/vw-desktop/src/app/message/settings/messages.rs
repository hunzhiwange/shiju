//! Host specialization of the domain settings envelope.
pub use desktop_settings_shell::messages::{
    AcpMessage, AgentConfigTabMessage, AgentsMessage,
    BrowserMessage, ChannelsMessage, GatewayClientMessage, GatewayMessage,
    GeneralSettingsMessage, HooksMessage, HttpRequestMessage, McpMessage, MemoryMessage,
    ModelConfigTabMessage, ModelRoutesMessage, MultimodalMessage, QueryClassificationMessage,
    QuickMessagesMessage, RuntimeMessage, ThemesMessage, WebSearchMessage,
};
pub type SettingsMessage = desktop_settings_shell::messages::SettingsMessage<
    crate::app::SettingsTab,
    crate::app::components::system_settings_search::SystemSettingsSearchTarget,
    super::AppsCatalogMessage,
    crate::app::message::app_providers::list::ListMessage,
>;

#[cfg(test)]
pub use desktop_settings_shell::messages::EmbeddingRoutesMessage;
#[cfg(test)]
pub use desktop_settings_shell::messages::StorageMessage;
