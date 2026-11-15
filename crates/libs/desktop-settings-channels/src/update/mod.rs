//! 渠道设置领域更新；宿主提供网关客户端和有序项目候选。
mod changes;
mod text_changes;
pub mod execution;
mod helpers;
pub mod persist;
pub mod projects;
mod toggles;
pub mod updates;
use crate::messages::ChannelsMessage;
use iced::Task;

pub struct Services {
    pub config: desktop_config::ConfigAccess,
    pub client: Result<vw_gateway_client::GatewayClient, String>,
    pub project_choices: Vec<(String, String)>,
}
#[derive(Debug, Clone)]
pub enum ToastKind { Error, Warning, Success }
#[derive(Debug, Clone)]
pub enum Effect {
    Channels(ChannelsMessage),
    Saved { tag: &'static str, result: Result<(), String> },
    WeComDelivery(ChannelsMessage),
    Toast(ToastKind, String, Vec<String>),
    Error(String),
    After(std::time::Duration, ChannelsMessage),
}
fn error_toast(error: String) -> Task<Effect> { Task::done(Effect::Error(error)) }
fn localized_toast(kind: ToastKind, text: &str, args: Vec<String>) -> Task<Effect> {
    Task::done(Effect::Toast(kind, text.into(), args))
}
fn parse_comma_or_newline_list(value: &str) -> Vec<String> {
    value.split([',', '\n']).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

pub mod wecom_delivery;
#[cfg(test)]
mod wecom_delivery_tests;
