//! # 通道配置收集模块
//!
//! 本模块负责从主配置对象中收集并实例化所有已配置的通信通道。
//!
//! ## 主要功能
//!
//! - **通道收集**：遍历配置中所有启用的通道类型，创建对应的通道实例
//! - **特性门控**：根据编译时特性标志有条件地启用或禁用特定通道
//! - **平台适配**：针对不同目标平台（如 wasm32）调整可用通道集合
//! - **异步初始化**：支持需要异步初始化的通道（如 Nostr）
//!
//! ## 支持的通道类型
//!
//! 本模块支持以下通信通道（具体可用性取决于编译特性）：
//!
//! - **即时通讯**：Telegram、Discord、Slack、Mattermost、iMessage、Signal、WhatsApp
//! - **企业协作**：Lark/Feishu、DingTalk、QQ、Nextcloud Talk
//! - **其他协议**：Matrix、IRC、Email、Nostr、Linq、WATI、ClawdTalk
//!
//! ## 架构说明
//!
//! 模块采用工厂模式，根据配置动态创建通道实例。每个通道都实现了 `Channel` trait，
//! 并被包装在 `ConfiguredChannel` 结构中以提供显示名称和统一的接口。

use super::*;
use crate::app::agent::channels::clawdtalk::ClawdTalkChannel;
#[cfg(feature = "channel-matrix")]
use vw_config_types::channels::chat::DEFAULT_CHANNEL_INSTANCE_ID;

fn instance_display_name(channel_name: &str, instance_id: &str) -> String {
    format!("{channel_name}({instance_id})")
}

#[cfg(feature = "channel-matrix")]
fn build_matrix_channel(
    config: &Config,
    matrix: &crate::app::agent::config::schema::MatrixConfig,
    instance_id: &str,
) -> MatrixChannel {
    MatrixChannel::new_with_session_hint_instance_id_and_vibewindow_dir(
        matrix.homeserver.clone(),
        matrix.access_token.clone(),
        matrix.room_id.clone(),
        matrix.allowed_users.clone(),
        matrix.user_id.clone(),
        matrix.device_id.clone(),
        Some(instance_id.to_string()),
        config.config_path.parent().map(|path| path.to_path_buf()),
    )
    .with_mention_only(matrix.mention_only)
    .with_transcription(config.transcription.clone())
}

#[cfg(feature = "channel-matrix")]
fn matrix_config_validation_error(
    matrix: &crate::app::agent::config::schema::MatrixConfig,
) -> Option<&'static str> {
    if matrix.homeserver.trim().is_empty() {
        return Some("homeserver is empty");
    }
    if matrix.access_token.trim().is_empty() {
        return Some("access_token is empty");
    }

    let room_id = matrix.room_id.trim();
    if !room_id.starts_with('!') && !room_id.starts_with('#') {
        return Some("room_id must start with '!' (room ID) or '#' (room alias)");
    }

    None
}

#[cfg(feature = "channel-matrix")]
fn append_matrix_channel_if_valid(
    config: &Config,
    matrix: &crate::app::agent::config::schema::MatrixConfig,
    instance_id: &str,
    display_name: String,
    channels: &mut Vec<ConfiguredChannel>,
) {
    if let Some(reason) = matrix_config_validation_error(matrix) {
        tracing::warn!(
            "{display_name} config is incomplete ({reason}); skipping optional channel."
        );
        return;
    }

    channels.push(ConfiguredChannel::new(
        display_name,
        Arc::new(build_matrix_channel(config, matrix, instance_id)),
    ));
}

/// 从配置对象中收集所有已配置的通道实例
///
/// 此函数遍历配置中的所有通道设置，为每个已配置且可用的通道创建实例。
/// 通道的可用性受编译时特性标志和目标平台的限制。
///
/// # 参数
///
/// * `config` - 应用主配置对象的引用，包含所有通道的配置信息
/// * `matrix_skip_context` - Matrix 通道跳过时的上下文信息，用于日志记录
///   （即使 Matrix 功能未编译也会保留此参数，以避免未使用变量警告）
///
/// # 返回值
///
/// 返回一个包含所有已配置且可用通道的向量。每个元素都是 `ConfiguredChannel` 结构，
/// 包含通道的显示名称和实现了 `Channel` trait 的通道实例。
///
/// # 平台限制
///
/// - 大多数通道在 `wasm32` 目标平台上不可用
/// - Matrix 通道需要启用 `channel-matrix` 特性
/// - WhatsApp Web 模式需要启用 `whatsapp-web` 特性
/// - Lark/Feishu 通道需要启用 `channel-lark` 特性
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::config::Config;
/// use crate::app::agent::channels::manager::channels_config::collect_configured_channels;
///
/// let config = Config::load_from_file("config.toml")?;
/// let channels = collect_configured_channels(&config, "startup");
///
/// for configured_channel in channels {
///     println!("已加载通道: {}", configured_channel.display_name);
/// }
/// ```
///
/// # 注意事项
///
/// - 函数会为配置不完整或特性缺失的通道记录警告日志
/// - WhatsApp 通道具有复杂的模式检测逻辑（Cloud API vs Web 模式）
/// - QQ 通道的 Webhook 模式会跳过 WebSocket 监听器启动
pub(crate) fn collect_configured_channels(
    config: &Config,
    matrix_skip_context: &str,
) -> Vec<ConfiguredChannel> {
    // 即使在 Matrix 支持已编译且 `#[cfg(not(feature = "channel-matrix"))]` 块被移除时，
    // 也要保持此符号被使用，避免编译器产生未使用参数警告
    let _ = matrix_skip_context;
    let mut channels = Vec::new();
    let channel_instance_profiles_valid = match config
        .channels_config
        .resolve_channel_instance_profiles()
    {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!(
                "Channel instance profile config is invalid: {err}; skipping Matrix, Feishu, and DingTalk channel instances."
            );
            false
        }
    };

    // ==================== Telegram 通道配置 ====================
    // Telegram 是一个流行的即时通讯平台，支持机器人 API
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ref tg) = config.channels_config.telegram {
        // 创建 Telegram 通道实例，配置基础参数
        let mut telegram = TelegramChannel::new(
            tg.bot_token.clone(),                               // 机器人的 API 令牌
            tg.allowed_users.clone(),                           // 允许与机器人交互的用户列表
            tg.effective_group_reply_mode().requires_mention(), // 群组回复是否需要 @ 提及
        )
        .with_group_reply_allowed_senders(tg.group_reply_allowed_sender_ids()) // 群组回复允许的发送者
        .with_streaming(tg.stream_mode, tg.draft_update_interval_ms) // 流式输出配置
        .with_transcription(config.transcription.clone()) // 语音转文字配置
        .with_workspace_dir(config.workspace_dir.clone()); // 工作目录配置

        // 如果配置了自定义 API 基础 URL（用于代理或私有实例），则覆盖默认值
        if let Some(ref base_url) = tg.base_url {
            telegram = telegram.with_api_base(base_url.clone());
        }

        // 将配置好的 Telegram 通道添加到通道列表
        channels.push(ConfiguredChannel::new("Telegram", Arc::new(telegram)));
    }

    // ==================== Discord 通道配置 ====================
    // Discord 是游戏社区和企业团队常用的通讯平台
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ref dc) = config.channels_config.discord {
        channels.push(ConfiguredChannel::new(
            "Discord",
            Arc::new(
                DiscordChannel::new(
                    dc.bot_token.clone(),                               // Discord 机器人令牌
                    dc.guild_id.clone(),                                // Discord 服务器（公会）ID
                    dc.allowed_users.clone(),                           // 允许的用户列表
                    dc.listen_to_bots,                                  // 是否监听其他机器人的消息
                    dc.effective_group_reply_mode().requires_mention(), // 群组回复提及要求
                )
                .with_group_reply_allowed_senders(dc.group_reply_allowed_sender_ids())
                .with_transcription(config.transcription.clone())
                .with_workspace_dir(config.workspace_dir.clone()),
            ),
        ));
    }

    // ==================== Slack 通道配置 ====================
    // Slack 是企业团队协作的常用平台
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ref sl) = config.channels_config.slack {
        channels.push(ConfiguredChannel::new(
            "Slack",
            Arc::new(
                SlackChannel::new(
                    sl.bot_token.clone(),     // Slack 机器人令牌
                    sl.channel_id.clone(),    // Slack 频道 ID
                    sl.allowed_users.clone(), // 允许的用户列表
                )
                .with_group_reply_policy(
                    sl.effective_group_reply_mode().requires_mention(),
                    sl.group_reply_allowed_sender_ids(),
                ),
            ),
        ));
    }

    // ==================== Mattermost 通道配置 ====================
    // Mattermost 是开源的团队协作平台，类似 Slack
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ref mm) = config.channels_config.mattermost {
        channels.push(ConfiguredChannel::new(
            "Mattermost",
            Arc::new(
                MattermostChannel::new(
                    mm.url.clone(),                    // Mattermost 服务器 URL
                    mm.bot_token.clone(),              // 机器人访问令牌
                    mm.channel_id.clone(),             // 频道 ID
                    mm.allowed_users.clone(),          // 允许的用户列表
                    mm.thread_replies.unwrap_or(true), // 是否在主题中回复，默认为 true
                    mm.effective_group_reply_mode().requires_mention(),
                )
                .with_group_reply_allowed_senders(mm.group_reply_allowed_sender_ids()),
            ),
        ));
    }

    // ==================== iMessage 通道配置 ====================
    // iMessage 是苹果设备的原生消息应用
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ref im) = config.channels_config.imessage {
        channels.push(ConfiguredChannel::new(
            "iMessage",
            Arc::new(IMessageChannel::new(im.allowed_contacts.clone())),
        ));
    }

    // ==================== Matrix 通道配置 ====================
    // Matrix 是去中心化的开放通信协议
    #[cfg(feature = "channel-matrix")]
    if channel_instance_profiles_valid && config.channels_config.matrix_enabled {
        if let Some(ref mx) = config.channels_config.matrix {
            append_matrix_channel_if_valid(
                config,
                mx,
                DEFAULT_CHANNEL_INSTANCE_ID,
                "Matrix".to_string(),
                &mut channels,
            );
        }

        for matrix_instance in &config.channels_config.matrix_instances {
            append_matrix_channel_if_valid(
                config,
                &matrix_instance.config,
                &matrix_instance.id,
                instance_display_name("Matrix", &matrix_instance.id),
                &mut channels,
            );
        }
    }

    // 当未启用 channel-matrix 特性但配置了 Matrix 时，记录警告
    #[cfg(not(feature = "channel-matrix"))]
    if config.channels_config.matrix_enabled
        && (config.channels_config.matrix.is_some()
            || !config.channels_config.matrix_instances.is_empty())
    {
        let matrix_instance_count = usize::from(config.channels_config.matrix.is_some())
            + config.channels_config.matrix_instances.len();
        tracing::warn!(
            "Matrix channel has {matrix_instance_count} configured instance(s) but this build was compiled without `channel-matrix`; skipping Matrix {matrix_skip_context}."
        );
    }

    // ==================== Signal 通道配置 ====================
    // Signal 是注重隐私的加密消息应用
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ref sig) = config.channels_config.signal {
        channels.push(ConfiguredChannel::new(
            "Signal",
            Arc::new(SignalChannel::new(
                sig.http_url.clone(),     // Signal HTTP 网关 URL
                sig.account.clone(),      // 关联的电话号码
                sig.group_id.clone(),     // 群组 ID（可选）
                sig.allowed_from.clone(), // 允许发送消息的号码列表
                sig.ignore_attachments,   // 是否忽略附件
                sig.ignore_stories,       // 是否忽略 Stories
            )),
        ));
    }

    // ==================== WhatsApp 通道配置 ====================
    // WhatsApp 支持两种后端模式：Cloud API 和 Web 模式
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ref wa) = config.channels_config.whatsapp {
        // 检测配置歧义：同时设置了 Cloud API 和 Web 模式的参数
        if wa.is_ambiguous_config() {
            tracing::warn!(
                "WhatsApp config has both phone_number_id and session_path set; preferring Cloud API mode. Remove one selector to avoid ambiguity."
            );
        }

        // 运行时协商：根据配置检测后端类型
        if wa.phone_number_id.is_some() {
            // Cloud API 模式：需要 phone_number_id、access_token 和 verify_token
            if wa.is_cloud_config() {
                channels.push(ConfiguredChannel::new(
                    "WhatsApp",
                    Arc::new(WhatsAppChannel::new(
                        wa.access_token.clone().unwrap_or_default(),
                        wa.phone_number_id.clone().unwrap_or_default(),
                        wa.verify_token.clone().unwrap_or_default(),
                        wa.allowed_numbers.clone(),
                    )),
                ));
            } else {
                tracing::warn!(
                    "WhatsApp Cloud API configured but missing required fields (phone_number_id, access_token, verify_token)"
                );
            }
        } else if wa.session_path.is_some() {
            // Web 模式：需要 session_path
            #[cfg(feature = "whatsapp-web")]
            if wa.is_web_config() {
                channels.push(ConfiguredChannel::new(
                    "WhatsApp",
                    Arc::new(WhatsAppWebChannel::new(
                        wa.session_path.clone().unwrap_or_default(),
                        wa.pair_phone.clone(),
                        wa.pair_code.clone(),
                        wa.allowed_numbers.clone(),
                    )),
                ));
            }
            #[cfg(not(feature = "whatsapp-web"))]
            {
                tracing::warn!(
                    "WhatsApp Web backend requires 'whatsapp-web' feature. Enable with: cargo build --features whatsapp-web"
                );
            }
        } else {
            tracing::warn!(
                "WhatsApp config invalid: neither phone_number_id (Cloud API) nor session_path (Web) is set"
            );
        }
    }

    // ==================== Linq 通道配置 ====================
    // Linq 是短信服务提供商
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ref lq) = config.channels_config.linq {
        channels.push(ConfiguredChannel::new(
            "Linq",
            Arc::new(LinqChannel::new(
                lq.api_token.clone(),       // Linq API 令牌
                lq.from_phone.clone(),      // 发送方电话号码
                lq.allowed_senders.clone(), // 允许的发送者列表
            )),
        ));
    }

    // ==================== WATI 通道配置 ====================
    // WATI 是 WhatsApp API 的第三方集成服务
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ref wati_cfg) = config.channels_config.wati {
        channels.push(ConfiguredChannel::new(
            "WATI",
            Arc::new(WatiChannel::new(
                wati_cfg.api_token.clone(),       // WATI API 令牌
                wati_cfg.api_url.clone(),         // WATI API 端点 URL
                wati_cfg.tenant_id.clone(),       // 租户 ID
                wati_cfg.allowed_numbers.clone(), // 允许的号码列表
            )),
        ));
    }

    // ==================== Nextcloud Talk 通道配置 ====================
    // Nextcloud Talk 是 Nextcloud 的视频会议和聊天功能
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ref nc) = config.channels_config.nextcloud_talk {
        channels.push(ConfiguredChannel::new(
            "Nextcloud Talk",
            Arc::new(NextcloudTalkChannel::new(
                nc.base_url.clone(),      // Nextcloud 服务器基础 URL
                nc.app_token.clone(),     // 应用访问令牌
                nc.allowed_users.clone(), // 允许的用户列表
            )),
        ));
    }

    // ==================== Email 通道配置 ====================
    // Email 通道支持通过 SMTP 接收和发送邮件
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ref email_cfg) = config.channels_config.email {
        channels
            .push(ConfiguredChannel::new("Email", Arc::new(EmailChannel::new(email_cfg.clone()))));
    }

    // ==================== IRC 通道配置 ====================
    // IRC 是经典的互联网中继聊天协议
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ref irc) = config.channels_config.irc {
        channels.push(ConfiguredChannel::new(
            "IRC",
            Arc::new(IrcChannel::new(irc::IrcChannelConfig {
                server: irc.server.clone(),                       // IRC 服务器地址
                port: irc.port,                                   // 服务器端口
                nickname: irc.nickname.clone(),                   // 昵称
                username: irc.username.clone(),                   // 用户名
                channels: irc.channels.clone(),                   // 要加入的频道列表
                allowed_users: irc.allowed_users.clone(),         // 允许的用户列表
                server_password: irc.server_password.clone(),     // 服务器密码（可选）
                nickserv_password: irc.nickserv_password.clone(), // NickServ 密码（可选）
                sasl_password: irc.sasl_password.clone(),         // SASL 认证密码（可选）
                verify_tls: irc.verify_tls.unwrap_or(true),       // 是否验证 TLS 证书，默认为 true
            })),
        ));
    }

    // ==================== Lark/Feishu 通道配置 ====================
    // Lark 是字节跳动的企业协作平台，Feishu 是其中国版
    #[cfg(feature = "channel-lark")]
    if let Some(ref lk) = config.channels_config.lark {
        // 处理 use_feishu 兼容性标志
        if lk.use_feishu {
            // 检查是否同时配置了独立的 feishu 配置段
            if config.channels_config.feishu.is_some() {
                tracing::warn!(
                    "Both [channels_config.feishu] and legacy [channels_config.lark].use_feishu=true are configured; ignoring legacy Feishu fallback in lark."
                );
            } else if lk.has_required_credentials() {
                // 使用遗留的 use_feishu 标志创建飞书通道
                tracing::warn!(
                    "Using legacy [channels_config.lark].use_feishu=true compatibility path; prefer [channels_config.feishu]."
                );
                channels
                    .push(ConfiguredChannel::new("Feishu", Arc::new(LarkChannel::from_config(lk))));
            }
        } else {
            // 创建 Lark 国际版通道
            channels
                .push(ConfiguredChannel::new("Lark", Arc::new(LarkChannel::from_lark_config(lk))));
        }
    }

    // 独立的 Feishu 配置段
    #[cfg(feature = "channel-lark")]
    if channel_instance_profiles_valid {
        if let Some(ref fs) = config.channels_config.feishu {
            if fs.has_required_credentials() {
                channels.push(ConfiguredChannel::new(
                    "Feishu",
                    Arc::new(LarkChannel::from_feishu_config(fs)),
                ));
            }
        }

        for feishu_instance in &config.channels_config.feishu_instances {
            if feishu_instance.config.has_required_credentials() {
                channels.push(ConfiguredChannel::new(
                    instance_display_name("Feishu", &feishu_instance.id),
                    Arc::new(LarkChannel::from_feishu_config(&feishu_instance.config)),
                ));
            }
        }
    }

    // 当未启用 channel-lark 特性但配置了 Lark/Feishu 时，记录警告
    #[cfg(not(feature = "channel-lark"))]
    if config.channels_config.lark.is_some()
        || config.channels_config.feishu.is_some()
        || !config.channels_config.feishu_instances.is_empty()
    {
        let lark_feishu_instance_count = usize::from(config.channels_config.lark.is_some())
            + usize::from(config.channels_config.feishu.is_some())
            + config.channels_config.feishu_instances.len();
        tracing::warn!(
            "Lark/Feishu channel has {lark_feishu_instance_count} configured instance(s) but this build was compiled without `channel-lark`; skipping Lark/Feishu health check."
        );
    }

    // ==================== DingTalk 通道配置 ====================
    // DingTalk 是阿里巴巴的企业协作平台
    #[cfg(not(target_arch = "wasm32"))]
    if channel_instance_profiles_valid {
        if let Some(ref dt) = config.channels_config.dingtalk {
            channels.push(ConfiguredChannel::new(
                "DingTalk",
                Arc::new(DingTalkChannel::new(
                    dt.client_id.clone(),     // 钉钉应用 Client ID
                    dt.client_secret.clone(), // 钉钉应用 Client Secret
                    dt.allowed_users.clone(), // 允许的用户列表
                )),
            ));
        }

        for dingtalk_instance in &config.channels_config.dingtalk_instances {
            let dt = &dingtalk_instance.config;
            channels.push(ConfiguredChannel::new(
                instance_display_name("DingTalk", &dingtalk_instance.id),
                Arc::new(DingTalkChannel::new(
                    dt.client_id.clone(),
                    dt.client_secret.clone(),
                    dt.allowed_users.clone(),
                )),
            ));
        }
    }

    // ==================== QQ 通道配置 ====================
    // QQ 是腾讯的即时通讯平台
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(ref qq) = config.channels_config.qq {
        // QQ 通道支持两种接收模式：WebSocket 和 Webhook
        if qq.receive_mode == crate::app::agent::config::schema::QQReceiveMode::Webhook {
            // Webhook 模式下，不启动 WebSocket 监听器
            tracing::info!(
                "QQ channel configured with receive_mode=webhook; websocket listener startup skipped."
            );
        } else {
            // WebSocket 模式下，启动 QQ 通道实例
            channels.push(ConfiguredChannel::new(
                "QQ",
                Arc::new(QQChannel::new(
                    qq.app_id.clone(),        // QQ 应用 ID
                    qq.app_secret.clone(),    // QQ 应用密钥
                    qq.allowed_users.clone(), // 允许的用户列表
                )),
            ));
        }
    }

    // ==================== ClawdTalk 通道配置 ====================
    // ClawdTalk 是自定义的通信通道（具体用途需要查看实现）
    if let Some(ref ct) = config.channels_config.clawdtalk {
        channels
            .push(ConfiguredChannel::new("ClawdTalk", Arc::new(ClawdTalkChannel::new(ct.clone()))));
    }

    // 返回所有已配置的通道列表
    channels
}

/// 异步添加 Nostr 通道（如果配置且可用）
///
/// Nostr 是一个去中心化的社交协议，需要异步初始化以连接到中继服务器。
/// 此函数作为独立函数存在，是因为 Nostr 的初始化可能涉及网络 I/O 操作。
///
/// # 参数
///
/// * `config` - 应用主配置对象的引用
/// * `channels` - 可变引用到已配置通道的向量，Nostr 通道将被追加到此向量
/// * `startup_context` - 启动上下文描述，用于错误日志（如 "health check" 或 "startup"）
///
/// # 返回值
///
/// - `None` - Nostr 通道成功添加，或 Nostr 未配置
/// - `Some(String)` - Nostr 通道初始化失败，返回包含错误原因的字符串
///
/// # 平台限制
///
/// 此函数在 `wasm32` 目标平台上不可用，会直接返回 `None`。
///
/// # 异步说明
///
/// 此函数是异步的，因为 Nostr 通道的初始化需要：
/// 1. 连接到中继服务器
/// 2. 验证私钥
/// 3. 可能的网络握手
///
/// # 错误处理
///
/// 如果 Nostr 初始化失败，函数会：
/// 1. 记录警告级别的日志
/// 2. 返回错误原因字符串，但不 panic
/// 3. 允许系统在没有 Nostr 通道的情况下继续运行
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::config::Config;
/// use crate::app::agent::channels::manager::channels_config::{
///     collect_configured_channels,
///     append_nostr_channel_if_available,
/// };
///
/// async fn setup_channels(config: &Config) -> Vec<ConfiguredChannel> {
///     // 首先收集同步通道
///     let mut channels = collect_configured_channels(&config, "startup");
///
///     // 然后异步添加 Nostr 通道
///     if let Some(error) = append_nostr_channel_if_available(
///         &config,
///         &mut channels,
///         "startup"
///     ).await {
///         eprintln!("Nostr 初始化失败: {}", error);
///     }
///
///     channels
/// }
/// ```
pub(crate) async fn append_nostr_channel_if_available(
    config: &Config,
    channels: &mut Vec<ConfiguredChannel>,
    startup_context: &str,
) -> Option<String> {
    // wasm32 平台不支持 Nostr，直接返回 None
    #[cfg(not(target_arch = "wasm32"))]
    {
        // 如果 Nostr 未配置，提前返回 None
        let ns = config.channels_config.nostr.as_ref()?;

        // 尝试创建 Nostr 通道实例
        match NostrChannel::new(&ns.private_key, ns.relays.clone(), &ns.allowed_pubkeys).await {
            Ok(channel) => {
                // 成功创建，添加到通道列表
                channels.push(ConfiguredChannel::new("Nostr", Arc::new(channel)));
                None
            }
            Err(err) => {
                // 初始化失败，记录警告并返回错误原因
                let reason = format!("Nostr init failed during {startup_context}: {err}");
                tracing::warn!("{reason}");
                Some(reason)
            }
        }
    }

    // wasm32 平台的桩实现
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}

#[cfg(test)]
#[path = "channels_config_tests.rs"]
mod channels_config_tests;
