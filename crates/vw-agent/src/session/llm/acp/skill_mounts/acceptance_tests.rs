//! Skill-only real CLI entries. No mock ACP backend is allowed here.
use serde_json::{Value, json};

fn accepted(observations: &Value) -> bool {
    observations["errors"] == 0
        && observations["model_requests"].as_u64().unwrap_or(0) > 0
        && observations["tool_requests"].as_u64().unwrap_or(0) >= 3
        && observations["tool_results"].as_u64().unwrap_or(0) >= 3
        && observations["discovery_events"].as_u64().unwrap_or(0) > 0
        && observations["first_prompt_clean"] == true
        && observations["attachment_read"] == true
        && observations["script_executed"] == true
}

#[test]
fn acp_skill_mount_harness_zero_requests_never_pass() {
    assert!(!accepted(&json!({"model_requests":0,"tool_results":0})));
}

#[test]
fn acp_skill_mount_harness_old_model_report_never_passes() {
    assert!(!accepted(&json!({"requests":[{"tool_results":5}],"errors":0})));
}

#[test]
fn acp_skill_mount_harness_requires_discovery_clean_prompt_and_real_results() {
    let evidence = json!({"errors":0,"model_requests":4,"tool_requests":3,"tool_results":3,
        "discovery_events":1,"first_prompt_clean":true,"attachment_read":true,"script_executed":true});
    assert!(accepted(&evidence));
    for key in ["model_requests", "tool_requests", "tool_results", "discovery_events"] {
        let mut invalid = evidence.clone();
        invalid[key] = json!(0);
        assert!(!accepted(&invalid));
    }
    for key in ["first_prompt_clean", "attachment_read", "script_executed"] {
        let mut invalid = evidence.clone();
        invalid[key] = json!(false);
        assert!(!accepted(&invalid));
    }
}

#[tokio::test]
async fn acp_skill_mount_harness_initial_prompt_contains_no_body() {
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let root = tempfile::tempdir().unwrap();
    let skill = root.path().join("skills/acceptance");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(
        skill.join("SKILL.md"),
        "---\nname: acceptance\ndescription: fixture\n---\nPRIVATE_SKILL_BODY\n",
    )
    .unwrap();
    let mut config = crate::config::Config::default();
    config.agents.insert("acp-codex".into(), serde_json::from_value(json!({"provider":"", "model":"",
        "defaults":{"skills":{"mode":"fixed","value":["acceptance"]},"mcp":{"mode":"fixed","value":[]}}})).unwrap());
    let mut parsed = super::super::ParsedAcpOptions::default();
    let messages = super::super::config::apply_agent_defaults(
        &config,
        &json!({"agent":"acp-codex"}),
        "codex",
        root.path(),
        &mut parsed,
        &json!([{"role":"user","content":"use the skill"}]),
    )
    .await
    .unwrap();
    let prompt = super::super::replay::build_request_prompt(
        &messages,
        false,
        super::super::AcpReplayStrategy::Discard,
        3,
    );
    assert!(!prompt.contains("PRIVATE_SKILL_BODY"));
    assert_eq!(parsed.skill_mounts.len(), 1);
    assert_eq!(std::path::Path::new(&parsed.skill_mounts[0].path), skill.canonicalize().unwrap());
}

async fn fixture(input: &Value, begin: Option<&str>) -> Value {
    let client = reqwest::Client::new();
    if let Some(mode) = begin {
        client
            .post(format!("{}/_begin", input["fixture"].as_str().unwrap()))
            .bearer_auth(input["key"].as_str().unwrap())
            .json(&json!({"mode":mode}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }
    client
        .get(format!("{}/_evidence", input["fixture"].as_str().unwrap()))
        .bearer_auth(input["key"].as_str().unwrap())
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn request(input: &Value, session: &str, mode: &str, close: bool) -> bool {
    use crate::{provider::provider, session::llm::StreamEvent};
    let before = fixture(
        input,
        Some(match mode {
            "denied" => "present",
            "isolated" => "absent",
            _ => mode,
        }),
    )
    .await;
    let info = provider::get_provider("skill-acceptance").await.unwrap().unwrap();
    let model = provider::get_model("skill-acceptance", "acp-fixture-primary").await.unwrap();
    let options = json!({"acp_agent":input["agent"], "agent":if mode == "isolated" { "skill-isolation".to_string() } else { format!("acp-{}",input["agent"].as_str().unwrap()) },
        "acp_model_source":if input["mode"] == "cli-local" { "cli_local" } else { "session_model" }, "cwd":input["root"], "acp_close_client_after_request":close});
    let messages = json!([{"role":"user","content":input["prompt"]}]);
    let expected = std::fs::read_to_string(
        std::path::Path::new(input["root"].as_str().unwrap())
            .join("skills")
            .join(input["skill"].as_str().unwrap())
            .join("references/value.txt"),
    )
    .unwrap();
    let mut output = String::new();
    let mut done = false;
    let mut event_errors = Vec::new();
    let mut on_event = |event| match event {
        StreamEvent::Delta(text) => output.push_str(&text),
        StreamEvent::Done { .. } => done = true,
        StreamEvent::Error(error) => event_errors.push(format!("{error:?}")),
        _ => {}
    };
    let (tx, rx) = tokio::sync::watch::channel(false);
    let cancellation = async {
        if mode == "cancel" {
            for _ in 0..1800 {
                if fixture(input, None).await["model_requests"].as_u64().unwrap()
                    > before["model_requests"].as_u64().unwrap()
                {
                    tx.send(true).unwrap();
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
            panic!("cancel never reached the model fixture");
        }
    };
    let production = super::super::do_stream_request_acp(
        &model,
        &info,
        None,
        &options,
        &messages,
        session,
        Some(&rx),
        &mut on_event,
    );
    let result = tokio::time::timeout(std::time::Duration::from_secs(90), async {
        tokio::join!(production, cancellation)
    })
    .await;
    let (result, ()) = match result {
        Ok(result) => result,
        Err(error) => {
            // This explicitly filtered real-CLI test owns the cache in its process.
            let clients: Vec<_> =
                super::super::ACP_CLIENT_CACHE.lock().drain().map(|(_, client)| client).collect();
            for client in clients {
                let _ = client.client.close().await;
            }
            panic!("real CLI skill request timed out: {error}");
        }
    };
    if let Err(error) = &result {
        let reason = error.to_string();
        let reason = regex::Regex::new(r#"https?://[^\s"]+|/[^\s"]+|[A-Za-z0-9_=-]{24,}"#)
            .unwrap()
            .replace_all(&reason, "[redacted]");
        eprintln!("SKILL_FAILURE: {}", reason.chars().take(400).collect::<String>());
    }
    if mode != "cancel" && (!event_errors.is_empty() || !done || !output.contains("SKILL_")) {
        let reason = format!("done={done}; output={output}; events={event_errors:?}");
        let reason = regex::Regex::new(r#"https?://[^\s"]+|/[^\s"]+|[A-Za-z0-9_=-]{24,}"#)
            .unwrap()
            .replace_all(&reason, "[redacted]");
        eprintln!("SKILL_FAILURE: {}", reason.chars().take(500).collect::<String>());
    }
    if mode == "cancel" {
        return matches!(result, Err(crate::session::llm::Error::Aborted));
    }
    if mode == "denied" {
        return result
            .err()
            .is_some_and(|error| error.to_string().contains("denied by permission"));
    }
    result.is_ok()
        && done
        && if matches!(mode, "absent" | "isolated") {
            output.contains("SKILL_ABSENT") && !output.contains(&expected)
        } else {
            output.contains("SKILL_MOUNT_REPLY") && output.contains(&expected)
        }
}

fn checkpoint(flow: &Value) {
    std::fs::write(
        std::env::var("VW_ACP_SKILL_ACCEPTANCE_FLOW").unwrap(),
        serde_json::to_vec(flow).unwrap(),
    )
    .unwrap();
}

async fn defaults(input: &Value, skills: &[&str], deny: bool) {
    let agent = format!("acp-{}", input["agent"].as_str().unwrap());
    crate::config::update(json!({"agents":{agent:{"enabled":true,"provider":"", "model":"",
        "permission":if deny { json!({"*":"deny"}) } else { json!({}) },
        "defaults":{"skills":{"mode":"fixed","value":skills}, "mcp":{"mode":"fixed","value":[]}}}}}))
        .await.unwrap();
}

async fn run(agent: &str) {
    use crate::project::instance;
    let _guard = crate::test_support::global_async_env_lock().lock().await;
    let input: Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var("VW_ACP_SKILL_ACCEPTANCE_MANIFEST")
                .expect("explicit real CLI manifest required"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(input["agent"], agent);
    let url = reqwest::Url::parse(input["fixture"].as_str().unwrap()).unwrap();
    assert_eq!(url.scheme(), "http");
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    assert!(url.port().is_some());
    let root = std::path::Path::new(input["root"].as_str().unwrap());
    assert!(root.is_absolute());
    let config: crate::config::schema::AcpAgentConfig =
        serde_json::from_value(input["config"].clone()).unwrap();
    assert!(std::path::Path::new(&config.command).is_absolute());
    crate::env::set("VW_ACP_SKILL_FIXTURE_KEY", input["key"].as_str().unwrap());
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, root).await.unwrap();
    let context = instance::Context {
        directory: root.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    crate::config::update(json!({"agents":{"skill-isolation":{"provider":"","model":"","runtime":{"kind":"acp","backend":agent}, "defaults":{"skills":{"mode":"fixed","value":[]},"mcp":{"mode":"fixed","value":[]}}}}})).await.unwrap();
    crate::config::update_project_for_tenant(&tenant, root, json!({"acp":{agent:config}, "providers":{"skill-acceptance":{"api":format!("{}/v1/{}",input["fixture"].as_str().unwrap(), match agent { "codex" => "responses", "claude" => "messages", _ => "chat/completions" }),
        "adapter":match agent { "codex" => "openai-responses", "claude" => "anthropic", _ => "openai-compatible" }, "env":["VW_ACP_SKILL_FIXTURE_KEY"], "models":{
        "acp-fixture-primary":{"id":"acp-fixture-primary", "status":"active","tool_call":true,"temperature":true,"reasoning":agent == "codex"}}}}})).await.unwrap();
    instance::provide_context(context, async {
        let name = input["skill"].as_str().unwrap();
        let session = format!("skill-acceptance-{name}");
        let mut flow = json!({});
        checkpoint(&flow);
        defaults(&input, &[name], false).await;
        flow["new"] = json!(request(&input, &session, "present", false).await);
        checkpoint(&flow);
        assert_eq!(
            flow["new"], true,
            "production directory mounting did not complete; this is not unsupported evidence"
        );
        let added = format!("{name}-added");
        let added_dir = root.join("skills").join(&added);
        std::fs::create_dir_all(&added_dir).unwrap();
        std::fs::write(
            added_dir.join("SKILL.md"),
            format!("---\nname: {added}\ndescription: Added skill\n---\nADDED_SKILL_BODY\n"),
        )
        .unwrap();
        defaults(&input, &[name, &added], false).await;
        flow["add"] = json!(request(&input, &session, "added", false).await);
        checkpoint(&flow);
        defaults(&input, &[], false).await;
        flow["remove"] = json!(request(&input, &session, "absent", false).await);
        defaults(&input, &[name], false).await;
        assert!(request(&input, &session, "present", false).await);
        flow["isolated"] =
            json!(request(&input, &format!("{session}-second"), "isolated", false).await);
        checkpoint(&flow);
        let skill = root.join("skills").join(name).join("SKILL.md");
        let content = std::fs::read_to_string(&skill).unwrap();
        std::fs::write(&skill, format!("{content}\nV2_SKILL_BODY\n")).unwrap();
        defaults(&input, &[name], false).await;
        flow["v2"] = json!(request(&input, &session, "present", false).await);
        flow["idle_update"] = json!(request(&input, &session, "present", false).await);
        checkpoint(&flow);
        flow["cancel"] = json!(request(&input, &session, "cancel", false).await);
        checkpoint(&flow);
        // First retire the actual CLI, then require recovery by the next request.
        assert!(request(&input, &session, "present", true).await);
        flow["restart"] = json!(request(&input, &session, "present", true).await);
        checkpoint(&flow);
        defaults(&input, &[name], true).await;
        let deny_config = crate::config::get_current().await.unwrap();
        assert_eq!(deny_config.agents[&format!("acp-{agent}")].permission, json!({"*":"deny"}));
        let before = fixture(&input, None).await;
        let denied = request(&input, &format!("{session}-denied"), "denied", true).await;
        let after = fixture(&input, None).await;
        flow["permission_denied"] =
            json!(denied && before["model_requests"] == after["model_requests"]);
        checkpoint(&flow);
        assert!(
            flow.as_object().unwrap().values().all(|v| v == true),
            "skill lifecycle incomplete"
        );
        assert!(accepted(&after), "real model/tool evidence incomplete");
        if matches!(
            agent,
            "gemini"
                | "codebuddy"
                | "goose"
                | "vibe"
                | "hermes"
                | "snow"
                | "kilocode"
                | "kiro"
                | "opencode"
                | "openclaw"
        ) {
            assert!(after["native_skill_invocations"].as_u64().unwrap_or(0) > 0);
            assert_eq!(after["skill_body_read"], true);
        }
    })
    .await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_codex() {
    run("codex").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_claude() {
    run("claude").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_qwen() {
    run("qwen").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_gemini() {
    run("gemini").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_codebuddy() {
    run("codebuddy").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_goose() {
    run("goose").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_vibe() {
    run("vibe").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_hermes() {
    run("hermes").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_snow() {
    run("snow").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_copilot() {
    run("copilot").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_kilocode() {
    run("kilocode").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_kimi() {
    run("kimi").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_kiro() {
    run("kiro").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_opencode() {
    run("opencode").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_openclaw() {
    run("openclaw").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_pi() {
    run("pi").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_auggie() {
    run("auggie").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_cursor() {
    run("cursor").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_droid() {
    run("droid").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_qoder() {
    run("qoder").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_trae() {
    run("trae").await;
}

#[tokio::test]
#[ignore = "requires a pinned real CLI and owned skill model fixture"]
async fn acp_skill_mount_real_iflow() {
    run("iflow").await;
}
