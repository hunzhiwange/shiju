//! Webhook authentication, payload mapping and channel dispatch.
use gateway_types::AppState;
use super::channel_runtime::GatewayChannels;
use channel_types::{Channel, ChannelMessage, SendMessage};
use gateway_pairing::pairing::constant_time_eq;
use axum::{body::Bytes, extract::{State, Query}, http::{HeaderMap, StatusCode}, response::{IntoResponse, Json, Response}};
use std::sync::Arc;
use channel_support::runtime_state::ChannelRuntimeContext;
pub type ChannelDispatch = fn(Arc<ChannelRuntimeContext>, ChannelMessage, tokio_util::sync::CancellationToken) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;
/// The WhatsApp implementation is supplied by the host, including local HTTP fixtures.
pub trait WhatsAppWebhook: Channel {
    fn verify_token(&self) -> &str;
    fn parse_webhook_payload(&self, payload: &serde_json::Value) -> Vec<ChannelMessage>;
}
impl WhatsAppWebhook for channel_whatsapp::WhatsAppChannel {
    fn verify_token(&self) -> &str { self.verify_token() }
    fn parse_webhook_payload(&self, payload: &serde_json::Value) -> Vec<ChannelMessage> { self.parse_webhook_payload(payload) }
}
async fn process_channel_messages<E, H, W, C>(
    state: &AppState<E, GatewayChannels<W>, H>,
    channel: &std::sync::Arc<C>,
    messages: &[ChannelMessage],
    runtime: fn(&AppState<E, GatewayChannels<W>, H>) -> anyhow::Result<Arc<ChannelRuntimeContext>>,
    dispatch: ChannelDispatch,
) where
    C: Channel + 'static,
{
    for msg in messages {
        let runtime = match runtime(state) {
            Ok(runtime) => runtime,
            Err(error) => {
                tracing::warn!("Unable to initialize webhook channel runtime: {error}");
                let _ = channel
                    .send(
                        &SendMessage::new("通道运行时不可用，请稍后重试。", &msg.reply_target)
                            .in_thread(msg.thread_ts.clone()),
                    )
                    .await;
                continue;
            }
        };
        let mut ctx = runtime.as_ref().clone();
        ctx.channels_by_name = std::sync::Arc::new(std::collections::HashMap::from([(
            channel_support::scope::channel_instance_key(&msg.channel, &msg.instance_id),
            channel.clone() as std::sync::Arc<dyn Channel>,
        )]));
        // Webhook 自身已持有较大的请求状态，不把完整会话 Future 内联到 HTTP Future。
        Box::pin(dispatch(
            std::sync::Arc::new(ctx),
            msg.clone(),
            tokio_util::sync::CancellationToken::new(),
        ))
        .await;
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
pub async fn handle_whatsapp_verify<E, H, W: WhatsAppWebhook + 'static>(
    State(state): State<AppState<E, GatewayChannels<W>, H>>,
    Query(params): Query<WhatsAppVerifyQuery>,
) -> Response {
    let channels = state.channels.lock().clone();
    let Some(ref wa) = channels.whatsapp else {
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
pub async fn handle_whatsapp_message<E, H, W: WhatsAppWebhook + 'static>(
    State(state): State<AppState<E, GatewayChannels<W>, H>>,
    headers: HeaderMap,
    body: Bytes,
 runtime: fn(&AppState<E, GatewayChannels<W>, H>) -> anyhow::Result<Arc<ChannelRuntimeContext>>, dispatch: ChannelDispatch) -> Response {
    let channels = state.channels.lock().clone();
    let Some(ref wa) = channels.whatsapp else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "WhatsApp not configured"})),
        )
            .into_response();
    };

    // ── Security: Verify X-Hub-Signature-256 if app_secret is configured ──
    if let Some(ref app_secret) = channels.whatsapp_app_secret {
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

    process_channel_messages(&state, &wa, &messages, runtime, dispatch)
    .await;

    // Acknowledge the webhook
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
}

/// 处理承载 iMessage、RCS 或 SMS 的 Linq Webhook 消息。
pub async fn handle_linq_webhook<E, H, W: WhatsAppWebhook + 'static>(
    State(state): State<AppState<E, GatewayChannels<W>, H>>,
    headers: HeaderMap,
    body: Bytes,
 runtime: fn(&AppState<E, GatewayChannels<W>, H>) -> anyhow::Result<Arc<ChannelRuntimeContext>>, dispatch: ChannelDispatch) -> Response {
    let channels = state.channels.lock().clone();
    let Some(ref linq) = channels.linq else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Linq not configured"})))
            .into_response();
    };

    let body_str = String::from_utf8_lossy(&body);

    // ── Security: Verify X-Webhook-Signature if signing_secret is configured ──
    if let Some(ref signing_secret) = channels.linq_signing_secret {
        let timestamp =
            headers.get("X-Webhook-Timestamp").and_then(|v| v.to_str().ok()).unwrap_or("");

        let signature =
            headers.get("X-Webhook-Signature").and_then(|v| v.to_str().ok()).unwrap_or("");

        if !channel_linq::verify_linq_signature(
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

    process_channel_messages(&state, &linq, &messages, runtime, dispatch)
    .await;

    // Acknowledge the webhook
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
}

/// 校验 WATI Webhook 握手并返回挑战值。
pub async fn handle_wati_verify<E, H, W: WhatsAppWebhook + 'static>(
    State(state): State<AppState<E, GatewayChannels<W>, H>>,
    Query(params): Query<WatiVerifyQuery>,
) -> Response {
    let channels = state.channels.lock().clone();
    if channels.wati.is_none() {
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
pub async fn handle_wati_webhook<E, H, W: WhatsAppWebhook + 'static>(State(state): State<AppState<E, GatewayChannels<W>, H>>, body: Bytes, runtime: fn(&AppState<E, GatewayChannels<W>, H>) -> anyhow::Result<Arc<ChannelRuntimeContext>>, dispatch: ChannelDispatch) -> Response {
    let channels = state.channels.lock().clone();
    let Some(ref wati) = channels.wati else {
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

    process_channel_messages(&state, &wati, &messages, runtime, dispatch)
    .await;

    // Acknowledge the webhook
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
}

/// 处理 Nextcloud Talk 机器人 Webhook 消息。
pub async fn handle_nextcloud_talk_webhook<E, H, W: WhatsAppWebhook + 'static>(
    State(state): State<AppState<E, GatewayChannels<W>, H>>,
    headers: HeaderMap,
    body: Bytes,
 runtime: fn(&AppState<E, GatewayChannels<W>, H>) -> anyhow::Result<Arc<ChannelRuntimeContext>>, dispatch: ChannelDispatch) -> Response {
    let channels = state.channels.lock().clone();
    let Some(ref nextcloud_talk) = channels.nextcloud_talk else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Nextcloud Talk not configured"})),
        )
            .into_response();
    };

    let body_str = String::from_utf8_lossy(&body);

    // ── Security: Verify Nextcloud Talk HMAC signature if secret is configured ──
    if let Some(ref webhook_secret) = channels.nextcloud_talk_webhook_secret {
        let random =
            headers.get("X-Nextcloud-Talk-Random").and_then(|v| v.to_str().ok()).unwrap_or("");

        let signature =
            headers.get("X-Nextcloud-Talk-Signature").and_then(|v| v.to_str().ok()).unwrap_or("");

        if !channel_nextcloud_talk::verify_nextcloud_talk_signature(
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

    process_channel_messages(&state, &nextcloud_talk, &messages, runtime, dispatch)
    .await;

    (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
}

/// 验证并处理 QQ 机器人 Webhook 消息。
pub async fn handle_qq_webhook<E, H, W: WhatsAppWebhook + 'static>(
    State(state): State<AppState<E, GatewayChannels<W>, H>>,
    headers: HeaderMap,
    body: Bytes,
 runtime: fn(&AppState<E, GatewayChannels<W>, H>) -> anyhow::Result<Arc<ChannelRuntimeContext>>, dispatch: ChannelDispatch) -> Response {
    let channels = state.channels.lock().clone();
    let Some(ref qq) = channels.qq else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "QQ not configured"})))
            .into_response();
    };

    if !channels.qq_webhook_enabled {
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

    process_channel_messages(&state, &qq, &messages, runtime, dispatch)
    .await;

    (StatusCode::OK, Json(serde_json::json!({"status": "ok"}))).into_response()
}
