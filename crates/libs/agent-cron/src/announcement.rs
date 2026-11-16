//! 定时结果的通道路由与发送；企微认证连接通过宿主端口提供。
use agent_config::runtime::Config;
use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use channel_types::{Channel, SendMessage};
#[cfg(not(target_arch = "wasm32"))]
use {channel_telegram::TelegramChannel, channel_discord::DiscordChannel,
    channel_email_channel::EmailChannel, channel_mattermost::MattermostChannel,
    channel_qq::QQChannel};
#[cfg(not(target_arch = "wasm32"))]
pub async fn deliver_announcement(
    host: &dyn crate::delivery::DeliveryHost,
    config: &Config,
    channel: &str,
    target: &str,
    output: &str,
) -> Result<()> {
    let channel = channel.to_ascii_lowercase();
    if super::delivery::is_wecom(&channel) {
        return super::delivery::send(host, config, &channel, target, output).await;
    }
    if config.channels_config.unavailable_channels.contains(&format!("{channel}:default")) {
        anyhow::bail!(
            "channel credentials unavailable; repair channel configuration before delivery"
        );
    }
    // 根据通道类型路由到对应的实现
    match channel.as_str() {
        "telegram" => {
            // 获取 Telegram 配置
            let tg = config
                .channels_config
                .telegram
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("telegram channel not configured"))?;
            // 创建 Telegram 通道实例
            let channel = TelegramChannel::new(
                tg.bot_token.clone(),
                tg.allowed_users.clone(),
                tg.mention_only,
            )
            .with_workspace_dir(config.workspace_dir.clone());
            // 发送消息
            channel.send(&SendMessage::new(output, target)).await?;
        }
        "discord" => {
            // 获取 Discord 配置
            let dc = config
                .channels_config
                .discord
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("discord channel not configured"))?;
            // 创建 Discord 通道实例
            let channel = DiscordChannel::new(
                dc.bot_token.clone(),
                dc.guild_id.clone(),
                dc.allowed_users.clone(),
                dc.listen_to_bots,
                dc.mention_only,
            )
            .with_workspace_dir(config.workspace_dir.clone());
            // 发送消息
            channel.send(&SendMessage::new(output, target)).await?;
        }
        "slack" => {
            // 获取 Slack 配置
            let sl = config
                .channels_config
                .slack
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("slack channel not configured"))?;
            // 创建 Slack 通道实例
            let channel = channel_slack::SlackChannel::new(
                sl.bot_token.clone(),
                sl.channel_id.clone(),
                sl.allowed_users.clone(),
            );
            // 发送消息
            channel.send(&SendMessage::new(output, target)).await?;
        }
        "mattermost" => {
            // 获取 Mattermost 配置
            let mm = config
                .channels_config
                .mattermost
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("mattermost channel not configured"))?;
            // 创建 Mattermost 通道实例
            let channel = MattermostChannel::new(
                mm.url.clone(),
                mm.bot_token.clone(),
                mm.channel_id.clone(),
                mm.allowed_users.clone(),
                mm.thread_replies.unwrap_or(true),
                mm.mention_only.unwrap_or(false),
            );
            // 发送消息
            channel.send(&SendMessage::new(output, target)).await?;
        }
        "qq" => {
            // 获取 QQ 配置
            let qq = config
                .channels_config
                .qq
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("qq channel not configured"))?;
            // 创建 QQ 通道实例
            let channel =
                QQChannel::new(qq.app_id.clone(), qq.app_secret.clone(), qq.allowed_users.clone());
            // 发送消息
            channel.send(&SendMessage::new(output, target)).await?;
        }
        "email" => {
            // 获取邮件配置
            let email = config
                .channels_config
                .email
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("email channel not configured"))?;
            // 创建邮件通道实例
            let channel = EmailChannel::new(email.clone());
            // 发送邮件
            channel.send(&SendMessage::new(output, target)).await?;
        }
        other => anyhow::bail!("unsupported delivery channel: {other}"),
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn deliver_announcement(
    _host: &dyn crate::delivery::DeliveryHost,
    _config: &Config,
    _channel: &str,
    _target: &str,
    _output: &str,
) -> Result<()> {
    if super::delivery::is_wecom(_channel) {
        anyhow::bail!("企微投递不支持 WASM，请使用原生 daemon");
    }
    Ok(())
}
