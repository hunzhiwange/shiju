//! 配置敏感字段的加密与解密编排。
//!
//! 本模块集中处理顶层配置、agent 配置、可靠性 fallback 以及渠道配置中的密钥字段。所有操作都通过
//! `SecretStore` 完成，调用方无需知道字段是否已经加密，只需要在加载后解密、保存前加密。

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

use crate::app::agent::config::schema::channels::{
    decrypt_channel_secrets_isolated, decrypt_optional_secret, encrypt_channel_secrets,
    encrypt_optional_secret,
};
use crate::app::agent::config::schema::config::Config;

/// 解密字符串数组中的密钥值。
///
/// 只有已经带加密标记的值会被解密，明文值保持不变。参数 `field_name` 用于构造错误上下文；
/// 任一元素解密失败都会返回错误并指出数组下标。
pub(crate) fn decrypt_vec_secrets(
    store: &crate::app::agent::security::SecretStore,
    values: &mut [String],
    field_name: &str,
) -> Result<()> {
    for (idx, value) in values.iter_mut().enumerate() {
        if crate::app::agent::security::SecretStore::is_encrypted(value) {
            *value = store
                .decrypt(value)
                .with_context(|| format!("Failed to decrypt {field_name}[{idx}]"))?;
        }
    }
    Ok(())
}

/// 解密字符串映射中的密钥值。
///
/// 只有已经加密的 value 会被解密，key 不会被修改。解密失败时错误信息会包含字段名和 map key，
/// 便于定位损坏或不匹配的密钥条目。
pub(crate) fn decrypt_map_secrets(
    store: &crate::app::agent::security::SecretStore,
    values: &mut HashMap<String, String>,
    field_name: &str,
) -> Result<()> {
    for (key, value) in values.iter_mut() {
        if crate::app::agent::security::SecretStore::is_encrypted(value) {
            *value = store
                .decrypt(value)
                .with_context(|| format!("Failed to decrypt {field_name}.{key}"))?;
        }
    }
    Ok(())
}

/// 加密字符串数组中的明文密钥值。
///
/// 已加密的值会被跳过，避免重复加密导致后续无法解密。参数 `field_name` 只用于错误上下文。
pub(crate) fn encrypt_vec_secrets(
    store: &crate::app::agent::security::SecretStore,
    values: &mut [String],
    field_name: &str,
) -> Result<()> {
    for (idx, value) in values.iter_mut().enumerate() {
        if !crate::app::agent::security::SecretStore::is_encrypted(value) {
            *value = store
                .encrypt(value)
                .with_context(|| format!("Failed to encrypt {field_name}[{idx}]"))?;
        }
    }
    Ok(())
}

/// 加密字符串映射中的明文密钥值。
///
/// 已加密的 value 会保持原样；明文 value 会通过 `SecretStore` 加密。任一条目加密失败都会返回带 key 的错误。
pub(crate) fn encrypt_map_secrets(
    store: &crate::app::agent::security::SecretStore,
    values: &mut HashMap<String, String>,
    field_name: &str,
) -> Result<()> {
    for (key, value) in values.iter_mut() {
        if !crate::app::agent::security::SecretStore::is_encrypted(value) {
            *value = store
                .encrypt(value)
                .with_context(|| format!("Failed to encrypt {field_name}.{key}"))?;
        }
    }
    Ok(())
}

/// 解密配置对象中的所有敏感字段。
///
/// `vibewindow_dir` 用于定位或初始化密钥存储。函数会原地更新 `config` 中的 API key、代理凭据、
/// 数据库 URL、gateway token、agent key 以及各渠道密钥。
///
/// 核心字段解密失败返回错误；渠道凭据失败仅隔离对应实例，保留原密文。
pub(crate) fn decrypt_config_secrets(config: &mut Config, vibewindow_dir: &Path) -> Result<()> {
    let store =
        crate::app::agent::security::SecretStore::new(vibewindow_dir, config.secrets.encrypt);
    decrypt_optional_secret(&store, &mut config.api_key, "config.api_key")?;
    decrypt_optional_secret(&store, &mut config.proxy.http_proxy, "config.proxy.http_proxy")?;
    decrypt_optional_secret(&store, &mut config.proxy.https_proxy, "config.proxy.https_proxy")?;
    decrypt_optional_secret(&store, &mut config.proxy.all_proxy, "config.proxy.all_proxy")?;
    decrypt_optional_secret(
        &store,
        &mut config.browser.computer_use.api_key,
        "config.browser.computer_use.api_key",
    )?;
    decrypt_optional_secret(
        &store,
        &mut config.web_search.brave_api_key,
        "config.web_search.brave_api_key",
    )?;
    decrypt_optional_secret(
        &store,
        &mut config.storage.provider.config.db_url,
        "config.storage.provider.config.db_url",
    )?;
    decrypt_vec_secrets(&store, &mut config.gateway.paired_tokens, "config.gateway.paired_tokens")?;
    for route in &mut config.embedding_routes {
        decrypt_optional_secret(&store, &mut route.api_key, "config.embedding_routes.*.api_key")?;
    }
    for agent in config.agents.values_mut() {
        decrypt_optional_secret(&store, &mut agent.api_key, "config.agents.*.api_key")?;
    }
    decrypt_channel_secrets_isolated(&store, &mut config.channels_config)?;
    Ok(())
}

/// 保存配置前加密所有敏感字段。
///
/// 函数根据 `config_to_save.config_path` 的父目录构造 `SecretStore`，然后原地加密顶层、agent、
/// fallback、gateway 和渠道密钥。若配置路径没有父目录或任一字段加密失败，会返回错误。
pub(crate) fn encrypt_config_secrets(config_to_save: &mut Config) -> Result<()> {
    let vibewindow_dir =
        config_to_save.config_path.parent().context("Config path must have a parent directory")?;
    let store = crate::app::agent::security::SecretStore::new(
        vibewindow_dir,
        config_to_save.secrets.encrypt,
    );

    encrypt_optional_secret(&store, &mut config_to_save.api_key, "config.api_key")?;
    encrypt_optional_secret(
        &store,
        &mut config_to_save.proxy.http_proxy,
        "config.proxy.http_proxy",
    )?;
    encrypt_optional_secret(
        &store,
        &mut config_to_save.proxy.https_proxy,
        "config.proxy.https_proxy",
    )?;
    encrypt_optional_secret(&store, &mut config_to_save.proxy.all_proxy, "config.proxy.all_proxy")?;

    encrypt_optional_secret(
        &store,
        &mut config_to_save.browser.computer_use.api_key,
        "config.browser.computer_use.api_key",
    )?;

    encrypt_optional_secret(
        &store,
        &mut config_to_save.web_search.brave_api_key,
        "config.web_search.brave_api_key",
    )?;

    encrypt_optional_secret(
        &store,
        &mut config_to_save.storage.provider.config.db_url,
        "config.storage.provider.config.db_url",
    )?;
    encrypt_vec_secrets(
        &store,
        &mut config_to_save.gateway.paired_tokens,
        "config.gateway.paired_tokens",
    )?;

    for route in &mut config_to_save.embedding_routes {
        encrypt_optional_secret(&store, &mut route.api_key, "config.embedding_routes.*.api_key")?;
    }

    for agent in config_to_save.agents.values_mut() {
        encrypt_optional_secret(&store, &mut agent.api_key, "config.agents.*.api_key")?;
    }

    encrypt_channel_secrets(&store, &mut config_to_save.channels_config)?;

    Ok(())
}
