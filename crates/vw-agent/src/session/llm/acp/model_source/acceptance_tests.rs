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
                && (r["route"] == "/v1/chat/completions"
                    || r["route"] == "/v1/messages"
                    || r["route"] == "/v1/responses"
                    || r["model"].as_str().is_some_and(|model| {
                        r["route"] == format!("/v1beta/models/{model}:generateContent")
                            || r["route"] == format!("/v1beta/models/{model}:streamGenerateContent")
                    }))
                && (r["model"] == "acp-fixture-primary"
                    || (mode == "session-model" && r["model"] == "acp-fixture-switched"))
        })
        && requests.iter().any(|r| r["tool_results"].as_u64().unwrap_or(0) >= 2)
}

#[test]
fn acp_model_source_gemini_harness_requires_exact_model_and_generation_path() {
    let request = json!({"model":"acp-fixture-primary","authenticated":true,"route":"/v1beta/models/acp-fixture-primary:streamGenerateContent","tool_results":2});
    let mut evidence =
        json!({"errors":0,"requests":[request.clone(),request.clone(),request.clone(),request]});
    assert!(accepted(&evidence, "cli-local"));
    evidence["requests"][0]["route"] =
        json!("/v1beta/models/forbidden-native-model:streamGenerateContent");
    assert!(!accepted(&evidence, "cli-local"));
    evidence["requests"][0]["route"] = json!("/v1beta/models/acp-fixture-primary:countTokens");
    assert!(!accepted(&evidence, "cli-local"));
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
    if cancel && (input["agent"] == "openclaw" || input["agent"] == "pi") {
        // Native completion events may follow the user's prompt in model input.
        // Arm the owned fixture directly so cancellation cannot race a fast reply.
        reqwest::Client::new()
            .post(format!("{}/_cancel-next", input["fixture"].as_str().unwrap()))
            .bearer_auth(input["control"].as_str().unwrap())
            .json(&json!({}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }
    let before = evidence(input).await["requests"].as_array().unwrap().len();
    let fixture = input.clone();
    let cancellation = async {
        if cancel {
            // Native CLI startup and session restoration are part of the enclosing
            // 60s deadline; cancellation must wait until the model request exists.
            for _ in 0..1800 {
                let count = evidence(&fixture).await["requests"].as_array().unwrap().len();
                if count > before {
                    let session =
                        format!("real-cli-offline-{}", fixture["challenge"].as_str().unwrap());
                    let busy = super::super::controls::select(
                        &session,
                        fixture["root"].as_str().unwrap(),
                        vw_api_types::acp_session::AcpSessionSelection {
                            kind: vw_api_types::acp_session::AcpOptionKind::Model,
                            value: "must-not-be-applied-while-busy".into(),
                        },
                    )
                    .await
                    .expect_err("actual model request must exclude session controls");
                    assert!(busy.contains("busy"), "unexpected control error: {busy}");
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
            && (!prompt.starts_with("/compact ") || output.contains("Compaction completed."))
            && output.contains("FIXTURE_MODEL_REPLY")
            && output.contains(input["challenge"].as_str().unwrap())
            && evidence(input).await["requests"].as_array().unwrap().len() > before
    }
}

fn checkpoint(flow: &Value) {
    let path = std::env::var("VW_ACP_ACCEPTANCE_FLOW").expect("flow output path required");
    std::fs::write(path, serde_json::to_vec(flow).unwrap()).unwrap();
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
        let native_value = if input["agent"] == "kiro" {
            match key.as_str() {
                "KIRO_KAS_ENDPOINT" | "KIRO_KAS_CONTROL_PLANE_ENDPOINT" => {
                    Some(value == input["fixture"].as_str().unwrap())
                }
                "KIRO_KAS_REGION" => Some(value == "us-east-1"),
                "KIRO_NO_AUTO_UPDATE" => Some(value == "1"),
                "KIRO_API_KEY" => Some(
                    value
                        == if mode == "cli-local" {
                            input["key"].as_str().unwrap()
                        } else {
                            "offline-native-account-must-not-be-used"
                        },
                ),
                _ => None,
            }
        } else {
            None
        };
        !matches!(key.to_ascii_uppercase().as_str(), "HOME" | "USERPROFILE" | "PATH")
            && native_value.unwrap_or_else(|| Path::new(value).starts_with(root))
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
    let (upstream_path, upstream_adapter) =
        match input["upstream_protocol"].as_str().unwrap_or("chat") {
            "chat" => ("/v1/chat/completions", "openai-compatible"),
            "messages" => ("/v1/messages", "anthropic"),
            _ => panic!("unsupported fixture protocol"),
        };
    crate::config::update_project_for_tenant(&tenant, root, json!({
        "acp": {agent: config},
        "providers": {"acp-offline-acceptance": {
            "api": format!("{}{upstream_path}", input["fixture"].as_str().unwrap()),
            "adapter":upstream_adapter, "env":["VW_ACP_OFFLINE_FIXTURE_KEY"],
            "models": {
                "acp-fixture-primary":{"id":"acp-fixture-primary","status":"active","tool_call":true,"temperature":true,"reasoning":agent == "copilot"},
                "acp-fixture-switched":{"id":"acp-fixture-switched","status":"active","tool_call":true,"temperature":true,"reasoning":agent == "copilot"}
            }
        }}
    })).await.unwrap();
    instance::provide_context(context, async {
        let info = provider::get_provider("acp-offline-acceptance").await.unwrap().unwrap();
        let model =
            provider::get_model("acp-offline-acceptance", "acp-fixture-primary").await.unwrap();
        let switched =
            provider::get_model("acp-offline-acceptance", "acp-fixture-switched").await.unwrap();
        let expected_launch: crate::config::schema::AcpAgentConfig =
            serde_json::from_value(input["config"].clone()).unwrap();
        let config = crate::config::get_current().await.expect("current project configuration");
        let (_, resolved_launch) = super::super::config::lookup_acp_command(
            &config, &model, &json!({"acp_agent":agent})
        ).expect("offline pinned launch override must resolve");
        assert!(resolved_launch.command == expected_launch.command
            && resolved_launch.args == expected_launch.args,
            "production did not select the pinned real CLI override");
        let mut flow = json!({"isolated":true});
        checkpoint(&flow);
        let prompt = "Read challenge.txt twice using your file read tool, then reply.";
        let first_prompt = if matches!(agent, "claude" | "kimi" | "kiro" | "openclaw") {
            "AUXILIARY_PROBE: Delegate reading challenge.txt twice to a general-purpose helper, then reply with its contents."
        } else { prompt };
        flow["new"] = json!(request(&input, &model, &info, first_prompt, false).await);
        checkpoint(&flow);
        let public = super::super::controls::read(
            &format!("real-cli-offline-{}", input["challenge"].as_str().unwrap()),
            &root.to_string_lossy(),
        ).await.unwrap();
        let selected = public.selected_model.expect("production selection must survive process retirement");
        assert_eq!(selected.source, if mode == "session-model" { vw_api_types::acp_model_source::AcpModelSource::SessionModel } else { vw_api_types::acp_model_source::AcpModelSource::CliLocal });
        if mode == "session-model" {
            assert_eq!(selected.provider_id.as_deref(), Some("acp-offline-acceptance"));
            assert_eq!(selected.model.as_deref(), Some("acp-fixture-primary"));
            let store = super::super::lifecycle::Store::open(
                &format!("real-cli-offline-{}", input["challenge"].as_str().unwrap()),
                &root.to_string_lossy(),
            ).unwrap();
            let saved = serde_json::to_value(store.read().unwrap().unwrap()).unwrap();
            let mut invalid = model.clone();
            invalid.status = "disabled".into();
            assert!(!request(&input, &invalid, &info, prompt, false).await,
                "a disabled replacement must be rejected before touching the CLI");
            assert_eq!(serde_json::to_value(store.read().unwrap().unwrap()).unwrap(), saved,
                "failed replacement must preserve the committed model and recovery anchor");
        }
        if agent == "pi" && mode == "session-model" {
            let saved = super::super::lifecycle::Store::open(
                &format!("real-cli-offline-{}", input["challenge"].as_str().unwrap()),
                &root.to_string_lossy(),
            ).unwrap().read().unwrap().expect("managed Pi selection must be committed");
            let mut record = vw_acp::resolve_session_record(&saved.record_id).await.unwrap();
            vw_acp::set_desired_model_id(&mut record, Some("session-provider/forbidden-native-model"));
            vw_acp::write_session_record(&record).await.unwrap();
        }
        flow["second_turn"] = json!(request(&input, &model, &info, prompt, false).await);
        checkpoint(&flow);
        if agent == "pi" {
            flow["auxiliary"] = json!(request(&input, &model, &info,
                "/compact preserve the challenge", false).await);
            checkpoint(&flow);
        }
        flow["cancel"] =
            json!(request(&input, &model, &info, "CANCEL_PROBE: read challenge.txt", true).await);
        checkpoint(&flow);
        flow["restart"] = json!(request(&input, &model, &info, prompt, false).await);
        checkpoint(&flow);
        flow["switch"] = json!(request(&input, &switched, &info, prompt, false).await);
        checkpoint(&flow);
        if agent == "pi" {
            flow["auxiliary_switched"] = json!(request(&input, &switched, &info,
                "/compact preserve the challenge", false).await);
            checkpoint(&flow);
        }
        flow["handshake"] = json!(
            vw_acp::list_sessions()
                .await
                .unwrap()
                .iter()
                .any(|r| { r.cwd == root.to_string_lossy() && !r.acp_session_id.is_empty() })
        );
        let observed = evidence(&input).await;
        if matches!(agent, "claude" | "kimi" | "kiro" | "openclaw") {
            assert!(observed["requests"].as_array().unwrap().iter().any(|request| request["auxiliary"] == true),
                "auxiliary model did not reach the fixture");
        }
        if agent == "pi" {
            for expected in if mode == "session-model" {
                vec!["acp-fixture-primary", "acp-fixture-switched"]
            } else { vec!["acp-fixture-primary"] } {
                assert!(observed["requests"].as_array().unwrap().iter().any(|request|
                    request["auxiliary"] == true && request["model"] == expected),
                    "native Pi compaction did not use the selected source model");
            }
            let path = std::env::var("VW_ACP_ACCEPTANCE_DIAGNOSTICS").unwrap();
            let stats: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            if mode == "session-model" {
                assert!(stats["out_model_primary"].as_u64().unwrap_or(0) >= 5
                    && stats["out_model_switched"].as_u64().unwrap_or(0) >= 2
                    && stats["out_model_other"].as_u64().unwrap_or(0) == 0,
                    "managed Pi must bind the exact provider/model by RPC before each prompt");
            } else {
                assert_eq!(stats["out_set_model"].as_u64().unwrap_or(0), 0,
                    "cli_local must not receive a client model override");
            }
        }
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

/// Closed, non-payload diagnostics for a real acceptance process. Never persist
/// messages, session identifiers, environment values, paths or credentials.
pub(in crate::session::llm::acp) fn observe_rpc(
    direction: vw_acp::AcpMessageDirection,
    message: &vw_acp::AcpJsonRpcMessage,
) {
    let Some(path) = std::env::var_os("VW_ACP_ACCEPTANCE_DIAGNOSTICS") else { return };
    let Ok(value) = serde_json::to_value(message) else { return };
    static STATS: std::sync::LazyLock<parking_lot::Mutex<Value>> =
        std::sync::LazyLock::new(|| parking_lot::Mutex::new(json!({})));
    let outbound = direction == vw_acp::AcpMessageDirection::Outbound;
    let key = match (outbound, value["method"].as_str()) {
        (true, Some("initialize")) => Some("out_initialize"),
        (true, Some("session/new")) => Some("out_new"),
        (true, Some("session/load")) => Some("out_load"),
        (true, Some("session/resume")) => Some("out_resume"),
        (true, Some("session/prompt")) => Some("out_prompt"),
        (true, Some("session/set_model")) => Some("out_set_model"),
        (false, Some("fs/read_text_file")) => Some("in_read"),
        (false, Some("terminal/create")) => Some("in_terminal"),
        (false, Some("session/request_permission")) => Some("in_permission"),
        (false, Some("session/update")) => Some("in_update"),
        _ => None,
    };
    let mut stats = STATS.lock();
    if let Some(key) = key {
        let count = stats[key].as_u64().unwrap_or(0) + 1;
        stats[key] = json!(count);
    }
    if outbound && value["method"] == "session/set_model" {
        let key = match value["params"]["modelId"].as_str() {
            Some("session-provider/acp-fixture-primary") => "out_model_primary",
            Some("session-provider/acp-fixture-switched") => "out_model_switched",
            _ => "out_model_other",
        };
        stats[key] = json!(stats[key].as_u64().unwrap_or(0) + 1);
    }
    if outbound && value["method"] == "session/new" {
        let settings = &value["params"]["_meta"]["claudeCode"]["options"]["settings"];
        stats["sdk_settings_sent"] = json!(settings.is_string());
        stats["sdk_settings_exists"] =
            json!(settings.as_str().is_some_and(|path| Path::new(path).is_file()));
    }
    if !outbound && value["result"].get("protocolVersion").is_some() {
        stats["initialize_returned"] = json!(true);
    }
    if !outbound && value["result"].get("sessionId").is_some() {
        stats["new_returned"] = json!(true);
    }
    if value.get("error").is_some_and(|error| !error.is_null()) {
        stats[if outbound { "out_error" } else { "in_error" }] = json!(true);
        if !outbound && stats.get("first_error_code").is_none() {
            stats["first_error_code"] = json!(value["error"]["code"].as_i64().unwrap_or(0));
            let message = value["error"]["message"].as_str().unwrap_or("").to_ascii_lowercase();
            stats["error_session_missing"] = json!(
                message.contains("no conversation found")
                    || (message.contains("session")
                        && (message.contains("not found")
                            || message.contains("does not exist")
                            || message.contains("unknown")))
            );
            stats["error_process_exit"] = json!(message.contains("process exited"));
            stats["error_auth"] =
                json!(message.contains("authentication") || message.contains("401"));
            stats["error_forbidden"] = json!(message.contains("403"));
            stats["error_unsupported"] =
                json!(message.contains("422") || message.contains("unsupported"));
            stats["error_validation"] =
                json!(message.contains("invalid") || message.contains("validation"));
        }
    }
    if let Ok(bytes) = serde_json::to_vec(&*stats) {
        let _ = std::fs::write(path, bytes);
    }
}
