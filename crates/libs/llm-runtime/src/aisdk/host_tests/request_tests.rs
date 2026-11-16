//! 验证 AI SDK 请求体的消息、工具、选项和供应商元数据。

use std::collections::HashMap;

use serde_json::{Value, json};

use crate::app::agent::provider::provider;
use crate::app::agent::tools;

use super::do_stream_request_aisdk;

fn test_model(api_url: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": "gpt-test",
        "providerID": "openai",
        "api": {
            "id": "gpt-test",
            "url": api_url,
            "adapter": "openai-compatible"
        },
        "name": "GPT Test",
        "family": null,
        "capabilities": {
            "temperature": true,
            "reasoning": true,
            "attachment": false,
            "toolcall": true,
            "input": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "output": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "interleaved": false
        },
        "cost": {
            "input": 0.0,
            "output": 0.0,
            "cache": {
                "read": 0.0,
                "write": 0.0
            },
            "experimental_over_200k": null
        },
        "limit": {
            "context": 8192,
            "input": null,
            "output": 4096
        },
        "status": "active",
        "options": {},
        "headers": {},
        "release_date": "2026-01-01",
        "variants": {}
    }))
    .expect("test model should deserialize")
}

fn test_provider(key: Option<&str>, env_keys: Vec<&str>) -> provider::Info {
    provider::Info {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        source: provider::ProviderSource::Custom,
        env: env_keys.into_iter().map(ToString::to_string).collect(),
        key: key.map(ToString::to_string),
        options: HashMap::new(),
        models: HashMap::new(),
    }
}

fn user_messages() -> Value {
    json!([{ "role": "user", "content": "hello" }])
}

#[tokio::test]
async fn native_tool_search_request_captures_native_and_ordinary_transport() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for native in [true, false] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/v1/responses", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            tokio::time::timeout(std::time::Duration::from_secs(30), async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut data = Vec::new();
                let mut buffer = [0; 4096];
                let end = loop {
                    let read = socket.read(&mut buffer).await.unwrap();
                    assert!(read > 0);
                    data.extend_from_slice(&buffer[..read]);
                    if let Some(index) = data.windows(4).position(|bytes| bytes == b"\r\n\r\n") { break index + 4; }
                };
                let headers = String::from_utf8_lossy(&data[..end]);
                let length: usize = headers.lines().find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length:").map(|value| value.trim().parse().unwrap())).unwrap();
                while data.len() < end + length {
                    let read = socket.read(&mut buffer).await.unwrap();
                    assert!(read > 0);
                    data.extend_from_slice(&buffer[..read]);
                }
                let request: Value = serde_json::from_slice(&data[end..end+length]).unwrap();
                let body = "data: {\"type\":\"response.completed\",\"sequence_number\":1,\"response\":{\"id\":\"fixture\",\"output\":[]}}\n\n";
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
                request
            }).await.unwrap()
        });
        let mut model = test_model(&url);
        model.api.adapter = "openai-responses".into();
        model.capabilities.native_tool_search = native;
        let search = tools::ToolSpec::new(
            "tool_search",
            "Find tools",
            json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"],"additionalProperties":false}),
        );
        let mut read = tools::ToolSpec::new(
            "stable_read",
            "Read",
            json!({"type":"object","properties":{},"additionalProperties":false}),
        );
        read.identity.namespace = Some("files".into());
        read.identity.name = "read".into();
        let mut hidden = tools::ToolSpec::new("secret", "Hidden", json!({"type":"object"}));
        hidden.exposure = vw_api_tools::ToolExposure::Hidden;
        // The request snapshot deliberately excludes unregistered/unauthorized tools.
        let mut specs =
            HashMap::from([("tool_search".into(), search), ("stable_read".into(), read)]);
        specs.insert("secret".into(), hidden);
        let result = do_stream_request_aisdk(
            &test_provider(Some("fixture-key"), vec![]),
            None,
            &HashMap::new(),
            &json!({}),
            &model,
            &json!([{"role":"assistant","content":"", "tool_calls":[{"id":"prior-call","type":"function","function":{"name":"stable_read","arguments":"{}"}}]}, {"role":"tool","tool_call_id":"prior-call","content":"previous result"}, {"role":"user","content":"continue"}]),
            &specs,
            None,
            None,
            None,
            0,
            None,
            &mut |_| {},
        )
        .await;
        assert!(result.is_ok(), "{result:?}");
        let wire = server.await.unwrap();
        assert!(!wire.to_string().contains("secret"));
        let declarations = wire["tools"].as_array().unwrap();
        if native {
            assert!(
                declarations
                    .iter()
                    .any(|tool| tool["type"] == "tool_search" && tool["execution"] == "client")
            );
            let namespace = declarations.iter().find(|tool| tool["type"] == "namespace").unwrap();
            assert_eq!(namespace["name"], "files");
            assert_eq!(namespace["tools"][0]["name"], "read");
            let previous = wire["input"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["type"] == "function_call")
                .unwrap();
            assert_eq!(previous["name"], "read");
            assert_eq!(previous["namespace"], "files");
            assert!(!wire.to_string().contains("secret"));
        } else {
            assert!(declarations.iter().all(|tool| tool["type"] == "function"));
            assert!(declarations.iter().any(|tool| tool["name"] == "tool_search"));
            assert!(declarations.iter().any(|tool| tool["name"] == "stable_read"));
        }
    }
}

#[tokio::test]
async fn native_tool_search_request_invalid_schema_fails_before_transport_for_both_modes() {
    for native in [false, true] {
        let mut model = test_model("http://127.0.0.1:9/v1/responses");
        model.api.adapter = "openai-responses".into();
        model.capabilities.native_tool_search = native;
        let specs =
            HashMap::from([("bad".into(), tools::ToolSpec::new("bad", "Bad schema", json!(42)))]);
        let result = do_stream_request_aisdk(
            &test_provider(Some("fixture-key"), vec![]),
            None,
            &HashMap::new(),
            &json!({}),
            &model,
            &user_messages(),
            &specs,
            None,
            None,
            None,
            0,
            None,
            &mut |_| {},
        )
        .await;
        assert!(result.unwrap_err().to_string().contains("invalid schema for tool bad"));
    }
}
