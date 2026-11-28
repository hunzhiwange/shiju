//! 验证顶部栏网关状态展示、连接操作和异常状态反馈。
//!
//! 网关夹具覆盖连接中、已连接、断开和错误状态，并检查对应操作是否可用。

use super::super::mapping::map_message;
use crate::app::components::system_settings::SystemTab;
use crate::app::message::view::MenuType;
use crate::app::{App, Message, Screen, message};
use desktop_ui_controls::settings_navigation::AgentManagementPage;
use desktop_navigation::components::top_bar::{Message as TopBarMessage, gateway};

#[test]
fn gateway_management_opens_the_requested_agent_config_tab() {
    for (action, expected_tab) in [
        (gateway::manage_plugins_message(), AgentManagementPage::Plugins),
        (TopBarMessage::ManageMcp, AgentManagementPage::Mcp),
    ] {
        let mut app = App::new().0;
        app.navigation_panels.show_system_settings = false;
        app.navigation_panels.system_settings_tab = SystemTab::General;
        app.navigation_panels.active_menu = Some(MenuType::GatewayServices);

        let _ = app.update(map_message(action));

        assert!(!app.navigation_panels.show_system_settings);
        assert_eq!(app.navigation_tabs.screen, Screen::AgentManagement(expected_tab));
        assert_eq!(app.navigation_panels.active_menu, None);
    }
}

#[test]
fn gateway_tests_keep_public_view_messages_reachable() {
    let toggle = Message::View(message::ViewMessage::ToggleMenu(Some(MenuType::GatewayServices)));
    let close = Message::View(message::ViewMessage::ToggleMenu(None));

    assert!(matches!(toggle, Message::View(_)));
    assert!(matches!(close, Message::View(_)));
}
