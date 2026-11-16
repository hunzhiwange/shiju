//! 设置分类与真实设置详情的移动入口。
use crate::app::{App, Message, components::system_settings::{self, SystemTab}, message::ViewMessage};
use iced::Element;
pub fn view(app: &App) -> Element<'_, Message> {
    if app.mobile_layout.settings_detail { return system_settings::view(app); }
    let mut categories = iced::widget::column![].spacing(8);
    for (label, tab) in [("常规", SystemTab::General), ("网关", SystemTab::GatewayClient),
        ("项目", SystemTab::Projects), ("模型", SystemTab::Models), ("智能体", SystemTab::Agents)] {
        categories = categories.push(super::action(label, ViewMessage::MobileSettingsTab(tab)));
    }
    categories.into()
}
