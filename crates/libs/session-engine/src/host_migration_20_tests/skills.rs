use super::*;
#[tokio::test]
async fn acp_skill_mount_outbound_defaults_preserve_rules_without_body() {
    isolated(async {
    let temp = tempfile::tempdir().unwrap();
    let skill_dir = temp.path().join("skills/selected");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: selected\ndescription: fixture\n---\nSCOPED_SKILL_CONTENT\n",
    )
    .unwrap();
    let mut cfg = config::Config::default();
    cfg.agents.insert(
        "acp-codex".into(),
        serde_json::from_value(json!({
            "provider":"", "model":"", "system_prompt":"TRUSTED_AGENT_RULE", "defaults":{
                "skills":{"mode":"fixed","value":["selected"]},
                "mcp":{"mode":"fixed","value":[]}
            }
        }))
        .unwrap(),
    );
    let options = json!({"agent":"acp-codex", "mcp_servers":[{"name":"UNTRUSTED"}]});
    let messages = json!([{"role":"user","content":"hello"}]);
    let mut parsed = ParsedAcpOptions::default();
    let resolved = apply_agent_defaults(
        &cfg,
        &options,
        "codex",
        temp.path(),
        &mut parsed,
        &messages,
    )
    .await
    .unwrap();
    assert!(parsed.mcp_servers.is_empty());
    let prompt = llm_runtime::acp::replay::build_request_prompt(
        &resolved,
        false,
        llm_runtime::acp::options::AcpReplayStrategy::Discard,
        3,
    );
    assert_eq!(prompt.matches("TRUSTED_AGENT_RULE").count(), 1);
    assert_eq!(prompt.matches("SCOPED_SKILL_CONTENT").count(), 0);
    assert_eq!(parsed.skill_mounts.len(), 1);
    assert_eq!(
        std::path::Path::new(&parsed.skill_mounts[0].path),
        skill_dir.canonicalize().unwrap()
    );
    // A protocol-error peer records the real JSON-RPC request without simulating skill discovery.
    let script = temp.path().join("mount-error-peer.py");
    let capture = temp.path().join("mount-requests.jsonl");
    std::fs::write(&script, r#"
import json, sys
for line in sys.stdin:
    r = json.loads(line)
    if 'method' not in r or 'id' not in r: continue
    with open(sys.argv[1], 'a') as f: f.write(line)
    if r['method'] == 'session/prompt':
        response = {'jsonrpc':'2.0','id':r['id'],'error':{'code':-32602,'message':'intentional protocol error'}}
    else:
        result = {'protocolVersion':1,'agentCapabilities':{},'authMethods':[]} if r['method'] == 'initialize' else {'sessionId':'mount-error'}
        response = {'jsonrpc':'2.0','id':r['id'],'result':result}
    print(json.dumps(response), flush=True)
"#).unwrap();
    let view = llm_runtime::acp::skill_mounts::MountedSkills::prepare(
        &parsed.skill_mounts,
        temp.path(),
        "mount-error",
        &cfg.skills.security,
        &Default::default(),
    )
    .unwrap()
    .unwrap();
    let selection =
        acp_types::SessionMountSelection { version: 1, skills: parsed.skill_mounts.clone() };
    let client = acp_client::AcpClient::new(
        "protocol-error-peer",
        acp_types::AcpAgentConfig {
            command: "python3".into(),
            args: vec![script.display().to_string(), capture.display().to_string()],
            env: Default::default(),
        },
    )
    .with_skill_mounts(Some(selection), Some(view.0));
    let result = client
        .run_prompt(acp_types::PromptRequest::new(temp.path().to_path_buf(), prompt), &mut |_| {})
        .await;
    assert!(result.is_err());
    client.close().await.unwrap();
    let requests: Vec<serde_json::Value> = std::fs::read_to_string(capture)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let new = requests.iter().find(|request| request["method"] == "session/new").unwrap();
    assert_eq!(
        new["params"]["_meta"][acp_types::SESSION_SKILL_MOUNTS_META_KEY]["skills"][0]["name"],
        "selected"
    );
    let first =
        requests.iter().find(|request| request["method"] == "session/prompt").unwrap().to_string();
    assert!(first.contains("TRUSTED_AGENT_RULE"));
    assert!(!first.contains("SCOPED_SKILL_CONTENT"));
    let again = apply_agent_defaults(
        &cfg,
        &options,
        "codex",
        temp.path(),
        &mut parsed,
        &resolved,
    )
    .await
    .unwrap();
    assert_eq!(resolved, again);
    STATE.with(|state| state.borrow_mut().mcp_servers = acp_config::parse_mcp_servers(&json!([{"name":"selected-mcp","command":"fixture-mcp"}]), "fixture").unwrap());
    cfg.agents.get_mut("acp-codex").unwrap().defaults.mcp =
        config_agent_profile::AgentDefault::Fixed(vec!["selected-mcp".into()]);
    apply_agent_defaults(
        &cfg,
        &options,
        "codex",
        temp.path(),
        &mut parsed,
        &messages,
    )
    .await
    .unwrap();
    assert_eq!(parsed.mcp_servers.len(), 1);
    assert_eq!(mcp_client::server_name(&parsed.mcp_servers[0]), "selected-mcp");
    STATE.with(|state| {
        let mut rule = acp_config::McpServerManagement::default_for_server("selected-mcp");
        rule.enabled = false;
        state.borrow_mut().mcp_server_management.push(rule);
    });
    let error = apply_agent_defaults(
        &cfg,
        &options,
        "codex",
        temp.path(),
        &mut parsed,
        &messages,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("MCP capability is disabled"));
    cfg.agents.get_mut("acp-codex").unwrap().defaults.mcp =
        config_agent_profile::AgentDefault::Fixed(vec![]);
    std::fs::write(agent_skills::local::local_skill_disabled_marker_path(&skill_dir), "").unwrap();
    assert!(
        apply_agent_defaults(
            &cfg,
            &options,
            "codex",
            temp.path(),
            &mut parsed,
            &messages
        )
        .await
        .is_err()
    );
    let agent = cfg.agents.get_mut("acp-codex").unwrap();
    agent.defaults.skills = config_agent_profile::AgentDefault::Fixed(vec![]);
    agent.defaults.permission = config_agent_profile::AgentDefault::Fixed(json!({"*":"allow"}));
    let error = apply_agent_defaults(
        &cfg,
        &options,
        "codex",
        temp.path(),
        &mut parsed,
        &messages,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("ACP does not support this fixed permission policy"));
    cfg.agents.get_mut("acp-codex").unwrap().defaults.permission =
        config_agent_profile::AgentDefault::Fixed(json!({"*":"deny"}));
    apply_agent_defaults(
        &cfg,
        &options,
        "codex",
        temp.path(),
        &mut parsed,
        &messages,
    )
    .await
    .unwrap();
    assert_eq!(parsed.permission_mode, Some(acp_types::PermissionMode::DenyAll));
    assert!(parsed.skill_mounts.is_empty());
    }).await;
}
