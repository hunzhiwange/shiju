use super::super::outbound_delivery::{ChannelDeliveryError, DeliveryFailureKind};
use super::QQChannel;
use super::auth::ensure_https;
use super::content::{
    build_markdown_message_body, build_media_message_body, build_text_message_body,
    parse_outgoing_content, sanitize_qq_plain_text,
};
use reqwest::StatusCode;
use serde_json::{Value, json};
use std::fmt;
use std::time::Duration;

const TEXT_BUDGET: usize = 3000;
const JSON_BUDGET: usize = 16 * 1024;

#[derive(Debug)]
struct QqPostError {
    kind: DeliveryFailureKind,
    code: Option<i64>,
    op: &'static str,
}

impl fmt::Display for QqPostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "QQ {} failed: {:?} (code {:?})", self.op, self.kind, self.code)
    }
}

impl std::error::Error for QqPostError {}

fn post_failure(op: &'static str, kind: DeliveryFailureKind, code: Option<i64>) -> anyhow::Error {
    QqPostError { kind, code, op }.into()
}

fn failure_kind(error: &anyhow::Error) -> DeliveryFailureKind {
    error
        .downcast_ref::<QqPostError>()
        .map(|error| error.kind)
        .unwrap_or(DeliveryFailureKind::NotSent)
}

fn should_fallback_from_markdown(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<QqPostError>(),
        Some(QqPostError { kind: DeliveryFailureKind::Rejected, code: Some(50056 | 40034012), .. })
    )
}

fn is_url_content_error(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<QqPostError>(),
        Some(QqPostError {
            kind: DeliveryFailureKind::Rejected,
            code: Some(304003 | 40034028),
            ..
        })
    )
}

/// 解析发送端点 URL。
fn resolve_send_endpoints(recipient: &str) -> (String, String) {
    if let Some(group_id) = recipient.strip_prefix("group:") {
        (
            format!("{}/v2/groups/{group_id}/messages", super::QQ_API_BASE),
            format!("{}/v2/groups/{group_id}/files", super::QQ_API_BASE),
        )
    } else {
        let raw_uid = recipient.strip_prefix("user:").unwrap_or(recipient);
        let user_id: String = raw_uid
            .chars()
            .filter(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        (
            format!("{}/v2/users/{user_id}/messages", super::QQ_API_BASE),
            format!("{}/v2/users/{user_id}/files", super::QQ_API_BASE),
        )
    }
}

/// Only numeric error codes and confirmation fields are retained; upstream text may contain secrets.
async fn response_json(
    client: &reqwest::Client,
    token: &str,
    url: &str,
    body: &Value,
    op: &'static str,
) -> anyhow::Result<Value> {
    ensure_https(url)?;
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        let response = client
            .post(url)
            .header("Authorization", format!("QQBot {token}"))
            .json(body)
            .send()
            .await
            .map_err(|_| post_failure(op, DeliveryFailureKind::Unknown, None))?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|_| post_failure(op, DeliveryFailureKind::Unknown, None))?;
        let payload = serde_json::from_slice::<Value>(&bytes).ok();
        let code = payload.as_ref().and_then(|value| value.get("code")).and_then(Value::as_i64);
        if status.is_server_error() || status == StatusCode::REQUEST_TIMEOUT {
            return Err(post_failure(op, DeliveryFailureKind::Unknown, code));
        }
        if status.is_client_error() || code.is_some_and(|code| code != 0) {
            return Err(post_failure(op, DeliveryFailureKind::Rejected, code));
        }
        if !status.is_success() {
            return Err(post_failure(op, DeliveryFailureKind::Unknown, code));
        }
        let payload =
            payload.ok_or_else(|| post_failure(op, DeliveryFailureKind::Unknown, None))?;
        if payload.get("code").is_some() && code.is_none() {
            return Err(post_failure(op, DeliveryFailureKind::Unknown, None));
        }
        Ok(payload)
    })
    .await;
    result.unwrap_or_else(|_| Err(post_failure(op, DeliveryFailureKind::Unknown, None)))
}

async fn post_json_with_client(
    client: &reqwest::Client,
    token: &str,
    url: &str,
    body: &Value,
    op: &'static str,
) -> anyhow::Result<()> {
    // QQ's official send API returns an id and timestamp, not a bare HTTP success.
    // https://github.com/tencent-connect/bot-docs/blob/main/docs/develop/api-v2/server-inter/message/send-receive/send.md
    let payload = response_json(client, token, url, body, op).await?;
    if !payload.get("id").and_then(Value::as_str).is_some_and(|id| !id.trim().is_empty()) {
        return Err(post_failure(op, DeliveryFailureKind::Unknown, None));
    }
    Ok(())
}

async fn upload_media_file_info_with_client(
    client: &reqwest::Client,
    token: &str,
    files_url: &str,
    media_url: &str,
) -> anyhow::Result<String> {
    ensure_https(media_url)?;
    let upload_body = json!({"file_type": 1, "url": media_url, "srv_send_msg": false});
    let payload = response_json(client, token, files_url, &upload_body, "upload media").await?;
    let file_info = payload
        .get("file_info")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("QQ upload media response missing file_info"))?;
    Ok(file_info.to_string())
}

fn check_payload(body: &Value) -> anyhow::Result<()> {
    let text = body
        .get("markdown")
        .and_then(|markdown| markdown.get("content"))
        .or_else(|| body.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("");
    anyhow::ensure!(text.len() <= TEXT_BUDGET, "QQ body exceeds local byte budget");
    anyhow::ensure!(
        serde_json::to_vec(body)?.len() <= JSON_BUDGET,
        "QQ JSON exceeds local byte budget"
    );
    Ok(())
}

fn delivery_error(confirmed: usize, total: usize, kind: DeliveryFailureKind) -> anyhow::Error {
    ChannelDeliveryError {
        confirmed_parts: confirmed,
        total_parts: total,
        failed_part_index: confirmed,
        kind,
    }
    .into()
}

async fn send_message_with_client(
    client: &reqwest::Client,
    token: &str,
    message_url: &str,
    files_url: &str,
    message: &crate::app::agent::channels::traits::SendMessage,
) -> anyhow::Result<()> {
    let passive_msg_id =
        message.thread_ts.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let (text_content, image_urls) = parse_outgoing_content(&message.content);
    let text_chunks = super::super::markdown_chunk::split_markdown(&text_content, TEXT_BUDGET)
        .map_err(|_| {
            delivery_error(
                0,
                usize::from(!text_content.is_empty()) + image_urls.len(),
                DeliveryFailureKind::NotSent,
            )
        })?;
    let total = text_chunks.len() + image_urls.len();
    let not_sent = || delivery_error(0, total, DeliveryFailureKind::NotSent);
    let prepared = (|| -> anyhow::Result<Vec<[Value; 3]>> {
        if total > 0 {
            ensure_https(message_url)?;
        }
        // The official QQ v2 send guide limits passive replies to five per incoming message.
        // This checks only this call, without claiming a cross-call quota or deduplication guarantee.
        anyhow::ensure!(
            passive_msg_id.is_none() || total <= 5,
            "QQ passive reply exceeds five parts"
        );
        let mut prepared = Vec::new();
        for (index, text) in text_chunks.iter().enumerate() {
            let seq = index as u64 + 1;
            let bodies = [
                build_markdown_message_body(text, passive_msg_id, seq),
                build_text_message_body(&sanitize_qq_plain_text(text, false), passive_msg_id, seq),
                build_text_message_body(&sanitize_qq_plain_text(text, true), passive_msg_id, seq),
            ];
            let [Some(markdown), Some(plain), Some(clean)] = bodies else {
                anyhow::bail!("QQ cannot construct non-empty fallback");
            };
            for body in [&markdown, &plain, &clean] {
                check_payload(body)?;
            }
            prepared.push([markdown, plain, clean]);
        }
        if !image_urls.is_empty() {
            ensure_https(files_url)?;
        }
        for (index, url) in image_urls.iter().enumerate() {
            ensure_https(url)?;
            check_payload(&json!({"file_type": 1, "url": url, "srv_send_msg": false}))?;
            check_payload(&build_media_message_body(
                "",
                passive_msg_id,
                (text_chunks.len() + index + 1) as u64,
            ))?;
        }
        Ok(prepared)
    })()
    .map_err(|_| not_sent())?;

    for (confirmed, [markdown, plain, clean]) in prepared.into_iter().enumerate() {
        let result = async {
            match post_json_with_client(client, token, message_url, &markdown, "send message").await
            {
                Ok(()) => return Ok(()),
                Err(error) if should_fallback_from_markdown(&error) => {}
                Err(error) => return Err(error),
            }
            match post_json_with_client(client, token, message_url, &plain, "send message").await {
                Ok(()) => Ok(()),
                Err(error) if is_url_content_error(&error) => {
                    post_json_with_client(client, token, message_url, &clean, "send message").await
                }
                Err(error) => Err(error),
            }
        }
        .await;
        result.map_err(|error| delivery_error(confirmed, total, failure_kind(&error)))?;
    }
    for (index, image_url) in image_urls.iter().enumerate() {
        let confirmed = text_chunks.len() + index;
        // Uploads explicitly use srv_send_msg=false: an upload failure has not submitted this part.
        let file_info = upload_media_file_info_with_client(client, token, files_url, image_url)
            .await
            .map_err(|_| delivery_error(confirmed, total, DeliveryFailureKind::NotSent))?;
        let media_body = build_media_message_body(&file_info, passive_msg_id, confirmed as u64 + 1);
        // file_info comes from the upload response, so its size can only be checked here.
        check_payload(&media_body)
            .map_err(|_| delivery_error(confirmed, total, DeliveryFailureKind::NotSent))?;
        post_json_with_client(client, token, message_url, &media_body, "send message")
            .await
            .map_err(|error| delivery_error(confirmed, total, failure_kind(&error)))?;
    }
    Ok(())
}

impl QQChannel {
    /// 发送 JSON POST 请求。
    async fn post_json(
        &self,
        token: &str,
        url: &str,
        body: &Value,
        op: &'static str,
    ) -> anyhow::Result<()> {
        let client = self.http_client();
        post_json_with_client(&client, token, url, body, op).await
    }

    /// 上传媒体文件并获取 file_info。
    async fn upload_media_file_info(
        &self,
        token: &str,
        files_url: &str,
        media_url: &str,
    ) -> anyhow::Result<String> {
        let client = self.http_client();
        upload_media_file_info_with_client(&client, token, files_url, media_url).await
    }

    /// 发送消息到指定接收者。
    pub(super) async fn send_message(
        &self,
        message: &crate::app::agent::channels::traits::SendMessage,
    ) -> anyhow::Result<()> {
        #[cfg(test)]
        if let Some(recorder) = &self.outbound_recorder {
            recorder.lock().await.push(message.clone());
            return Ok(());
        }
        let token = self.get_token().await?;
        let (message_url, files_url) = resolve_send_endpoints(&message.recipient);
        let client = self.http_client();
        send_message_with_client(&client, &token, &message_url, &files_url, message).await
    }
}

#[cfg(test)]
#[path = "outbound_tests.rs"]
mod outbound_tests;
