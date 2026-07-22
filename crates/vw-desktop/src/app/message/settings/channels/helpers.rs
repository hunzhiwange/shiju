//! 处理渠道设置子模块的状态变更、字段转换和持久化。

use crate::app::message::settings::util::parse_comma_or_newline_list;
use vw_config_types::channels::{
    DingTalkConfig, FeishuConfig, GroupReplyConfig, GroupReplyMode, LarkReceiveMode, MatrixConfig,
    QQReceiveMode,
};

pub(super) fn default_matrix_config() -> MatrixConfig {
    MatrixConfig {
        homeserver: String::new(),
        access_token: String::new(),
        user_id: None,
        device_id: None,
        room_id: String::new(),
        allowed_users: Vec::new(),
        mention_only: false,
    }
}

/// 处理 `default_feishu_config` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub(super) fn default_feishu_config() -> FeishuConfig {
    FeishuConfig {
        app_id: String::new(),
        app_secret: String::new(),
        encrypt_key: None,
        verification_token: None,
        allowed_users: Vec::new(),
        group_reply: None,
        receive_mode: LarkReceiveMode::Websocket,
        port: None,
        draft_update_interval_ms: 3000,
        max_draft_edits: 20,
    }
}

pub(super) fn default_dingtalk_config() -> DingTalkConfig {
    DingTalkConfig {
        client_id: String::new(),
        client_secret: String::new(),
        allowed_users: Vec::new(),
    }
}

#[cfg(test)]
#[path = "helpers_tests.rs"]
mod helpers_tests;

/// 处理 `trim_to_option` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回 `None` 表示输入为空或当前状态不需要生成后续值。
pub(super) fn trim_to_option(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

/// 处理 `parse_receive_mode` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub(super) fn parse_receive_mode(value: &str) -> LarkReceiveMode {
    match value.trim().to_ascii_lowercase().as_str() {
        "webhook" => LarkReceiveMode::Webhook,
        _ => LarkReceiveMode::Websocket,
    }
}

/// 处理 `parse_qq_receive_mode` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub(super) fn parse_qq_receive_mode(value: &str) -> QQReceiveMode {
    match value.trim().to_ascii_lowercase().as_str() {
        "websocket" => QQReceiveMode::Websocket,
        _ => QQReceiveMode::Webhook,
    }
}

/// 处理 `set_group_reply_mode` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub(super) fn set_group_reply_mode(group_reply: &mut Option<GroupReplyConfig>, value: &str) {
    let mode = match value.trim().to_ascii_lowercase().as_str() {
        "mention_only" => Some(GroupReplyMode::MentionOnly),
        _ => Some(GroupReplyMode::AllMessages),
    };
    let config = group_reply.get_or_insert_with(GroupReplyConfig::default);
    config.mode = mode;
}

/// 处理 `set_group_reply_allowed` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
pub(super) fn set_group_reply_allowed(group_reply: &mut Option<GroupReplyConfig>, value: &str) {
    let config = group_reply.get_or_insert_with(GroupReplyConfig::default);
    config.allowed_sender_ids = parse_comma_or_newline_list(value);
}
