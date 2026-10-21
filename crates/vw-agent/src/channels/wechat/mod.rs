//! 微信 iLink Bot HTTP/JSON 通道。

use super::traits::{Channel, ChannelMessage, SendMessage};
use anyhow::{Context, bail};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;
use vw_config_types::channels::DEFAULT_WECHAT_BASE_URL;

pub(crate) mod protocol;

use protocol::{CHANNEL_VERSION, with_json_headers};

#[derive(Clone)]
pub struct WeChatChannel {
    bot_token: String,
    base_url: String,
    client: Client,
    context_tokens: Arc<RwLock<HashMap<String, String>>>,
}

impl WeChatChannel {
    pub fn new(bot_token: String, base_url: String) -> anyhow::Result<Self> {
        let normalized = base_url.trim_end_matches('/');
        if normalized != DEFAULT_WECHAT_BASE_URL {
            bail!("微信 iLink base_url 不受信任");
        }
        Self::new_with_base_url(bot_token, normalized.to_string())
    }

    #[cfg(test)]
    fn new_for_test(bot_token: String, base_url: String) -> anyhow::Result<Self> {
        Self::new_with_base_url(bot_token, base_url.trim_end_matches('/').to_string())
    }

    fn new_with_base_url(bot_token: String, base_url: String) -> anyhow::Result<Self> {
        Ok(Self {
            bot_token,
            base_url,
            client: Client::builder().build().context("创建微信 iLink HTTP 客户端失败")?,
            context_tokens: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    async fn post(&self, path: &str, body: Value) -> anyhow::Result<Value> {
        let request = self.client.post(self.endpoint(path));
        let response = with_json_headers(request, Some(&self.bot_token))
            .json(&body)
            .send()
            .await
            .with_context(|| format!("请求微信 iLink {path} 失败"))?;
        let status = response.status();
        if status != StatusCode::OK {
            bail!("微信 iLink {path} 返回 HTTP {status}");
        }
        response.json().await.with_context(|| format!("解析微信 iLink {path} 响应失败"))
    }

    fn parse_message(message: &Value) -> anyhow::Result<Option<(ChannelMessage, String)>> {
        if message.get("message_type").and_then(Value::as_u64) != Some(1) {
            return Ok(None);
        }
        let sender = message
            .get("from_user_id")
            .and_then(Value::as_str)
            .filter(|sender| !sender.trim().is_empty())
            .context("微信文本消息缺少 from_user_id")?;
        let context_token = message
            .get("context_token")
            .and_then(Value::as_str)
            .filter(|token| !token.trim().is_empty())
            .context("微信文本消息缺少 context_token")?;
        let items =
            message.get("item_list").and_then(Value::as_array).context("微信消息缺少 item_list")?;
        let mut text = String::new();
        for item in items {
            let item_type = item.get("type").and_then(Value::as_u64).unwrap_or_default();
            if item_type != 1 {
                bail!("微信消息类型 `{item_type}` 暂不支持，当前仅支持 text");
            }
            if let Some(part) = item
                .get("text_item")
                .and_then(|text_item| text_item.get("text"))
                .and_then(Value::as_str)
            {
                text.push_str(part);
            }
        }
        let text = text.trim();
        if text.is_empty() {
            return Ok(None);
        }
        let id = message.get("msg_id").and_then(Value::as_str).unwrap_or(context_token).to_string();
        Ok(Some((
            ChannelMessage {
                id,
                sender: sender.to_string(),
                reply_target: sender.to_string(),
                content: text.to_string(),
                channel: "wechat".to_string(),
                timestamp: message
                    .get("create_time_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or_default(),
                thread_ts: None,
            },
            context_token.to_string(),
        )))
    }
}

#[async_trait]
impl Channel for WeChatChannel {
    fn name(&self) -> &str {
        "wechat"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        let context_token =
            self.context_tokens.read().await.get(&message.recipient).cloned().with_context(
                || format!("微信会话 {} 尚无可用的 context_token", message.recipient),
            )?;
        let response = self
            .post(
                "ilink/bot/sendmessage",
                json!({
                    "msg": {
                        "from_user_id": "",
                        "to_user_id": message.recipient,
                        "client_id": Uuid::new_v4().to_string(),
                        "message_type": 2,
                        "message_state": 2,
                        "context_token": context_token,
                        "item_list": [{"type": 1, "text_item": {"text": message.content}}]
                    },
                    "base_info": {
                        "channel_version": CHANNEL_VERSION,
                        "bot_agent": "VibeWindow"
                    }
                }),
            )
            .await?;
        if response.get("ret").and_then(Value::as_i64).unwrap_or_default() != 0 {
            bail!("微信 iLink sendmessage 返回失败状态");
        }
        Ok(())
    }

    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        if self.bot_token.trim().is_empty() {
            bail!("微信通道缺少 bot_token");
        }
        tracing::info!("微信：正在连接到 iLink...");
        let mut cursor = String::new();
        let mut connection_announced = false;
        loop {
            let response = self
                .post(
                    "ilink/bot/getupdates",
                    json!({
                        "get_updates_buf": cursor,
                        "base_info": {
                            "channel_version": CHANNEL_VERSION,
                            "bot_agent": "VibeWindow"
                        }
                    }),
                )
                .await?;
            if let Some(next_cursor) = response.get("get_updates_buf").and_then(Value::as_str) {
                cursor = next_cursor.to_string();
            }
            let messages = response.get("msgs").and_then(Value::as_array);
            let ret = response.get("ret").and_then(Value::as_i64);
            let errcode = response.get("errcode").and_then(Value::as_i64);
            if ret.is_some_and(|code| code != 0) || errcode.is_some_and(|code| code != 0) {
                let message = response
                    .get("errmsg")
                    .and_then(Value::as_str)
                    .filter(|message| !message.trim().is_empty())
                    .unwrap_or("未知错误");
                bail!(
                    "微信 iLink getupdates 返回失败状态 ret={} errcode={}：{message}",
                    ret.map_or_else(|| "缺失".to_string(), |code| code.to_string()),
                    errcode.map_or_else(|| "缺失".to_string(), |code| code.to_string())
                );
            }
            if !connection_announced {
                tracing::info!("微信：已连接，正在监听消息...");
                connection_announced = true;
            }
            for raw in messages.into_iter().flatten() {
                if let Some((message, context_token)) = Self::parse_message(raw)? {
                    self.context_tokens
                        .write()
                        .await
                        .insert(message.reply_target.clone(), context_token);
                    tx.send(message).await.context("微信消息处理通道已关闭")?;
                }
            }
        }
    }

    async fn health_check(&self) -> bool {
        !self.bot_token.trim().is_empty()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
