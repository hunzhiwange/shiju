//! Nostr 通道实现模块
//!
//! 本模块实现了基于 Nostr 协议的通道（Channel），支持两种私密消息协议：
//! - **NIP-04**：传统的加密直接消息（Kind 4），使用 ECDH 密钥协商和 AES-CBC 加密
//! - **NIP-17**：礼物包装（Gift Wrap）私密消息（Kind 1059），提供更好的元数据隐私保护
//!
//! # 核心特性
//!
//! - **协议追踪**：自动记录发送者使用的协议类型，确保回复使用相同的协议
//! - **发送者白名单**：支持配置允许的公钥列表或允许所有人（`*`）
//! - **中继池管理**：支持连接多个 Nostr 中继服务器
//! - **异步监听**：后台持续监听传入消息并转发到消息通道
//!
//! # 架构位置
//!
//! 本模块位于 `src/app/agent/channels/` 目录下，实现了 `Channel` trait，
//! 是 VibeWindow 多通道集成架构的一部分。与其他通道（如 Telegram、Slack 等）
//! 共享相同的接口约定，便于统一调度和管理。
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::agent::channels::nostr::NostrChannel;
//! use crate::app::agent::channels::traits::Channel;
//!
//! // 创建 Nostr 通道
//! let channel = NostrChannel::new(
//!     "nsec1...",                    // 私钥（bech32 或 hex 格式）
//!     vec!["wss://relay.damus.io".to_string()], // 中继服务器列表
//!     &["*".to_string()],            // 允许所有发送者
//! ).await?;
//!
//! // 检查健康状态
//! let healthy = channel.health_check().await;
//!
//! // 发送消息
//! let msg = SendMessage {
//!     recipient: "npub1...".to_string(),
//!     content: "Hello from VibeWindow!".to_string(),
//! };
//! channel.send(&msg).await?;
//! ```

use super::traits::{Channel, ChannelMessage, SendMessage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use nostr_sdk::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Nostr 消息协议类型枚举
///
/// 用于追踪发送者使用的协议类型，确保回复消息使用相同的协议。
/// 这样可以保持对话的协议一致性，避免协议不匹配导致的解密失败。
///
/// # 变体说明
///
/// - `Nip04`：传统加密直接消息协议（Kind 4）
/// - `Nip17`：礼物包装私密消息协议（Kind 1059），提供更好的隐私保护
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NostrProtocol {
    /// NIP-04 协议：传统的加密直接消息
    /// 使用 ECDH 密钥协商和 AES-256-CBC 加密
    Nip04,

    /// NIP-17 协议：礼物包装私密消息
    /// 使用密封发送者（Sealed Sender）和礼物包装技术，提供更好的元数据隐私
    Nip17,
}

/// 发送者白名单策略枚举
///
/// 定义了两种允许发送者的策略：
/// - 允许所有人发送消息（通配符 `*`）
/// - 仅允许特定公钥列表中的发送者
///
/// # 安全考虑
///
/// 在生产环境中，建议使用 `Set` 变体并明确指定允许的公钥列表，
/// 以避免接收来自未知发送者的消息。
#[derive(Debug, Clone)]
enum AllowList {
    /// 允许任何公钥发送消息
    /// 配置值为 `"*"` 时使用此变体
    Any,

    /// 仅允许指定公钥列表中的发送者
    /// 空列表表示拒绝所有发送者（deny-all）
    Set(Vec<PublicKey>),
}

impl AllowList {
    /// 将配置字符串列表解析为类型化的白名单
    ///
    /// # 解析规则
    ///
    /// - **空列表**：返回空的 `Set`，表示拒绝所有发送者
    /// - **包含 `"*"`**：返回 `Any`，表示允许所有发送者
    /// - **其他情况**：将每个字符串解析为公钥，返回包含这些公钥的 `Set`
    ///
    /// # 参数
    ///
    /// - `raw`：原始配置字符串切片，每个字符串应为有效的 Nostr 公钥（bech32 或 hex 格式）
    ///
    /// # 返回值
    ///
    /// - `Ok(AllowList)`：成功解析的白名单
    /// - `Err`：如果某个字符串无法解析为有效的公钥
    ///
    /// # 错误处理
    ///
    /// 如果任何一个公钥字符串格式无效，将返回错误并包含具体是哪个公钥解析失败。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 允许所有发送者
    /// let list = AllowList::parse(&["*".to_string()])?;
    /// assert!(list.is_allowed(&some_pubkey));
    ///
    /// // 仅允许特定发送者
    /// let list = AllowList::parse(&[
    ///     "npub1abcdef...".to_string(),
    ///     "npub1ghijkl...".to_string(),
    /// ])?;
    ///
    /// // 拒绝所有发送者
    /// let list = AllowList::parse(&[])?;
    /// assert!(!list.is_allowed(&any_pubkey));
    /// ```
    fn parse(raw: &[String]) -> Result<Self> {
        // 空列表表示拒绝所有发送者
        if raw.is_empty() {
            return Ok(Self::Set(Vec::new()));
        }

        // 如果包含通配符 "*"，则允许所有发送者
        if raw.iter().any(|p| p == "*") {
            return Ok(Self::Any);
        }

        // 解析每个公钥字符串
        let mut keys = Vec::with_capacity(raw.len());
        for s in raw {
            keys.push(PublicKey::parse(s).with_context(|| format!("无效的允许公钥: {s}"))?);
        }

        Ok(Self::Set(keys))
    }

    /// 检查指定公钥是否在白名单中
    ///
    /// # 参数
    ///
    /// - `pubkey`：要检查的 Nostr 公钥
    ///
    /// # 返回值
    ///
    /// - `true`：该公钥被允许发送消息
    /// - `false`：该公钥不在白名单中
    fn is_allowed(&self, pubkey: &PublicKey) -> bool {
        match self {
            // 如果是 Any 变体，允许所有公钥
            Self::Any => true,
            // 如果是 Set 变体，检查公钥是否在列表中
            Self::Set(keys) => keys.iter().any(|k| k == pubkey),
        }
    }
}

/// Nostr 通道实现
///
/// 实现了 `Channel` trait，支持通过 Nostr 协议发送和接收私密消息。
/// 同时支持 NIP-04（传统）和 NIP-17（礼物包装）两种私密消息协议。
///
/// # 协议选择策略
///
/// - **回复消息**：使用发送者最初使用的协议，确保对话一致性
/// - **主动发送**：默认使用 NIP-17 协议，提供更好的隐私保护
///
/// # 线程安全
///
/// - `client`：nostr-sdk 的 Client 是线程安全的，可以在多个任务间共享
/// - `sender_protocols`：使用 `Arc<RwLock>` 包装，支持并发读取和独占写入
///
/// # 字段说明
///
/// - `client`：Nostr SDK 客户端，管理中继连接和消息收发
/// - `public_key`：本节点的公钥，用于订阅发往本节点的消息
/// - `allowed`：发送者白名单策略
/// - `sender_protocols`：记录每个发送者使用的协议类型，用于回复时选择相同协议
pub struct NostrChannel {
    /// Nostr SDK 客户端实例
    /// 负责与中继服务器通信、签名和加解密
    client: Client,

    /// 本节点的公钥
    /// 用于订阅发往本节点的消息
    public_key: PublicKey,

    /// 发送者白名单
    /// 控制哪些公钥可以向本节点发送消息
    allowed: AllowList,

    /// 发送者协议映射表
    /// 记录每个发送者最后使用的协议类型，确保回复使用相同协议
    /// 使用 Arc<RwLock> 实现线程安全的共享访问
    sender_protocols: Arc<RwLock<HashMap<PublicKey, NostrProtocol>>>,
}

impl NostrChannel {
    /// 创建新的 Nostr 通道实例
    ///
    /// 此方法执行以下初始化步骤：
    /// 1. 解析私钥并派生公钥
    /// 2. 解析允许的公钥列表
    /// 3. 创建 Nostr 客户端并配置签名器
    /// 4. 添加中继服务器
    /// 5. 连接到所有中继服务器
    ///
    /// # 参数
    ///
    /// - `private_key`：Nostr 私钥，支持 bech32（`nsec1...`）或 hex 格式
    /// - `relays`：中继服务器 URL 列表，应使用 `wss://` 协议
    /// - `allowed_pubkeys`：允许的发送者公钥列表
    ///   - 空列表：拒绝所有发送者
    ///   - 包含 `"*"`：允许所有发送者
    ///   - 其他：仅允许列表中的公钥
    ///
    /// # 返回值
    ///
    /// - `Ok(NostrChannel)`：成功创建的通道实例
    /// - `Err`：初始化失败，可能原因包括：
    ///   - 私钥格式无效
    ///   - 公钥格式无效
    ///   - 中继服务器连接失败
    ///
    /// # 异步行为
    ///
    /// 此方法是异步的，因为需要与中继服务器建立连接。
    /// 连接过程可能需要几秒钟，取决于网络状况和中继服务器响应速度。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let channel = NostrChannel::new(
    ///     "nsec1abcdefghijklmnopqrstuvwxyz1234567890abcdefghijklmnopqrst",
    ///     vec![
    ///         "wss://relay.damus.io".to_string(),
    ///         "wss://nos.lol".to_string(),
    ///     ],
    ///     &["*".to_string()],
    /// ).await?;
    /// ```
    pub async fn new(
        private_key: &str,
        relays: Vec<String>,
        allowed_pubkeys: &[String],
    ) -> Result<Self> {
        // 解析私钥，同时派生公钥
        let keys = Keys::parse(private_key).context("无效的 Nostr 私钥")?;
        let public_key = keys.public_key();

        // 解析允许的公钥白名单
        let allowed = AllowList::parse(allowed_pubkeys)?;

        // 创建客户端并配置签名器
        let client = Client::builder().signer(keys).build();

        // 添加所有中继服务器
        for relay in &relays {
            client
                .add_relay(relay.as_str())
                .await
                .with_context(|| format!("添加中继服务器失败: {relay}"))?;
        }

        // 连接到所有中继服务器
        client.connect().await;

        Ok(Self {
            client,
            public_key,
            allowed,
            sender_protocols: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for NostrChannel {
    /// 返回通道名称
    ///
    /// # 返回值
    ///
    /// 固定返回 `"nostr"`，用于标识通道类型
    fn name(&self) -> &str {
        "nostr"
    }

    /// 发送消息给指定接收者
    ///
    /// 自动选择合适的协议发送消息：
    /// - 如果之前收到过该接收者的消息，使用相同的协议回复
    /// - 否则默认使用 NIP-17 协议（更好的隐私保护）
    ///
    /// # 参数
    ///
    /// - `message`：要发送的消息，包含接收者公钥和消息内容
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：消息发送成功
    /// - `Err`：发送失败，可能原因包括：
    ///   - 接收者公钥格式无效
    ///   - 客户端没有配置签名器
    ///   - 加密失败
    ///   - 中继服务器发送失败
    ///
    /// # 协议细节
    ///
    /// ## NIP-17（礼物包装）
    /// - 使用 Kind 1059（Gift Wrap）
    /// - 提供密封发送者和随机时间戳，保护元数据隐私
    /// - 是推荐的私密消息协议
    ///
    /// ## NIP-04（传统加密）
    /// - 使用 Kind 4（Encrypted Direct Message）
    /// - 使用 ECDH 和 AES-256-CBC 加密
    /// - 元数据（发送者、时间戳）是公开的
    /// - 保留用于向后兼容
    async fn send(&self, message: &SendMessage) -> Result<()> {
        // 解析接收者公钥
        let recipient = PublicKey::parse(&message.recipient).context("无效的接收者 Nostr 公钥")?;

        // 查找该接收者上次使用的协议；默认使用 NIP-17
        let protocol = {
            let map = self.sender_protocols.read().await;
            map.get(&recipient).copied().unwrap_or(NostrProtocol::Nip17)
        };

        match protocol {
            NostrProtocol::Nip17 => {
                // NIP-17：礼物包装私密消息（nostr-sdk 0.37 API）
                // 获取签名器，用于加密和签名
                let signer = self.client.signer().await.context("客户端没有签名器")?;

                // 构建并发送 NIP-17 消息
                let event = EventBuilder::private_msg(&signer, recipient, &message.content, [])
                    .await
                    .context("构建 NIP-17 消息失败")?;

                self.client.send_event(event).await.context("发送 NIP-17 消息失败")?;

                tracing::debug!(
                    "已发送 NIP-17 消息到 {}",
                    recipient.to_bech32().unwrap_or_default()
                );
            }
            NostrProtocol::Nip04 => {
                // NIP-04：传统加密直接消息（Kind 4）
                // 获取签名器用于加密
                let signer = self.client.signer().await.context("客户端没有签名器")?;

                // 使用 NIP-04 加密消息内容
                let encrypted = signer
                    .nip04_encrypt(&recipient, &message.content)
                    .await
                    .context("NIP-04 加密失败")?;

                // 构建 Kind 4 事件并添加接收者标签
                let builder = EventBuilder::new(Kind::EncryptedDirectMessage, encrypted)
                    .tag(Tag::public_key(recipient));

                // 发送事件
                self.client.send_event_builder(builder).await.context("发送 NIP-04 消息失败")?;

                tracing::debug!(
                    "已发送 NIP-04 消息到 {}",
                    recipient.to_bech32().unwrap_or_default()
                );
            }
        }

        Ok(())
    }

    /// 监听传入消息并转发到消息通道
    ///
    /// 此方法会阻塞运行，持续监听来自中继服务器的消息通知。
    /// 当收到有效的私密消息时，解密并转发到提供的消息通道。
    ///
    /// # 参数
    ///
    /// - `tx`：消息发送端，用于将接收到的消息转发给消费者
    ///
    /// # 返回值
    ///
    /// - `Ok(())`：监听正常结束（通常是因为消息通道关闭或中继池关闭）
    /// - `Err`：监听失败，可能原因包括：
    ///   - 订阅失败
    ///   - 通知通道关闭
    ///
    /// # 订阅策略
    ///
    /// 同时订阅两种私密消息类型：
    /// - Kind 4（NIP-04 加密直接消息）
    /// - Kind 1059（NIP-17 礼物包装）
    ///
    /// 使用 `limit(10)` 限制历史消息数量，提高中继兼容性。
    /// 监听开始前的旧消息会被过滤掉。
    ///
    /// # 消息处理流程
    ///
    /// 1. 接收中继通知
    /// 2. 过滤掉监听开始前的旧消息
    /// 3. 检查发送者是否在白名单中
    /// 4. 解密消息内容
    /// 5. 记录发送者使用的协议类型
    /// 6. 转发消息到输出通道
    ///
    /// # 时间戳处理
    ///
    /// - **NIP-04**：使用事件的 `created_at` 字段（无抖动）
    /// - **NIP-17**：使用内部 rumor 的 `created_at` 字段（外层礼物包装的时间戳有抖动）
    async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> Result<()> {
        // 记录监听开始时间，用于过滤旧消息
        let listen_start = Timestamp::now();

        // 订阅 NIP-04（Kind 4）和 NIP-17 礼物包装（Kind 1059）两种私密消息
        // 使用 limit(10) 提高中继兼容性；监听开始前的事件将在下方使用
        // 真实消息时间戳（NIP-17 使用 rumor.created_at，因为外层礼物包装时间戳有抖动）进行过滤
        let filter = Filter::new()
            .pubkey(self.public_key)
            .kinds(vec![Kind::EncryptedDirectMessage, Kind::GiftWrap])
            .limit(10);

        self.client.subscribe(vec![filter], None).await.context("订阅 Nostr 事件失败")?;

        tracing::info!(
            "Nostr 通道开始监听，身份：{}",
            self.public_key.to_bech32().unwrap_or_default()
        );

        // 克隆 Arc 以在异步循环中使用
        let sender_protocols = Arc::clone(&self.sender_protocols);
        let signer = self.client.signer().await.context("客户端没有签名器")?;

        // 主监听循环
        loop {
            // 接收来自中继池的通知
            let notification =
                self.client.notifications().recv().await.context("通知通道已关闭")?;

            match notification {
                RelayPoolNotification::Event { event, .. } => {
                    // 根据事件类型处理消息
                    let result = match event.kind {
                        Kind::EncryptedDirectMessage => {
                            // NIP-04：created_at 是真实时间戳（无抖动）

                            // 过滤掉监听开始前的旧消息
                            if event.created_at < listen_start {
                                continue;
                            }

                            // 检查发送者是否在白名单中
                            if !self.allowed.is_allowed(&event.pubkey) {
                                tracing::warn!(
                                    "Nostr: 忽略来自未授权公钥的 NIP-04 消息: {}",
                                    event.pubkey.to_hex()
                                );
                                continue;
                            }

                            // 尝试解密 NIP-04 消息
                            match signer.nip04_decrypt(&event.pubkey, &event.content).await {
                                Ok(content) => {
                                    let sender = event.pubkey;

                                    // 记录发送者使用的协议类型
                                    sender_protocols
                                        .write()
                                        .await
                                        .insert(sender, NostrProtocol::Nip04);

                                    // 返回解析后的消息数据
                                    Some((
                                        event.id.to_hex(),
                                        sender.to_hex(),
                                        content,
                                        event.created_at.as_u64(),
                                    ))
                                }
                                Err(e) => {
                                    tracing::warn!("解密 NIP-04 消息失败: {e}");
                                    None
                                }
                            }
                        }
                        Kind::GiftWrap => {
                            // NIP-17：先拆开礼物包装，然后检查 rumor 的 created_at
                            // （外层礼物包装的时间戳有抖动，用于隐私保护）

                            match self.client.unwrap_gift_wrap(&event).await {
                                Ok(unwrapped) => {
                                    let rumor = unwrapped.rumor;

                                    // 使用 rumor 的时间戳过滤旧消息
                                    if rumor.created_at < listen_start {
                                        continue;
                                    }

                                    let sender = rumor.pubkey;

                                    // 检查发送者是否在白名单中
                                    if !self.allowed.is_allowed(&sender) {
                                        tracing::warn!(
                                            "Nostr: 忽略来自未授权公钥的 NIP-17 消息: {}",
                                            sender.to_hex()
                                        );
                                        continue;
                                    }

                                    // 记录发送者使用的协议类型
                                    sender_protocols
                                        .write()
                                        .await
                                        .insert(sender, NostrProtocol::Nip17);

                                    // 返回解析后的消息数据
                                    Some((
                                        event.id.to_hex(),
                                        sender.to_hex(),
                                        rumor.content.clone(),
                                        rumor.created_at.as_u64(),
                                    ))
                                }
                                Err(e) => {
                                    tracing::warn!("拆开 NIP-17 礼物包装失败: {e}");
                                    None
                                }
                            }
                        }
                        _ => None, // 忽略其他类型的事件
                    };

                    // 如果成功解析消息，转发到输出通道
                    if let Some((id, sender_hex, content, timestamp)) = result {
                        let msg = ChannelMessage {
                            instance_id: "default".into(),
                            id,
                            sender: sender_hex.clone(),
                            reply_target: sender_hex,
                            content,
                            channel: "nostr".to_string(),
                            timestamp,
                            thread_ts: None,
                        };

                        // 发送消息到通道，如果通道关闭则退出监听
                        if tx.send(msg).await.is_err() {
                            tracing::info!("Nostr 监听器：消息总线已关闭，停止监听");
                            break;
                        }
                    }
                }
                RelayPoolNotification::Shutdown => {
                    // 中继池关闭，退出监听
                    tracing::info!("Nostr 中继池已关闭");
                    break;
                }
                // 忽略消息和认证通知
                RelayPoolNotification::Message { .. }
                | RelayPoolNotification::Authenticated { .. } => {}
                // 忽略其他通知类型（RelayStatus 在 0.37 中已弃用）
                _ => {}
            }
        }

        Ok(())
    }

    /// 检查通道健康状态
    ///
    /// 检查是否有任何中继服务器处于连接状态。
    /// 只要有一个中继连接正常，就认为通道是健康的。
    ///
    /// # 返回值
    ///
    /// - `true`：至少有一个中继服务器已连接
    /// - `false`：所有中继服务器都断开连接
    ///
    /// # 使用场景
    ///
    /// - 健康检查接口
    /// - 监控和告警系统
    /// - 降级策略决策
    async fn health_check(&self) -> bool {
        // 检查是否有任何中继服务器处于连接状态
        self.client.relays().await.values().any(|r| r.is_connected())
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
