//! 企业微信 AI 机器人 WebSocket 通道。

mod outbound;

use super::outbound_delivery::DeliveryFailureKind;
use super::outbound_format::format_wecom_markdown_tables;
use super::traits::{Channel, ChannelMessage, SendMessage};
use anyhow::{Context, bail};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use outbound::{OutboundItem, OutboundReplies, delivery_error};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{Duration, Instant, MissedTickBehavior};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

const WECOM_WS_URL: &str = "wss://openws.work.weixin.qq.com";
const WECOM_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct WeComChannel {
    bot_id: String,
    secret: String,
    endpoint: String,
    outbound: Arc<RwLock<Option<mpsc::Sender<OutboundItem>>>>,
    reply_requests: Arc<RwLock<HashMap<String, String>>>,
}

impl WeComChannel {
    pub fn new(bot_id: String, secret: String) -> Self {
        Self {
            bot_id,
            secret,
            endpoint: WECOM_WS_URL.to_string(),
            outbound: Arc::new(RwLock::new(None)),
            reply_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[cfg(test)]
    fn new_for_test(bot_id: String, secret: String, endpoint: String) -> Self {
        Self { endpoint, ..Self::new(bot_id, secret) }
    }

    fn subscribe_frame(&self) -> Value {
        json!({
            "cmd": "aibot_subscribe",
            "headers": {"req_id": format!("aibot_subscribe_{}", Uuid::new_v4())},
            "body": {"bot_id": self.bot_id, "secret": self.secret},
        })
    }

    fn validate_subscription_ack(frame: &Value, request_id: &str) -> anyhow::Result<()> {
        let ack_request_id = frame
            .get("headers")
            .and_then(|headers| headers.get("req_id"))
            .and_then(Value::as_str)
            .context("企微订阅响应缺少 headers.req_id")?;
        if ack_request_id != request_id {
            bail!("企微订阅响应 req_id 不匹配");
        }

        let errcode =
            frame.get("errcode").and_then(Value::as_i64).context("企微订阅响应缺少 errcode")?;
        if errcode != 0 {
            bail!("企微订阅失败（errcode: {errcode}）");
        }
        Ok(())
    }

    fn heartbeat_frame() -> Value {
        json!({
            "cmd": "ping",
            "headers": {"req_id": format!("ping_{}", Uuid::new_v4())},
        })
    }

    fn callback_message(frame: &Value) -> anyhow::Result<Option<(ChannelMessage, String)>> {
        if frame.get("cmd").and_then(Value::as_str) != Some("aibot_msg_callback") {
            return Ok(None);
        }

        let body = frame.get("body").context("企微消息回调缺少 body")?;
        let message_type = body.get("msgtype").and_then(Value::as_str).unwrap_or("");
        if message_type != "text" {
            bail!("企微消息类型 `{message_type}` 暂不支持，当前仅支持 text");
        }

        let content = body
            .get("text")
            .and_then(|text| text.get("content"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if content.is_empty() {
            return Ok(None);
        }

        let sender = body
            .get("from")
            .and_then(|from| from.get("userid"))
            .and_then(Value::as_str)
            .context("企微文本消息缺少 from.userid")?;
        let chat_id = body.get("chatid").and_then(Value::as_str).unwrap_or(sender);
        let request_id = frame
            .get("headers")
            .and_then(|headers| headers.get("req_id"))
            .and_then(Value::as_str)
            .context("企微文本消息缺少 headers.req_id")?;
        let message_id = body.get("msgid").and_then(Value::as_str).unwrap_or(request_id);

        Ok(Some((
            ChannelMessage {
                instance_id: "default".to_string(),
                id: message_id.to_string(),
                sender: sender.to_string(),
                reply_target: chat_id.to_string(),
                content: content.to_string(),
                channel: "wecom".to_string(),
                timestamp: body.get("send_time").and_then(Value::as_u64).unwrap_or_default(),
                thread_ts: None,
            },
            request_id.to_string(),
        )))
    }

    fn reply_frame(request_id: &str, content: &str) -> Value {
        let content = format_wecom_markdown_tables(content);
        json!({
            "cmd": "aibot_respond_msg",
            "headers": {"req_id": request_id},
            "body": {
                "msgtype": "stream",
                "stream": {"id": Uuid::new_v4().to_string(), "finish": true, "content": content},
            },
        })
    }
}

#[async_trait]
impl Channel for WeComChannel {
    fn name(&self) -> &str {
        "wecom"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        let request_id = self
            .reply_requests
            .read()
            .await
            .get(&message.recipient)
            .cloned()
            .ok_or_else(|| delivery_error(DeliveryFailureKind::NotSent))?;
        let sender = self
            .outbound
            .read()
            .await
            .clone()
            .ok_or_else(|| delivery_error(DeliveryFailureKind::NotSent))?;
        let (item, completion) =
            OutboundItem::new(request_id.clone(), Self::reply_frame(&request_id, &message.content));
        sender.send(item).await.map_err(|_| delivery_error(DeliveryFailureKind::NotSent))?;
        completion.await.unwrap_or(Err(DeliveryFailureKind::Unknown)).map_err(delivery_error)
    }

    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        if self.bot_id.trim().is_empty() || self.secret.trim().is_empty() {
            bail!("企微通道缺少 bot_id 或 secret");
        }

        let (stream, _) = tokio_tungstenite::connect_async(&self.endpoint)
            .await
            .context("连接企微 WebSocket 失败")?;
        let (mut write, mut read) = stream.split();
        let subscribe = self.subscribe_frame();
        let subscribe_request_id =
            subscribe["headers"]["req_id"].as_str().context("构造企微订阅请求失败")?.to_string();
        write.send(Message::Text(subscribe.to_string().into())).await?;

        let ack = read.next().await.context("企微 WebSocket 在订阅响应前已结束")??;
        let Message::Text(ack) = ack else {
            bail!("企微订阅响应不是文本帧");
        };
        let ack: Value = serde_json::from_str(ack.as_ref()).context("解析企微订阅响应失败")?;
        Self::validate_subscription_ack(&ack, &subscribe_request_id)?;

        let (outbound_tx, mut outbound_rx) = mpsc::channel(32);
        self.reply_requests.write().await.clear();
        *self.outbound.write().await = Some(outbound_tx);
        let mut heartbeat = tokio::time::interval_at(
            Instant::now() + WECOM_HEARTBEAT_INTERVAL,
            WECOM_HEARTBEAT_INTERVAL,
        );
        heartbeat.set_missed_tick_behavior(MissedTickBehavior::Skip);
        tracing::info!("企微：已连接，正在监听消息...");

        let result = async {
            let mut replies = OutboundReplies::default();
            let mut maintenance = tokio::time::interval(Duration::from_millis(25));
            maintenance.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                replies.maintain()?;
                if let Some(item) = replies.next_ready() {
                    let frame = item.frame.to_string();
                    replies.begin(item);
                    tokio::time::timeout(outbound::ACK_TIMEOUT, write.send(Message::Text(frame.into())))
                        .await.context("企微 WebSocket 写入超时")??;
                }
                tokio::select! {
                    _ = maintenance.tick(), if replies.has_pending() => {}
                    _ = heartbeat.tick() => {
                        tokio::time::timeout(outbound::ACK_TIMEOUT, write.send(Message::Text(Self::heartbeat_frame().to_string().into())))
                            .await.context("企微 WebSocket 心跳写入超时")??;
                    }
                    outbound = outbound_rx.recv() => {
                        let Some(item) = outbound else { bail!("企微 WebSocket 发送队列已关闭"); };
                        replies.enqueue(item);
                    }
                    incoming = read.next() => {
                        let Some(incoming) = incoming else { bail!("企微 WebSocket 流已结束"); };
                        let message = incoming.context("读取企微 WebSocket 帧失败")?;
                        let text = match message {
                            Message::Text(text) => text,
                            Message::Close(_) => bail!("企微 WebSocket 已关闭"),
                            _ => continue,
                        };
                        let frame: Value = serde_json::from_str(text.as_ref()).context("解析企微 WebSocket 帧失败")?;
                        replies.acknowledge(&frame);
                        if let Some((message, request_id)) = Self::callback_message(&frame)? {
                            let mut requests = self.reply_requests.write().await;
                            requests.insert(message.reply_target.clone(), request_id.clone());
                            requests.insert(message.sender.clone(), request_id);
                            drop(requests);
                            tx.try_send(message).context("企微消息处理通道不可用或已满")?;
                        }
                    }
                }
            }
        }.await;
        *self.outbound.write().await = None;
        result
    }

    async fn health_check(&self) -> bool {
        !self.bot_id.trim().is_empty() && !self.secret.trim().is_empty()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "outbound_tests.rs"]
mod outbound_tests;
