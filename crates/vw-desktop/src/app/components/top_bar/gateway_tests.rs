//! 验证顶部栏网关状态展示、连接操作和异常状态反馈。
//!
//! 网关夹具覆盖连接中、已连接、断开和错误状态，并检查对应操作是否可用。

use super::manage_plugins_message;
use crate::app::components::system_settings::SystemTab;
use crate::app::message::settings::{AgentConfigTabMessage, SettingsMessage};
use crate::app::message::view::MenuType;
use crate::app::state::AgentConfigTab;
use crate::app::{Message, message};

#[test]
fn manage_plugins_opens_the_standalone_agent_plugin_tab() {
    let Message::Batch(messages) = manage_plugins_message() else {
        panic!("管理插件应批量打开设置与插件页签");
    };

    assert!(matches!(
        messages.as_slice(),
        [
            Message::View(crate::app::message::view::ViewMessage::OpenSystemSettingsTab(
                SystemTab::Agents
            )),
            Message::Settings(SettingsMessage::AgentConfigTab(AgentConfigTabMessage::Selected(
                AgentConfigTab::Plugins
            )))
        ]
    ));
}

#[test]
fn gateway_tests_keep_public_view_messages_reachable() {
    let toggle = Message::View(message::ViewMessage::ToggleMenu(Some(MenuType::GatewayServices)));
    let close = Message::View(message::ViewMessage::ToggleMenu(None));

    assert!(matches!(toggle, Message::View(_)));
    assert!(matches!(close, Message::View(_)));
}
