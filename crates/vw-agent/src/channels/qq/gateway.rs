use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

use super::QQChannel;

#[derive(Debug, PartialEq)]
enum GatewayEvent {
    Ignore,
    SendHeartbeat,
    Break,
    Dispatch { event_type: String, payload: Value },
}

fn heartbeat_payload(sequence: i64) -> Value {
    let data = if sequence >= 0 { json!(sequence) } else { json!(null) };
    json!({ "op": 1, "d": data })
}

fn heartbeat_interval_from_hello(hello: &Message) -> anyhow::Result<u64> {
    let hello_data: Value = serde_json::from_str(&hello.to_string())?;
    Ok(hello_data
        .get("d")
        .and_then(|data| data.get("heartbeat_interval"))
        .and_then(Value::as_u64)
        .unwrap_or(41250))
}

fn identify_payload(token: &str) -> Value {
    let intents: u64 = (1 << 25) | (1 << 30);
    json!({
        "op": 2,
        "d": {
            "token": format!("QQBot {token}"),
            "intents": intents,
            "properties": {
                "os": "linux",
                "browser": "vibewindow",
                "device": "vibewindow",
            }
        }
    })
}

fn parse_gateway_text(text: &str, sequence: &mut i64) -> GatewayEvent {
    let event: Value = match serde_json::from_str(text) {
        Ok(event) => event,
        Err(_) => return GatewayEvent::Ignore,
    };

    if let Some(next_sequence) = event.get("s").and_then(Value::as_i64) {
        *sequence = next_sequence;
    }

    let op = event.get("op").and_then(Value::as_u64).unwrap_or(0);
    match op {
        1 => GatewayEvent::SendHeartbeat,
        7 | 9 => GatewayEvent::Break,
        0 => {
            let event_type = event.get("t").and_then(Value::as_str).unwrap_or("").to_string();
            let Some(payload) = event.get("d").cloned() else {
                return GatewayEvent::Ignore;
            };
            GatewayEvent::Dispatch { event_type, payload }
        }
        _ => GatewayEvent::Ignore,
    }
}

impl QQChannel {
    /// 启动 WebSocket 监听循环。
    #[allow(clippy::too_many_lines)]
    pub(super) async fn listen_gateway(
        &self,
        tx: tokio::sync::mpsc::Sender<crate::app::agent::channels::traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        tracing::info!("QQ: authenticating...");
        let token = self.get_token().await?;

        tracing::info!("QQ: fetching gateway URL...");
        let gw_url = self.get_gateway_url(&token).await?;

        tracing::info!("QQ: connecting to gateway WebSocket...");
        let (ws_stream, _) = tokio_tungstenite::connect_async(&gw_url).await?;
        let (mut write, mut read) = ws_stream.split();

        self.run_gateway_session(&token, tx, &mut write, &mut read).await
    }

    async fn run_gateway_session<W, R, RE>(
        &self,
        token: &str,
        tx: tokio::sync::mpsc::Sender<crate::app::agent::channels::traits::ChannelMessage>,
        write: &mut W,
        read: &mut R,
    ) -> anyhow::Result<()>
    where
        W: futures_util::Sink<Message> + Unpin,
        W::Error: std::error::Error + Send + Sync + 'static,
        R: futures_util::Stream<Item = Result<Message, RE>> + Unpin,
        RE: std::error::Error + Send + Sync + 'static,
    {
        let hello = read.next().await.ok_or(anyhow::anyhow!("QQ: no hello frame"))??;
        if matches!(hello, Message::Close(_)) {
            anyhow::bail!("QQ: no hello frame");
        }
        let heartbeat_interval = heartbeat_interval_from_hello(&hello)?;

        write.send(Message::Text(identify_payload(token).to_string().into())).await?;

        tracing::info!("QQ: connected and identified");

        let mut sequence: i64 = -1;
        let (hb_tx, mut hb_rx) = tokio::sync::mpsc::channel::<()>(1);
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_millis(heartbeat_interval));
            loop {
                interval.tick().await;
                if hb_tx.send(()).await.is_err() {
                    break;
                }
            }
        });

        loop {
            tokio::select! {
                _ = hb_rx.recv() => {
                    if write
                        .send(Message::Text(heartbeat_payload(sequence).to_string().into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                msg = read.next() => {
                    let text = match msg {
                        Some(Ok(Message::Text(text))) => text,
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => continue,
                    };

                    match parse_gateway_text(text.as_ref(), &mut sequence) {
                        GatewayEvent::Ignore => continue,
                        GatewayEvent::SendHeartbeat => {
                            if write
                                .send(Message::Text(heartbeat_payload(sequence).to_string().into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        GatewayEvent::Break => {
                            let op = serde_json::from_str::<Value>(text.as_ref())
                                .ok()
                                .and_then(|event| event.get("op").and_then(Value::as_u64))
                                .unwrap_or(0);
                            match op {
                                7 => tracing::warn!("QQ: received Reconnect (op 7)"),
                                9 => tracing::warn!("QQ: received Invalid Session (op 9)"),
                                _ => {}
                            }
                            break;
                        }
                        GatewayEvent::Dispatch { event_type, payload } => {
                            if let Some(channel_msg) =
                                self.parse_dispatch_message_event(&event_type, &payload).await
                            {
                                if tx.send(channel_msg).await.is_err() {
                                    tracing::warn!("QQ: message channel closed");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        anyhow::bail!("QQ WebSocket connection closed")
    }
}

#[cfg(test)]
#[path = "gateway_tests.rs"]
mod gateway_tests;
