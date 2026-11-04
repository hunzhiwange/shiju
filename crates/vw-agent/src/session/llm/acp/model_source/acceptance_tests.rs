//! Explicit external acceptance entries. The fixture is only the model upstream.
use crate::{project::instance, provider::provider, session::llm::StreamEvent};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

fn accepted(evidence: &Value, mode: &str) -> bool {
    let Some(requests) = evidence["requests"].as_array() else { return false };
    let primary = requests.iter().any(|r| r["model"] == "acp-fixture-primary");
    let switched = requests.iter().any(|r| r["model"] == "acp-fixture-switched");
    evidence["errors"] == 0
        && requests.len() >= 4
        && primary
        && (mode == "cli-local" || switched)
        && requests.iter().all(|r| {
            r["authenticated"] == true
                && r["route"] == "/v1/chat/completions"
                && (r["model"] == "acp-fixture-primary"
                    || (mode == "session-model" && r["model"] == "acp-fixture-switched"))
        })
        && requests.iter().any(|r| r["tool_results"].as_u64().unwrap_or(0) >= 2)
}

#[test]
fn acp_model_source_harness_contract_no_model_request_is_failure() {
    assert!(!accepted(&json!({"handshake": true, "requests": [], "errors": 0}), "session-model"));
}

#[test]
fn acp_model_source_harness_contract_handshake_is_not_model_success() {
    assert!(!accepted(&json!({"handshake": true, "model": "selected"}), "session-model"));
}

#[test]
fn acp_model_source_harness_contract_forged_model_is_failure() {
    assert!(!accepted(
        &json!({"handshake": true, "requests": [{"model": "selected"}], "errors": 0}),
        "session-model"
    ));
}

#[test]
fn acp_model_source_harness_contract_rejects_unauthenticated_or_wrong_model() {
    let request = json!({"model":"acp-fixture-primary", "authenticated":true,
                         "route":"/v1/chat/completions", "tool_results":2});
    let mut evidence =
        json!({"errors":0, "requests":[request.clone(),request.clone(),request.clone(),request]});
    assert!(accepted(&evidence, "cli-local"));
    assert!(!accepted(&evidence, "session-model"));
    evidence["requests"][3]["model"] = json!("acp-fixture-switched");
    assert!(accepted(&evidence, "session-model"));
    evidence["requests"][0]["authenticated"] = json!(false);
    assert!(!accepted(&evidence, "session-model"));
}

async fn evidence(input: &Value) -> Value {
    crate::model_relay::upstream_http_client()
        .unwrap()
        .get(format!("{}/_evidence", input["fixture"].as_str().unwrap()))
        .bearer_auth(input["control"].as_str().unwrap())
        .send()
        .await
        .expect("owned fixture unavailable")
        .error_for_status()
        .expect("owned fixture rejected control authentication")
        .json()
        .await
        .expect("invalid fixture evidence")
}

async fn request(
    input: &Value,
    model: &provider::Model,
    info: &provider::Info,
    prompt: &str,
    cancel: bool,
) -> bool {
    let (tx, rx) = tokio::sync::watch::channel(false);
    let before = evidence(input).await["requests"].as_array().unwrap().len();
    let fixture = input.clone();
    let cancellation = async {
        if cancel {
            for _ in 0..200 {
                let count = evidence(&fixture).await["requests"].as_array().unwrap().len();
                if count > before {
                    tx.send(true).unwrap();
                    return;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            panic!("cancellation never reached a model request");
        }
    };
    let options = json!({
        "acp_agent": input["agent"],
        "acp_model_source": if input["mode"] == "cli-local" { "cli_local" } else { "session_model" },
        "cwd": input["root"],
        "acp_close_client_after_request": true,
    });
    let messages = json!([{"role":"user", "content":prompt}]);
    let mut output = String::new();
    let mut done = false;
    let mut on_event = |event| match event {
        StreamEvent::Delta(text) => output.push_str(&text),
        StreamEvent::Done { .. } => done = true,
        _ => {}
    };
    let session = format!("real-cli-offline-{}", input["challenge"].as_str().unwrap());
    let production = super::super::do_stream_request_acp(
        model,
        info,
        None,
        &options,
        &messages,
        &session,
        Some(&rx),
        &mut on_event,
    );
    let (result, ()) = tokio::time::timeout(Duration::from_secs(60), async {
        tokio::join!(production, cancellation)
    })
    .await
    .expect("production ACP request timed out");
    if cancel {
        matches!(result, Err(crate::session::llm::Error::Aborted))
    } else {
        result.is_ok()
            && done
            && output.contains("FIXTURE_MODEL_REPLY")
            && output.contains(input["challenge"].as_str().unwrap())
            && evidence(input).await["requests"].as_array().unwrap().len() > before
    }
}

async fn run(mode: &str) {
    let manifest = std::env::var("VW_ACP_ACCEPTANCE_MANIFEST")
        .expect("explicit offline manifest required; absence is a failure, never a skip");
    let input: Value = serde_json::from_slice(&std::fs::read(manifest).unwrap()).unwrap();
    assert_eq!(input["mode"], mode, "acceptance source mismatch");
    let fixture = reqwest::Url::parse(input["fixture"].as_str().unwrap()).unwrap();
    assert_eq!(fixture.scheme(), "http");
    assert_eq!(fixture.host_str(), Some("127.0.0.1"));
    assert!(fixture.port().is_some(), "owned fixture port required");
    let root = Path::new(input["root"].as_str().unwrap());
    assert!(root.is_absolute());
    let config: crate::config::schema::AcpAgentConfig =
        serde_json::from_value(input["config"].clone()).unwrap();
    assert!(Path::new(&config.command).is_absolute());
    let isolation: std::collections::HashMap<String, String> =
        serde_json::from_value(input["isolation_env"].clone()).unwrap();
    assert!(!isolation.is_empty(), "native config isolation required");
    assert!(isolation.iter().all(|(key, value)| {
        !matches!(key.to_ascii_uppercase().as_str(), "HOME" | "USERPROFILE" | "PATH")
            && Path::new(value).starts_with(root)
            && std::env::var(key).is_ok_and(|inherited| inherited == *value)
    }));
    if mode == "cli-local" {
        assert_eq!(config.env, isolation);
    } else {
        assert!(config.env.is_empty(), "production profile owns managed overrides");
    }
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    crate::env::set("VW_ACP_OFFLINE_FIXTURE_KEY", input["key"].as_str().unwrap());
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, root).await.unwrap();
    let context = instance::Context {
        directory: root.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    let agent = input["agent"].as_str().unwrap();
    crate::config::update_project_for_tenant(&tenant, root, json!({
        "acp": {agent: config},
        "providers": {"acp-offline-acceptance": {
            "api": format!("{}/v1/chat/completions", input["fixture"].as_str().unwrap()),
            "adapter":"openai-compatible", "env":["VW_ACP_OFFLINE_FIXTURE_KEY"],
            "models": {
                "acp-fixture-primary":{"id":"acp-fixture-primary","status":"active","tool_call":true,"temperature":true},
                "acp-fixture-switched":{"id":"acp-fixture-switched","status":"active","tool_call":true,"temperature":true}
            }
        }}
    })).await.unwrap();
    instance::provide_context(context, async {
        let info = provider::get_provider("acp-offline-acceptance").await.unwrap().unwrap();
        let model =
            provider::get_model("acp-offline-acceptance", "acp-fixture-primary").await.unwrap();
        let switched =
            provider::get_model("acp-offline-acceptance", "acp-fixture-switched").await.unwrap();
        let mut flow = json!({"isolated":true});
        let prompt = "Read challenge.txt twice using your file read tool, then reply.";
        flow["new"] = json!(request(&input, &model, &info, prompt, false).await);
        flow["second_turn"] = json!(request(&input, &model, &info, prompt, false).await);
        flow["cancel"] =
            json!(request(&input, &model, &info, "CANCEL_PROBE: read challenge.txt", true).await);
        flow["restart"] = json!(request(&input, &model, &info, prompt, false).await);
        flow["switch"] = json!(request(&input, &switched, &info, prompt, false).await);
        flow["handshake"] = json!(
            vw_acp::list_sessions()
                .await
                .unwrap()
                .iter()
                .any(|r| { r.cwd == root.to_string_lossy() && !r.acp_session_id.is_empty() })
        );
        let observed = evidence(&input).await;
        let path = std::env::var("VW_ACP_ACCEPTANCE_FLOW").expect("flow output path required");
        std::fs::write(path, serde_json::to_vec(&flow).unwrap()).unwrap();
        assert!(
            flow.as_object().unwrap().values().all(|v| v == true),
            "production ACP lifecycle did not complete"
        );
        assert!(accepted(&observed, mode), "actual model observations did not satisfy acceptance");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires explicit pinned real CLI manifest and owned offline model fixture"]
async fn acp_model_source_real_cli_session_model() {
    run("session-model").await;
}

#[tokio::test]
#[ignore = "requires explicit pinned real CLI manifest and owned offline model fixture"]
async fn acp_model_source_real_cli_cli_local() {
    run("cli-local").await;
}
