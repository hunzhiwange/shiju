//! 渠道配置敏感信息加密/解密模块
//!
//! 本模块提供了对各个通信渠道配置中敏感信息（如令牌、密钥、密码等）进行
//! 加密和解密的功能。这是安全配置管理的关键组件，确保敏感数据在存储时
//! 得到保护。
//!
//! # 主要功能
//!
//! - **解密**：将加密存储的敏感信息还原为明文，供运行时使用
//! - **加密**：将明文敏感信息加密存储，保护配置安全
//!
//! # 支持的渠道
//!
//! 本模块处理以下渠道的敏感信息：
//! - **Telegram**：机器人令牌（bot_token）
//! - **Discord**：机器人令牌（bot_token）
//! - **Slack**：机器人令牌（bot_token）和应用程序令牌（app_token）
//! - **Mattermost**：机器人令牌（bot_token）
//! - **Webhook**：Webhook 密钥（secret）
//! - **WhatsApp**：访问令牌、应用密钥、验证令牌
//! - **Linq**：API 令牌和签名密钥
//! - **Nextcloud Talk**：应用令牌和 Webhook 密钥
//! - **IRC**：服务器密码、NickServ 密码、SASL 密码
//! - **Lark（飞书）**：应用密钥、加密密钥、验证令牌
//! - **DingTalk（钉钉）**：客户端密钥
//! - **QQ**：应用密钥
//! - **Nostr**：私钥
//! - **ClawdTalk**：API 密钥和 Webhook 密钥
//!
//! # 安全考虑
//!
//! - 使用 `SecretStore` 进行安全的加密/解密操作
//! - 敏感字段使用可选加密/解密，支持空值处理
//! - 每个字段都有唯一的配置路径标识，用于错误追踪

use crate::app::agent::config::schema::ChannelsConfig;
use crate::app::agent::config::schema::channels::helpers::{
    decrypt_optional_secret, decrypt_secret, encrypt_optional_secret, encrypt_secret,
};
use crate::app::agent::security::SecretStore;
use anyhow::Result;

fn channel_instance_secret_path(channel_type: &str, instance_id: &str, field_name: &str) -> String {
    format!("config.channels_config.{channel_type}_instances.{instance_id}.{field_name}")
}

/// 解密所有渠道配置中的敏感信息
///
/// 此函数遍历 `ChannelsConfig` 中的各个渠道配置，对每个渠道的敏感字段
/// 进行解密操作。解密后的明文值将替换原有的加密值。
///
/// # 参数
///
/// - `store`：安全存储实例，提供解密功能
/// - `channels`：可变的渠道配置引用，解密后的值将直接写入此配置
///
/// # 返回值
///
/// - `Ok(())`：所有敏感信息解密成功
/// - `Err(e)`：解密过程中发生错误
///
/// # 错误处理
///
/// 当解密失败时，函数会返回错误，包含具体的配置路径信息，
/// 便于定位问题所在。
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::security::SecretStore;
/// use crate::app::agent::config::schema::ChannelsConfig;
///
/// let store = SecretStore::new(encryption_key);
/// let mut config: ChannelsConfig = load_config()?;
///
/// // 解密所有渠道的敏感信息
/// decrypt_channel_secrets(&store, &mut config)?;
///
/// // 现在可以安全地使用 config 中的明文凭证
/// ```
///
/// # 安全说明
///
/// - 解密后的明文值仅在内存中存在，不应被持久化
/// - 解密后的配置应谨慎处理，避免泄露
pub fn decrypt_channel_secrets(store: &SecretStore, channels: &mut ChannelsConfig) -> Result<()> {
    channels.resolve_channel_instance_profiles()?;

    // 解密 Telegram 渠道配置
    // bot_token 是必填项，使用 decrypt_secret 处理
    if let Some(ref mut telegram) = channels.telegram {
        decrypt_secret(
            store,
            &mut telegram.bot_token,
            "config.channels_config.telegram.bot_token",
        )?;
    }

    // 解密 Discord 渠道配置
    // bot_token 是必填项，使用 decrypt_secret 处理
    if let Some(ref mut discord) = channels.discord {
        decrypt_secret(store, &mut discord.bot_token, "config.channels_config.discord.bot_token")?;
    }

    // 解密 Slack 渠道配置
    // bot_token 是必填项，app_token 是可选项
    if let Some(ref mut slack) = channels.slack {
        decrypt_secret(store, &mut slack.bot_token, "config.channels_config.slack.bot_token")?;
        decrypt_optional_secret(
            store,
            &mut slack.app_token,
            "config.channels_config.slack.app_token",
        )?;
    }

    // 解密 Mattermost 渠道配置
    // bot_token 是必填项
    if let Some(ref mut mattermost) = channels.mattermost {
        decrypt_secret(
            store,
            &mut mattermost.bot_token,
            "config.channels_config.mattermost.bot_token",
        )?;
    }

    // 解密 Webhook 渠道配置
    // secret 是可选项，某些 Webhook 可能不需要验证
    if let Some(ref mut webhook) = channels.webhook {
        decrypt_optional_secret(
            store,
            &mut webhook.secret,
            "config.channels_config.webhook.secret",
        )?;
    }

    // 解密 WhatsApp 渠道配置
    // 所有字段都是可选项，根据实际部署需求配置
    if let Some(ref mut whatsapp) = channels.whatsapp {
        decrypt_optional_secret(
            store,
            &mut whatsapp.access_token,
            "config.channels_config.whatsapp.access_token",
        )?;
        decrypt_optional_secret(
            store,
            &mut whatsapp.app_secret,
            "config.channels_config.whatsapp.app_secret",
        )?;
        decrypt_optional_secret(
            store,
            &mut whatsapp.verify_token,
            "config.channels_config.whatsapp.verify_token",
        )?;
    }

    // 解密 Linq 渠道配置
    // api_token 是必填项，signing_secret 是可选项
    if let Some(ref mut linq) = channels.linq {
        decrypt_secret(store, &mut linq.api_token, "config.channels_config.linq.api_token")?;
        decrypt_optional_secret(
            store,
            &mut linq.signing_secret,
            "config.channels_config.linq.signing_secret",
        )?;
    }

    // 解密 Nextcloud Talk 渠道配置
    // app_token 是必填项，webhook_secret 是可选项
    if let Some(ref mut nextcloud) = channels.nextcloud_talk {
        decrypt_secret(
            store,
            &mut nextcloud.app_token,
            "config.channels_config.nextcloud_talk.app_token",
        )?;
        decrypt_optional_secret(
            store,
            &mut nextcloud.webhook_secret,
            "config.channels_config.nextcloud_talk.webhook_secret",
        )?;
    }

    // 解密 IRC 渠道配置
    // 所有密码字段都是可选项，取决于服务器配置
    if let Some(ref mut irc) = channels.irc {
        decrypt_optional_secret(
            store,
            &mut irc.server_password,
            "config.channels_config.irc.server_password",
        )?;
        decrypt_optional_secret(
            store,
            &mut irc.nickserv_password,
            "config.channels_config.irc.nickserv_password",
        )?;
        decrypt_optional_secret(
            store,
            &mut irc.sasl_password,
            "config.channels_config.irc.sasl_password",
        )?;
    }

    // 解密 Lark（飞书）渠道配置
    // app_secret 是必填项，其他字段是可选项
    if let Some(ref mut lark) = channels.lark {
        decrypt_secret(store, &mut lark.app_secret, "config.channels_config.lark.app_secret")?;
        decrypt_optional_secret(
            store,
            &mut lark.encrypt_key,
            "config.channels_config.lark.encrypt_key",
        )?;
        decrypt_optional_secret(
            store,
            &mut lark.verification_token,
            "config.channels_config.lark.verification_token",
        )?;
    }
    for feishu in &mut channels.feishu_instances {
        let app_secret_path = channel_instance_secret_path("feishu", &feishu.id, "app_secret");
        decrypt_secret(store, &mut feishu.config.app_secret, &app_secret_path)?;

        let encrypt_key_path = channel_instance_secret_path("feishu", &feishu.id, "encrypt_key");
        decrypt_optional_secret(store, &mut feishu.config.encrypt_key, &encrypt_key_path)?;

        let verification_token_path =
            channel_instance_secret_path("feishu", &feishu.id, "verification_token");
        decrypt_optional_secret(
            store,
            &mut feishu.config.verification_token,
            &verification_token_path,
        )?;
    }

    // 解密 DingTalk（钉钉）渠道配置
    // client_secret 是必填项
    if let Some(ref mut dingtalk) = channels.dingtalk {
        decrypt_secret(
            store,
            &mut dingtalk.client_secret,
            "config.channels_config.dingtalk.client_secret",
        )?;
    }
    for dingtalk in &mut channels.dingtalk_instances {
        let path = channel_instance_secret_path("dingtalk", &dingtalk.id, "client_secret");
        decrypt_secret(store, &mut dingtalk.config.client_secret, &path)?;
    }

    // 解密 QQ 渠道配置
    // app_secret 是必填项
    if let Some(ref mut qq) = channels.qq {
        decrypt_secret(store, &mut qq.app_secret, "config.channels_config.qq.app_secret")?;
    }

    if let Some(ref mut wecom) = channels.wecom {
        decrypt_secret(store, &mut wecom.secret, "config.channels_config.wecom.secret")?;
    }

    // 解密 Nostr 渠道配置
    // private_key 是必填项，用于签名消息
    if let Some(ref mut nostr) = channels.nostr {
        decrypt_secret(store, &mut nostr.private_key, "config.channels_config.nostr.private_key")?;
    }

    // 解密 ClawdTalk 渠道配置
    // api_key 是必填项，webhook_secret 是可选项
    if let Some(ref mut clawdtalk) = channels.clawdtalk {
        decrypt_secret(store, &mut clawdtalk.api_key, "config.channels_config.clawdtalk.api_key")?;
        decrypt_optional_secret(
            store,
            &mut clawdtalk.webhook_secret,
            "config.channels_config.clawdtalk.webhook_secret",
        )?;
    }

    Ok(())
}

/// 加密所有渠道配置中的敏感信息
///
/// 此函数遍历 `ChannelsConfig` 中的各个渠道配置，对每个渠道的敏感字段
/// 进行加密操作。加密后的密文值将替换原有的明文值，用于安全存储。
///
/// # 参数
///
/// - `store`：安全存储实例，提供加密功能
/// - `channels`：可变的渠道配置引用，加密后的值将直接写入此配置
///
/// # 返回值
///
/// - `Ok(())`：所有敏感信息加密成功
/// - `Err(e)`：加密过程中发生错误
///
/// # 错误处理
///
/// 当加密失败时，函数会返回错误，包含具体的配置路径信息，
/// 便于定位问题所在。
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::security::SecretStore;
/// use crate::app::agent::config::schema::ChannelsConfig;
///
/// let store = SecretStore::new(encryption_key);
/// let mut config: ChannelsConfig = load_config()?;
///
/// // 在保存配置前，加密所有敏感信息
/// encrypt_channel_secrets(&store, &mut config)?;
///
/// // 现在可以安全地将 config 持久化到存储
/// save_config(&config)?;
/// ```
///
/// # 使用场景
///
/// - 配置文件保存前的预处理
/// - 敏感信息更新后的重新加密
/// - 配置迁移或备份时的数据保护
///
/// # 安全说明
///
/// - 加密后应确保原始明文不被泄露
/// - 加密操作应在受限环境中进行
pub fn encrypt_channel_secrets(store: &SecretStore, channels: &mut ChannelsConfig) -> Result<()> {
    channels.resolve_channel_instance_profiles()?;

    // 加密 Telegram 渠道配置
    // bot_token 是必填项，使用 encrypt_secret 处理
    if let Some(ref mut telegram) = channels.telegram {
        encrypt_secret(
            store,
            &mut telegram.bot_token,
            "config.channels_config.telegram.bot_token",
        )?;
    }

    // 加密 Discord 渠道配置
    // bot_token 是必填项
    if let Some(ref mut discord) = channels.discord {
        encrypt_secret(store, &mut discord.bot_token, "config.channels_config.discord.bot_token")?;
    }

    // 加密 Slack 渠道配置
    // bot_token 是必填项，app_token 是可选项
    if let Some(ref mut slack) = channels.slack {
        encrypt_secret(store, &mut slack.bot_token, "config.channels_config.slack.bot_token")?;
        encrypt_optional_secret(
            store,
            &mut slack.app_token,
            "config.channels_config.slack.app_token",
        )?;
    }

    // 加密 Mattermost 渠道配置
    // bot_token 是必填项
    if let Some(ref mut mattermost) = channels.mattermost {
        encrypt_secret(
            store,
            &mut mattermost.bot_token,
            "config.channels_config.mattermost.bot_token",
        )?;
    }

    // 加密 Webhook 渠道配置
    // secret 是可选项
    if let Some(ref mut webhook) = channels.webhook {
        encrypt_optional_secret(
            store,
            &mut webhook.secret,
            "config.channels_config.webhook.secret",
        )?;
    }

    // 加密 WhatsApp 渠道配置
    // 所有字段都是可选项
    if let Some(ref mut whatsapp) = channels.whatsapp {
        encrypt_optional_secret(
            store,
            &mut whatsapp.access_token,
            "config.channels_config.whatsapp.access_token",
        )?;
        encrypt_optional_secret(
            store,
            &mut whatsapp.app_secret,
            "config.channels_config.whatsapp.app_secret",
        )?;
        encrypt_optional_secret(
            store,
            &mut whatsapp.verify_token,
            "config.channels_config.whatsapp.verify_token",
        )?;
    }

    // 加密 Linq 渠道配置
    // api_token 是必填项，signing_secret 是可选项
    if let Some(ref mut linq) = channels.linq {
        encrypt_secret(store, &mut linq.api_token, "config.channels_config.linq.api_token")?;
        encrypt_optional_secret(
            store,
            &mut linq.signing_secret,
            "config.channels_config.linq.signing_secret",
        )?;
    }

    // 加密 Nextcloud Talk 渠道配置
    // app_token 是必填项，webhook_secret 是可选项
    if let Some(ref mut nextcloud) = channels.nextcloud_talk {
        encrypt_secret(
            store,
            &mut nextcloud.app_token,
            "config.channels_config.nextcloud_talk.app_token",
        )?;
        encrypt_optional_secret(
            store,
            &mut nextcloud.webhook_secret,
            "config.channels_config.nextcloud_talk.webhook_secret",
        )?;
    }

    // 加密 IRC 渠道配置
    // 所有密码字段都是可选项
    if let Some(ref mut irc) = channels.irc {
        encrypt_optional_secret(
            store,
            &mut irc.server_password,
            "config.channels_config.irc.server_password",
        )?;
        encrypt_optional_secret(
            store,
            &mut irc.nickserv_password,
            "config.channels_config.irc.nickserv_password",
        )?;
        encrypt_optional_secret(
            store,
            &mut irc.sasl_password,
            "config.channels_config.irc.sasl_password",
        )?;
    }

    // 加密 Lark（飞书）渠道配置
    // app_secret 是必填项，其他字段是可选项
    if let Some(ref mut lark) = channels.lark {
        encrypt_secret(store, &mut lark.app_secret, "config.channels_config.lark.app_secret")?;
        encrypt_optional_secret(
            store,
            &mut lark.encrypt_key,
            "config.channels_config.lark.encrypt_key",
        )?;
        encrypt_optional_secret(
            store,
            &mut lark.verification_token,
            "config.channels_config.lark.verification_token",
        )?;
    }
    for feishu in &mut channels.feishu_instances {
        let app_secret_path = channel_instance_secret_path("feishu", &feishu.id, "app_secret");
        encrypt_secret(store, &mut feishu.config.app_secret, &app_secret_path)?;

        let encrypt_key_path = channel_instance_secret_path("feishu", &feishu.id, "encrypt_key");
        encrypt_optional_secret(store, &mut feishu.config.encrypt_key, &encrypt_key_path)?;

        let verification_token_path =
            channel_instance_secret_path("feishu", &feishu.id, "verification_token");
        encrypt_optional_secret(
            store,
            &mut feishu.config.verification_token,
            &verification_token_path,
        )?;
    }

    // 加密 DingTalk（钉钉）渠道配置
    // client_secret 是必填项
    if let Some(ref mut dingtalk) = channels.dingtalk {
        encrypt_secret(
            store,
            &mut dingtalk.client_secret,
            "config.channels_config.dingtalk.client_secret",
        )?;
    }
    for dingtalk in &mut channels.dingtalk_instances {
        let path = channel_instance_secret_path("dingtalk", &dingtalk.id, "client_secret");
        encrypt_secret(store, &mut dingtalk.config.client_secret, &path)?;
    }

    // 加密 QQ 渠道配置
    // app_secret 是必填项
    if let Some(ref mut qq) = channels.qq {
        encrypt_secret(store, &mut qq.app_secret, "config.channels_config.qq.app_secret")?;
    }

    if let Some(ref mut wecom) = channels.wecom {
        encrypt_secret(store, &mut wecom.secret, "config.channels_config.wecom.secret")?;
    }

    // 加密 Nostr 渠道配置
    // private_key 是必填项
    if let Some(ref mut nostr) = channels.nostr {
        encrypt_secret(store, &mut nostr.private_key, "config.channels_config.nostr.private_key")?;
    }

    // 加密 ClawdTalk 渠道配置
    // api_key 是必填项，webhook_secret 是可选项
    if let Some(ref mut clawdtalk) = channels.clawdtalk {
        encrypt_secret(store, &mut clawdtalk.api_key, "config.channels_config.clawdtalk.api_key")?;
        encrypt_optional_secret(
            store,
            &mut clawdtalk.webhook_secret,
            "config.channels_config.clawdtalk.webhook_secret",
        )?;
    }

    Ok(())
}
