//! Preflight every Markdown payload before sending an acknowledged prefix.

use super::*;
use crate::app::agent::channels::markdown_chunk::split_markdown;
use crate::app::agent::channels::outbound_delivery::{ChannelDeliveryError, DeliveryFailureKind};
use crate::app::agent::channels::outbound_format::normalize_chat_markdown;
use anyhow::Context;
use std::time::Duration;

// Conservative local budgets; these do not claim to be platform protocol limits.
const BODY_BYTES: usize = 3000;
const JSON_BYTES: usize = 16 * 1024;
const MARKDOWN_MAX_CHARS: usize = 3500;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

fn delivery_error(
    confirmed: usize,
    total: usize,
    index: usize,
    kind: DeliveryFailureKind,
) -> anyhow::Error {
    ChannelDeliveryError {
        confirmed_parts: confirmed,
        total_parts: total,
        failed_part_index: index,
        kind,
    }
    .into()
}

impl DingTalkChannel {
    fn webhook_reply_body(content: &str, title: &str) -> serde_json::Value {
        serde_json::json!({
            "msgtype": "markdown",
            "markdown": { "title": title, "text": content },
        })
    }

    pub(super) fn webhook_reply_payloads(message: &SendMessage) -> anyhow::Result<Vec<Vec<u8>>> {
        let normalized = normalize_chat_markdown(&message.content);
        let chunks = split_markdown(&normalized, BODY_BYTES)
            .map_err(|_| delivery_error(0, 1, 0, DeliveryFailureKind::NotSent))?;
        let total = chunks.len();
        let title = message.subject.as_deref().unwrap_or("VibeWindow");
        chunks
            .iter()
            .enumerate()
            .map(|(index, chunk)| {
                let error = || delivery_error(0, total, index, DeliveryFailureKind::NotSent);
                if chunk.len() > BODY_BYTES || chunk.chars().count() > MARKDOWN_MAX_CHARS {
                    return Err(error());
                }
                let bytes = serde_json::to_vec(&Self::webhook_reply_body(chunk, title))
                    .map_err(|_| error())?;
                if bytes.len() > JSON_BYTES {
                    return Err(error());
                }
                Ok(bytes)
            })
            .collect()
    }

    pub(super) async fn send_webhook_reply(&self, message: &SendMessage) -> anyhow::Result<()> {
        // Clone the route once; neither cache updates nor a slow endpoint hold the mapping lock.
        let webhook_url = self.session_webhooks.read().await.get(&message.recipient).cloned();
        let payloads = Self::webhook_reply_payloads(message)?;
        let total = payloads.len();
        let webhook_url = webhook_url
            .ok_or_else(|| delivery_error(0, total, 0, DeliveryFailureKind::NotSent))
            .context("未找到会话 webhook，用户必须先发送消息以建立会话")?;
        let builder = crate::channels::http::client_builder()
            .retry(reqwest::retry::never())
            .redirect(reqwest::redirect::Policy::none());
        let client =
            crate::app::agent::config::apply_runtime_proxy_to_builder(builder, "channel.dingtalk")
                .build()
                .map_err(|_| delivery_error(0, total, 0, DeliveryFailureKind::NotSent))?;
        self.deliver_webhook_payloads(&client, &webhook_url, payloads, REQUEST_TIMEOUT).await
    }

    pub(super) async fn deliver_webhook_payloads(
        &self,
        client: &reqwest::Client,
        webhook_url: &str,
        payloads: Vec<Vec<u8>>,
        timeout: Duration,
    ) -> anyhow::Result<()> {
        let total = payloads.len();
        for (index, payload) in payloads.into_iter().enumerate() {
            let error = |kind| delivery_error(index, total, index, kind);
            let response = client
                .post(webhook_url)
                .timeout(timeout)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(payload)
                .send()
                .await
                .map_err(|_| error(DeliveryFailureKind::Unknown))?;
            let status = response.status();
            if !status.is_success() {
                let kind =
                    if status.is_client_error() && status != reqwest::StatusCode::REQUEST_TIMEOUT {
                        DeliveryFailureKind::Rejected
                    } else {
                        DeliveryFailureKind::Unknown
                    };
                return Err(error(kind)).context(format!("钉钉 webhook 回复失败 ({status})"));
            }
            let acknowledgement: serde_json::Value =
                response.json().await.map_err(|_| error(DeliveryFailureKind::Unknown))?;
            match acknowledgement.get("errcode").and_then(serde_json::Value::as_i64) {
                Some(0) => {}
                Some(_) => return Err(error(DeliveryFailureKind::Rejected)),
                None => return Err(error(DeliveryFailureKind::Unknown)),
            }
        }
        Ok(())
    }
}
