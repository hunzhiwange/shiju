//! 推理 JSON、事件 SSE 与 WebSocket chat 客户端协议测试。

use std::net::TcpListener;
use std::sync::mpsc;

use serde_json::json;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::Message;
use vw_api_types::agent_scheduler::AgentSchedulerScopeDto;
use vw_api_types::id::ProjectId;

use super::GatewayAgentRequest;
use crate::client::test_support;
use crate::endpoint::GatewayEndpoint;

#[tokio::test]
async fn inference_and_stream_clients_preserve_protocol_events() {
    let sse = |value| format!("id: 7\nevent: update\ndata: {value}\n\n");
    let server = test_support::server_raw(vec![
        (200, json!({"response":"hello"}).to_string()),
        (200, json!({"id":"chat-1","choices":[]}).to_string()),
        (200, sse(json!({"type":"gateway"}))),
        (200, sse(json!({"type":"global"}))),
        (200, sse(json!({"type":"instance"}))),
        (200, sse(json!({"type":"scheduler"}))),
    ]);

    let agent = server
        .client()
        .inference_agent(&GatewayAgentRequest { message: "hello".to_string() })
        .await
        .expect("agent inference");
    assert_eq!(agent.response, "hello");
    let openai = server
        .client()
        .inference_openai_chat(&json!({"model":"auto","messages":[{"role":"user","content":"hello"}]}))
        .await
        .expect("OpenAI inference");
    assert_eq!(openai["id"], "chat-1");

    let mut events = Vec::new();
    server.client().stream_gateway_events(|event| { events.push(event); true }).await.unwrap();
    server.client().stream_global_events(|event| { events.push(event); true }).await.unwrap();
    server
        .client()
        .stream_instance_events(Some("/workspace"), |event| { events.push(event); true })
        .await
        .unwrap();
    let scope = AgentSchedulerScopeDto::for_default_tenant(ProjectId::from("project/one"))
        .expect("scope");
    server
        .client()
        .stream_agent_scheduler_events(&scope, Some(6), |event| { events.push(event); true })
        .await
        .unwrap();
    assert_eq!(events.iter().map(|event| event.id).collect::<Vec<_>>(), vec![Some(7); 4]);
    assert!(events.iter().all(|event| event.event.as_deref() == Some("update")));

    let requests = (0..6).map(|_| server.take_request()).collect::<Vec<_>>();
    assert_eq!(requests[0].path, "/v1/agent");
    assert_eq!(requests[1].path, "/v1/chat/completions");
    assert_eq!(requests[2].path, "/v1/events");
    assert_eq!(requests[3].path, "/v1/global/event");
    assert_eq!(requests[4].path, "/v1/event?directory=%2Fworkspace");
    assert_eq!(
        requests[5].path,
        "/v1/agent-scheduler/projects/project%2Fone/events/stream?after_cursor=6"
    );
    server.join();

    let (endpoint, handshake) = websocket_server();
    let client = crate::client::GatewayClient::new(endpoint).expect("WebSocket client");
    let mut socket = client.connect_chat_websocket().await.expect("connect chat WebSocket");
    socket.send_json(&json!({"type":"message","content":"hello"})).await.unwrap();
    let event = socket.next_event().await.unwrap().expect("text event");
    assert_eq!(serde_json::to_value(event).unwrap()["data"]["type"], "done");
    let closed = socket.next_event().await.unwrap().expect("close event");
    assert!(closed.is_closed());
    let handshake = handshake.recv().expect("handshake");
    assert_eq!(handshake.0, "/v1/ws/chat");
    assert!(handshake.1.contains("vibewindow.v1"));
}

fn websocket_server() -> (GatewayEndpoint, mpsc::Receiver<(String, String)>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind WebSocket server");
    let port = listener.local_addr().expect("address").port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept WebSocket");
        let callback = move |request: &Request, mut response: Response| {
            tx.send((
                request.uri().to_string(),
                request
                    .headers()
                    .get("sec-websocket-protocol")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default()
                    .to_string(),
            ))
            .expect("record handshake");
            response
                .headers_mut()
                .insert("sec-websocket-protocol", "vibewindow.v1".parse().unwrap());
            Ok(response)
        };
        let mut socket = tokio_tungstenite::tungstenite::accept_hdr(stream, callback)
            .expect("upgrade WebSocket");
        let message = socket.read().expect("client message");
        assert!(matches!(message, Message::Text(_)));
        socket
            .send(Message::Text(json!({"type":"done","full_response":"hello"}).to_string().into()))
            .expect("send done");
        socket.close(None).expect("close WebSocket");
    });
    (GatewayEndpoint::new("127.0.0.1", port), rx)
}
