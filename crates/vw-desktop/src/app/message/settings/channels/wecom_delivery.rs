//! 将宿主状态与任务映射到渠道领域。
use crate::app::{App, Message};
use super::super::messages::ChannelsMessage;
use iced::Task;

pub(super) fn update(app: &mut App, message: ChannelsMessage) -> Task<Message> {
    let services = super::updates::services(app);
    desktop_settings_channels::update::wecom_delivery::update(&mut app.channels_settings, &services, message)
        .map(super::updates::map_effect)
}
