//! Lark（飞书）WebSocket 长连接模块
//!
//! 本模块实现了 Lark/飞书平台的 WebSocket 长连接事件监听功能。
//! 通过 WebSocket 协议与 Lark 服务器建立持久化连接，实时接收和处理消息事件。
//!
//! # 主要功能
//!
//! - 建立与 Lark 服务器的 WebSocket 长连接
//! - 实现 Protobuf 协议的消息帧编解码
//! - 处理心跳保活机制（ping/pong）
//! - 支持消息分片重组
//! - 解析和处理各类消息事件（文本、富文本、图片）
//! - 实现消息去重和权限校验
//! - 自动发送消息确认（ACK）和表情回应
//!
//! # 协议说明
//!
//! Lark WebSocket 使用 Protobuf 格式的帧（PbFrame）进行通信，
//! 每个帧包含序号、服务标识、方法、头部和载荷等信息。
//! 帧类型分为控制帧（method=0）和数据帧（method>0）。

use super::LarkChannel;
use super::ack::random_lark_ack_reaction;
use super::constants::{LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT, WS_HEARTBEAT_TIMEOUT};
use super::parsing::{
    message_thread, parse_image_key, parse_post_content_details, should_respond_in_group,
    strip_at_placeholders,
};
use super::types::{
    LarkEvent, MsgReceivePayload, PbFrame, PbHeader, WsClientConfig, WsEndpointResp,
};
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message as WsMsg;
use uuid::Uuid;

/// 判断 WebSocket 消息是否应该刷新最后接收时间
///
/// 当接收到表示活跃流量的 WebSocket 帧时，应刷新心跳看门狗计时器，
/// 以防止连接因超时而断开。
///
/// # 参数
///
/// * `msg` - WebSocket 消息引用
///
/// # 返回值
///
/// 如果消息是活跃流量（Binary/Ping/Pong），返回 `true`；否则返回 `false`
///
/// # 示例
///
/// ```ignore
/// use tokio_tungstenite::tungstenite::Message as WsMsg;
///
/// let binary_msg = WsMsg::Binary(vec![1, 2, 3]);
/// assert!(should_refresh_last_recv(&binary_msg));
///
/// let text_msg = WsMsg::Text("hello".to_string());
/// assert!(!should_refresh_last_recv(&text_msg));
/// ```
pub(crate) fn should_refresh_last_recv(msg: &WsMsg) -> bool {
    matches!(msg, WsMsg::Binary(_) | WsMsg::Ping(_) | WsMsg::Pong(_))
}

impl LarkChannel {
    /// 获取 WebSocket 连接端点
    ///
    /// 调用 Lark API 获取 WebSocket 长连接的 URL 和客户端配置信息。
    /// 该配置包括心跳间隔等参数，用于后续建立和维护连接。
    ///
    /// # 返回值
    ///
    /// 成功时返回元组 `(wss_url, client_config)`：
    /// - `wss_url`: WebSocket 安全连接地址
    /// - `client_config`: 客户端配置，包含心跳间隔等参数
    ///
    /// # 错误
    ///
    /// 当 API 返回非零错误码或响应数据为空时，返回错误。
    ///
    /// # HTTP 请求
    ///
    /// - 方法: POST
    /// - 路径: `/callback/ws/endpoint`
    /// - 请求体: JSON 格式，包含 `AppID` 和 `AppSecret`
    async fn get_ws_endpoint(&self) -> anyhow::Result<(String, WsClientConfig)> {
        self.get_ws_endpoint_from_base(self.ws_base()).await
    }

    async fn get_ws_endpoint_from_base(
        &self,
        ws_base: &str,
    ) -> anyhow::Result<(String, WsClientConfig)> {
        let resp = self
            .http_client()
            .post(format!("{ws_base}/callback/ws/endpoint"))
            .header("locale", self.platform.locale_header())
            .json(&serde_json::json!({
                "AppID": self.app_id,
                "AppSecret": self.app_secret,
            }))
            .send()
            .await?
            .json::<WsEndpointResp>()
            .await?;

        // 检查 API 响应码，非零表示请求失败
        if resp.code != 0 {
            anyhow::bail!(
                "Lark WS endpoint failed: code={} msg={}",
                resp.code,
                resp.msg.as_deref().unwrap_or("(none)")
            );
        }

        // 提取响应数据中的端点信息
        let ep = resp.data.ok_or_else(|| anyhow::anyhow!("Lark WS endpoint: empty data"))?;
        Ok((ep.url, ep.client_config.unwrap_or_default()))
    }

    /// WebSocket 长连接事件循环
    /// 建立并维护与 Lark 服务器的 WebSocket 长连接，持续监听和处理消息事件。
    /// 当连接关闭或发生不可恢复的错误时返回，由调用方负责重连。
    ///
    /// # 参数
    ///
    /// * `tx` - 消息发送通道，用于将解析后的消息传递给上层处理
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 连接正常关闭，应触发重连
    /// - `Err(e)`: 发生错误，连接异常终止
    ///
    /// # 功能说明
    ///
    /// 该方法实现以下核心功能：
    ///
    /// 1. **连接建立**: 获取 WebSocket 端点并建立连接
    /// 2. **心跳保活**: 定期发送 ping 帧，监控 pong 响应以检测连接健康
    /// 3. **超时检测**: 检查心跳超时，超时则断开重连
    /// 4. **消息处理**: 解码 Protobuf 帧，处理控制帧和数据帧
    /// 5. **分片重组**: 对分片消息进行缓存和重组
    /// 6. **事件分发**: 解析消息事件并通过通道发送给上层
    ///
    /// # 消息流程
    ///
    /// ```text
    /// WebSocket 帧 → Protobuf 解码 → 分片检查/重组
    ///     → 事件解析 → 权限校验 → 去重 → 内容提取
    ///     → ACK 回复 → 表情回应 → 消息分发
    /// ```
    pub(crate) async fn listen_ws(
        &self,
        tx: tokio::sync::mpsc::Sender<super::ChannelMessage>,
    ) -> anyhow::Result<()> {
        self.ensure_bot_open_id().await;
        let endpoint = self.get_ws_endpoint().await?;
        self.listen_ws_with_endpoint(tx, endpoint).await
    }

    #[cfg(test)]
    async fn listen_ws_with_endpoint_base(
        &self,
        tx: tokio::sync::mpsc::Sender<super::ChannelMessage>,
        ws_base: &str,
    ) -> anyhow::Result<()> {
        self.ensure_bot_open_id().await;
        let endpoint = self.get_ws_endpoint_from_base(ws_base).await?;
        self.listen_ws_with_endpoint(tx, endpoint).await
    }

    #[allow(clippy::too_many_lines)]
    async fn listen_ws_with_endpoint(
        &self,
        tx: tokio::sync::mpsc::Sender<super::ChannelMessage>,
        endpoint: (String, WsClientConfig),
    ) -> anyhow::Result<()> {
        let (wss_url, client_config) = endpoint;
        // 从 WebSocket URL 查询参数中提取 service_id
        // service_id 用于标识 Lark 后端服务实例
        let service_id = wss_url
            .split('?')
            .nth(1)
            .and_then(|qs| {
                qs.split('&')
                    .find(|kv| kv.starts_with("service_id="))
                    .and_then(|kv| kv.split('=').nth(1))
                    .and_then(|v| v.parse::<i32>().ok())
            })
            .unwrap_or(0);
        tracing::info!("Lark: connecting to {wss_url}");

        // 建立 WebSocket 连接
        let (ws_stream, _) = tokio_tungstenite::connect_async(&wss_url).await?;
        let (mut write, mut read) = ws_stream.split();
        tracing::info!("Lark: WS connected (service_id={service_id})");

        // 初始化心跳定时器
        // ping_secs: ping 发送间隔，默认 120 秒，最小 10 秒
        // hb_interval: 心跳定时器
        // timeout_check: 超时检查定时器，每 10 秒检查一次
        let mut ping_secs = client_config.ping_interval.unwrap_or(120).max(10);
        let mut hb_interval = tokio::time::interval(Duration::from_secs(ping_secs));
        let mut timeout_check = tokio::time::interval(Duration::from_secs(10));
        hb_interval.tick().await; // 消费立即触发的初始 tick

        // 消息序号计数器，用于 Protobuf 帧标识
        let mut seq: u64 = 0;
        // 最后一次接收消息的时间，用于心跳超时检测
        let mut last_recv = Instant::now();

        // 立即发送初始 ping 帧（模拟官方 SDK 行为）
        // 这样服务器会开始响应 pong，我们可以据此校准 ping_interval
        seq = seq.wrapping_add(1);
        let initial_ping = PbFrame {
            seq_id: seq,
            log_id: 0,
            service: service_id,
            method: 0,
            headers: vec![PbHeader { key: "type".into(), value: "ping".into() }],
            payload: None,
        };
        if write.send(WsMsg::Binary(initial_ping.encode_to_vec().into())).await.is_err() {
            anyhow::bail!("Lark: initial ping failed");
        }

        // 消息分片缓存：message_id -> (分片槽数组, 创建时间)
        // 用于重组被分片的 WebSocket 消息
        type FragEntry = (Vec<Option<Vec<u8>>>, Instant);
        let mut frag_cache: HashMap<String, FragEntry> = HashMap::new();

        // 主事件循环
        loop {
            tokio::select! {
                biased; // 按分支顺序优先处理

                // 心跳定时器触发
                _ = hb_interval.tick() => {
                    // 构造并发送 ping 帧
                    seq = seq.wrapping_add(1);
                    let ping = PbFrame {
                        seq_id: seq, log_id: 0, service: service_id, method: 0,
                        headers: vec![PbHeader { key: "type".into(), value: "ping".into() }],
                        payload: None,
                    };
                    if write.send(WsMsg::Binary(ping.encode_to_vec().into())).await.is_err() {
                        tracing::warn!("Lark: ping failed, reconnecting");
                        break;
                    }

                    // 清理超过 5 分钟的过期分片缓存，防止内存泄漏
                    let cutoff = Instant::now().checked_sub(Duration::from_secs(300)).unwrap_or(Instant::now());
                    frag_cache.retain(|_, (_, ts)| *ts > cutoff);
                }

                // 超时检查定时器触发
                _ = timeout_check.tick() => {
                    // 如果超过心跳超时阈值未收到任何消息，则断开重连
                    if last_recv.elapsed() > WS_HEARTBEAT_TIMEOUT {
                        tracing::warn!("Lark: heartbeat timeout, reconnecting");
                        break;
                    }
                }

                // 接收 WebSocket 消息
                msg = read.next() => {
                    // 解析原始 WebSocket 消息
                    let raw = match msg {
                        Some(Ok(ws_msg)) => {
                            // 刷新活跃流量的最后接收时间
                            if should_refresh_last_recv(&ws_msg) {
                                last_recv = Instant::now();
                            }
                            match ws_msg {
                                WsMsg::Binary(b) => b, // 二进制消息，继续处理
                                WsMsg::Ping(d) => { let _ = write.send(WsMsg::Pong(d)).await; continue; } // 响应 Ping
                                WsMsg::Close(_) => { tracing::info!("Lark: WS closed — reconnecting"); break; } // 连接关闭
                                _ => continue, // 忽略其他类型消息
                            }
                        }
                        None => { tracing::info!("Lark: WS closed — reconnecting"); break; } // 流结束
                        Some(Err(e)) => { tracing::error!("Lark: WS read error: {e}"); break; } // 读取错误
                    };

                    // 解码 Protobuf 帧
                    let frame = match PbFrame::decode(&raw[..]) {
                        Ok(f) => f,
                        Err(e) => { tracing::error!("Lark: proto decode: {e}"); continue; }
                    };

                    // 处理控制帧（method = 0）
                    if frame.method == 0 {
                        // 处理 pong 响应，更新心跳配置
                        if frame.header_value("type") == "pong" {
                            if let Some(p) = &frame.payload {
                                if let Ok(cfg) = serde_json::from_slice::<WsClientConfig>(p) {
                                    if let Some(secs) = cfg.ping_interval {
                                        let secs = secs.max(10);
                                        // 如果服务器建议的 ping 间隔与当前不同，则更新
                                        if secs != ping_secs {
                                            ping_secs = secs;
                                            hb_interval = tokio::time::interval(Duration::from_secs(ping_secs));
                                            hb_interval.tick().await;
                                            tracing::info!("Lark: ping_interval → {ping_secs}s");
                                        }
                                    }
                                }
                            }
                        }
                        continue;
                    }

                    // 处理数据帧（method > 0）
                    // 提取帧头部信息
                    let msg_type = frame.header_value("type").to_string();
                    let msg_id   = frame.header_value("message_id").to_string();
                    let sum      = frame.header_value("sum").parse::<usize>().unwrap_or(1);
                    let seq_num  = frame.header_value("seq").parse::<usize>().unwrap_or(0);

                    // 立即发送 ACK 确认（飞书要求 3 秒内响应）
                    {
                        let mut ack = frame.clone();
                        ack.payload = Some(br#"{\"code\":200,\"headers\":{},\"data\":[]}"#.to_vec());
                        ack.headers.push(PbHeader { key: "biz_rt".into(), value: "0".into() });
                        let _ = write.send(WsMsg::Binary(ack.encode_to_vec().into())).await;
                    }

                    // 消息分片重组逻辑
                    let sum = if sum == 0 { 1 } else { sum };
                    let payload: Vec<u8> = if sum == 1 || msg_id.is_empty() || seq_num >= sum {
                        // 单片消息或无效分片信息，直接使用原始载荷
                        frame.payload.clone().unwrap_or_default()
                    } else {
                        // 多片消息，需要重组
                        let entry = frag_cache.entry(msg_id.clone())
                            .or_insert_with(|| (vec![None; sum], Instant::now()));
                        // 如果分片数量不匹配，重置缓存
                        if entry.0.len() != sum { *entry = (vec![None; sum], Instant::now()); }
                        // 存储当前分片
                        entry.0[seq_num] = frame.payload.clone();
                        // 检查是否所有分片都已接收
                        if entry.0.iter().all(|s| s.is_some()) {
                            // 所有分片已到齐，合并并移除缓存
                            let full: Vec<u8> = entry.0.iter()
                                .flat_map(|s| s.as_deref().unwrap_or(&[]))
                                .copied().collect();
                            frag_cache.remove(&msg_id);
                            full
                        } else { continue; } // 等待更多分片
                    };

                    // 只处理事件类型消息
                    if msg_type != "event" { continue; }

                    // 解析 Lark 事件
                    let event: LarkEvent = match serde_json::from_slice(&payload) {
                        Ok(e) => e,
                        Err(e) => { tracing::error!("Lark: event JSON: {e}"); continue; }
                    };

                    // 只处理即时消息接收事件
                    if event.header.event_type != "im.message.receive_v1" { continue; }

                    let event_payload = event.event;

                    // 解析消息接收载荷
                    let recv: MsgReceivePayload = match serde_json::from_value(event_payload.clone()) {
                        Ok(r) => r,
                        Err(e) => { tracing::error!("Lark: payload parse: {e}"); continue; }
                    };

                    // 忽略应用和机器人发送的消息，避免自己回复自己
                    if recv.sender.sender_type == "app" || recv.sender.sender_type == "bot" { continue; }

                    // 权限检查：发送者必须在允许列表中
                    let sender_open_id = recv.sender.sender_id.open_id.as_deref().unwrap_or("");
                    if !self.is_user_allowed(sender_open_id) {
                        tracing::warn!("Lark WS: ignoring {sender_open_id} (not in allowed_users)");
                        continue;
                    }

                    let lark_msg = &recv.message;

                    // 消息去重：使用 message_id 防止重复处理
                    {
                        let now = Instant::now();
                        let mut seen = self.ws_seen_ids.write().await;
                        // 清理超过 30 分钟的旧记录
                        seen.retain(|_, t| now.duration_since(*t) < Duration::from_secs(30 * 60));
                        // 检查是否已处理过该消息
                        if seen.contains_key(&lark_msg.message_id) {
                            tracing::debug!("Lark WS: dup {}", lark_msg.message_id);
                            continue;
                        }
                        seen.insert(lark_msg.message_id.clone(), now);
                    }

                    // 根据消息类型解码内容（与 clawdbot-feishu 解析逻辑保持一致）
                    let (text, post_mentioned_open_ids) = match lark_msg.message_type.as_str() {
                        // 纯文本消息
                        "text" => {
                            let v: serde_json::Value = match serde_json::from_str(&lark_msg.content) {
                                Ok(v) => v,
                                Err(_) => continue,
                            };
                            // 提取 text 字段，忽略空文本
                            match v.get("text").and_then(|t| t.as_str()).filter(|s| !s.is_empty()) {
                                Some(t) => (t.to_string(), Vec::new()),
                                None => continue,
                            }
                        }
                        // 富文本（帖子）消息
                        "post" => match parse_post_content_details(&lark_msg.content) {
                            Some(details) => (details.text, details.mentioned_open_ids),
                            None => continue,
                        },
                        // 图片消息
                        "image" => {
                            let text = if let Some(image_key) = parse_image_key(&lark_msg.content) {
                                // 尝试下载并识别图片内容
                                match self.fetch_image_marker(&image_key).await {
                                    Ok(marker) => marker,
                                    Err(error) => {
                                        tracing::warn!(
                                            "Lark WS: failed to download image {image_key}: {error}"
                                        );
                                        // 图片下载失败时使用备用文本
                                        LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT.to_string()
                                    }
                                }
                            } else {
                                tracing::warn!(
                                    "Lark WS: image content missing image_key; using fallback text"
                                );
                                LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT.to_string()
                            };
                            (text, Vec::new())
                        }
                        // 不支持的消息类型
                        _ => { tracing::debug!("Lark WS: skipping unsupported type '{}'", lark_msg.message_type); continue; }
                    };

                    // 移除 @_user_N 格式的占位符
                    let text = strip_at_placeholders(&text);
                    let text = text.trim().to_string();
                    if text.is_empty() { continue; }

                    // 群聊消息：只有在明确被 @ 提及时才响应
                    let bot_open_id = self.resolved_bot_open_id();
                    if lark_msg.chat_type == "group"
                        && !should_respond_in_group(
                            self.mention_only,
                            sender_open_id,
                            &self.group_reply_allowed_sender_ids,
                            bot_open_id.as_deref(),
                            &lark_msg.mentions,
                            &post_mentioned_open_ids,
                        )
                    {
                        continue;
                    }

                    // 选择随机的确认表情
                    let ack_emoji =
                        random_lark_ack_reaction(Some(&event_payload), &text).to_string();
                    let reaction_channel = self.clone();
                    let reaction_message_id = lark_msg.message_id.clone();

                    // 异步发送确认表情回应
                    tokio::spawn(async move {
                        reaction_channel
                            .try_add_ack_reaction(&reaction_message_id, &ack_emoji)
                            .await;
                    });

                    // 构造通道消息，发送给上层处理
                    let channel_msg = super::ChannelMessage {
                        instance_id: "default".to_string(),
                        id: Uuid::new_v4().to_string(),
                        sender: sender_open_id.to_string(),
                        reply_target: lark_msg.chat_id.clone(),
                        content: text,
                        channel: self.channel_name().to_string(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        thread_ts: message_thread(&event_payload),
                    };

                    tracing::debug!("Lark WS: message in {}", lark_msg.chat_id);
                    if tx.send(channel_msg).await.is_err() { break; }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "ws_tests.rs"]
mod ws_tests;
