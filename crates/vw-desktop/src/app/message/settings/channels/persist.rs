//! 处理渠道设置子模块的状态变更、字段转换和持久化。

use crate::app::config::update_channels_config_async;
use crate::app::{App, Message, state::ChannelsSettingsState};
use iced::Task;
use vw_config_types::channels::ChannelsConfig;

fn enabled_channels(settings: &ChannelsSettingsState) -> Vec<&'static str> {
    let mut channels = Vec::new();
    if settings.cli {
        channels.push("CLI");
    }
    if settings.telegram.is_some() {
        channels.push("Telegram");
    }
    if settings.discord.is_some() {
        channels.push("Discord");
    }
    if settings.slack.is_some() {
        channels.push("Slack");
    }
    if settings.mattermost.is_some() {
        channels.push("Mattermost");
    }
    if settings.webhook.is_some() {
        channels.push("Webhook");
    }
    if settings.imessage.is_some() {
        channels.push("iMessage");
    }
    if settings.signal.is_some() {
        channels.push("Signal");
    }
    if settings.whatsapp.is_some() {
        channels.push("WhatsApp");
    }
    if settings.linq.is_some() {
        channels.push("Linq");
    }
    if settings.wati.is_some() {
        channels.push("WATI");
    }
    if settings.nextcloud_talk.is_some() {
        channels.push("Nextcloud Talk");
    }
    #[cfg(not(target_arch = "wasm32"))]
    if settings.email.is_some() {
        channels.push("Email");
    }
    if settings.irc.is_some() {
        channels.push("IRC");
    }
    if settings.lark.is_some() {
        channels.push("Lark");
    }
    if settings.feishu.is_some() || !settings.feishu_instances.is_empty() {
        channels.push("飞书");
    }
    if settings.dingtalk.is_some() || !settings.dingtalk_instances.is_empty() {
        channels.push("钉钉");
    }
    if settings.qq.is_some() {
        channels.push("QQ");
    }
    if settings.wecom.is_some() {
        channels.push("企业微信");
    }
    if settings.wechat.is_some() {
        channels.push("微信");
    }
    if settings.nostr.is_some() {
        channels.push("Nostr");
    }
    if settings.clawdtalk.is_some() {
        channels.push("ClawdTalk");
    }
    channels
}

#[cfg(test)]
#[path = "persist_tests.rs"]
mod persist_tests;

pub(super) fn build_channels_config_from_settings(
    settings: &ChannelsSettingsState,
) -> Result<ChannelsConfig, String> {
    let channels = ChannelsConfig {
        project_dir: settings.project_dir(),
        instance_default_projects: Default::default(),
        cli: settings.cli,
        telegram: settings.telegram.clone(),
        discord: settings.discord.clone(),
        slack: settings.slack.clone(),
        mattermost: settings.mattermost.clone(),
        webhook: settings.webhook.clone(),
        imessage: settings.imessage.clone(),
        signal: settings.signal.clone(),
        whatsapp: settings.whatsapp.clone(),
        linq: settings.linq.clone(),
        wati: settings.wati.clone(),
        nextcloud_talk: settings.nextcloud_talk.clone(),
        #[cfg(not(target_arch = "wasm32"))]
        email: settings.email.clone(),
        irc: settings.irc.clone(),
        lark: settings.lark.clone(),
        feishu: settings.feishu.clone(),
        feishu_instances: settings.feishu_instances.clone(),
        dingtalk: settings.dingtalk.clone(),
        dingtalk_instances: settings.dingtalk_instances.clone(),
        qq: settings.qq.clone(),
        wecom: settings.wecom.clone(),
        wechat: settings.wechat.clone(),
        nostr: settings.nostr.clone(),
        clawdtalk: settings.clawdtalk.clone(),
        message_timeout_secs: settings.message_timeout_secs.max(1) as u64,
    };
    channels.resolve_channel_instance_profiles().map_err(|err| err.to_string())?;
    Ok(channels)
}

/// 处理 `persist_channels_settings` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
/// 返回 `None` 表示输入为空或当前状态不需要生成后续值。
pub(super) fn persist_channels_settings(app: &mut App) -> Option<Task<Message>> {
    let settings = &app.channels_settings;

    let enabled = enabled_channels(settings);
    if enabled.is_empty() {
        app.channels_settings.save_error =
            Some("当前未启用任何通道。至少保留一个可用入口可避免运行时无消息源。".to_string());
        return None;
    }

    let mut channels_config = match build_channels_config_from_settings(settings) {
        Ok(channels_config) => channels_config,
        Err(err) => {
            app.channels_settings.save_error = Some(err);
            return None;
        }
    };

    app.channels_settings.save_error = None;
    Some(update_channels_config_async(move |channels| {
        channels_config.instance_default_projects.clone_from(&channels.instance_default_projects);
        *channels = channels_config;
    }))
}
