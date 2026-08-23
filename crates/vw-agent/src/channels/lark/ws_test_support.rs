//! Lark WebSocket 测试使用的 HTTP、套接字服务与协议帧夹具。
//!
//! 夹具覆盖端点发现、二进制帧分片、事件载荷以及 ping/ack 协议断言。

use super::super::*;
use prost::Message;
use serde_json::json;
use std::future::Future;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_tungstenite::WebSocketStream;

/// 捕获一次端点发现请求的本地 HTTP 服务。
pub(super) struct TestHttpServer {
    /// 供待测通道请求的本地根地址。
    pub(super) base_url: String,
    requests: mpsc::Receiver<String>,
    handle: tokio::task::JoinHandle<()>,
}

impl TestHttpServer {
    /// 使用固定状态码和 JSON 响应启动单次请求服务。
    pub(super) async fn spawn(status: u16, body: serde_json::Value) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, requests) = mpsc::channel(1);
        let body = body.to_string();

        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_http_request(&mut stream).await;
            let _ = tx.send(request).await;
            let response = http_response(status, &body);
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        Self { base_url: format!("http://{addr}"), requests, handle }
    }

    /// 等待并返回服务捕获的请求。
    pub(super) async fn next_request(&mut self) -> String {
        timeout(Duration::from_secs(2), self.requests.recv())
            .await
            .expect("request should arrive")
            .expect("request channel should stay open")
    }
}

impl Drop for TestHttpServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn read_http_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0_u8; 1024];

    loop {
        let n = stream.read(&mut tmp).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);

        let Some(header_end) = buf.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&buf[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);

        if buf.len() >= header_end + 4 + content_length {
            break;
        }
    }

    String::from_utf8_lossy(&buf).into_owned()
}

fn http_response(status: u16, body: &str) -> String {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        500 => "Internal Server Error",
        _ => "Error",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// 返回原始 HTTP 请求的正文部分。
pub(super) fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or_default()
}

/// 启动只接受一个连接的本地 WebSocket 服务。
pub(super) async fn spawn_ws_server<F, Fut>(handler: F) -> (String, tokio::task::JoinHandle<()>)
where
    F: FnOnce(WebSocketStream<tokio::net::TcpStream>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let websocket = tokio_tungstenite::accept_async(stream).await.unwrap();
        handler(websocket).await;
    });

    (format!("ws://{addr}"), handle)
}

/// 创建带可选提及限制的 Lark WebSocket 测试通道。
pub(super) fn test_channel(mention_only: bool) -> LarkChannel {
    let ch = LarkChannel::new(
        "cli_ws_test".into(),
        "ws_secret".into(),
        "verify".into(),
        None,
        vec!["ou_allowed".into()],
        mention_only,
    );
    ch.set_resolved_bot_open_id(Some("ou_bot".to_string()));
    ch
}

fn pb_frame(
    seq_id: u64,
    method: i32,
    headers: &[(&str, &str)],
    payload: Option<Vec<u8>>,
) -> PbFrame {
    PbFrame {
        seq_id,
        log_id: 0,
        service: 42,
        method,
        headers: headers
            .iter()
            .map(|(key, value)| PbHeader { key: (*key).to_string(), value: (*value).to_string() })
            .collect(),
        payload,
    }
}

/// 构造指定服务、方法和序号的二进制协议帧。
pub(super) fn binary_frame(
    seq_id: u64,
    method: i32,
    headers: &[(&str, &str)],
    payload: Option<Vec<u8>>,
) -> WsMsg {
    WsMsg::Binary(pb_frame(seq_id, method, headers, payload).encode_to_vec().into())
}

/// 构造完整事件帧。
pub(super) fn event_frame(seq_id: u64, header_message_id: &str, payload: Vec<u8>) -> WsMsg {
    binary_frame(seq_id, 1, &[("type", "event"), ("message_id", header_message_id)], Some(payload))
}

/// 构造分片事件帧，用于验证重组和乱序处理。
pub(super) fn fragment_frame(
    seq_id: u64,
    header_message_id: &str,
    sum: &str,
    seq: &str,
    payload: Vec<u8>,
) -> WsMsg {
    binary_frame(
        seq_id,
        1,
        &[("type", "event"), ("message_id", header_message_id), ("sum", sum), ("seq", seq)],
        Some(payload),
    )
}

/// 生成 Lark 消息事件的 JSON 载荷。
pub(super) fn message_event(
    sender: (&str, &str),
    message: (&str, &str, &str, &str),
    mentions: serde_json::Value,
) -> Vec<u8> {
    let (sender_type, open_id) = sender;
    let (message_id, chat_type, message_type, content) = message;
    json!({
        "header": {
            "event_type": "im.message.receive_v1",
            "event_id": "evt_ws"
        },
        "event": {
            "sender": {
                "sender_id": {
                    "open_id": open_id
                },
                "sender_type": sender_type
            },
            "message": {
                "message_id": message_id,
                "chat_id": "oc_chat",
                "chat_type": chat_type,
                "message_type": message_type,
                "content": content,
                "mentions": mentions
            }
        }
    })
    .to_string()
    .into_bytes()
}

/// 生成纯文本消息事件的编码字节。
pub(super) fn text_event(open_id: &str, message_id: &str, text: &str) -> Vec<u8> {
    let content = json!({ "text": text }).to_string();
    message_event(("user", open_id), (message_id, "p2p", "text", &content), json!([]))
}

/// 读取下一条二进制 WebSocket 消息并解码为协议帧。
pub(super) async fn next_ws_message(ws: &mut WebSocketStream<tokio::net::TcpStream>) -> WsMsg {
    timeout(Duration::from_secs(2), ws.next())
        .await
        .expect("websocket message should arrive")
        .expect("websocket should stay open")
        .expect("websocket message should decode")
}

/// 断言消息是目标服务的 ping 帧。
pub(super) fn assert_ping_message(message: &WsMsg, service: i32) {
    let WsMsg::Binary(bytes) = message else {
        panic!("expected binary ping frame");
    };
    let frame = PbFrame::decode(&bytes[..]).expect("ping frame should decode");
    assert_eq!(frame.service, service);
    assert_eq!(frame.method, 0);
    assert_eq!(frame.header_value("type"), "ping");
}

/// 断言消息是成功 ack 帧。
pub(super) fn assert_ack_message(message: &WsMsg) {
    let WsMsg::Binary(bytes) = message else {
        panic!("expected binary ack frame");
    };
    let frame = PbFrame::decode(&bytes[..]).expect("ack frame should decode");
    assert_eq!(frame.header_value("biz_rt"), "0");
}
