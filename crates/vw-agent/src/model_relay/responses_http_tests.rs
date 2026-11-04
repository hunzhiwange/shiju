use super::*;
use axum::{
    Router,
    body::{Body, Bytes},
    http::HeaderMap,
    response::Response,
    routing::post,
};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

struct EnvGuard(Option<std::ffi::OsString>, Option<std::ffi::OsString>);
impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            crate::env::remove("SHIJU_RELAY_RESPONSES_TEST_KEY");
            match self.1.take() {
                Some(v) => {
                    crate::env::set(
                        "SHIJU_RELAY_RESPONSES_TEST_KEY",
                        v.to_string_lossy().to_string(),
                    );
                    std::env::set_var("SHIJU_RELAY_RESPONSES_TEST_KEY", v);
                }
                None => std::env::remove_var("SHIJU_RELAY_RESPONSES_TEST_KEY"),
            }
            match self.0.take() {
                Some(v) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", v),
                None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
            }
        }
    }
}
fn reply(value: Value) -> Response {
    Response::builder()
        .header("content-type", "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}
fn sse(wire: String, wait: bool) -> Response {
    let chunks = wire
        .as_bytes()
        .chunks(7)
        .map(|v| Ok::<_, std::io::Error>(Bytes::copy_from_slice(v)))
        .collect::<Vec<_>>();
    let stream = futures_util::stream::iter(chunks);
    let body = if wait {
        Body::from_stream(stream.chain(futures_util::stream::pending()))
    } else {
        Body::from_stream(stream)
    };
    Response::builder().header("content-type", "text/event-stream").body(body).unwrap()
}
#[tokio::test]
async fn acp_model_source_relay_responses_http_roundtrip_stream_errors_cancel_and_ownership() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();
    let _env = EnvGuard(
        std::env::var_os("VIBEWINDOW_CONFIG_DIR"),
        std::env::var_os("SHIJU_RELAY_RESPONSES_TEST_KEY"),
    );
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", dir.path().join("config"));
        std::env::set_var("SHIJU_RELAY_RESPONSES_TEST_KEY", "fixture-secret");
        crate::env::set("SHIJU_RELAY_RESPONSES_TEST_KEY", "fixture-secret");
    }
    let captured = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let calls = captured.clone();
    let router = Router::new().route("/v1/chat/completions", post(move |headers: HeaderMap, bytes: Bytes| {
        let calls = calls.clone();
        async move {
            let body: Value = serde_json::from_slice(&bytes).unwrap();
            calls.lock().push((headers,body.clone()));
            let prompt = body["messages"].as_array().unwrap().last().unwrap()["content"].as_str().unwrap_or("").to_owned();
            if prompt == "error" { return Response::builder().status(500).body(Body::from("private upstream secret")).unwrap(); }
            if body["stream"] == true {
                let values = [
                    json!({"id":"chat-id","choices":[{"index":0,"delta":{"role":"assistant","content":"你"}}]}),
                    json!({"id":"chat-id","choices":[{"index":0,"delta":{"content":"好","tool_calls":[{"index":0,"id":"call-a","type":"function","function":{"name":"lookup","arguments":"{\"city\":"}},{"index":1,"id":"call-b","type":"function","function":{"name":"other","arguments":"{}"}}]}}]}),
                    json!({"id":"chat-id","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"杭州\"}"}}]},"finish_reason":"tool_calls"}]}),
                    json!({"id":"chat-id","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":3,"total_tokens":10}})
                ];
                let mut wire = values.iter().map(|v| format!("data: {v}\r\n\r\n")).collect::<String>();
                if prompt != "wait" && prompt != "truncated" { wire.push_str("data: [DONE]\r\n\r\n"); }
                return sse(wire, prompt == "wait");
            }
            reply(json!({"id":"chat-id","choices":[{"message":{"role":"assistant","content":"answer","tool_calls":[{"id":"call-a","type":"function","function":{"name":"lookup","arguments":"{\"city\":\"杭州\"}"}}]},"finish_reason":if prompt=="length" {"length"} else {"tool_calls"}}],"usage":{"prompt_tokens":7,"completion_tokens":3,"total_tokens":10}}))
        }
    }));
    let calls = captured.clone();
    let router=router.route("/v1/responses",post(move |headers:HeaderMap,bytes:Bytes| {
        let calls=calls.clone();
        async move {
            let body:Value=serde_json::from_slice(&bytes).unwrap();
            calls.lock().push((headers,body.clone()));
            let response=json!({"id":"resp-native","object":"response","model":"wire-model","status":"completed","previous_response_id":body.get("previous_response_id").cloned().unwrap_or(Value::Null),"output":[{"type":"reasoning","id":"rs-a","encrypted_content":"native-signature","summary":[]}],"usage":{"input_tokens":2,"output_tokens":1,"total_tokens":3},"vendor_extension":{"kept":true}});
            if body["stream"]==true {
                let frames=[json!({"type":"response.created","sequence_number":0,"response":{"id":"resp-native","object":"response","model":"wire-model","status":"in_progress","output":[]}}),json!({"type":"response.completed","sequence_number":1,"response":response})];
                sse(frames.iter().map(|v|format!("event: {}\ndata: {v}\n\n",v["type"].as_str().unwrap())).collect(),false)
            } else { reply(response) }
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let stop = CancellationToken::new();
    let stopping = stop.clone();
    let upstream = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(stopping.cancelled_owned())
            .await
            .unwrap();
    });
    let tenant = vw_api_types::id::TenantScope::new("responses", "one").unwrap();
    let root = dir.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let (project, _) = crate::project::from_directory(&tenant, &root).await.unwrap();
    crate::config::update_project_for_tenant(&tenant,&root,json!({"providers":{"relay-responses":{"api":format!("http://{address}/v1/chat/completions"),"env":["SHIJU_RELAY_RESPONSES_TEST_KEY"],"models":{"selected":{"id":"wire-model","status":"active","tool_call":true}}}}})).await.unwrap();
    let context = crate::project::instance::Context {
        directory: root.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project: project.clone(),
    };
    let identity = RelayIdentity {
        tenant: tenant.clone(),
        project: project.id,
        session: "one".into(),
        provider: "relay-responses".into(),
        model: "selected".into(),
        generation: 1,
    };
    let backend = Arc::new(ProviderRelayBackend::new().unwrap());
    let binding = RelayBinding::new(identity.clone(), Duration::from_secs(60)).unwrap();
    let server =
        RelayServer::start(binding.clone(), backend.clone(), context.clone()).await.unwrap();
    let url = format!("http://{}/v1/responses", server.address());
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut body = json!({"model":"selected","instructions":"Be concise","input":[{"role":"user","content":[{"type":"input_text","text":"hello"}]},{"type":"function_call","call_id":"call-a","name":"lookup","arguments":"{\"city\":\"杭州\"}"},{"type":"function_call_output","call_id":"call-a","output":"sunny"}],"tools":[{"type":"function","name":"lookup","parameters":{"type":"object"}}],"max_output_tokens":64,"store":false});
    let result =
        client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
    assert_eq!(result.status(), 200);
    let result: Value = result.json().await.unwrap();
    assert_eq!(result["model"], "selected");
    assert_eq!(result["output"][0]["content"][0]["text"], "answer");
    assert_eq!(result["output"][1]["call_id"], "call-a");
    assert_eq!(
        serde_json::from_str::<Value>(result["output"][1]["arguments"].as_str().unwrap()).unwrap(),
        json!({"city":"杭州"})
    );
    assert_eq!(result["usage"]["input_tokens"], 7);
    {
        let calls = captured.lock();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0["authorization"], "Bearer fixture-secret");
        assert_eq!(calls[0].1["model"], "wire-model");
        assert_eq!(
            calls[0].1["messages"][3],
            json!({"role":"tool","tool_call_id":"call-a","content":"sunny"})
        );
    }
    body["stream"] = true.into();
    let wire = client
        .post(&url)
        .bearer_auth(binding.bearer_token())
        .json(&body)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let events = parse_events(&wire);
    assert_eq!(events[0]["type"], "response.created");
    assert_eq!(events.last().unwrap()["type"], "response.completed");
    for (n, event) in events.iter().enumerate() {
        assert_eq!(event["sequence_number"], n);
    }
    assert_eq!(
        events
            .iter()
            .filter(|e| e["type"] == "response.output_text.delta")
            .map(|e| e["delta"].as_str().unwrap())
            .collect::<String>(),
        "你好"
    );
    let completed = &events.last().unwrap()["response"];
    assert_eq!(completed["output"].as_array().unwrap().len(), 3);
    assert_eq!(completed["output"][1]["arguments"], "{\"city\":\"杭州\"}");
    assert_eq!(completed["output"][2]["call_id"], "call-b");
    assert_eq!(completed["usage"]["total_tokens"], 10);
    assert!(!wire.contains("wire-model"));
    for bad in [
        json!({"previous_response_id":"resp-another-session"}),
        json!({"tools":[{"type":"web_search"}]}),
        json!({"input":[{"type":"reasoning","encrypted_content":"signature"}]}),
        json!({"input":[{"role":"user","content":[{"type":"input_image","image_url":"https://example.invalid"}]}]}),
    ] {
        let mut request = body.clone();
        request.as_object_mut().unwrap().extend(bad.as_object().unwrap().clone());
        let response = client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&request)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 422);
        assert!(response.json::<Value>().await.unwrap()["error"]["message"].is_string());
    }
    assert_eq!(captured.lock().len(), 2);
    body["input"] = json!("error");
    body["stream"] = false.into();
    let response =
        client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
    assert_eq!(response.status(), 502);
    let error = response.text().await.unwrap();
    assert!(!error.contains("private"));
    assert!(error.contains("error"));
    body["input"] = json!("length");
    let result: Value = client
        .post(&url)
        .bearer_auth(binding.bearer_token())
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(result["status"], "incomplete");
    assert_eq!(result["incomplete_details"]["reason"], "max_output_tokens");
    body["input"] = json!("truncated");
    body["stream"] = true.into();
    let wire = client
        .post(&url)
        .bearer_auth(binding.bearer_token())
        .json(&body)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(parse_events(&wire).last().unwrap()["type"], "response.failed");
    // Same-protocol reasoning and extensions survive; upstream response references stay capability-local.
    crate::config::update_project_for_tenant(&tenant,&root,json!({"providers":{"relay-responses":{"api":format!("http://{address}/v1/responses"),"adapter":"openai-responses","models":{"selected":{"reasoning":true}}}}})).await.unwrap();
    body = json!({"model":"selected","input":"hello","store":true,"include":["reasoning.encrypted_content"]});
    let result: Value = client
        .post(&url)
        .bearer_auth(binding.bearer_token())
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(result["output"][0]["encrypted_content"], "native-signature");
    assert_eq!(result["vendor_extension"]["kept"], true);
    assert_eq!(result["model"], "selected");
    let reference = result["id"].as_str().unwrap().to_owned();
    let item_request = json!({"model":"selected","input":[{"type":"item_reference","id":"rs-a"}]});
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&item_request)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    let raw_reference =
        json!({"model":"selected","input":"hello","previous_response_id":"resp-native"});
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&raw_reference)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    body["previous_response_id"] = reference.clone().into();
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    assert_eq!(captured.lock().last().unwrap().1["previous_response_id"], "resp-native");
    let other_tenant = vw_api_types::id::TenantScope::new("responses", "two").unwrap();
    let other_root = dir.path().join("other-project");
    std::fs::create_dir(&other_root).unwrap();
    let (other_project, _) =
        crate::project::from_directory(&other_tenant, &other_root).await.unwrap();
    crate::config::update_project_for_tenant(&other_tenant,&other_root,json!({"providers":{"relay-responses":{"api":format!("http://{address}/v1/responses"),"adapter":"openai-responses","env":["SHIJU_RELAY_RESPONSES_TEST_KEY"],"models":{"selected":{"id":"wire-model","status":"active","tool_call":true,"reasoning":true}}}}})).await.unwrap();
    let other_context = crate::project::instance::Context {
        directory: other_root.to_string_lossy().into(),
        worktree: other_project.worktree.clone(),
        project: other_project.clone(),
    };
    let mut other_id = identity.clone();
    other_id.tenant = other_tenant;
    other_id.project = other_project.id;
    let other = RelayBinding::new(other_id, Duration::from_secs(60)).unwrap();
    let other_server =
        RelayServer::start(other.clone(), backend.clone(), other_context).await.unwrap();
    let other_url = format!("http://{}/v1/responses", other_server.address());
    assert_eq!(
        client
            .post(&other_url)
            .bearer_auth(other.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        client
            .post(&other_url)
            .bearer_auth(other.bearer_token())
            .json(&item_request)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        client
            .post(&other_url)
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    body["stream"] = true.into();
    let wire = client
        .post(&url)
        .bearer_auth(binding.bearer_token())
        .json(&body)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!wire.contains("wire-model"));
    assert!(wire.contains("native-signature"));
    assert_eq!(parse_events(&wire).last().unwrap()["type"], "response.completed");
    // Probe the real relay from a subprocess, without claiming any supplier CLI acceptance.
    body["stream"] = false.into();
    use tokio::io::AsyncWriteExt;
    let mut child=tokio::process::Command::new("python3").arg("-c").arg("import sys,json,urllib.request; v=json.load(sys.stdin); r=urllib.request.urlopen(urllib.request.Request(v['url'],data=json.dumps(v['body']).encode(),headers={'Authorization':'Bearer '+v['token'],'Content-Type':'application/json'})); print(json.load(r)['model'])").stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            json!({"url":url,"token":binding.bearer_token(),"body":body}).to_string().as_bytes(),
        )
        .await
        .unwrap();
    let output = child.wait_with_output().await.unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "selected");
    crate::config::update_project_for_tenant(&tenant,&root,json!({"providers":{"relay-responses":{"api":format!("http://{address}/v1/chat/completions"),"adapter":"openai"}}})).await.unwrap();
    body = json!({"model":"selected","input":"wait","stream":true});
    let response =
        client.post(&url).bearer_auth(binding.bearer_token()).json(&body).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let mut stream = response.bytes_stream();
    assert!(stream.next().await.unwrap().unwrap().starts_with(b"event: response.created"));
    binding.revoke();
    tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(chunk) = stream.next().await {
            if chunk.is_err() {
                break;
            }
        }
    })
    .await
    .unwrap();
    drop(stream);
    server.shutdown().await.unwrap();
    other_server.shutdown().await.unwrap();
    stop.cancel();
    upstream.await.unwrap();
}
fn parse_events(wire: &str) -> Vec<Value> {
    wire.lines()
        .filter_map(|s| s.strip_prefix("data: "))
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
