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
            crate::env::remove("SHIJU_RELAY_GEMINI_TEST_KEY");
            match self.1.take() {
                Some(v) => {
                    crate::env::set("SHIJU_RELAY_GEMINI_TEST_KEY", v.to_string_lossy().to_string());
                    std::env::set_var("SHIJU_RELAY_GEMINI_TEST_KEY", v);
                }
                None => std::env::remove_var("SHIJU_RELAY_GEMINI_TEST_KEY"),
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
async fn model_protocol_018_gemini_http_tools_stream_metadata_errors_and_cancel() {
    http(false).await;
}
#[tokio::test]
#[ignore = "requires installed real Gemini CLI; run explicitly for supplier evidence"]
async fn acp_model_source_relay_gemini_real_cli() {
    http(true).await;
}
async fn http(run_cli: bool) {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let dir = tempfile::tempdir().unwrap();
    let _env = EnvGuard(
        std::env::var_os("VIBEWINDOW_CONFIG_DIR"),
        std::env::var_os("SHIJU_RELAY_GEMINI_TEST_KEY"),
    );
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", dir.path().join("config"));
        std::env::set_var("SHIJU_RELAY_GEMINI_TEST_KEY", "fixture-secret");
        crate::env::set("SHIJU_RELAY_GEMINI_TEST_KEY", "fixture-secret");
    }
    let captured = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let calls = captured.clone();
    let router = Router::new().route("/v1/chat/completions", post(move |headers: HeaderMap, bytes: Bytes| {
        let calls = calls.clone();
        async move {
            let body: Value = serde_json::from_slice(&bytes).unwrap();
            calls.lock().push((headers,body.clone()));
            let prompt=body["messages"].as_array().unwrap().last().unwrap()["content"].as_str().unwrap_or("").to_owned();
            if prompt=="error" { return Response::builder().status(500).body(Body::from("private credential")).unwrap(); }
            if body["stream"]==true {
                let values=[json!({"id":"chat","choices":[{"index":0,"delta":{"role":"assistant","content":"你"}}]}),json!({"id":"chat","choices":[{"index":0,"delta":{"content":"好","tool_calls":[{"index":0,"id":"a","type":"function","function":{"name":"lookup","arguments":"{\"city\":"}}]}}]}),json!({"id":"chat","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"杭州\"}"}}]},"finish_reason":"tool_calls"}]}),json!({"id":"chat","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":3,"total_tokens":10}})];
                let mut wire=values.iter().map(|v|format!("data: {v}\r\n\r\n")).collect::<String>();
                if prompt!="wait" && prompt!="truncated" {wire.push_str("data: [DONE]\r\n\r\n");}
                return sse(wire,prompt=="wait");
            }
            reply(json!({"choices":[{"index":0,"message":{"role":"assistant","content":"{\"weather\":\"sunny\"}","tool_calls":[{"id":"a","type":"function","function":{"name":"lookup","arguments":"{\"city\":\"杭州\"}"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":7,"completion_tokens":3,"total_tokens":10}}))
        }
    }));
    let native_calls = captured.clone();
    let router=router.route("/v1beta/models/wire-model:generateContent",post(move |headers:HeaderMap,bytes:Bytes|{
        let calls=native_calls.clone();async move {
            let body:Value=serde_json::from_slice(&bytes).unwrap();calls.lock().push((headers,body));
            reply(json!({"candidates":[{"index":0,"content":{"role":"model","parts":[{"text":"TASK8_REAL_CLI_OK","thoughtSignature":"native-signature"}]},"finishReason":"STOP"}],"modelVersion":"wire-model","responseId":"upstream-ref","usageMetadata":{"promptTokenCount":7,"candidatesTokenCount":3,"totalTokenCount":10},"vendorExtension":true}))
        }
    }));
    let native_calls = captured.clone();
    let router=router.route("/v1beta/models/wire-model:streamGenerateContent",post(move |headers:HeaderMap,bytes:Bytes|{
        let calls=native_calls.clone();async move {
            let body:Value=serde_json::from_slice(&bytes).unwrap();calls.lock().push((headers,body));
            sse(format!("data: {}\n\n",json!({"candidates":[{"index":0,"content":{"role":"model","parts":[{"text":"TASK8_REAL_CLI_OK","thoughtSignature":"native-signature"}]},"finishReason":"STOP"}],"modelVersion":"wire-model","usageMetadata":{"promptTokenCount":7,"candidatesTokenCount":3,"totalTokenCount":10}})),false)
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
    let tenant = vw_api_types::id::TenantScope::new("gemini", "one").unwrap();
    let root = dir.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let (project, _) = crate::project::from_directory(&tenant, &root).await.unwrap();
    crate::config::update_project_for_tenant(&tenant,&root,json!({"providers":{"relay-gemini":{"api":format!("http://{address}/v1/chat/completions"),"env":["SHIJU_RELAY_GEMINI_TEST_KEY"],"models":{"selected":{"id":"wire-model","status":"active","tool_call":true},"selected/variant":{"id":"wire-model","status":"active","tool_call":true}}}}})).await.unwrap();
    let context = crate::project::instance::Context {
        directory: root.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project: project.clone(),
    };
    let identity = RelayIdentity {
        tenant: tenant.clone(),
        project: project.id,
        session: "one".into(),
        provider: "relay-gemini".into(),
        model: "selected".into(),
        generation: 1,
    };
    let backend = Arc::new(ProviderRelayBackend::new().unwrap());
    let binding = RelayBinding::new(identity.clone(), Duration::from_secs(120)).unwrap();
    let server =
        RelayServer::start(binding.clone(), backend.clone(), context.clone()).await.unwrap();
    let base = format!("http://{}/v1beta/models", server.address());
    let url = format!("{base}/selected:generateContent");
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut body = json!({"systemInstruction":{"parts":[{"text":"concise"}]},"contents":[{"role":"user","parts":[{"text":"hello"}]},{"role":"model","parts":[{"functionCall":{"id":"a","name":"lookup","args":{"city":"杭州"}}},{"text":"checking weather"}]},{"role":"user","parts":[{"functionResponse":{"id":"a","name":"lookup","response":{"result":"sunny"}}}]}],"tools":[{"functionDeclarations":[{"name":"lookup","parameters":{"type":"OBJECT","properties":{"city":{"type":"STRING"}}}}]}],"generationConfig":{"maxOutputTokens":64}});
    let response = client
        .post(&url)
        .header("x-goog-api-key", binding.bearer_token())
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let result: Value = response.json().await.unwrap();
    assert_eq!(
        result["candidates"][0]["content"]["parts"][1]["functionCall"],
        json!({"id":"a","name":"lookup","args":{"city":"杭州"}})
    );
    assert_eq!(result["usageMetadata"]["totalTokenCount"], 10);
    assert_eq!(result["modelVersion"], "selected");
    {
        let calls = captured.lock();
        assert_eq!(calls[0].0["authorization"], "Bearer fixture-secret");
        assert_eq!(calls[0].1["model"], "wire-model");
        assert_eq!(
            calls[0].1["messages"][3],
            json!({"role":"tool","tool_call_id":"a","content":"{\"result\":\"sunny\"}"})
        );
        assert_eq!(
            calls[0].1["tools"][0]["function"]["parameters"]["properties"]["city"]["type"],
            "string"
        );
    }
    let mut structured = body.clone();
    structured["generationConfig"] = json!({"responseMimeType":"application/json","responseSchema":{"type":"OBJECT","properties":{"weather":{"type":"STRING"}}}});
    let response = client
        .post(&url)
        .bearer_auth(binding.bearer_token())
        .json(&structured)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let structured_result: Value = response.json().await.unwrap();
    assert_eq!(structured_result["candidates"][0]["finishReason"], "STOP");
    assert_eq!(
        captured.lock().last().unwrap().1["response_format"],
        json!({"type":"json_schema","json_schema":{"name":"response","schema":{"type":"object","properties":{"weather":{"type":"string"}}}}})
    );
    let meta: Value = client
        .get(&base)
        .bearer_auth(binding.bearer_token())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(meta["models"].as_array().unwrap().len(), 1);
    assert_eq!(meta["models"][0]["name"], "models/selected");
    assert_eq!(
        client
            .get(format!("{base}/other"))
            .bearer_auth(binding.bearer_token())
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        client
            .post(&url)
            .header("x-goog-api-key", "wrong")
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let stream_url = format!("{base}/selected:streamGenerateContent?alt=sse");
    let wire = client
        .post(&stream_url)
        .bearer_auth(binding.bearer_token())
        .json(&body)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let frames = events(&wire);
    assert_eq!(
        frames
            .iter()
            .filter_map(|v| v["candidates"][0]["content"]["parts"][0]["text"].as_str())
            .collect::<String>(),
        "你好"
    );
    assert_eq!(frames.last().unwrap()["candidates"][0]["finishReason"], "STOP");
    assert_eq!(frames.last().unwrap()["usageMetadata"]["totalTokenCount"], 10);
    assert!(wire.contains("杭州"));
    assert!(!wire.contains("wire-model"));
    let before_rejections = captured.lock().len();
    for bad in [
        json!({"cachedContent":"tenants/other"}),
        json!({"generationConfig":{"thinkingConfig":{"thinkingBudget":1024}}}),
        json!({"tools":[{"googleSearch":{}}]}),
        json!({"contents":[{"role":"user","parts":[{"text":"x","thoughtSignature":"secret"}]}]}),
        json!({"contents":[{"role":"user","parts":[{"inlineData":{"mimeType":"image/png","data":"AAAA"}}]}]}),
    ] {
        let mut request = body.clone();
        request.as_object_mut().unwrap().extend(bad.as_object().unwrap().clone());
        let result = client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&request)
            .send()
            .await
            .unwrap();
        assert_eq!(result.status(), 422);
        assert_eq!(result.json::<Value>().await.unwrap()["error"]["status"], "INVALID_ARGUMENT");
    }
    assert_eq!(captured.lock().len(), before_rejections);
    let mut slash_id = identity.clone();
    slash_id.model = "selected/variant".into();
    let slash_binding = RelayBinding::new(slash_id, Duration::from_secs(60)).unwrap();
    let slash_server =
        RelayServer::start(slash_binding.clone(), backend.clone(), context.clone()).await.unwrap();
    let response: Value = client
        .post(format!(
            "http://{}/v1beta/models/selected/variant:generateContent",
            slash_server.address()
        ))
        .bearer_auth(slash_binding.bearer_token())
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(response["modelVersion"], "selected/variant");
    slash_server.shutdown().await.unwrap();
    // Native protocol preserves signatures, tool declarations and provider extensions.
    crate::config::update_project_for_tenant(&tenant,&root,json!({"providers":{"relay-gemini":{"api":format!("http://{address}/v1beta"),"adapter":"google","models":{"selected":{"reasoning":true,"temperature":true,"limit":{"context":1000000,"output":65536}}}}}})).await.unwrap();
    for extra in [
        json!({"cachedContent":"other-tenant/cache"}),
        json!({"continuationToken":"other-tenant/token"}),
        json!({"contents":[{"role":"user","parts":[{"fileData":{"mimeType":"image/png","fileUri":"https://generativelanguage.googleapis.com/v1beta/files/other-tenant"}}]}]}),
    ] {
        let mut invalid = body.clone();
        invalid.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        assert_eq!(
            client
                .post(&url)
                .bearer_auth(binding.bearer_token())
                .json(&invalid)
                .send()
                .await
                .unwrap()
                .status(),
            422
        );
    }
    let mut native_body = body.clone();
    native_body["contents"][1]["parts"][0]["thoughtSignature"] = json!("native-input");
    let result: Value = client
        .post(&url)
        .bearer_auth(binding.bearer_token())
        .json(&native_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        result["candidates"][0]["content"]["parts"][0]["thoughtSignature"],
        "native-signature"
    );
    assert_eq!(result["vendorExtension"], true);
    assert_eq!(result["modelVersion"], "selected");
    assert!(result.get("responseId").is_none());
    {
        let calls = captured.lock();
        let (headers, request) = calls.last().unwrap();
        assert_eq!(headers["x-goog-api-key"], "fixture-secret");
        assert!(headers.get("authorization").is_none());
        assert_eq!(request["contents"][1]["parts"][0]["thoughtSignature"], "native-input");
        assert!(request.get("model").is_none());
    }
    let wire = client
        .post(&stream_url)
        .bearer_auth(binding.bearer_token())
        .json(&native_body)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(wire.contains("native-signature"));
    assert!(!wire.contains("wire-model"));
    assert!(events(&wire).last().unwrap().get("error").is_none());
    // A signature or thinking request must not reach a model lacking that capability.
    crate::config::update_project_for_tenant(
        &tenant,
        &root,
        json!({"providers":{"relay-gemini":{"models":{"selected":{"reasoning":false}}}}}),
    )
    .await
    .unwrap();
    let before_rejections = captured.lock().len();
    for extra in [
        json!({"contents":native_body["contents"]}),
        json!({"generationConfig":{"thinkingConfig":{"thinkingBudget":1024}}}),
        json!({"contents":[{"role":"user","parts":[{"inlineData":{"mimeType":"application/pdf","data":"AAAA"}}]}]}),
    ] {
        let mut invalid = body.clone();
        invalid.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        assert_eq!(
            client
                .post(&url)
                .bearer_auth(binding.bearer_token())
                .json(&invalid)
                .send()
                .await
                .unwrap()
                .status(),
            422
        );
    }
    assert_eq!(captured.lock().len(), before_rejections);
    crate::config::update_project_for_tenant(&tenant,&root,json!({"providers":{"relay-gemini":{"models":{"selected":{"reasoning":true,"modalities":{"input":["text","image","audio","video","pdf"],"output":["text"]}}}}}})).await.unwrap();
    for mime in ["image/png", "audio/wav", "video/mp4", "application/pdf"] {
        let mut media = native_body.clone();
        media["contents"][0]["parts"] = json!([{"inlineData":{"mimeType":mime,"data":"AAAA"}}]);
        media["generationConfig"]["thinkingConfig"] = json!({"thinkingBudget":1024});
        let response = client
            .post(&url)
            .bearer_auth(binding.bearer_token())
            .json(&media)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200, "{mime}");
        assert_eq!(
            response.json::<Value>().await.unwrap()["candidates"][0]["finishReason"],
            "STOP"
        );
        let calls = captured.lock();
        let (headers, request) = calls.last().unwrap();
        assert_eq!(headers["x-goog-api-key"], "fixture-secret");
        assert_eq!(request["contents"], media["contents"]);
        assert_eq!(request["generationConfig"], media["generationConfig"]);
    }
    let mut other_id = identity.clone();
    other_id.tenant = vw_api_types::id::TenantScope::new("gemini", "two").unwrap();
    let other = RelayBinding::new(other_id, Duration::from_secs(60)).unwrap();
    let other_server =
        RelayServer::start(other.clone(), backend.clone(), context.clone()).await.unwrap();
    assert_eq!(
        client
            .post(format!(
                "http://{}/v1beta/models/selected:generateContent",
                other_server.address()
            ))
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        client
            .post(format!(
                "http://{}/v1beta/models/selected:generateContent",
                other_server.address()
            ))
            .bearer_auth(other.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    other_server.shutdown().await.unwrap();
    if run_cli {
        let home = dir.path().join("cli-home");
        std::fs::create_dir_all(home.join(".gemini")).unwrap();
        std::fs::write(home.join(".gemini/settings.json"),json!({"security":{"auth":{"selectedType":"gemini-api-key"}},"telemetry":{"enabled":false},"general":{"enableAutoUpdate":false},"privacy":{"usageStatisticsEnabled":false}}).to_string()).unwrap();
        let cli_root = dir.path().join("cli-project");
        std::fs::create_dir(&cli_root).unwrap();
        let before = captured.lock().len();
        let mut command = tokio::process::Command::new("gemini");
        command
            .env_clear()
            .env("PATH", "/opt/homebrew/bin:/usr/bin:/bin")
            .env("HOME", &home)
            .env("GEMINI_CLI_HOME", &home)
            .env("GEMINI_CLI_SYSTEM_SETTINGS_PATH", home.join("absent.json"))
            .env("GEMINI_API_KEY", binding.bearer_token())
            .env("GOOGLE_GEMINI_BASE_URL", format!("http://{}", server.address()))
            .env("GOOGLE_GENAI_API_VERSION", "v1beta")
            .env("NO_PROXY", "127.0.0.1,localhost")
            .env("CI", "true")
            .current_dir(&cli_root)
            .args([
                "--skip-trust",
                "-m",
                "selected",
                "-p",
                "Reply with a short marker. Do not use tools.",
                "-o",
                "json",
            ])
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true);
        let output = tokio::time::timeout(Duration::from_secs(60), command.output())
            .await
            .expect("real CLI timeout")
            .unwrap();
        let stdout =
            String::from_utf8_lossy(&output.stdout).replace(binding.bearer_token(), "[REDACTED]");
        let stderr =
            String::from_utf8_lossy(&output.stderr).replace(binding.bearer_token(), "[REDACTED]");
        assert!(output.status.success(), "real Gemini CLI failed: {stdout} {stderr}");
        assert!(stdout.contains("TASK8_REAL_CLI_OK"), "{stdout} {stderr}");
        let calls = captured.lock();
        assert!(calls.len() > before);
        assert!(
            calls[before..]
                .iter()
                .any(|(_, b)| b["contents"].to_string().contains("Reply with a short marker"))
        );
        println!(
            "real Gemini CLI marker verified through ProviderRelayBackend; upstream requests={}",
            calls.len() - before
        );
    }
    crate::config::update_project_for_tenant(&tenant,&root,json!({"providers":{"relay-gemini":{"api":format!("http://{address}/v1/chat/completions"),"adapter":"openai"}}})).await.unwrap();
    for prompt in ["error", "truncated", "wait"] {
        body["contents"] = json!([{"role":"user","parts":[{"text":prompt}]}]);
        let response = client
            .post(if prompt == "error" { &url } else { &stream_url })
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap();
        if prompt == "error" {
            assert_eq!(response.status(), 502);
            let error = response.text().await.unwrap();
            assert!(!error.contains("private"));
            assert!(error.contains("error"));
        } else if prompt == "truncated" {
            let wire = response.text().await.unwrap();
            assert!(events(&wire).last().unwrap().get("error").is_some());
        } else {
            let mut stream = response.bytes_stream();
            assert!(stream.next().await.unwrap().is_ok());
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
        }
    }
    server.shutdown().await.unwrap();
    stop.cancel();
    upstream.await.unwrap();
}
fn events(wire: &str) -> Vec<Value> {
    wire.lines()
        .filter_map(|s| s.strip_prefix("data: "))
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}

#[test]
fn acp_model_source_relay_gemini_rejects_lossy_state_and_ambiguous_tools() {
    use super::gemini::{from_chat, to_chat};
    for part in [
        json!({"text":"x","thought":true}),
        json!({"text":"x","thoughtSignature":"signature"}),
        json!({"fileData":{"fileUri":"private"}}),
        json!({"executableCode":{"code":"x"}}),
    ] {
        assert!(
            to_chat(&json!({"model":"selected","contents":[{"role":"user","parts":[part]}]}))
                .is_err()
        );
    }
    assert!(to_chat(&json!({"model":"selected","contents":[{"role":"model","parts":[{"functionCall":{"name":"f","args":{}}},{"functionCall":{"name":"f","args":{}}}]},{"role":"user","parts":[{"functionResponse":{"name":"f","response":{}}}]}]})).is_err());
    let base = json!({"choices":[{"message":{"role":"assistant","content":"x"},"finish_reason":"length"}],"usage":{"prompt_tokens":3,"completion_tokens":2,"total_tokens":5}});
    assert_eq!(
        from_chat(&base, "selected").unwrap()["candidates"][0]["finishReason"],
        "MAX_TOKENS"
    );
    for extra in [
        json!({"reasoning_content":"private"}),
        json!({"content":[{"type":"image_url"}]}),
        json!({"annotations":[{"url":"private"}]}),
        json!({"refusal":"no"}),
    ] {
        let mut bad = base.clone();
        bad["choices"][0]["message"]
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert!(from_chat(&bad, "selected").is_err());
    }
}

#[test]
fn acp_model_source_relay_gemini_matches_idless_tools_and_preserves_reasoning_usage() {
    let request = json!({"model":"selected","contents":[{"role":"model","parts":[{"functionCall":{"name":"f"}}]},{"role":"user","parts":[{"functionResponse":{"name":"f","response":{"ok":true}}}]}]});
    let chat = super::gemini::to_chat(&request).unwrap();
    assert_eq!(chat["messages"][0]["tool_calls"][0]["id"], chat["messages"][1]["tool_call_id"]);
    assert_eq!(chat["messages"][0]["tool_calls"][0]["function"]["arguments"], "{}");
    let response=super::gemini::from_chat(&json!({"choices":[{"message":{"role":"assistant","content":"x"},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":5,"total_tokens":8,"prompt_tokens_details":{"cached_tokens":2},"completion_tokens_details":{"reasoning_tokens":2}}}),"selected").unwrap();
    assert_eq!(
        response["usageMetadata"],
        json!({"promptTokenCount":3,"candidatesTokenCount":3,"totalTokenCount":8,"cachedContentTokenCount":2,"thoughtsTokenCount":2})
    );
}

#[test]
fn model_protocol_018_gemini_native_requirements_are_selected_explicitly() {
    use vw_provider_resolver::protocol_routes::RouteCapability as C;
    for (extra, capability) in [
        (json!({"generationConfig":{"thinkingConfig":{"thinkingBudget":1024}}}), C::Thinking),
        (json!({"generationConfig":{"responseMimeType":"application/json"}}), C::StructuredOutput),
        (
            json!({"contents":[{"role":"model","parts":[{"text":"thought","thoughtSignature":"signed"}]}]}),
            C::ThinkingReplay,
        ),
        (
            json!({"contents":[{"parts":[{"inlineData":{"mimeType":"image/png","data":"AAAA"}}]}]}),
            C::Image,
        ),
        (
            json!({"contents":[{"parts":[{"inlineData":{"mimeType":"application/pdf","data":"AAAA"}}]}]}),
            C::Pdf,
        ),
        (
            json!({"contents":[{"role":"model","parts":[{"functionCall":{"name":"lookup","args":{}}}]}]}),
            C::Tools,
        ),
    ] {
        assert!(super::routing::required(&extra, false).contains(&capability), "{extra}");
    }
}

#[test]
fn model_protocol_018_gemini_tool_turn_preserves_text_after_call() {
    let body = json!({"model":"selected","contents":[
        {"role":"model","parts":[{"functionCall":{"id":"one","name":"lookup","args":{}}},{"text":"checking"}]},
        {"role":"user","parts":[{"functionResponse":{"id":"one","name":"lookup","response":{"ok":true}}}]}
    ]});
    let chat = super::gemini::to_chat(&body).unwrap();
    assert_eq!(chat["messages"][0]["content"], "checking");
    assert_eq!(chat["messages"][1]["tool_call_id"], "one");
    let mut unfinished = body;
    unfinished["contents"][1] =
        json!({"role":"model","parts":[{"functionCall":{"id":"two","name":"lookup","args":{}}}]});
    assert!(super::gemini::to_chat(&unfinished).is_err());
}
