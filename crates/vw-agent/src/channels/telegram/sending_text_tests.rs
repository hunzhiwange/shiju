use super::{TelegramChannel, message_utils::split_message_for_telegram};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

#[test]
fn sending_text_uses_plain_single_chunk_for_empty_message() {
    assert_eq!(split_message_for_telegram(""), vec!["".to_string()]);
}

async fn read_http_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];

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

async fn telegram_server(
    responses: Vec<(u16, &'static str)>,
) -> (String, mpsc::Receiver<String>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel(responses.len());

    let handle = tokio::spawn(async move {
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_http_request(&mut stream).await;
            let _ = tx.send(request).await;
            let reason = if status == 200 { "OK" } else { "Bad Request" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        }
    });

    (format!("http://{addr}"), rx, handle)
}

fn request_json(request: &str) -> serde_json::Value {
    let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();
    serde_json::from_str(body).unwrap()
}

#[tokio::test]
async fn send_text_chunks_posts_html_body_with_thread_id() {
    let (base_url, mut requests, server) =
        telegram_server(vec![(200, r#"{"ok":true,"result":{"message_id":1}}"#)]).await;
    let channel =
        TelegramChannel::new("token".into(), vec!["*".into()], false).with_api_base(base_url);

    channel.send_text_chunks("**hello** <world>", "chat-1", Some("77")).await.unwrap();

    let request = requests.recv().await.unwrap();
    assert!(request.starts_with("POST /bottoken/sendMessage "));
    let body = request_json(&request);
    assert_eq!(body["chat_id"], "chat-1");
    assert_eq!(body["message_thread_id"], "77");
    assert_eq!(body["parse_mode"], "HTML");
    assert_eq!(body["text"], "<b>hello</b> &lt;world&gt;");
    server.await.unwrap();
}

#[tokio::test]
async fn send_text_chunks_falls_back_to_plain_text_when_html_send_fails() {
    let (base_url, mut requests, server) = telegram_server(vec![
        (400, r#"{"ok":false,"description":"can't parse entities"}"#),
        (200, r#"{"ok":true,"result":{"message_id":2}}"#),
    ])
    .await;
    let channel =
        TelegramChannel::new("token".into(), vec!["*".into()], false).with_api_base(base_url);

    channel.send_text_chunks("literal <broken>", "chat-1", None).await.unwrap();

    let html_request = requests.recv().await.unwrap();
    let plain_request = requests.recv().await.unwrap();
    assert_eq!(request_json(&html_request)["parse_mode"], "HTML");

    let plain_body = request_json(&plain_request);
    assert_eq!(plain_body["chat_id"], "chat-1");
    assert_eq!(plain_body["text"], "literal <broken>");
    assert!(plain_body.get("parse_mode").is_none());
    server.await.unwrap();
}

#[tokio::test]
async fn send_text_chunks_reports_sanitized_markdown_and_plain_errors() {
    let (base_url, _requests, server) = telegram_server(vec![
        (400, r#"{"description":"bad https://api.telegram.org/bot123:SECRET/sendMessage"}"#),
        (401, r#"{"description":"bad bot123:SECRET"}"#),
    ])
    .await;
    let channel =
        TelegramChannel::new("123:SECRET".into(), vec!["*".into()], false).with_api_base(base_url);

    let err = channel.send_text_chunks("hello", "chat-1", None).await.unwrap_err().to_string();

    assert!(err.contains("markdown 400"));
    assert!(err.contains("plain 401"));
    assert!(err.contains("bot[redacted]"));
    assert!(!err.contains("SECRET"));
    server.await.unwrap();
}

#[tokio::test]
async fn send_text_chunks_marks_continuation_chunks() {
    let long_message = "abcdefghij ".repeat(900);
    let chunk_count = split_message_for_telegram(&long_message).len();
    assert!(chunk_count > 1);

    let responses =
        (0..chunk_count).map(|_| (200, r#"{"ok":true,"result":{"message_id":1}}"#)).collect();
    let (base_url, mut requests, server) = telegram_server(responses).await;
    let channel =
        TelegramChannel::new("token".into(), vec!["*".into()], false).with_api_base(base_url);

    channel.send_text_chunks(&long_message, "chat-1", None).await.unwrap();

    let first = request_json(&requests.recv().await.unwrap());
    assert!(first["text"].as_str().unwrap().contains("(continues...)"));

    let mut last = first;
    for _ in 1..chunk_count {
        last = request_json(&requests.recv().await.unwrap());
    }
    assert!(last["text"].as_str().unwrap().starts_with("(continued)"));
    assert!(!last["text"].as_str().unwrap().contains("(continues...)"));
    server.await.unwrap();
}

#[tokio::test]
async fn send_text_chunks_sends_single_empty_chunk_as_plain_html_body() {
    // 空消息只产生一个片段，走 chunk.to_string() 分支且不附带 thread_id。
    let (base_url, mut requests, server) =
        telegram_server(vec![(200, r#"{"ok":true,"result":{"message_id":1}}"#)]).await;
    let channel =
        TelegramChannel::new("token".into(), vec!["*".into()], false).with_api_base(base_url);

    channel.send_text_chunks("", "chat-1", None).await.unwrap();

    let body = request_json(&requests.recv().await.unwrap());
    assert_eq!(body["chat_id"], "chat-1");
    assert_eq!(body["parse_mode"], "HTML");
    assert_eq!(body["text"], "");
    assert!(body.get("message_thread_id").is_none());
    server.await.unwrap();
}

#[tokio::test]
async fn send_text_chunks_marks_middle_chunk_with_both_continuation_hints() {
    // 三段以上消息会命中中间片段的 "(continued)...(continues...)" 双向提示分支。
    let long_message = "abcdefghij ".repeat(1200);
    let chunk_count = split_message_for_telegram(&long_message).len();
    assert!(chunk_count >= 3, "expected >=3 chunks, got {chunk_count}");

    let responses =
        (0..chunk_count).map(|_| (200, r#"{"ok":true,"result":{"message_id":1}}"#)).collect();
    let (base_url, mut requests, server) = telegram_server(responses).await;
    let channel =
        TelegramChannel::new("token".into(), vec!["*".into()], false).with_api_base(base_url);

    channel.send_text_chunks(&long_message, "chat-1", None).await.unwrap();

    let _first = requests.recv().await.unwrap();
    let middle = request_json(&requests.recv().await.unwrap());
    let middle_text = middle["text"].as_str().unwrap();
    assert!(middle_text.starts_with("(continued)"));
    assert!(middle_text.ends_with("(continues...)"));

    for _ in 2..chunk_count {
        let _ = requests.recv().await.unwrap();
    }
    server.await.unwrap();
}

#[tokio::test]
async fn send_text_chunks_plain_fallback_keeps_thread_id_and_continues_after_first_chunk() {
    let long_message = "a".repeat(4097);
    let chunk_count = split_message_for_telegram(&long_message).len();
    assert_eq!(chunk_count, 2);

    let (base_url, mut requests, server) = telegram_server(vec![
        (400, r#"{"ok":false,"description":"can't parse entities"}"#),
        (200, r#"{"ok":true,"result":{"message_id":2}}"#),
        (200, r#"{"ok":true,"result":{"message_id":3}}"#),
    ])
    .await;
    let channel =
        TelegramChannel::new("token".into(), vec!["*".into()], false).with_api_base(base_url);

    channel.send_text_chunks(&long_message, "chat-1", Some("77")).await.unwrap();

    let html_request = request_json(&requests.recv().await.unwrap());
    assert_eq!(html_request["parse_mode"], "HTML");
    assert_eq!(html_request["message_thread_id"], "77");

    let plain_request = request_json(&requests.recv().await.unwrap());
    assert_eq!(plain_request["chat_id"], "chat-1");
    assert_eq!(plain_request["message_thread_id"], "77");
    assert!(plain_request.get("parse_mode").is_none());
    assert!(plain_request["text"].as_str().unwrap().ends_with("(continues...)"));

    let next_html_request = request_json(&requests.recv().await.unwrap());
    assert_eq!(next_html_request["parse_mode"], "HTML");
    assert_eq!(next_html_request["message_thread_id"], "77");
    assert!(next_html_request["text"].as_str().unwrap().starts_with("(continued)"));
    server.await.unwrap();
}
