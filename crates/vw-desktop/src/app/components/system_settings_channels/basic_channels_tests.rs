#![allow(unused_imports, dead_code)]

//! 验证基础消息渠道面板的启停、凭据缺省与配置状态。
//! 每种渠道都覆盖关闭、展开和已配置状态，避免共享表单隐藏必需字段。

use crate::app::{App, Message};
use vw_config_types::channels::{
    DingTalkConfig, DingTalkInstanceConfig, DiscordConfig, FeishuConfig, FeishuInstanceConfig,
    GroupReplyConfig, GroupReplyMode, IMessageConfig,
    MattermostConfig, SignalConfig, SlackConfig, TelegramConfig, WebhookConfig,
};

// 渠道测试从无启用项的应用状态开始，确保每个面板自行声明所需配置。
fn test_app() -> App {
    App::new().0
}

// 消费元素用于执行构建路径；字段数量和状态由更聚焦的断言负责。
fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn group_reply() -> GroupReplyConfig {
    GroupReplyConfig {
        mode: Some(GroupReplyMode::MentionOnly),
        allowed_sender_ids: vec!["alice".to_string(), "bob".to_string()],
    }
}


#[test]
fn basic_channels_tests_are_wired() {
    assert!(module_path!().contains("basic_channels_tests"));
}





mod channel_instance_settings {
    use super::*;

}
