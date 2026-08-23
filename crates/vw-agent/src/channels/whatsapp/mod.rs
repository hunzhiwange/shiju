//! WhatsApp Business Cloud API 通道及其 Webhook 接入边界。
//!
//! 生产出站地址必须使用 HTTPS；入站解析仅接受准入号码发送的文本消息。

use super::traits::{Channel, ChannelMessage, SendMessage};
use async_trait::async_trait;
use uuid::Uuid;

/// `WhatsApp` channel — uses `WhatsApp` Business Cloud API
/// 使用 Meta Cloud API 的 WhatsApp 通道。
///
/// This channel operates in webhook mode (push-based) rather than polling.
/// Messages are received via the gateway's `/whatsapp` webhook endpoint.
/// The `listen` method here is a no-op placeholder; actual message handling
/// happens in the gateway when Meta sends webhook events.
fn ensure_https(url: &str) -> anyhow::Result<()> {
    if !url.starts_with("https://") {
        anyhow::bail!(
            "Refusing to transmit sensitive data over non-HTTPS URL: URL scheme must be https"
        );
    }
    Ok(())
}

///
/// # Runtime Negotiation
///
/// This Cloud API channel is automatically selected when `phone_number_id` is set in the config.
/// Use `WhatsAppWebChannel` (with `session_path`) for native Web mode.
/// 配置 `phone_number_id` 时由通道工厂选择本实现；Web 会话配置则选择原生 Web 通道。
pub struct WhatsAppChannel {
    access_token: String,
    endpoint_id: String,
    verify_token: String,
    allowed_numbers: Vec<String>,
    #[cfg(test)]
    api_base_url: Option<String>,
}

impl WhatsAppChannel {
    /// 创建尚未启动监听的 Cloud API 通道。
    pub fn new(
        access_token: String,
        endpoint_id: String,
        verify_token: String,
        allowed_numbers: Vec<String>,
    ) -> Self {
        Self {
            access_token,
            endpoint_id,
            verify_token,
            allowed_numbers,
            #[cfg(test)]
            api_base_url: None,
        }
    }

    fn http_client(&self) -> reqwest::Client {
        crate::app::agent::config::build_runtime_proxy_client("channel.whatsapp")
    }

    #[cfg(test)]
    /// 创建允许本地 HTTP 根地址的测试通道；该绕过不进入生产构建。
    pub(crate) fn new_with_api_base_url(
        access_token: String,
        endpoint_id: String,
        verify_token: String,
        allowed_numbers: Vec<String>,
        api_base_url: String,
    ) -> Self {
        let mut channel = Self::new(access_token, endpoint_id, verify_token, allowed_numbers);
        channel.api_base_url = Some(api_base_url);
        channel
    }

    fn graph_api_url(&self, path: &str) -> String {
        #[cfg(test)]
        if let Some(base_url) = &self.api_base_url {
            return format!("{}/{}", base_url.trim_end_matches('/'), path.trim_start_matches('/'));
        }

        format!("https://graph.facebook.com/v18.0/{}", path.trim_start_matches('/'))
    }

    fn ensure_transport_allowed(&self, url: &str) -> anyhow::Result<()> {
        #[cfg(test)]
        if self.api_base_url.is_some() {
            return Ok(());
        }

        ensure_https(url)
    }

    /// Check if a phone number is allowed (E.164 format: +1234567890)
    fn is_number_allowed(&self, phone: &str) -> bool {
        self.allowed_numbers.iter().any(|n| n == "*" || n == phone)
    }

    /// 返回 Webhook 握手校验令牌。
    pub fn verify_token(&self) -> &str {
        &self.verify_token
    }

    /// 从 Meta Webhook 载荷提取通过号码准入检查的文本消息。
    pub fn parse_webhook_payload(&self, payload: &serde_json::Value) -> Vec<ChannelMessage> {
        let mut messages = Vec::new();

        // WhatsApp Cloud API webhook structure:
        // { "object": "whatsapp_business_account", "entry": [...] }
        let Some(entries) = payload.get("entry").and_then(|e| e.as_array()) else {
            return messages;
        };

        for entry in entries {
            let Some(changes) = entry.get("changes").and_then(|c| c.as_array()) else {
                continue;
            };

            for change in changes {
                let Some(value) = change.get("value") else {
                    continue;
                };

                // Extract messages array
                let Some(msgs) = value.get("messages").and_then(|m| m.as_array()) else {
                    continue;
                };

                for msg in msgs {
                    // Get sender phone number
                    let Some(from) = msg.get("from").and_then(|f| f.as_str()) else {
                        continue;
                    };

                    // Check allowlist
                    let normalized_from =
                        if from.starts_with('+') { from.to_string() } else { format!("+{from}") };

                    if !self.is_number_allowed(&normalized_from) {
                        tracing::warn!(
                            "WhatsApp: ignoring message from unauthorized number: {normalized_from}. \
                            Add it to channels.whatsapp.allowed_numbers in system settings."
                        );
                        continue;
                    }

                    // Extract text content (support text messages only for now)
                    let content = if let Some(text_obj) = msg.get("text") {
                        text_obj.get("body").and_then(|b| b.as_str()).unwrap_or("").to_string()
                    } else {
                        // Could be image, audio, etc. — skip for now
                        tracing::debug!("WhatsApp: skipping non-text message from {from}");
                        continue;
                    };

                    if content.is_empty() {
                        continue;
                    }

                    // Get timestamp
                    let timestamp = msg
                        .get("timestamp")
                        .and_then(|t| t.as_str())
                        .and_then(|t| t.parse::<u64>().ok())
                        .unwrap_or_else(|| {
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs()
                        });

                    messages.push(ChannelMessage {
                        id: Uuid::new_v4().to_string(),
                        reply_target: normalized_from.clone(),
                        sender: normalized_from,
                        content,
                        channel: "whatsapp".to_string(),
                        timestamp,
                        thread_ts: None,
                    });
                }
            }
        }

        messages
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Channel for WhatsAppChannel {
    fn name(&self) -> &str {
        "whatsapp"
    }

    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        // WhatsApp Cloud API: POST to /v18.0/{phone_number_id}/messages
        let url = self.graph_api_url(&format!("{}/messages", self.endpoint_id));

        // Normalize recipient (remove leading + if present for API)
        let to = message.recipient.strip_prefix('+').unwrap_or(&message.recipient);

        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "text",
            "text": {
                "preview_url": false,
                "body": message.content
            }
        });

        self.ensure_transport_allowed(&url)?;

        let resp = self
            .http_client()
            .post(&url)
            .bearer_auth(&self.access_token)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let error_body = resp.text().await.unwrap_or_default();
            let sanitized = crate::app::agent::providers::sanitize_api_error(&error_body);
            tracing::error!("WhatsApp send failed: {status} — {sanitized}");
            anyhow::bail!("WhatsApp API error: {status}");
        }

        Ok(())
    }

    async fn listen(&self, _tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> anyhow::Result<()> {
        // WhatsApp uses webhooks (push-based), not polling.
        // Messages are received via the gateway's /whatsapp endpoint.
        // This method keeps the channel "alive" but doesn't actively poll.
        tracing::info!(
            "WhatsApp channel active (webhook mode). \
            Configure Meta webhook to POST to your gateway's /whatsapp endpoint."
        );

        // Keep the task alive — it will be cancelled when the channel shuts down
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    async fn health_check(&self) -> bool {
        // Check if we can reach the WhatsApp API
        let url = self.graph_api_url(&self.endpoint_id);

        if self.ensure_transport_allowed(&url).is_err() {
            return false;
        }

        self.http_client()
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
