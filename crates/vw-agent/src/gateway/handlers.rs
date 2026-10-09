//! 外部消息通道的 Webhook 处理器，统一校验、对话执行与回复投递。
//!
//! 各提供者先完成签名或令牌验证，再把消息规范化为通道模型；失败响应不进入代理执行。

use super::chat::{log_channel_message, run_gateway_chat_with_tools, sanitize_gateway_response};
use super::state::AppState;
use super::util::{
    linq_memory_key, nextcloud_talk_memory_key, qq_memory_key, wati_memory_key, whatsapp_memory_key,
};
use crate::app::agent::channels::SendMessage;
use crate::app::agent::channels::traits::{Channel, ChannelMessage};
use crate::app::agent::memory::MemoryCategory;
use crate::app::agent::security::pairing::constant_time_eq;
use crate::app::agent::tools::Tool;
use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use std::future::Future;
use std::pin::Pin;

type GatewayChatFuture<'a> = Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'a>>;
type GatewayChatRunner = for<'a> fn(&'a AppState, &'a str, &'a str) -> GatewayChatFuture<'a>;

fn run_gateway_chat_with_tools_boxed<'a>(
    state: &'a AppState,
    message: &'a str,
    key: &'a str,
) -> GatewayChatFuture<'a> {
    Box::pin(run_gateway_chat_with_tools(state, message, key))
}

async fn process_channel_messages<C, K>(
    state: &AppState,
    channel_name: &str,
    channel: &C,
    messages: &[ChannelMessage],
    memory_key: K,
    llm_error_label: &str,
    include_thread: bool,
) where
    C: Channel + ?Sized,
    K: Fn(&ChannelMessage) -> String,
{
    process_channel_messages_with_chat(
        state,
        channel_name,
        channel,
        messages,
        memory_key,
        llm_error_label,
        include_thread,
        run_gateway_chat_with_tools_boxed,
    )
    .await;
}

async fn process_channel_messages_with_chat<C, K>(
    state: &AppState,
    channel_name: &str,
    channel: &C,
    messages: &[ChannelMessage],
    memory_key: K,
    llm_error_label: &str,
    include_thread: bool,
    run_chat: GatewayChatRunner,
) where
    C: Channel + ?Sized,
    K: Fn(&ChannelMessage) -> String,
{
    for msg in messages {
        log_channel_message(channel_name, &msg.sender, &msg.content);

        let key = memory_key(msg);
        if state.auto_save {
            let _ = state.mem.store(&key, &msg.content, MemoryCategory::Conversation, None).await;
        }

        match run_chat(state, &msg.content, &key).await {
            Ok(response) => {
                send_successful_channel_reply(
                    channel_name,
                    channel,
                    msg,
                    &response,
                    state.tools_registry_exec.as_ref(),
                    include_thread,
                )
                .await;
            }
            Err(e) => {
                tracing::error!("LLM error for {llm_error_label}: {e:#}");
                let mut reply = SendMessage::new(
                    "Sorry, I couldn't process your message right now.",
                    &msg.reply_target,
                );
                if include_thread {
                    reply = reply.in_thread(msg.thread_ts.clone());
                }
                let _ = channel.send(&reply).await;
            }
        }
    }
}

async fn send_successful_channel_reply<C: Channel + ?Sized>(
    channel_name: &str,
    channel: &C,
    msg: &ChannelMessage,
    response: &str,
    tools: &[Box<dyn Tool>],
    include_thread: bool,
) {
    let safe_response = sanitize_gateway_response(response, tools);
    let mut reply = SendMessage::new(safe_response, &msg.reply_target);
    if include_thread {
        reply = reply.in_thread(msg.thread_ts.clone());
    }
    if let Err(e) = channel.send(&reply).await {
        tracing::error!("Failed to send {channel_name} reply: {e}");
    }
}

/// Meta WhatsApp Webhook 握手查询参数。
#[derive(serde::Deserialize)]
pub struct WhatsAppVerifyQuery {
    /// Meta 请求的订阅模式。
    #[serde(rename = "hub.mode")]
    pub mode: Option<String>,
    /// Meta 回传的配置校验令牌。
    #[serde(rename = "hub.verify_token")]
    pub verify_token: Option<String>,
    /// 验证成功时需要原样返回的挑战值。
    #[serde(rename = "hub.challenge")]
    pub challenge: Option<String>,
}

/// 校验 WhatsApp Webhook 握手参数并返回挑战值。
pub async fn handle_whatsapp_verify(
    State(state): State<AppState>,
    Query(params): Query<WhatsAppVerifyQuery>,
) -> Response {
    let Some(ref wa) = state.whatsapp else {
        return (StatusCode::NOT_FOUND, "WhatsApp not configured".to_string()).into_response();
    };

    // Verify the token matches (constant-time comparison to prevent timing attacks)
    let token_matches =
        params.verify_token.as_deref().is_some_and(|t| constant_time_eq(t, wa.verify_token()));
    if params.mode.as_deref() == Some("subscribe") && token_matches {
        if let Some(ch) = params.challenge {
            tracing::info!("WhatsApp webhook verified successfully");
            return (StatusCode::OK, ch).into_response();
        }
        return (StatusCode::BAD_REQUEST, "Missing hub.challenge".to_string()).into_response();
    }

    tracing::warn!("WhatsApp webhook verification failed — token mismatch");
    (StatusCode::FORBIDDEN, "Forbidden".to_string()).into_response()
}

/// 使用应用密钥校验 `X-Hub-Signature-256` HMAC 签名。
/// See: <https://developers.facebook.com/docs/graph-api/webhooks/getting-started#verification-requests>
pub fn verify_whatsapp_signature(app_secret: &str, body: &[u8], signature_header: &str) -> bool {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    // Signature format: "sha256=<hex_signature>"
    let Some(hex_sig) = signature_header.strip_prefix("sha256=") else {
        return false;
    };

    // Decode hex signature
    let Ok(expected) = hex::decode(hex_sig) else {
        return false;
    };

    // Compute HMAC-SHA256
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(app_secret.as_bytes()) else {
        return false;
    };
    mac.update(body);

    // Constant-time comparison
    mac.verify_slice(&expected).is_ok()
}

/// 验证并处理 WhatsApp Cloud API 入站消息。
pub async fn handle_whatsapp_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(ref wa) = state.whatsapp else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "WhatsApp not configured"})),
        )
            .into_response();
    };

    // ── Security: Verify X-Hub-Signature-256 if app_secret is configured ──
    if let Some(ref app_secret) = state.whatsapp_app_secret {
        let signature =
            headers.get("X-Hub-Signature-256").and_then(|v| v.to_str().ok()).unwrap_or("");

        if !verify_whatsapp_signature(app_secret, &body, signature) {
            tracing::warn!(
                "WhatsApp webhook signature verification failed (signature: {})",
                if signature.is_empty() { "missing" } else { "invalid" }
            );
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid signature"})),
            )
                .into_response();
        }
    }

    // Parse JSON body
    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid JSON payload"})),
        )
            .into_response();
    };

    // Parse messages from the webhook payload
    let messages = wa.parse_webhook_payload(&payload);

    if messages.is_empty() {
        // Acknowledge the webhook even if no messages (could be status updates)
        return (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response();
    }

    process_channel_messages(
        &state,
        "WhatsApp",
        wa.as_ref(),
        &messages,
        whatsapp_memory_key,
        "WhatsApp message",
        false,
    )
    .await;

    // Acknowledge the webhook
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
}

/// 处理承载 iMessage、RCS 或 SMS 的 Linq Webhook 消息。
pub async fn handle_linq_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(ref linq) = state.linq else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Linq not configured"})))
            .into_response();
    };

    let body_str = String::from_utf8_lossy(&body);

    // ── Security: Verify X-Webhook-Signature if signing_secret is configured ──
    if let Some(ref signing_secret) = state.linq_signing_secret {
        let timestamp =
            headers.get("X-Webhook-Timestamp").and_then(|v| v.to_str().ok()).unwrap_or("");

        let signature =
            headers.get("X-Webhook-Signature").and_then(|v| v.to_str().ok()).unwrap_or("");

        if !crate::app::agent::channels::linq::verify_linq_signature(
            signing_secret,
            &body_str,
            timestamp,
            signature,
        ) {
            tracing::warn!(
                "Linq webhook signature verification failed (signature: {})",
                if signature.is_empty() { "missing" } else { "invalid" }
            );
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid signature"})),
            )
                .into_response();
        }
    }

    // Parse JSON body
    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid JSON payload"})),
        )
            .into_response();
    };

    // Parse messages from the webhook payload
    let messages = linq.parse_webhook_payload(&payload);

    if messages.is_empty() {
        if payload
            .get("event_type")
            .and_then(|v| v.as_str())
            .is_some_and(|event| event == "message.received")
        {
            tracing::warn!(
                "Linq webhook message.received produced no actionable messages (possible unsupported payload shape)"
            );
        }
        // Acknowledge the webhook even if no messages (could be status/delivery events)
        return (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response();
    }

    process_channel_messages(
        &state,
        "Linq",
        linq.as_ref(),
        &messages,
        linq_memory_key,
        "Linq message",
        false,
    )
    .await;

    // Acknowledge the webhook
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
}

/// 校验 WATI Webhook 握手并返回挑战值。
pub async fn handle_wati_verify(
    State(state): State<AppState>,
    Query(params): Query<WatiVerifyQuery>,
) -> Response {
    if state.wati.is_none() {
        return (StatusCode::NOT_FOUND, "WATI not configured".to_string()).into_response();
    }

    // WATI may use Meta-style webhook verification; echo the challenge
    if let Some(challenge) = params.challenge {
        tracing::info!("WATI webhook verified successfully");
        return (StatusCode::OK, challenge).into_response();
    }

    (StatusCode::BAD_REQUEST, "Missing hub.challenge".to_string()).into_response()
}

#[derive(Debug, serde::Deserialize)]
/// WATI Webhook 握手查询参数。
pub struct WatiVerifyQuery {
    /// 验证成功时返回的挑战值。
    #[serde(rename = "hub.challenge")]
    pub challenge: Option<String>,
}

/// 解析并处理 WATI 入站消息。
pub async fn handle_wati_webhook(State(state): State<AppState>, body: Bytes) -> Response {
    let Some(ref wati) = state.wati else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "WATI not configured"})))
            .into_response();
    };

    // Parse JSON body
    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid JSON payload"})),
        )
            .into_response();
    };

    // Parse messages from the webhook payload
    let messages = wati.parse_webhook_payload(&payload);

    if messages.is_empty() {
        return (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response();
    }

    process_channel_messages(
        &state,
        "WATI",
        wati.as_ref(),
        &messages,
        wati_memory_key,
        "WATI message",
        false,
    )
    .await;

    // Acknowledge the webhook
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
}

/// 处理 Nextcloud Talk 机器人 Webhook 消息。
pub async fn handle_nextcloud_talk_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(ref nextcloud_talk) = state.nextcloud_talk else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Nextcloud Talk not configured"})),
        )
            .into_response();
    };

    let body_str = String::from_utf8_lossy(&body);

    // ── Security: Verify Nextcloud Talk HMAC signature if secret is configured ──
    if let Some(ref webhook_secret) = state.nextcloud_talk_webhook_secret {
        let random =
            headers.get("X-Nextcloud-Talk-Random").and_then(|v| v.to_str().ok()).unwrap_or("");

        let signature =
            headers.get("X-Nextcloud-Talk-Signature").and_then(|v| v.to_str().ok()).unwrap_or("");

        if !crate::app::agent::channels::nextcloud_talk::verify_nextcloud_talk_signature(
            webhook_secret,
            random,
            &body_str,
            signature,
        ) {
            tracing::warn!(
                "Nextcloud Talk webhook signature verification failed (signature: {})",
                if signature.is_empty() { "missing" } else { "invalid" }
            );
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid signature"})),
            )
                .into_response();
        }
    }

    // Parse JSON body
    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid JSON payload"})),
        )
            .into_response();
    };

    // Parse messages from webhook payload
    let messages = nextcloud_talk.parse_webhook_payload(&payload);
    if messages.is_empty() {
        // Acknowledge webhook even if payload does not contain actionable user messages.
        return (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response();
    }

    process_channel_messages(
        &state,
        "Nextcloud Talk",
        nextcloud_talk.as_ref(),
        &messages,
        nextcloud_talk_memory_key,
        "Nextcloud Talk message",
        false,
    )
    .await;

    (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
}

/// 验证并处理 QQ 机器人 Webhook 消息。
pub async fn handle_qq_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(ref qq) = state.qq else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "QQ not configured"})))
            .into_response();
    };

    if !state.qq_webhook_enabled {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "QQ webhook mode not enabled"})),
        )
            .into_response();
    }

    let app_id_header =
        headers.get("X-Bot-Appid").and_then(|v| v.to_str().ok()).map(str::trim).unwrap_or("");
    if !app_id_header.is_empty() && !constant_time_eq(app_id_header, qq.app_id()) {
        tracing::warn!("QQ webhook rejected due to mismatched X-Bot-Appid");
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid X-Bot-Appid"})),
        )
            .into_response();
    }

    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid JSON payload"})),
        )
            .into_response();
    };

    if let Some(validation_response) = qq.build_webhook_validation_response(&payload) {
        tracing::info!("QQ webhook validation challenge accepted");
        return (StatusCode::OK, Json(validation_response)).into_response();
    }

    let messages = qq.parse_webhook_payload(&payload).await;
    if messages.is_empty() {
        return (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response();
    }

    process_channel_messages(
        &state,
        "QQ",
        qq.as_ref(),
        &messages,
        qq_memory_key,
        "QQ webhook message",
        true,
    )
    .await;

    (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
}

#[cfg(test)]
#[path = "handlers_tests.rs"]
mod handlers_tests;
