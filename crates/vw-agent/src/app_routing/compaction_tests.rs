use crate::app_routing::{
    RoutingListener, SharedTransport, VerifiedRouteBinding,
    aggregation_routing::{AggregationModel, AggregationPublication, AggregationTarget},
    dispatch::JsonTarget,
};
use axum::{Json, Router, body::Body, http::StatusCode, response::Response, routing::post};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::{net::TcpListener, sync::mpsc};
use vw_agent_apps::providers::{
    live::commit::{AppState, Mode, ModeState},
    mode_state::StackState,
};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

#[tokio::test]
async fn aggregation_t25_aggregation_compaction_preserves_contract() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = upstream.local_addr().unwrap();
    let (record, mut requests) = mpsc::unbounded_channel();
    let router = Router::new().route(
        "/v1/chat/completions",
        post(move |headers: axum::http::HeaderMap, Json(body): Json<Value>| async move {
            let model = body["model"].clone();
            record.send((headers["authorization"].clone(), body)).unwrap();
            if model == "unavailable" {
                return Response::builder()
                    .status(500)
                    .body(Body::from("private upstream"))
                    .unwrap();
            }
            let frame = json!({"choices":[{"index":0,"delta":{"content":"摘要"}}]});
            let finish = json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}],
                "usage":{"prompt_tokens":3,"completion_tokens":2}});
            Response::builder()
                .header("content-type", "text/event-stream")
                .body(Body::from(format!(
                    "data: {frame}\r\n\r\ndata: {finish}\r\n\r\ndata: [DONE]\r\n\r\n"
                )))
                .unwrap()
        }),
    );
    let upstream_task = tokio::spawn(async move { axum::serve(upstream, router).await.unwrap() });
    let listener = RoutingListener::default();
    let local = listener.start(0).unwrap();
    let tenant = TenantScope::new("compaction", "fixture").unwrap();
    let binding = VerifiedRouteBinding::new(
        ProjectScope::new(tenant.clone(), "project".into()).unwrap(),
        "host".into(),
        AppId::Codex,
        "account".into(),
        1,
    );
    let revision = "revision-1";
    let publication = AggregationPublication {
        binding: binding.clone(),
        revision: revision.into(),
        state: AppState {
            mode: Some(ModeState {
                mode: Mode::Proxy,
                attached: true,
                proxy_route: Some("member".into()),
                contract: None,
            }),
            mode_tenant: Some(tenant.clone()),
            stack: StackState {
                enabled: true,
                members: ["member".into()].into(),
                keys: [("member".into(), "member".into())].into(),
            },
            stack_tenant: Some(tenant),
            ..Default::default()
        },
        models: ["wire-model", "unavailable"]
            .into_iter()
            .map(|upstream_model| AggregationModel {
                id: format!("ccs-member/{upstream_model}"),
                provider_id: "member".into(),
                upstream_model: upstream_model.into(),
            })
            .collect(),
        default: AggregationTarget {
            messages_api_key: false,
            binding,
            revision: revision.into(),
            provider_id: "member".into(),
            target: JsonTarget {
                url: format!("http://{address}/v1/chat/completions").parse().unwrap(),
                authorization: "Bearer upstream-fixture".parse().unwrap(),
                transport: SharedTransport::new().unwrap(),
                request_options: Default::default(),
            },
        },
        members: Vec::new(),
    };
    let mut token = String::new();
    let revoke = listener
        .publish_aggregation(publication, None, |address, issued| {
            assert_eq!(address, local);
            token = issued.into();
            Ok(())
        })
        .unwrap();
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    let url = format!("http://{local}/v1/responses");
    let send = |body: &Value| client.post(&url).bearer_auth(&token).json(body).send();
    let mut request = json!({"model":"ccs-member/wire-model","stream":true,"input":[
        {"role":"user","content":"remember this"},
        {"type":"compaction_trigger"}
    ]});
    assert_eq!(
        client.post(&url).json(&request).send().await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    assert!(requests.try_recv().is_err());

    let response = send(&request).await.unwrap().error_for_status().unwrap();
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    let events = events(&response.text().await.unwrap());
    let completed = events.last().unwrap();
    assert_eq!(completed["type"], "response.completed");
    assert_eq!(completed["response"]["model"], "ccs-member/wire-model");
    let output = completed["response"]["output"].as_array().unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(output[0]["type"], "compaction");
    assert_eq!(
        events.iter().filter(|event| event["type"] == "response.output_item.done").count(),
        1
    );
    let (credential, body) = requests.try_recv().unwrap();
    assert_eq!(credential, "Bearer upstream-fixture");
    assert_eq!(body["model"], "wire-model");
    assert!(body["messages"][1]["content"].as_str().unwrap().contains("CONTEXT CHECKPOINT"));

    let replay = json!({"model":"ccs-member/wire-model","stream":true,"input":[
        output[0],
        {"type":"compaction","encrypted_content":"foreign-fixture"},
        {"role":"user","content":"continue"}
    ]});
    let wire = send(&replay).await.unwrap().error_for_status().unwrap().text().await.unwrap();
    let replay_events = self::events(&wire);
    assert_eq!(replay_events.last().unwrap()["type"], "response.completed");
    let (_, body) = requests.try_recv().unwrap();
    assert!(body["messages"][0]["content"].as_str().unwrap().ends_with("\n摘要"));
    assert!(body["messages"][1]["content"].as_str().unwrap().contains("cannot read"));
    assert!(!body.to_string().contains("foreign-fixture"));

    request["model"] = "ccs-member/unavailable".into();
    let response = send(&request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(!response.text().await.unwrap().contains("private upstream"));
    assert_eq!(requests.try_recv().unwrap().1["model"], "unavailable");

    if let Some(binary) = std::env::var_os("SHIJU_T25_REAL_CODEX") {
        run_real_codex(&binary, local, &token).await;
        let (_, compact) = requests.try_recv().unwrap();
        assert_eq!(compact["model"], "wire-model");
        assert_eq!(compact["tool_choice"], "none");
        let compact_messages = compact["messages"].as_array().unwrap();
        assert!(
            compact_messages.last().unwrap()["content"]
                .as_str()
                .unwrap()
                .contains("CONTEXT CHECKPOINT")
        );
        let (_, replay) = requests.try_recv().unwrap();
        assert_eq!(replay["model"], "wire-model");
        assert!(replay["messages"].as_array().unwrap().iter().any(|message| {
            message["content"].as_str().is_some_and(|text| text.ends_with("\n摘要"))
        }));
    }

    listener.revoke_aggregation(&revoke).unwrap();
    assert_eq!(send(&replay).await.unwrap().status(), StatusCode::UNAUTHORIZED);
    assert!(requests.try_recv().is_err());
    listener.stop().await.unwrap();
    upstream_task.abort();
    assert!(upstream_task.await.unwrap_err().is_cancelled());
}

fn events(wire: &str) -> Vec<Value> {
    wire.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|data| serde_json::from_str(data).unwrap())
        .collect()
}

async fn run_real_codex(binary: &std::ffi::OsStr, address: std::net::SocketAddr, token: &str) {
    use std::process::Stdio;
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        process::Command,
        time::timeout,
    };

    let home = tempfile::tempdir().unwrap();
    let mut command = Command::new(binary);
    command
        .arg("app-server")
        .env_clear()
        .env("CODEX_HOME", home.path())
        .env("SHIJU_T25_ROUTE_TOKEN", token)
        .env("NO_PROXY", "127.0.0.1,localhost")
        .current_dir(home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    for name in ["PATH", "HOME", "TMPDIR", "LANG"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let base_url = format!("model_providers.fixture.base_url=\"http://{address}/v1\"");
    for config in [
        "model_provider=\"fixture\"",
        "model=\"ccs-member/wire-model\"",
        "web_search=\"disabled\"",
        "approval_policy=\"never\"",
        "sandbox_mode=\"read-only\"",
        "model_providers.fixture.name=\"OpenAI\"",
        &base_url,
        "model_providers.fixture.env_key=\"SHIJU_T25_ROUTE_TOKEN\"",
        "model_providers.fixture.wire_api=\"responses\"",
        "model_providers.fixture.requires_openai_auth=false",
        "model_providers.fixture.supports_websockets=false",
    ] {
        command.args(["-c", config]);
    }
    let mut child = command.spawn().unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap()).lines();
    async fn send(input: &mut tokio::process::ChildStdin, request: Value) {
        input.write_all(format!("{request}\n").as_bytes()).await.unwrap();
    }
    let mut next = async |field: &str, value: Value| {
        timeout(Duration::from_secs(30), async {
            loop {
                let line = output.next_line().await.unwrap().expect("Codex app-server exited");
                let message: Value = serde_json::from_str(&line).unwrap();
                if message[field] == value {
                    return message;
                }
            }
        })
        .await
        .expect("Codex app-server timed out")
    };
    send(
        &mut input,
        json!({"id":1,"method":"initialize","params":{
            "clientInfo":{"name":"shiju_test","title":"ShiJu Test","version":"0.1"},
            "capabilities":{"experimentalApi":true}
        }}),
    )
    .await;
    assert!(next("id", json!(1)).await.get("result").is_some());
    send(&mut input, json!({"method":"initialized"})).await;
    send(&mut input, json!({"id":2,"method":"thread/resume","params":{
        "threadId":"00000000-0000-4000-8000-000000000001","cwd":home.path(),"history":[
            {"type":"message","role":"user","content":[{"type":"input_text","text":"fixture context"}]}
        ]
    }})).await;
    let resumed = next("id", json!(2)).await;
    let thread = resumed["result"]["thread"]["id"].as_str().expect("Codex resume failed");
    send(&mut input, json!({"id":3,"method":"thread/compact/start","params":{"threadId":thread}}))
        .await;
    assert!(next("id", json!(3)).await.get("result").is_some());
    let completed = next("method", json!("turn/completed")).await;
    assert_eq!(completed["params"]["turn"]["status"], "completed");
    send(
        &mut input,
        json!({"id":4,"method":"turn/start","params":{
            "threadId":thread,"input":[{"type":"text","text":"continue","text_elements":[]}]
        }}),
    )
    .await;
    assert!(next("id", json!(4)).await.get("result").is_some());
    let completed = next("method", json!("turn/completed")).await;
    assert_eq!(completed["params"]["turn"]["status"], "completed");
    child.kill().await.unwrap();
}
