//! Codex real CLI acceptance through the actual host ports.
use super::AcceptanceHost;
use agent_provider::provider;
use crate::StreamEvent;
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

pub fn load_manifest(mode: &str) -> Value {
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
    let config: agent_config::schema::AcpAgentConfig =
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
    input
}

fn codex_accepted(evidence: &Value, mode: &str) -> bool {
    let Some(requests) = evidence["requests"].as_array() else {
        return false;
    };
    let primary = requests.iter().any(|r| r["model"] == "acp-fixture-primary");
    let switched = requests.iter().any(|r| r["model"] == "acp-fixture-switched");
    let route = if mode == "session-model-chat" { "/v1/chat/completions" } else { "/v1/responses" };
    evidence["errors"] == 0
        && requests.len() >= 4
        && primary
        && (mode == "cli-local" || switched)
        && requests.iter().all(|r| {
            r["authenticated"] == true
                && r["route"] == route
                && (r["model"] == "acp-fixture-primary"
                    || (mode != "cli-local" && r["model"] == "acp-fixture-switched"))
        })
        && requests.iter().any(|r| r["tool_results"].as_u64().unwrap_or(0) >= 2)
}

async fn codex_evidence(input: &Value) -> Value {
    model_transport::upstream_http_client()
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

async fn codex_request<H: AcceptanceHost>(
    input: &Value,
    model: &provider::Model,
    info: &provider::Info,
    prompt: &str,
    cancel: bool,
) -> bool {
    let (tx, rx) = tokio::sync::watch::channel(false);
    let before = codex_evidence(input).await["requests"].as_array().unwrap().len();
    let fixture = input.clone();
    let cancellation = async {
        if cancel {
            for _ in 0..200 {
                let count = codex_evidence(&fixture).await["requests"].as_array().unwrap().len();
                if count > before {
                    tx.send(true).unwrap();
                    return;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            panic!("cancellation never reached a model request");
        }
    };
    let mut options = json!({
        "acp_agent": input["agent"],
        "acp_model_source": if input["mode"] == "cli-local" { "cli_local" } else { "session_model" },
        "cwd": input["root"],
        "acp_close_client_after_request": true,
    });
    if let Some(effort) = input.get("reasoning_effort") {
        options["reasoning_effort"] = effort.clone();
    }
    let messages = json!([{"role":"user", "content":prompt}]);
    let mut output = String::new();
    let mut done = false;
    let mut on_event = |event| match event {
        StreamEvent::Delta(text) => output.push_str(&text),
        StreamEvent::Done { .. } => done = true,
        _ => {}
    };
    let challenge = input["challenge"].as_str().unwrap();
    let session = if input["mode"] == "session-model-chat" {
        // A started session keeps its selected model. Test the second model in a new session.
        format!("real-cli-offline-{challenge}-{}", model.id)
    } else {
        format!("real-cli-offline-{challenge}")
    };
    let production = crate::acp::request::do_stream_request_acp::<H>(
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
    // Persist only fixed boolean classifications, never error text or supplier payloads.
    let error_text = result.as_ref().err().map(ToString::to_string).unwrap_or_default();
    let diagnostic = json!({
        "result_ok": result.is_ok(), "done": done, "has_output": !output.is_empty(),
        "fixture_reply": output.contains("FIXTURE_MODEL_REPLY"),
        "http_401": error_text.contains("401") || output.contains("401"),
        "http_403": error_text.contains("403") || output.contains("403"),
        "http_422": error_text.contains("422") || output.contains("422"),
        "transport_metadata": error_text.contains("transport metadata"),
        "unsupported_transport": error_text.contains("Responses model"),
        "missing_credentials": error_text.contains("credential"),
        "stale_state": error_text.contains("no rollout") || error_text.contains("session not found"),
        "profile_reissue": error_text.contains("reissued"),
        "initialize_error": error_text.contains("initializ"),
        "aborted": matches!(result, Err(crate::Error::Aborted)),
    });
    let path = std::path::PathBuf::from(std::env::var("VW_ACP_ACCEPTANCE_FLOW").unwrap())
        .with_extension("codex-diag.json");
    let mut diagnostics = std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Vec<Value>>(&bytes).ok())
        .unwrap_or_default();
    diagnostics.push(diagnostic);
    std::fs::write(path, serde_json::to_vec(&diagnostics).unwrap()).unwrap();
    if cancel {
        matches!(result, Err(crate::Error::Aborted))
    } else {
        result.is_ok()
            && done
            && output.contains("FIXTURE_MODEL_REPLY")
            && output.contains(input["challenge"].as_str().unwrap())
            && codex_evidence(input).await["requests"].as_array().unwrap().len() > before
    }
}

pub async fn run<H: AcceptanceHost>(input: Value, mode: &str, effort_check: bool) {
    let root = Path::new(input["root"].as_str().unwrap());
    let config: agent_config::schema::AcpAgentConfig = serde_json::from_value(input["config"].clone()).unwrap();
    let agent = input["agent"].as_str().unwrap();
    assert_eq!(agent, "codex");
    let (route, adapter) = if mode == "session-model-chat" {
        ("/v1/chat/completions", "openai-compatible")
    } else {
        ("/v1/responses", "openai-responses")
    };
    // Use a model from the pinned engine's catalog so ACP publishes native efforts.
    let primary = if effort_check { "gpt-5.5" } else { "acp-fixture-primary" };
    H::update_project(root, json!({
        "acp": {agent: config},
        "providers": {"acp-offline-acceptance": {
            "api": format!("{}{route}", input["fixture"].as_str().unwrap()),
            "adapter":adapter, "env":["VW_ACP_OFFLINE_FIXTURE_KEY"],
            "models": {
                primary:{"id":primary,"status":"active","tool_call":true,"temperature":true,"reasoning":true},
                "acp-fixture-switched":{"id":"acp-fixture-switched","status":"active","tool_call":true,"temperature":true,"reasoning":true}
            }
        }}
    })).await.unwrap();
    {
        let info = provider::get_provider("acp-offline-acceptance").await.unwrap().unwrap();
        let model = provider::get_model("acp-offline-acceptance", primary).await.unwrap();
        if effort_check {
            codex_effort_run::<H>(&input, &model, &info).await;
            return;
        }
        let switched =
            provider::get_model("acp-offline-acceptance", "acp-fixture-switched").await.unwrap();
        let mut flow = json!({"isolated":true});
        let prompt = "Read challenge.txt twice using your file read tool, then reply.";
        let first_prompt = if agent == "claude" {
            "AUXILIARY_PROBE: Delegate reading challenge.txt twice to a general-purpose helper, then reply with its contents."
        } else { prompt };
        flow["new"] = json!(codex_request::<H>(&input, &model, &info, first_prompt, false).await);
        flow["second_turn"] = json!(codex_request::<H>(&input, &model, &info, prompt, false).await);
        flow["cancel"] =
            json!(codex_request::<H>(&input, &model, &info, "CANCEL_PROBE: read challenge.txt", true).await);
        flow["restart"] = json!(codex_request::<H>(&input, &model, &info, prompt, false).await);
        flow["switch"] = json!(codex_request::<H>(&input, &switched, &info, prompt, false).await);
        flow["handshake"] = json!(
            acp_storage::list_sessions()
                .await
                .unwrap()
                .iter()
                .any(|r| { r.cwd == root.to_string_lossy() && !r.acp_session_id.is_empty() })
        );
        let observed = codex_evidence(&input).await;
        if agent == "claude" {
            assert!(observed["requests"].as_array().unwrap().iter().any(|request| request["auxiliary"] == true),
                "Claude auxiliary model did not reach the fixture");
        }
        let path = std::env::var("VW_ACP_ACCEPTANCE_FLOW").expect("flow output path required");
        std::fs::write(path, serde_json::to_vec(&flow).unwrap()).unwrap();
        assert!(
            flow.as_object().unwrap().values().all(|v| v == true),
            "production ACP lifecycle did not complete"
        );
        assert!(codex_accepted(&observed, mode), "actual model observations did not satisfy acceptance");
    }
}

async fn codex_effort_run<H: AcceptanceHost>(input: &Value, model: &provider::Model, info: &provider::Info) {
    use super::controls;
    use vw_api_acp_model::{
        acp_model_source::AcpModelSource,
        acp_session::{AcpOptionKind, AcpSessionOptions, AcpSessionSelection},
    };

    let directory = input["root"].as_str().unwrap();
    let session = format!("real-cli-offline-{}", input["challenge"].as_str().unwrap());
    let path = std::env::var("VW_ACP_ACCEPTANCE_FLOW").unwrap();
    let mut flow = json!({});
    let mut checkpoint = |key: &str, passed: bool| {
        flow[key] = json!(passed);
        std::fs::write(&path, serde_json::to_vec(&flow).unwrap()).unwrap();
        assert!(passed, "Codex effort checkpoint failed: {key}");
    };

    checkpoint(
        "default_turn",
        codex_request::<H>(input, model, info, "ACP_EFFORT_DEFAULT", false).await,
    );
    let store = H::open_selection(&session, directory).unwrap();
    let anchor = store.read().unwrap().unwrap();
    let native_id = acp_storage::resolve_session_record(&anchor.record_id).await.unwrap().acp_session_id;
    let options = controls::read::<H>(&session, directory).await.unwrap();
    checkpoint(
        "managed_model",
        anchor.agent == "codex"
            && anchor.model.source == AcpModelSource::SessionModel
            && anchor.model.model.as_deref() == Some(model.id.as_str())
            && options.acp_model_source == AcpModelSource::SessionModel
            && options.selected_model.as_ref().and_then(|selection| selection.model.as_deref())
                == Some("acp-offline-acceptance/gpt-5.5"),
    );
    let effort = options.options.iter().find(
        |option| matches!(&option.kind, AcpOptionKind::Config { id } if id == "reasoning_effort"),
    );
    checkpoint(
        "advertised",
        effort.is_some_and(|option| {
            ["low", "high"]
                .iter()
                .all(|value| option.choices.iter().any(|choice| choice.id == *value))
                && !option.choices.iter().any(|choice| choice.id == "max")
        }),
    );
    let kind = effort.unwrap().kind.clone();
    let select = |value: &str| {
        controls::select::<H>(
            &session,
            directory,
            AcpSessionSelection { kind: kind.clone(), value: value.into() },
        )
    };
    let current_is = |options: AcpSessionOptions, value: &str| {
        options.options.iter().any(|option| option.kind == kind && option.current == value)
    };

    assert!(current_is(select("low").await.expect("select low"), "low"));
    assert!(codex_request::<H>(input, model, info, "ACP_EFFORT_A", false).await);
    assert!(current_is(select("high").await.expect("select high"), "high"));
    let mut conflicting_request = input.clone();
    conflicting_request["reasoning_effort"] = json!("low");
    checkpoint(
        "request_precedence",
        codex_request::<H>(&conflicting_request, model, info, "ACP_EFFORT_B", false).await,
    );
    let invalid = select("max").await;
    checkpoint("invalid_rejected", invalid.is_err_and(|e| e.contains("Unsupported value")));
    checkpoint(
        "previous_preserved",
        current_is(controls::read::<H>(&session, directory).await.unwrap(), "high"),
    );
    // Each managed request tears down its process and relay lease. The next
    // request must restore both the original native thread and its saved effort.
    checkpoint("restored", codex_request::<H>(input, model, info, "ACP_EFFORT_RESTORED", false).await);
    let latest = store.read().unwrap().unwrap();
    let record = acp_storage::resolve_session_record(&latest.record_id).await.unwrap();
    checkpoint(
        "same_session",
        latest.record_id == anchor.record_id
            && !native_id.is_empty()
            && record.acp_session_id == native_id,
    );
}
