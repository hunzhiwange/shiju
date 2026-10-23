use super::*;
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn model(provider: &str, id: &str, adapter: &str, url: &str) -> Model {
    let raw_provider =
        serde_json::from_value(json!({"id": provider, "api": url, "adapter": adapter})).unwrap();
    let raw_model = if provider == "shiju" {
        let catalog: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../vw-provider-resolver/assets/model/shiju.json"
        )))
        .unwrap();
        serde_json::from_value(catalog["shiju"]["models"][id].clone()).unwrap()
    } else {
        serde_json::from_value(json!({"id": id, "reasoning": true})).unwrap()
    };
    vw_shared::provider::state::from_models_dev_model(&raw_provider, &raw_model)
}

#[test]
fn reasoning_validation_rejects_unknown_invalid_disabled_and_collapsed_levels() {
    for (id, effort) in [
        ("qwen3.7-plus", "low"),
        ("deepseek-v4-pro", "low"),
        ("deepseek-v4-flash-0731", "medium"),
        ("qwen3.8-max", "high"),
        ("unknown", "high"),
    ] {
        assert!(
            validate_reasoning(
                &model("token-plan", id, "openai-compatible", ""),
                &json!({"reasoning_effort": effort})
            )
            .is_err()
        );
    }
    let mut model = model("shiju", "deepseek-v4.1-flash", "openai-compatible", "");
    for options in [
        json!({"reasoning_effort": 3}),
        json!({"reasoning_effort": "extreme"}),
        json!({"reasoning_effort": "high", "thinking_budget": 100}),
    ] {
        assert!(validate_reasoning(&model, &options).is_err());
    }
    assert!(validate_reasoning(&model, &json!({})).is_ok());
    assert!(validate_reasoning(&model, &json!({"reasoning_effort": "max"})).is_ok());
    model.capabilities.reasoning = false;
    assert!(validate_reasoning(&model, &json!({"reasoning_effort": "high"})).is_err());
}

#[tokio::test]
async fn reasoning_selection_reaches_real_sdk_http_body_for_every_supported_level() {
    use super::super::{convert::compute_request_info, driver::dispatch_stream_request};
    for (provider, id, adapter, endpoint) in [
        ("openai", "gpt-5", "openai-compatible", "/chat/completions"),
        ("openai", "gpt-5.4", "openai", "/responses"),
        ("anthropic", "claude-sonnet-4-6", "anthropic", "/messages"),
        ("anthropic", "claude-opus-4-7", "anthropic", "/messages"),
        ("shiju", "deepseek-v4-pro-0813", "openai-compatible", "/chat/completions"),
        ("shiju", "deepseek-v4.1-flash", "openai-compatible", "/chat/completions"),
        ("shiju", "deepseek-v4.1-flash-1017", "openai-compatible", "/chat/completions"),
        ("shiju", "glm-5.3", "openai-compatible", "/chat/completions"),
        ("shiju", "kimi-k3", "openai-compatible", "/chat/completions"),
        ("shiju", "qwen3.8-2.4t-a95b", "openai-compatible", "/chat/completions"),
        ("shiju", "qwen3.8-27b", "openai-compatible", "/chat/completions"),
        ("shiju", "qwen3.8-flash", "openai-compatible", "/chat/completions"),
        ("shiju", "qwen3.8-max", "openai-compatible", "/chat/completions"),
        ("shiju", "qwen3.8-omni-flash", "openai-compatible", "/chat/completions"),
        ("token-plan", "deepseek-v4-flash-0731", "openai-compatible", "/chat/completions"),
        ("token-plan", "glm-5.2", "openai-compatible", "/chat/completions"),
        ("token-plan", "qwen3.8-max", "openai-compatible", "/chat/completions"),
    ] {
        let metadata = model(provider, id, adapter, endpoint);
        let levels = reasoning_levels(
            id,
            adapter,
            provider,
            endpoint,
            metadata.capabilities.reasoning_options.as_deref(),
        );
        assert!(!levels.is_empty(), "{provider}/{id} must exercise requests");
        for effort in &levels {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}/v1{endpoint}", listener.local_addr().unwrap());
            let receive = tokio::spawn(async move {
                let (mut socket, _) =
                    tokio::time::timeout(std::time::Duration::from_secs(10), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                let mut data = Vec::new();
                let mut buf = [0; 8192];
                let header_end = loop {
                    let n = socket.read(&mut buf).await.unwrap();
                    assert!(n > 0);
                    data.extend_from_slice(&buf[..n]);
                    if let Some(i) = data.windows(4).position(|s| s == b"\r\n\r\n") {
                        break i + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&data[..header_end]);
                assert!(
                    headers.starts_with(&format!("POST /v1{endpoint} ")),
                    "unexpected request: {}",
                    headers.lines().next().unwrap()
                );
                let length: usize = headers
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|s| s.trim().parse().unwrap())
                    })
                    .unwrap();
                while data.len() < header_end + length {
                    let n = socket.read(&mut buf).await.unwrap();
                    assert!(n > 0);
                    data.extend_from_slice(&buf[..n]);
                }
                let body: Value =
                    serde_json::from_slice(&data[header_end..header_end + length]).unwrap();
                socket.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
                body
            });
            let model = model(provider, id, adapter, &url);
            let info = compute_request_info(&model, &json!([{"role": "user", "content": "hello"}]))
                .unwrap();
            // The local server deliberately rejects after recording the exact wire payload.
            let result = dispatch_stream_request(
                provider,
                &model,
                "test-key",
                info,
                &HashMap::new(),
                None,
                None,
                Some(4096),
                &json!({"reasoning_effort": effort}),
                0,
                None,
                &mut |_| {},
            )
            .await;
            assert!(result.is_err());
            let body = receive.await.unwrap();
            assert_eq!(body["model"], id);
            if endpoint == "/responses" {
                assert_eq!(body["reasoning"]["effort"], *effort);
                assert!(body.get("reasoning_effort").is_none());
            } else if endpoint == "/messages" {
                assert_eq!(body["output_config"]["effort"], *effort);
                assert_eq!(body["thinking"]["type"], "adaptive");
                assert!(body["thinking"].get("budget_tokens").is_none());
            } else if effort == "none" && provider != "openai" {
                assert!(body.get("reasoning_effort").is_none());
            } else {
                assert_eq!(body["reasoning_effort"], *effort, "{provider}/{id}/{effort}");
            }
            if provider == "token-plan" || (provider == "shiju" && id.starts_with("qwen")) {
                assert_eq!(body["enable_thinking"], effort != "none");
            }
            if provider == "shiju" && id.starts_with("deepseek-") {
                assert_eq!(
                    body["thinking"]["type"],
                    if effort == "none" { "disabled" } else { "enabled" }
                );
            }
        }
    }
}
