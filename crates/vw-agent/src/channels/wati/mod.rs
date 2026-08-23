//! WATI Webhook 通道，负责号码准入校验和 WhatsApp Business 消息发送。

use super::traits::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
use uuid::Uuid;

/// WATI WhatsApp Business API channel.
///
/// This channel operates in webhook mode (push-based) rather than polling.
/// Messages are received via the gateway's `/wati` webhook endpoint.
/// The `listen` method here is a keepalive placeholder; actual message handling
/// happens in the gateway when WATI sends webhook events.
pub struct WatiChannel {
    api_token: String,
    api_url: String,
    tenant_id: Option<String>,
    allowed_numbers: Vec<String>,
    client: reqwest::Client,
}

impl WatiChannel {
    pub fn new(
        api_token: String,
        api_url: String,
        tenant_id: Option<String>,
        allowed_numbers: Vec<String>,
    ) -> Self {
        Self {
            api_token,
            api_url,
            tenant_id,
            allowed_numbers,
            client: crate::app::agent::config::build_runtime_proxy_client("channel.wati"),
        }
    }

    /// Check if a phone number is allowed (E.164 format: +1234567890).
    fn is_number_allowed(&self, phone: &str) -> bool {
        self.allowed_numbers.iter().any(|n| n == "*" || n == phone)
    }

    /// Build the target field for the WATI API, prefixing with tenant_id if set.
    fn build_target(&self, phone: &str) -> String {
        // Strip leading '+' — WATI expects bare digits
        let bare = phone.strip_prefix('+').unwrap_or(phone);
        if let Some(ref tid) = self.tenant_id {
            if bare.starts_with(&format!("{tid}:")) {
                bare.to_string()
            } else {
                format!("{tid}:{bare}")
            }
        } else {
            bare.to_string()
        }
    }

    /// Parse an incoming webhook payload from WATI and extract messages.
    ///
    /// WATI's webhook payloads have variable field names depending on the API
    /// version and configuration, so we try multiple paths for each field.
    pub fn parse_webhook_payload(&self, payload: &serde_json::Value) -> Vec<ChannelMessage> {
        let mut messages = Vec::new();

        // Extract text — try multiple field paths
        let text = payload
            .get("text")
            .and_then(|v| v.as_str())
            .or_else(|| {
                payload
                    .get("message")
                    .and_then(|m| m.get("text").or_else(|| m.get("body")))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("")
            .trim();

        if text.is_empty() {
            return messages;
        }

        // Check fromMe — skip outgoing messages
        let from_me = payload
            .get("fromMe")
            .or_else(|| payload.get("from_me"))
            .or_else(|| payload.get("owner"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if from_me {
            tracing::debug!("WATI: skipping fromMe message");
            return messages;
        }

        // Extract waId (sender phone number)
        let wa_id = payload
            .get("waId")
            .or_else(|| payload.get("wa_id"))
            .or_else(|| payload.get("from"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        if wa_id.is_empty() {
            return messages;
        }

        // Normalize phone to E.164 format
        let normalized_phone =
            if wa_id.starts_with('+') { wa_id.to_string() } else { format!("+{wa_id}") };

        // Check allowlist
        if !self.is_number_allowed(&normalized_phone) {
            tracing::warn!(
                "WATI: ignoring message from unauthorized sender: {normalized_phone}. \
                Add it to channels.wati.allowed_numbers in system settings."
            );
            return messages;
        }

        // Extract timestamp — handle unix seconds, unix ms, or ISO string
        let timestamp = payload
            .get("timestamp")
            .or_else(|| payload.get("created"))
            .map(|t| {
                if let Some(secs) = t.as_u64() {
                    // Distinguish seconds from milliseconds (ms > 10_000_000_000)
                    if secs > 10_000_000_000 { secs / 1000 } else { secs }
                } else if let Some(s) = t.as_str() {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.timestamp().cast_unsigned())
                        .unwrap_or_else(|| {
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs()
                        })
                } else {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                }
            })
            .unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            });

        messages.push(ChannelMessage {
            id: Uuid::new_v4().to_string(),
            reply_target: normalized_phone.clone(),
            sender: normalized_phone,
            content: text.to_string(),
            channel: "wati".to_string(),
            timestamp,
            thread_ts: None,
        });

        messages
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for WatiChannel {
    fn name(&self) -> &str {
        "wati"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        let target = self.build_target(&message.recipient);

        let body = serde_json::json!({
            "target": target,
            "text": message.content
        });

        let url = format!("{}/api/ext/v3/conversations/messages/text", self.api_url);

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_token)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error_body = resp.text().await.unwrap_or_default();
            let sanitized = crate::app::agent::providers::sanitize_api_error(&error_body);
            tracing::error!("WATI send failed: {status} — {sanitized}");
            anyhow::bail!("WATI API error: {status}");
        }

        Ok(())
    }

    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        // WATI uses webhooks (push-based), not polling.
        // Messages are received via the gateway's /wati endpoint.
        tracing::info!(
            "WATI channel active (webhook mode). \
            Configure WATI webhook to POST to your gateway's /wati endpoint."
        );

        // Keep the task alive — it will be cancelled when the channel shuts down
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    async fn health_check(&self) -> bool {
        let url = format!("{}/api/ext/v3/contacts/count", self.api_url);

        self.client
            .get(&url)
            .bearer_auth(&self.api_token)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn start_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        // WATI API does not support typing indicators
        Ok(())
    }

    async fn stop_typing(&self, _recipient: &str) -> anyhow::Result<()> {
        // WATI API does not support typing indicators
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
