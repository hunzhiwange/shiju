//! Host specialization of the domain settings envelope.
pub use desktop_settings_shell::messages::{
    AcpMessage, AgentConfigTabMessage, AgentsMessage, BashPolicyCheck, BashValidatorCheck,
    BrowserMessage, ChannelsMessage, EmbeddingRoutesMessage, GatewayClientMessage, GatewayMessage,
    GeneralSettingsMessage, HooksMessage, HttpRequestMessage, McpMessage, MemoryMessage,
    ModelConfigTabMessage, ModelRoutesMessage, MultimodalMessage, QueryClassificationMessage,
    QuickMessagesMessage, RuntimeMessage, StorageMessage, ThemeChromeField, ThemeColorPickerTarget,
    ThemePaletteField, ThemesMessage, TunnelMessage, WebSearchMessage,
};
pub type SettingsMessage = desktop_settings_shell::messages::SettingsMessage<
    crate::app::SettingsTab,
    crate::app::components::system_settings_search::SystemSettingsSearchTarget,
    super::AppsCatalogMessage,
    crate::app::message::app_providers::list::ListMessage,
>;
