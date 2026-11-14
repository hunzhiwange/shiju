use super::{JsonTarget, VerifiedRouteBinding};
use crate::app_routing::{RoutingListener, SharedTransport};
use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::post,
};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::{net::TcpListener, sync::mpsc, time::timeout};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

#[tokio::test]
async fn cc_route_014_listener_filters_the_authorized_outgoing_json() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = format!("http://{}/fixture", upstream.local_addr().unwrap()).parse().unwrap();
    let (requests, mut received) = mpsc::unbounded_channel();
    let upstream_app = Router::new().route(
        "/fixture",
        post(move |headers: HeaderMap, Json(body): Json<Value>| async move {
            requests.send((headers, body)).unwrap();
            ([(header::SET_COOKIE, "fixture=private")], Json(json!({"id": "upstream-response"})))
                .into_response()
        }),
    );
    let upstream_task = tokio::spawn(async move {
        axum::serve(upstream, upstream_app).await.unwrap();
    });

    let listener = RoutingListener::default();
    let address = listener.start(0).unwrap();
    let base = format!("http://{address}");
    let chat = format!("{base}/v1/chat/completions");
    let binding = VerifiedRouteBinding {
        project: ProjectScope::new(
            TenantScope::new("platform", "company").unwrap(),
            "project".into(),
        )
        .unwrap(),
        host: "host".into(),
        app: AppId::Codex,
        account: "account".into(),
        account_generation: 1,
    };
    let unready = listener.auth.issue(binding.clone()).unwrap();
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    assert_eq!(
        client
            .post(&chat)
            .bearer_auth(&unready.token)
            .body("not parsed")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::SERVICE_UNAVAILABLE,
    );

    let active = listener
        .auth
        .issue_with_target(
            binding,
            Some(JsonTarget {
                #[cfg(target_os = "macos")]
                managed: None,
                url: target,
                authorization: "Bearer provider-fixture".parse().unwrap(),
                transport: SharedTransport::new().unwrap(),
                request_options: Default::default(),
            }),
        )
        .unwrap();
    assert_eq!(
        client.post(&chat).bearer_auth(&unready.token).send().await.unwrap().status(),
        StatusCode::UNAUTHORIZED,
    );
    for (builder, status) in [
        (client.get(&chat).bearer_auth(&active.token), StatusCode::METHOD_NOT_ALLOWED),
        (
            client.post(format!("{base}/unsupported")).bearer_auth(&active.token),
            StatusCode::NOT_FOUND,
        ),
        (
            client.post(&chat).bearer_auth(&active.token).json(&json!({"stream": "true"})),
            StatusCode::BAD_REQUEST,
        ),
        (
            client.post(&chat).bearer_auth(&active.token).body("untyped"),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
    ] {
        assert_eq!(builder.send().await.unwrap().status(), status);
    }
    assert!(received.try_recv().is_err());

    let response = client
        .post(&chat)
        .bearer_auth(&active.token)
        .header("x-upstream-url", "http://example.invalid")
        .json(&json!({
            "model": "selected",
            "_credential": "fixture",
            "metadata": {"properties": {"_private": true}},
            "tools": [{"type": "function", "function": {"name": "lookup", "parameters": {
                "properties": {"_id": {"type": "string", "_note": true}}
            }}}]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.headers().contains_key(header::SET_COOKIE));
    assert_eq!(response.json::<Value>().await.unwrap(), json!({"id": "upstream-response"}));
    let (headers, body) = timeout(Duration::from_secs(10), received.recv()).await.unwrap().unwrap();
    assert_eq!(headers[header::AUTHORIZATION], "Bearer provider-fixture");
    assert!(!headers.contains_key("x-upstream-url"));
    assert_eq!(
        body,
        json!({
            "model": "selected",
            "metadata": {"properties": {}},
            "tools": [{"type": "function", "function": {"name": "lookup", "parameters": {
                "properties": {"_id": {"type": "string"}}
            }}}]
        }),
    );
    listener.stop().await.unwrap();
    upstream_task.abort();
    assert!(upstream_task.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn cc_route_034_authorized_native_chat_json_and_sse() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target =
        format!("http://{}/v1/chat/completions", upstream.local_addr().unwrap()).parse().unwrap();
    let (requests, mut received) = mpsc::unbounded_channel();
    let result = json!({"id":"fixture","model":"fixture","choices":[{"index":0,"message":{"role":"assistant","tool_calls":[{"id":"call_1","type":"function","function":{"name":"lookup","arguments":"{\"city\":\"上海\"}"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":2,"completion_tokens":3,"total_tokens":5}});
    let stream = format!(
        "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
        json!({"model":"fixture","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"lookup","arguments":"{}"}}]},"finish_reason":null}]}),
        json!({"model":"fixture","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":2,"completion_tokens":3,"total_tokens":5}})
    );
    let expected_stream = stream.clone();
    let expected_result = result.clone();
    let upstream_task = tokio::spawn(async move {
        axum::serve(
            upstream,
            Router::new().route(
                "/v1/chat/completions",
                post(move |headers: HeaderMap, Json(body): Json<Value>| {
                    let result = result.clone();
                    let stream = stream.clone();
                    let requests = requests.clone();
                    async move {
                        let streaming = body["stream"] == true;
                        requests.send((headers, body)).unwrap();
                        if streaming {
                            ([(header::CONTENT_TYPE, "text/event-stream")], stream).into_response()
                        } else {
                            Json(result).into_response()
                        }
                    }
                }),
            ),
        )
        .await
        .unwrap();
    });
    let listener = RoutingListener::default();
    let address = listener.start(0).unwrap();
    let binding = VerifiedRouteBinding {
        project: ProjectScope::new(
            TenantScope::new("platform", "company").unwrap(),
            "project".into(),
        )
        .unwrap(),
        host: "host".into(),
        app: AppId::Codex,
        account: "account".into(),
        account_generation: 1,
    };
    let active = listener
        .auth
        .issue_with_target(
            binding,
            Some(JsonTarget {
                #[cfg(target_os = "macos")]
                managed: None,
                url: target,
                authorization: "Bearer fixture-upstream".parse().unwrap(),
                transport: SharedTransport::new().unwrap(),
                request_options: Default::default(),
            }),
        )
        .unwrap();
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    for path in [
        "/chat/completions",
        "/v1/chat/completions",
        "/v1/v1/chat/completions",
        "/codex/v1/chat/completions",
    ] {
        let url = format!("http://{address}{path}");
        assert_eq!(
            client.post(&url).json(&json!({})).send().await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        for streaming in [false, true] {
            let body = json!({"model":"fixture","stream":streaming,"messages":[{"role":"tool","tool_call_id":"call_0","content":"done"}],"tools":[{"type":"function","function":{"name":"lookup","parameters":{"type":"object"}}}],"vendor_option":true});
            let response =
                client.post(&url).bearer_auth(&active.token).json(&body).send().await.unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            if streaming {
                assert_eq!(response.text().await.unwrap(), expected_stream);
            } else {
                assert_eq!(response.json::<Value>().await.unwrap(), expected_result);
            }
            let (headers, outgoing) =
                timeout(Duration::from_secs(10), received.recv()).await.unwrap().unwrap();
            assert_eq!(headers[header::AUTHORIZATION], "Bearer fixture-upstream");
            assert_eq!(outgoing, body);
        }
    }
    for path in ["/other/v1/chat/completions", "/v1/chat/completions?x=1", "/v1/chat/completions/"]
    {
        let response = client
            .post(format!("http://{address}{path}"))
            .bearer_auth(&active.token)
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    assert!(received.try_recv().is_err());
    listener.stop().await.unwrap();
    upstream_task.abort();
    assert!(upstream_task.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn cc_route_035_authorized_native_messages_json_and_sse() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = format!("http://{}/v1/messages", upstream.local_addr().unwrap()).parse().unwrap();
    let (requests, mut received) = mpsc::unbounded_channel();
    let result = json!({"id":"m","type":"message","role":"assistant","model":"fixture","content":[{"type":"thinking","thinking":"reason","signature":"signed-fixture"},{"type":"tool_use","id":"call_1","name":"lookup","input":{"city":"上海"}}],"stop_reason":"tool_use","usage":{"input_tokens":2,"output_tokens":3}});
    let stream = [
        json!({"type":"message_start","message":{"id":"m","type":"message","role":"assistant","model":"fixture","content":[]}}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"reason"}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"signed-fixture"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"call_1","name":"lookup","input":{}}}),
        json!({"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"city\":\"上海\"}"}}),
        json!({"type":"content_block_stop","index":1}),
        json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":3}}),
        json!({"type":"message_stop"}),
    ].iter().map(|event| format!("data: {event}\n\n")).collect::<String>();
    let expected_stream = stream.clone();
    let expected_result = result.clone();
    let upstream_task = tokio::spawn(async move {
        axum::serve(
            upstream,
            Router::new().route(
                "/v1/messages",
                post(move |headers: HeaderMap, Json(body): Json<Value>| {
                    let result = result.clone();
                    let stream = stream.clone();
                    let requests = requests.clone();
                    async move {
                        let streaming = body["stream"] == true;
                        requests.send((headers, body)).unwrap();
                        if streaming {
                            ([(header::CONTENT_TYPE, "text/event-stream")], stream).into_response()
                        } else {
                            Json(result).into_response()
                        }
                    }
                }),
            ),
        )
        .await
        .unwrap();
    });
    let listener = RoutingListener::default();
    let address = listener.start(0).unwrap();
    let binding = VerifiedRouteBinding {
        project: ProjectScope::new(
            TenantScope::new("platform", "company").unwrap(),
            "project".into(),
        )
        .unwrap(),
        host: "host".into(),
        app: AppId::Claude,
        account: "account".into(),
        account_generation: 1,
    };
    let active = listener
        .auth
        .issue_with_target(
            binding,
            Some(JsonTarget {
                #[cfg(target_os = "macos")]
                managed: None,
                url: target,
                authorization: "Bearer fixture-upstream".parse().unwrap(),
                transport: SharedTransport::new().unwrap(),
                request_options: Default::default(),
            }),
        )
        .unwrap();
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    for path in ["/v1/messages", "/claude/v1/messages"] {
        let url = format!("http://{address}{path}");
        assert_eq!(
            client.post(&url).json(&json!({})).send().await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        for streaming in [false, true] {
            let body = json!({"model":"fixture","max_tokens":128,"stream":streaming,"thinking":{"type":"enabled","budget_tokens":64},"messages":[{"role":"assistant","content":[{"type":"thinking","thinking":"reason","signature":"signed-fixture"},{"type":"tool_use","id":"call_0","name":"lookup","input":{}}]},{"role":"user","content":[{"type":"tool_result","tool_use_id":"call_0","content":"done"},{"type":"image","source":{"type":"base64","media_type":"image/png","data":"fixture"}},{"type":"text","text":"你好"}]}],"tools":[{"name":"lookup","input_schema":{"type":"object"}}],"vendor_option":true});
            let response =
                client.post(&url).bearer_auth(&active.token).json(&body).send().await.unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            if streaming {
                assert_eq!(response.text().await.unwrap(), expected_stream);
            } else {
                assert_eq!(response.json::<Value>().await.unwrap(), expected_result);
            }
            let (headers, outgoing) =
                timeout(Duration::from_secs(10), received.recv()).await.unwrap().unwrap();
            assert_eq!(headers[header::AUTHORIZATION], "Bearer fixture-upstream");
            assert_eq!(outgoing, body);
        }
    }
    for path in
        ["/other/v1/messages", "/v1/messages?x=1", "/v1/messages/", "/claude-desktop/v1/messages"]
    {
        let response = client
            .post(format!("http://{address}{path}"))
            .bearer_auth(&active.token)
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    assert!(received.try_recv().is_err());
    listener.stop().await.unwrap();
    upstream_task.abort();
    assert!(upstream_task.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn cc_route_036_authorized_native_responses_json_and_sse() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target: reqwest::Url =
        format!("http://{}/v1/responses", upstream.local_addr().unwrap()).parse().unwrap();
    let (requests, mut received) = mpsc::unbounded_channel();
    let result = json!({"id":"resp_fixture","object":"response","status":"completed","output":[{"type":"function_call","id":"item_fixture","call_id":"call_fixture","name":"lookup","arguments":"{\"city\":\"上海\"}"}],"usage":{"input_tokens":2,"output_tokens":3}});
    let stream = format!(
        "data: {}\n\ndata: {}\n\ndata: {}\n\n",
        json!({"type":"response.created","response":{"id":"resp_fixture","status":"in_progress"}}),
        json!({"type":"response.output_text.delta","delta":"你好"}),
        json!({"type":"response.completed","response":result})
    );
    let expected_stream = stream.clone();
    let expected_result = result.clone();
    let upstream_task = tokio::spawn(async move {
        axum::serve(
            upstream,
            Router::new().route(
                "/v1/responses",
                post(move |headers: HeaderMap, Json(body): Json<Value>| {
                    let result = result.clone();
                    let stream = stream.clone();
                    let requests = requests.clone();
                    async move {
                        let streaming = body["stream"] == true;
                        requests.send((headers, body)).unwrap();
                        if streaming {
                            ([(header::CONTENT_TYPE, "text/event-stream")], stream).into_response()
                        } else {
                            Json(result).into_response()
                        }
                    }
                }),
            ),
        )
        .await
        .unwrap();
    });
    let listener = RoutingListener::default();
    let address = listener.start(0).unwrap();
    let binding = VerifiedRouteBinding {
        project: ProjectScope::new(
            TenantScope::new("platform", "company").unwrap(),
            "project".into(),
        )
        .unwrap(),
        host: "host".into(),
        app: AppId::Codex,
        account: "account".into(),
        account_generation: 1,
    };
    let active = listener
        .auth
        .issue_with_target(
            binding.clone(),
            Some(JsonTarget {
                #[cfg(target_os = "macos")]
                managed: None,
                url: target.clone(),
                authorization: "Bearer fixture-upstream".parse().unwrap(),
                transport: SharedTransport::new().unwrap(),
                request_options: Default::default(),
            }),
        )
        .unwrap();
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    let websocket = client
        .get(format!("http://{address}/v1/responses"))
        .bearer_auth(&active.token)
        .send()
        .await
        .unwrap();
    assert_eq!(websocket.status(), StatusCode::UPGRADE_REQUIRED);
    assert_eq!(
        websocket.json::<Value>().await.unwrap()["error"]["code"],
        "websocket_not_supported"
    );
    for path in [
        "/responses",
        "/v1/responses",
        "/v1/v1/responses",
        "/codex/v1/responses",
        "/grokbuild/v1/responses",
    ] {
        let url = format!("http://{address}{path}");
        assert_eq!(
            client.post(&url).json(&json!({})).send().await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        for streaming in [false, true] {
            let body = json!({"model":"fixture","stream":streaming,"input":"你好","tools":[{"type":"function","name":"lookup","parameters":{"type":"object"}}],"vendor_option":true});
            let response =
                client.post(&url).bearer_auth(&active.token).json(&body).send().await.unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            if streaming {
                assert_eq!(response.text().await.unwrap(), expected_stream);
            } else {
                assert_eq!(response.json::<Value>().await.unwrap(), expected_result);
            }
            let (headers, outgoing) =
                timeout(Duration::from_secs(10), received.recv()).await.unwrap().unwrap();
            assert_eq!(headers[header::AUTHORIZATION], "Bearer fixture-upstream");
            assert_eq!(outgoing, body);
        }
    }
    for path in [
        "/other/v1/responses",
        "/v1/responses?x=1",
        "/v1/responses/",
        "/claude-desktop/v1/responses",
    ] {
        let response = client
            .post(format!("http://{address}{path}"))
            .bearer_auth(&active.token)
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    let url = format!("http://{address}/v1/responses");
    let history = json!({"model":"fixture","previous_response_id":"resp_fixture","input":[{"type":"item_reference","id":"item_fixture"}]});
    assert_eq!(
        client.post(&url).bearer_auth(&active.token).json(&history).send().await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(received.recv().await.unwrap().1, history);
    for body in [
        json!({"previous_response_id":"foreign"}),
        json!({"input":[{"type":"item_reference","id":"foreign"}]}),
    ] {
        assert_eq!(
            client.post(&url).bearer_auth(&active.token).json(&body).send().await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }
    let other = listener
        .auth
        .issue_with_target(
            VerifiedRouteBinding { host: "other-host".into(), ..binding.clone() },
            Some(JsonTarget {
                #[cfg(target_os = "macos")]
                managed: None,
                url: target.clone(),
                authorization: "Bearer fixture-upstream".parse().unwrap(),
                transport: SharedTransport::new().unwrap(),
                request_options: Default::default(),
            }),
        )
        .unwrap();
    assert_eq!(
        client.post(&url).bearer_auth(&other.token).json(&history).send().await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert!(received.try_recv().is_err());
    listener.stop().await.unwrap();
    upstream_task.abort();
    assert!(upstream_task.await.unwrap_err().is_cancelled());
}
