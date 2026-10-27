//! 运行期按渠道实例隔离解密失败，源配置保留密文以便修复和再次保存。

use super::{ChannelsConfig, decrypt_channel_secrets};
use crate::app::agent::security::SecretStore;
use anyhow::Result;

pub(crate) fn decrypt_channel_secrets_isolated(
    store: &SecretStore,
    channels: &mut ChannelsConfig,
) -> Result<()> {
    channels.resolve_channel_instance_profiles()?;
    channels.unavailable_channels.clear();
    decrypt_channel(store, channels, "telegram:default", |config| &mut config.telegram);
    decrypt_channel(store, channels, "discord:default", |config| &mut config.discord);
    decrypt_channel(store, channels, "slack:default", |config| &mut config.slack);
    decrypt_channel(store, channels, "mattermost:default", |config| &mut config.mattermost);
    decrypt_channel(store, channels, "webhook:default", |config| &mut config.webhook);
    decrypt_channel(store, channels, "whatsapp:default", |config| &mut config.whatsapp);
    decrypt_channel(store, channels, "linq:default", |config| &mut config.linq);
    decrypt_channel(store, channels, "nextcloud_talk:default", |config| &mut config.nextcloud_talk);
    decrypt_channel(store, channels, "irc:default", |config| &mut config.irc);
    decrypt_channel(store, channels, "lark:default", |config| &mut config.lark);
    decrypt_channel(store, channels, "dingtalk:default", |config| &mut config.dingtalk);
    decrypt_channel(store, channels, "qq:default", |config| &mut config.qq);
    decrypt_channel(store, channels, "wecom:default", |config| &mut config.wecom);
    decrypt_channel(store, channels, "wechat:default", |config| &mut config.wechat);
    decrypt_channel(store, channels, "nostr:default", |config| &mut config.nostr);
    decrypt_channel(store, channels, "clawdtalk:default", |config| &mut config.clawdtalk);
    for instance in &mut channels.feishu_instances {
        let mut isolated = ChannelsConfig {
            feishu_instances: vec![instance.clone()],
            ..ChannelsConfig::default()
        };
        if decrypt_channel_secrets(store, &mut isolated).is_ok() {
            *instance = isolated.feishu_instances.remove(0);
        } else {
            record_unavailable(
                &mut channels.unavailable_channels,
                &format!("feishu:{}", instance.id),
            );
        }
    }
    for instance in &mut channels.dingtalk_instances {
        let mut isolated = ChannelsConfig {
            dingtalk_instances: vec![instance.clone()],
            ..ChannelsConfig::default()
        };
        if decrypt_channel_secrets(store, &mut isolated).is_ok() {
            *instance = isolated.dingtalk_instances.remove(0);
        } else {
            record_unavailable(
                &mut channels.unavailable_channels,
                &format!("dingtalk:{}", instance.id),
            );
        }
    }
    Ok(())
}

fn decrypt_channel<T: Clone>(
    store: &SecretStore,
    channels: &mut ChannelsConfig,
    profile: &str,
    field: fn(&mut ChannelsConfig) -> &mut Option<T>,
) {
    if field(channels).is_none() {
        return;
    }
    let mut isolated = ChannelsConfig::default();
    *field(&mut isolated) = field(channels).clone();
    if decrypt_channel_secrets(store, &mut isolated).is_ok() {
        *field(channels) = field(&mut isolated).take();
    } else {
        record_unavailable(&mut channels.unavailable_channels, profile);
    }
}

fn record_unavailable(unavailable: &mut std::collections::BTreeSet<String>, profile: &str) {
    unavailable.insert(profile.to_owned());
    // 不输出底层错误，避免错误链中包含原始凭据。
    tracing::warn!(
        channel = profile,
        "Channel credentials could not be decrypted; channel disabled until repaired"
    );
}
