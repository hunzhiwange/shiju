use super::types;
#[derive(Debug, Clone)]
pub enum Message {
 TabSelected(String), TabClosed(String), TabHovered(Option<String>), OpenApps,
 ToggleSystemSettings, ToggleSettingsPanel, ToggleDiffPanel, FileManagerPanelVisible(bool), WorkSidebarToggled, WorkResourcesToggled, ToggleTerminalPanel, Search, TaskPetToggleWindow, WindowDragPressed,
 OpenProjectInExternalPreferred, OpenProjectInExternalWith(types::ExternalOpenApp), CopyProjectPath,
 ToggleMenu(Option<Menu>), MenuHovered(Menu), MenuAction(Box<Message>),
 GatewayServicesTabSelected(types::TopBarGatewayTab), RefreshMcp, ManageMcp, PluginsRefresh, PluginsSetEnabledRequested { plugin_id: String, enabled: bool }, ManagePlugins, SelectServer(String), ManageGateway, AddGateway,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu { OpenExternal, GatewayServices }
