use serde_json::json;

#[tokio::test]
async fn model_protocol_016_http_sse_thinking_receipt_replays() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    super::super::messages_thinking_http_tests::thinking_replays_through_bound_http().await;
}

fn chat_wire(done: bool) -> String {
    let chunks = [
        json!({"id":"reply","choices":[{"index":0,"delta":{"role":"assistant","content":"你好"}}]}),
        json!({"id":"reply","choices":[{"index":0,"delta":{"tool_calls":[
            {"index":1,"id":"b","type":"function","function":{"name":"second","arguments":"{"}},
            {"index":0,"id":"a","type":"function","function":{"name":"first","arguments":"{\"城市\":"}}]}}]}),
        json!({"id":"reply","choices":[{"index":0,"delta":{"content":"","reasoning_content":null,"tool_calls":[
            {"index":0,"function":{"arguments":"\"杭州\"}"}},
            {"index":1,"function":{"arguments":"}"}}]},"finish_reason":"tool_calls"}]}),
        json!({"id":"reply","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":9}}),
    ];
    let mut wire = chunks.iter().map(|v| format!("data: {v}\r\n\r\n")).collect::<String>();
    if done {
        wire.push_str("data: [DONE]\r\n\r\n");
    }
    wire
}


#[tokio::test]
async fn model_protocol_016_thinking_http_errors_never_issue_success() {
    use super::super::{RelayIdentity, messages_thinking::Context};
    use crate::app_routing::reference_scope::References;
    use axum::{Router, routing::get};
    use std::sync::Arc;
    let context = Context::new(
        &RelayIdentity {
            tenant: vw_api_transport::TenantScope::new("stream", "fixture").unwrap(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected".into(),
            generation: 1,
        },
        "deployment".into(),
        Arc::new(References::default()),
    );
    for (wire, cancelled) in [
        (chat_wire(false), false),
        ("data: {\"error\":{\"message\":\"private-payload\"}}\n\n".into(), false),
        (chat_wire(true), true),
    ] {
        let router = Router::new().route(
            "/",
            get(move || {
                let wire = wire.clone();
                async move { ([("content-type", "text/event-stream")], wire) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let response = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(format!("http://{address}/"))
            .send()
            .await
            .unwrap();
        let cancel = tokio_util::sync::CancellationToken::new();
        if cancelled {
            cancel.cancel();
        }
        let response = super::super::messages_thinking::response(
            response,
            context.clone(),
            "selected",
            true,
            cancel,
        )
        .await
        .unwrap();
        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let text = std::str::from_utf8(&body).unwrap();
        if cancelled {
            assert!(text.is_empty());
        } else {
            assert!(text.contains("event: error"));
        }
        assert!(!text.contains("message_stop") && !text.contains("signature_delta"));
        assert!(!text.contains("private-payload"));
        server.abort();
        let _ = server.await;
    }
}
