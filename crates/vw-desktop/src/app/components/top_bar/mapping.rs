//! 顶栏局部动作到桌面跨域消息的映射。
use crate::app::Message;

pub(in crate::app::components) fn map_message(message: desktop_navigation::components::top_bar::Message) -> Message {
 use desktop_navigation::components::top_bar::Message as Local;
 use crate::app::message::ViewMessage as View;
 use crate::app::message::settings::{SettingsMessage, McpMessage, AgentConfigTabMessage, GatewayClientMessage};
 use crate::app::components::system_settings::SystemTab;
 use crate::app::state::AgentConfigTab;
 match message {
 Local::ToggleSystemSettings => Message::View(View::ToggleSystemSettings),
 Local::ToggleSettingsPanel => Message::View(View::ToggleSettingsPanel),
 Local::ToggleDiffPanel => Message::View(View::ToggleDiffPanel),
 Local::WorkSidebarToggled => Message::View(View::WorkSidebarToggled),
 Local::WorkResourcesToggled => Message::View(View::WorkResourcesToggled),
 Local::ToggleTerminalPanel => Message::View(View::ToggleTerminalPanel),
 Local::TaskPetToggleWindow => Message::View(View::TaskPetToggleWindow),
 Local::WindowDragPressed => Message::View(View::WindowDragPressed),
 Local::OpenProjectInExternalPreferred => Message::View(View::OpenProjectInExternalPreferred),
 Local::CopyProjectPath => Message::View(View::CopyProjectPath),
 Local::OpenApps => Message::View(View::OpenApps),
 Local::TabSelected(value) => Message::View(View::TabSelected(value)),
 Local::TabClosed(value) => Message::View(View::TabClosed(value)),
 Local::TabHovered(value) => Message::View(View::TabHovered(value)),
 Local::FileManagerPanelVisible(value) => Message::View(View::FileManagerPanelVisible(value)),
 Local::OpenProjectInExternalWith(value) => Message::View(View::OpenProjectInExternalWith(value)),
 Local::GatewayServicesTabSelected(value) => Message::View(View::GatewayServicesTabSelected(value)),
 Local::Search => Message::Search(crate::app::message::SearchMessage::Open),
 Local::ToggleMenu(menu) => Message::View(View::ToggleMenu(menu.map(map_menu))),
 Local::MenuHovered(menu) => Message::View(View::MenuHovered(map_menu(menu))),
 Local::MenuAction(action) => Message::View(View::MenuAction(Box::new(map_message(*action)))),
 Local::RefreshMcp => Message::Settings(SettingsMessage::Mcp(McpMessage::Refresh)),
 Local::ManageMcp => Message::Batch(vec![Message::View(View::OpenSystemSettingsTab(SystemTab::Agents)), Message::Settings(SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(AgentConfigTab::Mcp)))]),
 Local::PluginsRefresh => Message::Settings(SettingsMessage::PluginsRefresh),
 Local::PluginsSetEnabledRequested { plugin_id, enabled } => Message::Settings(SettingsMessage::PluginsSetEnabledRequested { plugin_id, enabled }),
 Local::ManagePlugins => Message::Batch(vec![Message::View(View::OpenSystemSettingsTab(SystemTab::Agents)), Message::Settings(SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(AgentConfigTab::Plugins)))]),
 Local::SelectServer(id) => Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::SelectServer(id))),
 Local::ManageGateway => Message::View(View::OpenSystemSettingsTab(SystemTab::GatewayClient)),
 Local::AddGateway => Message::Batch(vec![Message::Settings(SettingsMessage::GatewayClient(GatewayClientMessage::AddServer)),Message::View(View::OpenSystemSettingsTab(SystemTab::GatewayClient))]),
 }
}
pub(super) fn map_menu(menu: desktop_navigation::components::top_bar::Menu) -> crate::app::message::view::MenuType {
 use desktop_navigation::components::top_bar::Menu;
 use crate::app::message::view::MenuType;
 match menu { Menu::OpenExternal => MenuType::OpenExternal, Menu::GatewayServices => MenuType::GatewayServices }
}
pub(super) fn local_menu(menu: Option<crate::app::message::view::MenuType>) -> Option<desktop_navigation::components::top_bar::Menu> {
 use desktop_navigation::components::top_bar::Menu;
 use crate::app::message::view::MenuType;
 match menu { Some(MenuType::OpenExternal) => Some(Menu::OpenExternal), Some(MenuType::GatewayServices) => Some(Menu::GatewayServices), _ => None }
}
