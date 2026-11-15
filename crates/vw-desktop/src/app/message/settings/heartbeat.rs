//! 桌面壳：注入配置入口并映射领域结果。
use crate::app::{App, Message};
use super::messages::SettingsMessage;
use desktop_settings_cron::update::heartbeat as domain;
use iced::Task;
use super::cron::mapping;
pub fn update(app: &mut App, event: SettingsMessage) -> Task<Message> {

 let config = crate::app::config::agent_config_access();
 let task = domain::update(&mut domain::Inputs { heartbeat_settings: &mut app.heartbeat_settings, config: &config,  }, mapping::event(event)).map(mapping::message);
 task
}
