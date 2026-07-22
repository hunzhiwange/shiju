use super::QQChannel;
use super::auth::ensure_https;
use super::content::{build_media_message_body, build_text_message_body, parse_outgoing_content};
use serde_json::{Value, json};

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

async fn post_json_with_client(
    client: &reqwest::Client,
    token: &str,
    url: &str,
    body: &Value,
    op: &str,
) -> anyhow::Result<()> {
    ensure_https(url)?;

    let resp = client
        .post(url)
        .header("Authorization", format!("QQBot {token}"))
        .json(body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err = resp.text().await.unwrap_or_default();
        let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
        anyhow::bail!("QQ {op} failed ({status}): {sanitized}");
    }

    Ok(())
}

async fn upload_media_file_info_with_client(
    client: &reqwest::Client,
    token: &str,
    files_url: &str,
    media_url: &str,
) -> anyhow::Result<String> {
    ensure_https(files_url)?;
    ensure_https(media_url)?;

    let upload_body = json!({
        "file_type": 1,
        "url": media_url,
        "srv_send_msg": false
    });

    let resp = client
        .post(files_url)
        .header("Authorization", format!("QQBot {token}"))
        .json(&upload_body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err = resp.text().await.unwrap_or_default();
        let sanitized = crate::app::agent::providers::sanitize_api_error(&err);
        anyhow::bail!("QQ upload media failed ({status}): {sanitized}");
    }

    let payload: Value = resp.json().await?;
    let file_info = payload
        .get("file_info")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("QQ upload media response missing file_info"))?;

    Ok(file_info.to_string())
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
    let mut msg_seq: u64 = 1;

    let (text_content, image_urls) = parse_outgoing_content(&message.content);

    if let Some(body) = build_text_message_body(&text_content, passive_msg_id, msg_seq) {
        post_json_with_client(client, token, message_url, &body, "send message").await?;
        if passive_msg_id.is_some() {
            msg_seq += 1;
        }
    }

    for image_url in image_urls {
        let file_info =
            upload_media_file_info_with_client(client, token, files_url, &image_url).await?;
        let media_body = build_media_message_body(&file_info, passive_msg_id, msg_seq);
        post_json_with_client(client, token, message_url, &media_body, "send message").await?;
        if passive_msg_id.is_some() {
            msg_seq += 1;
        }
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
        op: &str,
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
        let token = self.get_token().await?;
        let (message_url, files_url) = resolve_send_endpoints(&message.recipient);
        let client = self.http_client();
        send_message_with_client(&client, &token, &message_url, &files_url, message).await
    }
}

#[cfg(test)]
#[path = "outbound_tests.rs"]
mod outbound_tests;
