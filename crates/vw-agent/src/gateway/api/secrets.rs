//! 配置 API 敏感字段掩码与恢复辅助模块
//!
//! 本模块提供配置对象中敏感信息（如 API 密钥、令牌、密码等）的掩码处理和恢复功能。
//! 主要用于在配置 API 响应中隐藏敏感字段，防止敏感信息泄露；
//! 同时支持在保存配置时恢复被掩码的字段值。
//!
//! # 核心功能
//!
//! - **掩码处理**：将敏感字段替换为 `***MASKED***` 占位符
//! - **恢复处理**：检测被掩码的字段，用当前配置中的实际值替换
//! - **TOML 规范化**：处理 Dashboard 特定的 TOML 配置格式转换
//!
//! # 使用场景
//!
//! 1. 读取配置 API 响应前，调用 `mask_sensitive_fields` 隐藏敏感信息
//! 2. 保存配置前，调用 `hydrate_config_for_save` 恢复被掩码的字段
//! 3. 处理 Dashboard 发送的 TOML 配置时，调用 `normalize_dashboard_config_toml` 规范化格式

use crate::app::agent::config::Config;

/// 敏感字段掩码占位符
///
/// 当敏感字段需要被隐藏时，将使用此常量作为占位符替换原值。
/// 恢复逻辑会检测此占位符并替换回实际值。
pub(crate) const MASKED_SECRET: &str = "***MASKED***";

/// 检查给定字符串是否为掩码后的敏感值
///
/// # 参数
///
/// - `value`: 待检查的字符串切片
///
/// # 返回值
///
/// 如果字符串等于 `MASKED_SECRET` 则返回 `true`，否则返回 `false`
fn is_masked_secret(value: &str) -> bool {
    value == MASKED_SECRET
}

/// 掩码可选类型的敏感字段
///
/// 如果 `Option<String>` 包含值（`Some`），将其替换为 `Some(MASKED_SECRET)`。
/// 如果值为 `None`，则保持不变。
///
/// # 参数
///
/// - `value`: 可变引用，指向需要掩码的可选字符串
fn mask_optional_secret(value: &mut Option<String>) {
    if value.is_some() {
        *value = Some(MASKED_SECRET.to_string());
    }
}

/// 掩码必填类型的敏感字段
///
/// 如果字符串非空，将其替换为 `MASKED_SECRET`。
/// 空字符串保持不变（视为未配置）。
///
/// # 参数
///
/// - `value`: 可变引用，指向需要掩码的字符串
fn mask_required_secret(value: &mut String) {
    if !value.is_empty() {
        *value = MASKED_SECRET.to_string();
    }
}

/// 掩码字符串数组中的敏感字段
///
/// 遍历数组中的所有字符串，将非空字符串替换为 `MASKED_SECRET`。
/// 主要用于处理如 `reliability.api_keys` 这样的多值字段。
///
/// # 参数
///
/// - `values`: 可变切片，包含需要掩码的字符串数组
fn mask_vec_secrets(values: &mut [String]) {
    for value in values.iter_mut() {
        if !value.is_empty() {
            *value = MASKED_SECRET.to_string();
        }
    }
}

/// 恢复可选类型的被掩码敏感字段
///
/// 检测传入值是否为掩码占位符，如果是则用当前配置中的实际值替换。
/// 用于在保存配置时恢复用户未修改的敏感字段。
///
/// # 参数
///
/// - `value`: 可变引用，指向传入配置中的可选字符串（可能被掩码）
/// - `current`: 当前配置中的对应值引用，用于恢复
///
/// # 示例
///
/// ```ignore
/// let mut incoming = Some("***MASKED***".to_string());
/// let current = Some("actual_secret".to_string());
/// restore_optional_secret(&mut incoming, &current);
/// // incoming 现在为 Some("actual_secret")
/// ```
#[allow(clippy::ref_option)]
fn restore_optional_secret(value: &mut Option<String>, current: &Option<String>) {
    // 仅当传入值为 Some 且内容为掩码占位符时才恢复
    if value.as_deref().is_some_and(is_masked_secret) {
        *value = current.clone();
    }
}

/// 恢复必填类型的被掩码敏感字段
///
/// 检测传入值是否为掩码占位符，如果是则用当前配置中的实际值替换。
///
/// # 参数
///
/// - `value`: 可变引用，指向传入配置中的字符串（可能被掩码）
/// - `current`: 当前配置中的对应字符串切片，用于恢复
///
/// # 示例
///
/// ```ignore
/// let mut incoming = "***MASKED***".to_string();
/// let current = "actual_token";
/// restore_required_secret(&mut incoming, current);
/// // incoming 现在为 "actual_token"
/// ```
fn restore_required_secret(value: &mut String, current: &str) {
    // 仅当传入值为掩码占位符时才恢复
    if is_masked_secret(value) {
        *value = current.to_string();
    }
}

/// 恢复字符串数组中被掩码的敏感字段
///
/// 遍历数组，对每个被掩码的元素用当前配置中对应位置的值替换。
/// 如果当前配置数组较短，超出部分的掩码值将保持不变。
///
/// # 参数
///
/// - `values`: 可变切片，指向传入配置中的字符串数组
/// - `current`: 当前配置中的字符串数组切片，用于恢复
fn restore_vec_secrets(values: &mut [String], current: &[String]) {
    for (idx, value) in values.iter_mut().enumerate() {
        // 仅处理被掩码的字段
        if is_masked_secret(value) {
            // 从当前配置的对应位置获取实际值
            if let Some(existing) = current.get(idx) {
                *value = existing.clone();
            }
        }
    }
}

/// 掩码配置对象中的所有敏感字段
///
/// 创建配置的深拷贝，并将其中所有敏感字段（API 密钥、令牌、密码等）
/// 替换为 `MASKED_SECRET` 占位符。原始配置对象不会被修改。
///
/// # 参数
///
/// - `config`: 配置对象的不可变引用
///
/// # 返回值
///
/// 返回一个新的 `Config` 对象，其中敏感字段已被掩码
///
/// # 处理的敏感字段
///
/// - 全局 API 密钥和可靠性配置中的多个 API 密钥
/// - 代理、浏览器、Web 获取/搜索相关密钥
/// - 存储数据库 URL
/// - 隧道服务（Cloudflare、Ngrok）的令牌
/// - 各代理的 API 密钥
/// - 所有通道配置中的令牌和密钥（Telegram、Discord、Slack、Mattermost、
///   Webhook、Matrix、WhatsApp、Linq、WATI、Nextcloud、Email、IRC、
///   Lark/飞书、钉钉、QQ、Nostr、ClawdTalk）
///
/// # 示例
///
/// ```ignore
/// let config = load_config()?;
/// let masked = mask_sensitive_fields(&config);
/// // 返回 masked 给 API 客户端，敏感信息已隐藏
/// ```
pub fn mask_sensitive_fields(config: &Config) -> Config {
    // 克隆配置以避免修改原始对象
    let mut masked = config.clone();

    // 掩码全局配置中的敏感字段
    mask_optional_secret(&mut masked.api_key);
    mask_vec_secrets(&mut masked.reliability.api_keys);
    mask_vec_secrets(&mut masked.gateway.paired_tokens);

    // 掩码代理配置
    mask_optional_secret(&mut masked.proxy.http_proxy);
    mask_optional_secret(&mut masked.proxy.https_proxy);
    mask_optional_secret(&mut masked.proxy.all_proxy);

    // 掩码浏览器和 Web 相关密钥
    mask_optional_secret(&mut masked.browser.computer_use.api_key);
    mask_optional_secret(&mut masked.web_fetch.api_key);
    mask_optional_secret(&mut masked.web_search.api_key);
    mask_optional_secret(&mut masked.web_search.brave_api_key);

    // 掩码存储配置中的数据库 URL
    mask_optional_secret(&mut masked.storage.provider.config.db_url);

    // 掩码隧道服务令牌
    if let Some(cloudflare) = masked.tunnel.cloudflare.as_mut() {
        mask_required_secret(&mut cloudflare.token);
    }
    if let Some(ngrok) = masked.tunnel.ngrok.as_mut() {
        mask_required_secret(&mut ngrok.auth_token);
    }
    if let Some(custom) = masked.tunnel.custom.as_mut() {
        mask_optional_secret(&mut custom.auth_token);
    }

    // 掩码所有代理的 API 密钥
    for agent in masked.agents.values_mut() {
        mask_optional_secret(&mut agent.api_key);
    }

    // 掩码嵌入路由的 API 密钥
    for route in &mut masked.embedding_routes {
        mask_optional_secret(&mut route.api_key);
    }

    // 掩码 Telegram 通道配置
    if let Some(telegram) = masked.channels_config.telegram.as_mut() {
        mask_required_secret(&mut telegram.bot_token);
    }

    // 掩码 Discord 通道配置
    if let Some(discord) = masked.channels_config.discord.as_mut() {
        mask_required_secret(&mut discord.bot_token);
    }

    // 掩码 Slack 通道配置
    if let Some(slack) = masked.channels_config.slack.as_mut() {
        mask_required_secret(&mut slack.bot_token);
        mask_optional_secret(&mut slack.app_token);
    }

    // 掩码 Mattermost 通道配置
    if let Some(mattermost) = masked.channels_config.mattermost.as_mut() {
        mask_required_secret(&mut mattermost.bot_token);
    }

    // 掩码 Webhook 通道配置
    if let Some(webhook) = masked.channels_config.webhook.as_mut() {
        mask_optional_secret(&mut webhook.secret);
    }

    // 掩码 Matrix 通道配置
    if let Some(matrix) = masked.channels_config.matrix.as_mut() {
        mask_required_secret(&mut matrix.access_token);
    }

    // 掩码 WhatsApp 通道配置
    if let Some(whatsapp) = masked.channels_config.whatsapp.as_mut() {
        mask_optional_secret(&mut whatsapp.access_token);
        mask_optional_secret(&mut whatsapp.app_secret);
        mask_optional_secret(&mut whatsapp.verify_token);
    }

    // 掩码 Linq 通道配置
    if let Some(linq) = masked.channels_config.linq.as_mut() {
        mask_required_secret(&mut linq.api_token);
        mask_optional_secret(&mut linq.signing_secret);
    }

    // 掩码 WATI 通道配置
    if let Some(wati) = masked.channels_config.wati.as_mut() {
        mask_required_secret(&mut wati.api_token);
    }

    // 掩码 Nextcloud Talk 通道配置
    if let Some(nextcloud) = masked.channels_config.nextcloud_talk.as_mut() {
        mask_required_secret(&mut nextcloud.app_token);
        mask_optional_secret(&mut nextcloud.webhook_secret);
    }

    // 掩码 Email 通道配置
    if let Some(email) = masked.channels_config.email.as_mut() {
        mask_required_secret(&mut email.password);
    }

    // 掩码 IRC 通道配置
    if let Some(irc) = masked.channels_config.irc.as_mut() {
        mask_optional_secret(&mut irc.server_password);
        mask_optional_secret(&mut irc.nickserv_password);
        mask_optional_secret(&mut irc.sasl_password);
    }

    // 掩码 Lark（飞书国际版）通道配置
    if let Some(lark) = masked.channels_config.lark.as_mut() {
        mask_required_secret(&mut lark.app_secret);
        mask_optional_secret(&mut lark.encrypt_key);
        mask_optional_secret(&mut lark.verification_token);
    }

    // 掩码 Feishu（飞书国内版）通道配置
    if let Some(feishu) = masked.channels_config.feishu.as_mut() {
        mask_required_secret(&mut feishu.app_secret);
        mask_optional_secret(&mut feishu.encrypt_key);
        mask_optional_secret(&mut feishu.verification_token);
    }

    // 掩码钉钉通道配置
    if let Some(dingtalk) = masked.channels_config.dingtalk.as_mut() {
        mask_required_secret(&mut dingtalk.client_secret);
    }

    // 掩码 QQ 通道配置
    if let Some(qq) = masked.channels_config.qq.as_mut() {
        mask_required_secret(&mut qq.app_secret);
    }

    // 掩码 Nostr 通道配置
    if let Some(nostr) = masked.channels_config.nostr.as_mut() {
        mask_required_secret(&mut nostr.private_key);
    }

    // 掩码 ClawdTalk 通道配置
    if let Some(clawdtalk) = masked.channels_config.clawdtalk.as_mut() {
        mask_required_secret(&mut clawdtalk.api_key);
        mask_optional_secret(&mut clawdtalk.webhook_secret);
    }

    masked
}

/// 恢复传入配置中被掩码的敏感字段
///
/// 此函数是 `mask_sensitive_fields` 的逆操作，用于在保存配置前
/// 将用户未修改（仍为掩码占位符）的敏感字段恢复为实际值。
///
/// # 参数
///
/// - `incoming`: 可变引用，指向传入的配置对象（可能包含掩码字段）
/// - `current`: 当前配置对象的引用，用于恢复被掩码的字段
///
/// # 处理逻辑
///
/// 对于每个敏感字段，如果传入值是 `MASKED_SECRET`，则用当前配置中的
/// 对应值替换。这样用户在只修改部分配置时，未修改的敏感字段不会丢失。
fn restore_masked_sensitive_fields(incoming: &mut Config, current: &Config) {
    // 恢复全局配置中的敏感字段
    restore_optional_secret(&mut incoming.api_key, &current.api_key);
    restore_vec_secrets(&mut incoming.reliability.api_keys, &current.reliability.api_keys);
    restore_vec_secrets(&mut incoming.gateway.paired_tokens, &current.gateway.paired_tokens);

    // 恢复代理配置
    restore_optional_secret(&mut incoming.proxy.http_proxy, &current.proxy.http_proxy);
    restore_optional_secret(&mut incoming.proxy.https_proxy, &current.proxy.https_proxy);
    restore_optional_secret(&mut incoming.proxy.all_proxy, &current.proxy.all_proxy);

    // 恢复浏览器和 Web 相关密钥
    restore_optional_secret(
        &mut incoming.browser.computer_use.api_key,
        &current.browser.computer_use.api_key,
    );
    restore_optional_secret(&mut incoming.web_fetch.api_key, &current.web_fetch.api_key);
    restore_optional_secret(&mut incoming.web_search.api_key, &current.web_search.api_key);
    restore_optional_secret(
        &mut incoming.web_search.brave_api_key,
        &current.web_search.brave_api_key,
    );

    // 恢复存储配置中的数据库 URL
    restore_optional_secret(
        &mut incoming.storage.provider.config.db_url,
        &current.storage.provider.config.db_url,
    );

    // 恢复隧道服务令牌（需同时检查 incoming 和 current 是否都存在）
    if let (Some(incoming_tunnel), Some(current_tunnel)) =
        (incoming.tunnel.cloudflare.as_mut(), current.tunnel.cloudflare.as_ref())
    {
        restore_required_secret(&mut incoming_tunnel.token, &current_tunnel.token);
    }
    if let (Some(incoming_tunnel), Some(current_tunnel)) =
        (incoming.tunnel.ngrok.as_mut(), current.tunnel.ngrok.as_ref())
    {
        restore_required_secret(&mut incoming_tunnel.auth_token, &current_tunnel.auth_token);
    }
    if let (Some(incoming_tunnel), Some(current_tunnel)) =
        (incoming.tunnel.custom.as_mut(), current.tunnel.custom.as_ref())
    {
        restore_optional_secret(&mut incoming_tunnel.auth_token, &current_tunnel.auth_token);
    }

    // 恢复所有代理的 API 密钥
    for (name, agent) in &mut incoming.agents {
        if let Some(current_agent) = current.agents.get(name) {
            restore_optional_secret(&mut agent.api_key, &current_agent.api_key);
        }
    }

    // 恢复嵌入路由 API 密钥
    for incoming_route in &mut incoming.embedding_routes {
        let Some(current_route) =
            current.embedding_routes.iter().find(|route| route.hint == incoming_route.hint)
        else {
            continue;
        };
        restore_optional_secret(&mut incoming_route.api_key, &current_route.api_key);
    }

    // 恢复 Telegram 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.telegram.as_mut(), current.channels_config.telegram.as_ref())
    {
        restore_required_secret(&mut incoming_ch.bot_token, &current_ch.bot_token);
    }

    // 恢复 Discord 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.discord.as_mut(), current.channels_config.discord.as_ref())
    {
        restore_required_secret(&mut incoming_ch.bot_token, &current_ch.bot_token);
    }

    // 恢复 Slack 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.slack.as_mut(), current.channels_config.slack.as_ref())
    {
        restore_required_secret(&mut incoming_ch.bot_token, &current_ch.bot_token);
        restore_optional_secret(&mut incoming_ch.app_token, &current_ch.app_token);
    }

    // 恢复 Mattermost 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.mattermost.as_mut(), current.channels_config.mattermost.as_ref())
    {
        restore_required_secret(&mut incoming_ch.bot_token, &current_ch.bot_token);
    }

    // 恢复 Webhook 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.webhook.as_mut(), current.channels_config.webhook.as_ref())
    {
        restore_optional_secret(&mut incoming_ch.secret, &current_ch.secret);
    }

    // 恢复 Matrix 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.matrix.as_mut(), current.channels_config.matrix.as_ref())
    {
        restore_required_secret(&mut incoming_ch.access_token, &current_ch.access_token);
    }

    // 恢复 WhatsApp 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.whatsapp.as_mut(), current.channels_config.whatsapp.as_ref())
    {
        restore_optional_secret(&mut incoming_ch.access_token, &current_ch.access_token);
        restore_optional_secret(&mut incoming_ch.app_secret, &current_ch.app_secret);
        restore_optional_secret(&mut incoming_ch.verify_token, &current_ch.verify_token);
    }

    // 恢复 Linq 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.linq.as_mut(), current.channels_config.linq.as_ref())
    {
        restore_required_secret(&mut incoming_ch.api_token, &current_ch.api_token);
        restore_optional_secret(&mut incoming_ch.signing_secret, &current_ch.signing_secret);
    }

    // 恢复 WATI 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.wati.as_mut(), current.channels_config.wati.as_ref())
    {
        restore_required_secret(&mut incoming_ch.api_token, &current_ch.api_token);
    }

    // 恢复 Nextcloud Talk 通道配置
    if let (Some(incoming_ch), Some(current_ch)) = (
        incoming.channels_config.nextcloud_talk.as_mut(),
        current.channels_config.nextcloud_talk.as_ref(),
    ) {
        restore_required_secret(&mut incoming_ch.app_token, &current_ch.app_token);
        restore_optional_secret(&mut incoming_ch.webhook_secret, &current_ch.webhook_secret);
    }

    // 恢复 Email 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.email.as_mut(), current.channels_config.email.as_ref())
    {
        restore_required_secret(&mut incoming_ch.password, &current_ch.password);
    }

    // 恢复 IRC 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.irc.as_mut(), current.channels_config.irc.as_ref())
    {
        restore_optional_secret(&mut incoming_ch.server_password, &current_ch.server_password);
        restore_optional_secret(&mut incoming_ch.nickserv_password, &current_ch.nickserv_password);
        restore_optional_secret(&mut incoming_ch.sasl_password, &current_ch.sasl_password);
    }

    // 恢复 Lark（飞书国际版）通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.lark.as_mut(), current.channels_config.lark.as_ref())
    {
        restore_required_secret(&mut incoming_ch.app_secret, &current_ch.app_secret);
        restore_optional_secret(&mut incoming_ch.encrypt_key, &current_ch.encrypt_key);
        restore_optional_secret(
            &mut incoming_ch.verification_token,
            &current_ch.verification_token,
        );
    }

    // 恢复 Feishu（飞书国内版）通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.feishu.as_mut(), current.channels_config.feishu.as_ref())
    {
        restore_required_secret(&mut incoming_ch.app_secret, &current_ch.app_secret);
        restore_optional_secret(&mut incoming_ch.encrypt_key, &current_ch.encrypt_key);
        restore_optional_secret(
            &mut incoming_ch.verification_token,
            &current_ch.verification_token,
        );
    }

    // 恢复钉钉通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.dingtalk.as_mut(), current.channels_config.dingtalk.as_ref())
    {
        restore_required_secret(&mut incoming_ch.client_secret, &current_ch.client_secret);
    }

    // 恢复 QQ 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.qq.as_mut(), current.channels_config.qq.as_ref())
    {
        restore_required_secret(&mut incoming_ch.app_secret, &current_ch.app_secret);
    }

    // 恢复 Nostr 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.nostr.as_mut(), current.channels_config.nostr.as_ref())
    {
        restore_required_secret(&mut incoming_ch.private_key, &current_ch.private_key);
    }

    // 恢复 ClawdTalk 通道配置
    if let (Some(incoming_ch), Some(current_ch)) =
        (incoming.channels_config.clawdtalk.as_mut(), current.channels_config.clawdtalk.as_ref())
    {
        restore_required_secret(&mut incoming_ch.api_key, &current_ch.api_key);
        restore_optional_secret(&mut incoming_ch.webhook_secret, &current_ch.webhook_secret);
    }
}

/// 为保存操作补全配置
///
/// 此函数在保存配置前执行两个关键操作：
/// 1. 恢复被掩码的敏感字段（用户未修改的字段）
/// 2. 同步系统管理字段（配置路径和工作目录）
///
/// # 参数
///
/// - `incoming`: 传入的配置对象（可能包含掩码字段）
/// - `current`: 当前配置对象的引用
///
/// # 返回值
///
/// 返回一个新的 `Config` 对象，其中：
/// - 被掩码的敏感字段已恢复为当前配置中的实际值
/// - `config_path` 和 `workspace_dir` 已同步为当前值
///
/// # 使用场景
///
/// 当用户通过 Dashboard 或 API 修改配置时，敏感字段会显示为掩码。
/// 如果用户不修改这些字段直接保存，此函数确保原始值不会丢失。
///
/// # 示例
///
/// ```ignore
/// let incoming_config = parse_from_request(request)?;
/// let hydrated = hydrate_config_for_save(incoming_config, &current_config);
/// save_config(&hydrated)?;
/// ```
pub fn hydrate_config_for_save(mut incoming: Config, current: &Config) -> Config {
    // 恢复被掩码的敏感字段
    restore_masked_sensitive_fields(&mut incoming, current);

    // 同步系统管理字段（这些字段不应被用户修改）
    incoming.config_path = current.config_path.clone();
    incoming.workspace_dir = current.workspace_dir.clone();

    incoming
}

/// 规范化 Dashboard 发送的 TOML 配置
///
/// Dashboard 可能以非标准格式发送某些配置字段。
/// 此函数处理这些特殊情况，确保配置格式正确。
///
/// 当前处理的特殊情况：
/// - `reliability.api_keys`：可以是字符串或数组，统一转换为数组格式
///
/// # 参数
///
/// - `root`: TOML 值的可变引用，指向配置树的根节点
///
/// # 处理逻辑
///
/// 如果 `reliability.api_keys` 是单个字符串，将其转换为包含该字符串的数组。
/// 这样可以支持用户输入单个 API 密钥或多个密钥列表。
///
/// # 示例
///
/// 输入 TOML:
/// ```toml
/// [reliability]
/// api_keys = "single_key"
/// ```
///
/// 规范化后:
/// ```toml
/// [reliability]
/// api_keys = ["single_key"]
/// ```
pub fn normalize_dashboard_config_toml(root: &mut toml::Value) {
    // 获取根表，如果不是表则直接返回
    let Some(root_table) = root.as_table_mut() else {
        return;
    };

    // 获取 reliability 表，如果不存在则直接返回
    let Some(reliability) = root_table.get_mut("reliability").and_then(toml::Value::as_table_mut)
    else {
        return;
    };

    // 获取 api_keys 字段，如果不存在则直接返回
    let Some(api_keys) = reliability.get_mut("api_keys") else {
        return;
    };

    // 如果 api_keys 是单个字符串，转换为数组格式
    if let Some(single) = api_keys.as_str() {
        *api_keys = toml::Value::Array(vec![toml::Value::String(single.to_string())]);
    }
}

#[cfg(test)]
#[path = "secrets_tests.rs"]
mod secrets_tests;
