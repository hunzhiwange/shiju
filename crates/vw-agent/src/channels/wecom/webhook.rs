//! Restricted, outbound-only group robot Markdown sender.

use super::super::markdown_chunk::split_markdown;
use super::super::outbound_delivery::{ChannelDeliveryError, DeliveryFailureKind};
use super::super::outbound_format::format_wecom_markdown_tables;
use serde_json::{Value, json};
use std::time::Duration;

const BODY_BYTES: usize = 3000;
const RESPONSE_BYTES: usize = 16 * 1024;

/// One default group robot endpoint. Deliberately does not expose credentials through Debug.
pub struct WeComWebhook {
    endpoint: reqwest::Url,
    client: reqwest::Client,
    timeout: Duration,
}

fn failure(
    confirmed: usize,
    total: usize,
    kind: DeliveryFailureKind,
    stage: &str,
) -> anyhow::Error {
    anyhow::Error::new(ChannelDeliveryError {
        confirmed_parts: confirmed,
        total_parts: total,
        failed_part_index: confirmed,
        kind,
    })
    .context(format!("企微群机器人 {stage}"))
}

impl WeComWebhook {
    /// Accept only the official HTTPS send endpoint with one nonempty key.
    pub fn new(url: &str) -> anyhow::Result<Self> {
        let invalid = || failure(0, 1, DeliveryFailureKind::NotSent, "URL 校验失败");
        let endpoint = reqwest::Url::parse(url).map_err(|_| invalid())?;
        let parameters = endpoint.query_pairs().collect::<Vec<_>>();
        if endpoint.scheme() != "https"
            || endpoint.host_str() != Some("qyapi.weixin.qq.com")
            || endpoint.port_or_known_default() != Some(443)
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || url.split('/').nth(2).is_some_and(|authority| authority.contains('@'))
            || endpoint.fragment().is_some()
            || endpoint.path() != "/cgi-bin/webhook/send"
            || parameters.len() != 1
            || parameters[0].0 != "key"
            || parameters[0].1.trim().is_empty()
        {
            return Err(invalid());
        }
        Self::build(endpoint, Duration::from_secs(30))
    }

    fn build(endpoint: reqwest::Url, timeout: Duration) -> anyhow::Result<Self> {
        let builder = super::super::http::client_builder()
            .retry(reqwest::retry::never())
            .redirect(reqwest::redirect::Policy::none());
        let client =
            crate::config::apply_runtime_proxy_to_builder(builder, "channel.wecom_webhook")
                .build()
                .map_err(|_| failure(0, 1, DeliveryFailureKind::NotSent, "客户端初始化失败"))?;
        Ok(Self { endpoint, client, timeout })
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(endpoint: reqwest::Url, timeout: Duration) -> Self {
        assert_eq!(endpoint.scheme(), "http");
        assert_eq!(endpoint.host_str(), Some("127.0.0.1"));
        let client = reqwest::Client::builder()
            .no_proxy()
            .retry(reqwest::retry::never())
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        Self { endpoint, client, timeout }
    }

    /// Send sequentially, stop at the first unconfirmed part, and never retry automatically.
    pub async fn send(&self, content: &str) -> anyhow::Result<()> {
        let preflight = || failure(0, 1, DeliveryFailureKind::NotSent, "正文预检失败");
        if content.trim().is_empty() {
            return Err(failure(0, 1, DeliveryFailureKind::NotSent, "正文为空"));
        }
        let normalized = format_wecom_markdown_tables(content);
        // Reserve enough ASCII bytes for two usize indices and the part marker.
        let reserve = 2 * usize::MAX.to_string().len() + 6;
        let chunks = split_markdown(&normalized, BODY_BYTES - reserve).map_err(|_| preflight())?;
        let total = chunks.len();
        let payloads = chunks
            .into_iter()
            .enumerate()
            .map(|(index, chunk)| {
                let content =
                    if total > 1 { format!("[{}/{}]\n{chunk}", index + 1, total) } else { chunk };
                if content.len() > BODY_BYTES {
                    return Err(preflight());
                }
                serde_json::to_vec(&json!({"msgtype":"markdown", "markdown":{"content":content}}))
                    .map_err(|_| preflight())
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        for (index, payload) in payloads.into_iter().enumerate() {
            let error = |kind, stage: &str| failure(index, total, kind, stage);
            let mut response = self
                .client
                .post(self.endpoint.clone())
                .timeout(self.timeout)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(payload)
                .send()
                .await
                .map_err(|err| {
                    error(
                        if err.is_connect() {
                            DeliveryFailureKind::NotSent
                        } else {
                            DeliveryFailureKind::Unknown
                        },
                        "请求失败",
                    )
                })?;
            let status = response.status();
            if !status.is_success() {
                // Non-success HTTP cannot establish the platform's delivery outcome.
                return Err(error(
                    DeliveryFailureKind::Unknown,
                    &format!("HTTP {}", status.as_u16()),
                ));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| error(DeliveryFailureKind::Unknown, "读取回执失败"))?
            {
                if bytes.len() + chunk.len() > RESPONSE_BYTES {
                    return Err(error(DeliveryFailureKind::Unknown, "回执超限"));
                }
                bytes.extend_from_slice(&chunk);
            }
            let ack: Value = serde_json::from_slice(&bytes)
                .map_err(|_| error(DeliveryFailureKind::Unknown, "回执 JSON 无效"))?;
            match ack.get("errcode").and_then(Value::as_i64) {
                Some(0) => {}
                Some(code) => {
                    return Err(error(DeliveryFailureKind::Rejected, &format!("errcode {code}")));
                }
                None => return Err(error(DeliveryFailureKind::Unknown, "回执缺少 errcode")),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "webhook_tests.rs"]
mod tests;
