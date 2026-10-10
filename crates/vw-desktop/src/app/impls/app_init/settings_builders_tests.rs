#![allow(unused_imports, dead_code)]

//! 验证设置构建辅助函数对配置值和默认值的映射。
//!
//! 构建器使用独立配置片段生成设置状态，重点检查默认值与路径字段的所有权转换。

use std::path::PathBuf;

use vw_config_types::channels::{
    ChannelsConfig, ClawdTalkConfig, DingTalkConfig, DingTalkInstanceConfig, DiscordConfig,
    FeishuConfig, FeishuInstanceConfig, GroupReplyConfig, IMessageConfig, IrcConfig, LarkConfig,
    LinqConfig, MattermostConfig, NextcloudTalkConfig,
    NostrConfig, QQConfig, SignalConfig, SlackConfig, TelegramConfig, WatiConfig, WebhookConfig,
    WhatsAppConfig,
};
use vw_config_types::staff::{StaffConfig, StaffMemberConfig};

#[test]
fn build_channels_settings_maps_empty_config_to_default_state() {
    let settings = super::build_channels_settings(&ChannelsConfig::default());

    assert!(settings.cli);
    assert_eq!(settings.project_dir_input, "");
    assert_eq!(settings.message_timeout_secs, 300);
    assert_eq!(settings.active_tab, super::super::state::ChannelsSettingsTab::Global);
    assert!(settings.text_inputs.is_empty());
    assert!(settings.text_editors.is_empty());
    assert!(settings.save_error.is_none());
}

#[test]
fn build_channels_settings_saturates_message_timeout_at_u32_max() {
    let config = ChannelsConfig { message_timeout_secs: u64::MAX, ..ChannelsConfig::default() };

    let settings = super::build_channels_settings(&config);

    assert_eq!(settings.message_timeout_secs, u32::MAX);
}

#[test]
fn build_channels_settings_preserves_zero_and_u32_max_message_timeout() {
    let zero = ChannelsConfig { message_timeout_secs: 0, ..ChannelsConfig::default() };
    let max = ChannelsConfig { message_timeout_secs: u32::MAX as u64, ..ChannelsConfig::default() };

    assert_eq!(super::build_channels_settings(&zero).message_timeout_secs, 0);
    assert_eq!(super::build_channels_settings(&max).message_timeout_secs, u32::MAX);
}

#[test]
fn build_channels_settings_preserves_project_dir_and_disabled_cli() {
    let config = ChannelsConfig {
        cli: false,
        project_dir: Some(PathBuf::from("/tmp/vibe-window/project")),
        ..ChannelsConfig::default()
    };

    let settings = super::build_channels_settings(&config);

    assert!(!settings.cli);
    assert_eq!(settings.project_dir_input, "/tmp/vibe-window/project");
    assert_eq!(settings.project_dir(), Some(PathBuf::from("/tmp/vibe-window/project")));
}



#[test]
fn build_staff_settings_uses_full_agent_staff_config() {
    let mut full_agent_cfg = vw_config_types::config::Config::default();
    full_agent_cfg.staff = StaffConfig {
        members: vec![StaffMemberConfig {
            id: "planner".to_string(),
            name: "Planner".to_string(),
            role: "planner".to_string(),
            level: "manager".to_string(),
            status: "available".to_string(),
            enabled: true,
            capacity: 2,
            delegate_agent: Some("main".to_string()),
            acp_agent: None,
            ipc_agent_id: None,
            channel_bindings: Vec::new(),
            capabilities: Default::default(),
        }],
        ..StaffConfig::default()
    };

    let settings = super::build_staff_settings(&full_agent_cfg);

    assert_eq!(settings.config, full_agent_cfg.staff);
    assert_eq!(settings.selected_member_id.as_deref(), Some("planner"));
    assert_eq!(settings.member_drafts.len(), 1);
    assert_eq!(settings.capability_drafts.len(), 1);
    assert!(!settings.loading);
}

mod channel_instance_settings {
    use super::*;

}


fn group_reply(sender_id: &str) -> GroupReplyConfig {
    GroupReplyConfig { mode: None, allowed_sender_ids: vec![sender_id.to_string()] }
}

#[cfg(not(target_arch = "wasm32"))]
fn email_config() -> vw_config_types::channels::EmailConfig {
    vw_config_types::channels::EmailConfig {
        imap_host: "imap.test".to_string(),
        smtp_host: "smtp.test".to_string(),
        username: "mail-user".to_string(),
        password: "mail-password".to_string(),
        from_address: "bot@example.com".to_string(),
        allowed_senders: vec!["sender@example.com".to_string()],
        ..Default::default()
    }
}
