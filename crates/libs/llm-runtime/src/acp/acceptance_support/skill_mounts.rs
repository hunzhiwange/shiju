//! Skill-only real CLI entries. No mock ACP backend is allowed here.
use super::AcceptanceHost;
use serde_json::{Value, json};

pub(crate) fn accepted(observations: &Value) -> bool {
    observations["errors"] == 0
        && observations["model_requests"].as_u64().unwrap_or(0) > 0
        && observations["tool_requests"].as_u64().unwrap_or(0) >= 3
        && observations["tool_results"].as_u64().unwrap_or(0) >= 3
        && observations["discovery_events"].as_u64().unwrap_or(0) > 0
        && observations["first_prompt_clean"] == true
        && observations["attachment_read"] == true
        && observations["script_executed"] == true
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

async fn request<H: AcceptanceHost>(input: &Value, session: &str, mode: &str, close: bool) -> bool {
    use agent_provider::provider;
    use crate::StreamEvent;
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
    let production = crate::acp::request::do_stream_request_acp::<H>(
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
                crate::acp::cache::ACP_CLIENT_CACHE.lock().drain().map(|(_, client)| client).collect();
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
        return matches!(result, Err(crate::Error::Aborted));
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

async fn defaults<H: AcceptanceHost>(input: &Value, skills: &[&str], deny: bool) {
    let agent = format!("acp-{}", input["agent"].as_str().unwrap());
    H::update(json!({"agents":{agent:{"enabled":true,"provider":"", "model":"",
        "permission":if deny { json!({"*":"deny"}) } else { json!({}) },
        "defaults":{"skills":{"mode":"fixed","value":skills}, "mcp":{"mode":"fixed","value":[]}}}}}))
        .await.unwrap();
}

pub fn load_manifest(agent: &str) -> Value {
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
    let config: agent_config::schema::AcpAgentConfig =
        serde_json::from_value(input["config"].clone()).unwrap();
    assert!(std::path::Path::new(&config.command).is_absolute());
    input
}

pub async fn run<H: AcceptanceHost>(input: Value, agent: &str) {
    let root = std::path::Path::new(input["root"].as_str().unwrap());
    let config: agent_config::schema::AcpAgentConfig = serde_json::from_value(input["config"].clone()).unwrap();
    H::update(json!({"agents":{"skill-isolation":{"provider":"","model":"","runtime":{"kind":"acp","backend":agent}, "defaults":{"skills":{"mode":"fixed","value":[]},"mcp":{"mode":"fixed","value":[]}}}}})).await.unwrap();
    H::update_project(root, json!({"acp":{agent:config}, "providers":{"skill-acceptance":{"api":format!("{}/v1/{}",input["fixture"].as_str().unwrap(), match agent { "codex" => "responses", "claude" => "messages", _ => "chat/completions" }),
        "adapter":match agent { "codex" => "openai-responses", "claude" => "anthropic", _ => "openai-compatible" }, "env":["VW_ACP_SKILL_FIXTURE_KEY"], "models":{
        "acp-fixture-primary":{"id":"acp-fixture-primary", "status":"active","tool_call":true,"temperature":true,"reasoning":agent == "codex"}}}}})).await.unwrap();
    {
        let name = input["skill"].as_str().unwrap();
        let session = format!("skill-acceptance-{name}");
        let mut flow = json!({});
        checkpoint(&flow);
        defaults::<H>(&input, &[name], false).await;
        flow["new"] = json!(request::<H>(&input, &session, "present", false).await);
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
        defaults::<H>(&input, &[name, &added], false).await;
        flow["add"] = json!(request::<H>(&input, &session, "added", false).await);
        checkpoint(&flow);
        defaults::<H>(&input, &[], false).await;
        flow["remove"] = json!(request::<H>(&input, &session, "absent", false).await);
        defaults::<H>(&input, &[name], false).await;
        assert!(request::<H>(&input, &session, "present", false).await);
        flow["isolated"] =
            json!(request::<H>(&input, &format!("{session}-second"), "isolated", false).await);
        checkpoint(&flow);
        let skill = root.join("skills").join(name).join("SKILL.md");
        let content = std::fs::read_to_string(&skill).unwrap();
        std::fs::write(&skill, format!("{content}\nV2_SKILL_BODY\n")).unwrap();
        defaults::<H>(&input, &[name], false).await;
        flow["v2"] = json!(request::<H>(&input, &session, "present", false).await);
        flow["idle_update"] = json!(request::<H>(&input, &session, "present", false).await);
        checkpoint(&flow);
        flow["cancel"] = json!(request::<H>(&input, &session, "cancel", false).await);
        checkpoint(&flow);
        // First retire the actual CLI, then require recovery by the next request.
        assert!(request::<H>(&input, &session, "present", true).await);
        flow["restart"] = json!(request::<H>(&input, &session, "present", true).await);
        checkpoint(&flow);
        defaults::<H>(&input, &[name], true).await;
        let deny_config = H::config().await.unwrap();
        assert_eq!(deny_config.agents[&format!("acp-{agent}")].permission, json!({"*":"deny"}));
        let before = fixture(&input, None).await;
        let denied = request::<H>(&input, &format!("{session}-denied"), "denied", true).await;
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
            "iflow"
                | "gemini"
                | "codebuddy"
                | "goose"
                | "vibe"
                | "hermes"
                | "snow"
                | "kilocode"
                | "kiro"
                | "opencode"
                | "openclaw"
                | "pi"
        ) {
            assert!(after["native_skill_invocations"].as_u64().unwrap_or(0) > 0);
            assert_eq!(after["skill_body_read"], true);
        }
    }
}
