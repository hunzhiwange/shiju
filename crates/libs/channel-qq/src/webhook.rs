use super::QQChannel;
use super::content::{build_channel_message, compose_message_content};
use ring::signature::Ed25519KeyPair;
use serde_json::{Value, json};

/// 从密钥生成 QQ Ed25519 签名种子。
fn qq_seed_from_secret(secret: &str) -> Option<[u8; 32]> {
    let bytes = secret.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut seed = [0_u8; 32];
    for (idx, slot) in seed.iter_mut().enumerate() {
        *slot = bytes[idx % bytes.len()];
    }
    Some(seed)
}

/// 生成 QQ Webhook 验证签名。
fn qq_webhook_validation_signature(
    app_secret: &str,
    event_ts: &str,
    plain_token: &str,
) -> Option<String> {
    let seed = qq_seed_from_secret(app_secret)?;
    let key_pair = Ed25519KeyPair::from_seed_unchecked(&seed).ok()?;
    let mut payload = String::with_capacity(event_ts.len() + plain_token.len());
    payload.push_str(event_ts);
    payload.push_str(plain_token);
    Some(hex::encode(key_pair.sign(payload.as_bytes()).as_ref()))
}

impl QQChannel {
    /// 解析分发消息事件。
    pub(super) async fn parse_dispatch_message_event(
        &self,
        event_type: &str,
        payload: &Value,
    ) -> Option<channel_types::ChannelMessage> {
        match event_type {
            "C2C_MESSAGE_CREATE" => {
                let msg_id = payload.get("id").and_then(Value::as_str).unwrap_or("");
                if self.is_duplicate(msg_id).await {
                    return None;
                }

                let content = compose_message_content(payload)?;
                let author_id = payload
                    .get("author")
                    .and_then(|author| author.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let user_openid = payload
                    .get("author")
                    .and_then(|author| author.get("user_openid"))
                    .and_then(Value::as_str)
                    .unwrap_or(author_id);

                if !self.is_user_allowed(user_openid) {
                    tracing::warn!(
                        "QQ: ignoring C2C message from unauthorized user: {user_openid}"
                    );
                    return None;
                }

                let chat_id = format!("user:{user_openid}");
                Some(build_channel_message(user_openid, chat_id, content, msg_id))
            }
            "GROUP_AT_MESSAGE_CREATE" => {
                let msg_id = payload.get("id").and_then(Value::as_str).unwrap_or("");
                if self.is_duplicate(msg_id).await {
                    return None;
                }

                let content = compose_message_content(payload)?;
                let author_id = payload
                    .get("author")
                    .and_then(|author| author.get("member_openid"))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                if !self.is_user_allowed(author_id) {
                    tracing::warn!(
                        "QQ: ignoring group message from unauthorized user: {author_id}"
                    );
                    return None;
                }

                let group_openid =
                    payload.get("group_openid").and_then(Value::as_str).unwrap_or("unknown");
                let chat_id = format!("group:{group_openid}");
                Some(build_channel_message(author_id, chat_id, content, msg_id))
            }
            _ => None,
        }
    }

    /// 构建 Webhook 验证响应。
    pub fn build_webhook_validation_response(&self, payload: &Value) -> Option<Value> {
        let op = payload.get("op").and_then(Value::as_u64).unwrap_or_default();
        if op != 13 {
            return None;
        }

        let validation = payload.get("d")?;
        let plain_token = validation
            .get("plain_token")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())?;
        let event_ts = validation
            .get("event_ts")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())?;

        let signature = qq_webhook_validation_signature(&self.app_secret, event_ts, plain_token)?;
        Some(json!({
            "plain_token": plain_token,
            "signature": signature
        }))
    }

    /// 解析 Webhook 载荷。
    pub async fn parse_webhook_payload(
        &self,
        payload: &Value,
    ) -> Vec<channel_types::ChannelMessage> {
        let op = payload.get("op").and_then(Value::as_u64).unwrap_or_default();
        if op != 0 {
            return Vec::new();
        }

        let event_type = payload.get("t").and_then(Value::as_str).map(str::trim).unwrap_or("");
        if event_type.is_empty() {
            return Vec::new();
        }

        let Some(dispatch_payload) = payload.get("d") else {
            return Vec::new();
        };

        self.parse_dispatch_message_event(event_type, dispatch_payload).await.into_iter().collect()
    }
}

#[cfg(test)]
#[path = "webhook_tests.rs"]
mod webhook_tests;
