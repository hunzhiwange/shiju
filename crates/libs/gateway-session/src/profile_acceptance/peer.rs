use super::*;

// Keep the real profile resolver and gateway transport; only the external ACP peer is controlled.
pub(super) fn assert_acp_profile<H: ProfileAcceptanceHost>(
    runtime: &tokio::runtime::Runtime,
    workspace: &std::path::Path,
    backend: &str,
) {
    let profile = format!("acp-{backend}");
    let peer = workspace.join("acceptance-peer.py");
    let capture = workspace.join("acceptance-acp.jsonl");
    std::fs::write(&peer, r#"
import json, sys
for line in sys.stdin:
    r=json.loads(line)
    if 'id' not in r or 'method' not in r: continue
    with open(sys.argv[1], 'a') as f: f.write(line)
    m=r['method']
    if m=='initialize': result={'protocolVersion':1,'agentCapabilities':{'_meta':{'compass.owner-approval.v1':True}},'_meta':{'ownerApprovalRequired':True},'authMethods':[]}
    elif m=='session/new': result={'sessionId':'acceptance','_meta':{'ownerApprovalRequired':True}}
    elif m=='session/prompt':
        print(json.dumps({'jsonrpc':'2.0','id':900,'method':'fs/write_text_file','params':{'sessionId':'acceptance','path':sys.argv[1]+'.denied','content':'must not write'}}),flush=True)
        permission=json.loads(sys.stdin.readline())
        if 'error' not in permission: raise Exception('fixed permission must deny writes')
        print(json.dumps({'jsonrpc':'2.0','method':'session/update','params':{'sessionId':'acceptance','update':{'sessionUpdate':'agent_message_chunk','content':{'type':'text','text':'验收回复'}}}}),flush=True)
        result={'stopReason':'end_turn'}
    else: result={}
    print(json.dumps({'jsonrpc':'2.0','id':r['id'],'result':result}),flush=True)
"#).unwrap();
    std::fs::write(
        workspace.join(".vwacprc.json"),
        serde_json::to_vec(&json!({
            "mcpServers": if matches!(backend, "dirac" | "nova" | "sigit" | "amp-acp") { json!([]) } else if backend == "dimcode" { json!([{"name":"selected","type":"http","url":"https://example.test/mcp"}]) } else { json!([{"name":"selected","command":"controlled-mcp"}]) }
        }))
        .unwrap(),
    )
    .unwrap();
    // Only the external process is replaced. The native receipt verifier, catalog,
    // persisted configuration and gateway routing remain production code.
    let command = if backend == "amp-acp" {
        std::env::var("SHIJU_AMP_REAL_ENTRY")
            .expect("Amp acceptance requires the installed reviewed runtime")
    } else if backend == "sigit" {
        use sha2::{Digest, Sha256};
        let entry = workspace.join("sigit");
        std::fs::write(&entry, format!(
            "#!/usr/bin/env python3\nimport runpy,sys\nsys.argv={}\nrunpy.run_path(sys.argv[0],run_name='__main__')\n",
            json!([peer, capture])
        )).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&entry, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let contracts: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../acp-agent-registry/src/managed_runtime/contracts.json"
        )))
        .unwrap();
        std::fs::write(
            workspace.join("build-receipt.json"),
            serde_json::to_vec(&json!({
                "version": contracts["sigit"]["version"],
                "source_sha256": contracts["sigit"]["source_sha256"],
                "patch_sha256": contracts["sigit"]["patch_sha256"],
                "sha256": hex::encode(Sha256::digest(std::fs::read(&entry).unwrap()))
            }))
            .unwrap(),
        )
        .unwrap();
        entry.to_string_lossy().into_owned()
    } else {
        "python3".to_owned()
    };
    runtime
        .block_on(agent_config::runtime::update(json!({
            "acp":{backend:{"command":command,"args":if backend == "sigit" { json!(["--acp"]) } else if backend == "amp-acp" { json!([]) } else { json!([peer,capture]) },"env":{}}},
            "agents":{profile.clone():{
                "label":"已启用 ACP 验收", "enabled":true,"system_prompt":"ACP_ACCEPTANCE_RULE",
                "defaults":{
                    "permission":{"mode":"fixed","value":{"*":"deny"}},
                    "skills":{"mode":"fixed","value":[]},
                    "mcp":{"mode":"fixed","value":if matches!(backend, "dirac" | "nova" | "sigit" | "amp-acp") { json!([]) } else { json!(["selected"]) }}
                }
            }}
        })))
        .unwrap();
    if backend == "nova" && std::env::var_os("SHIJU_NOVA_REAL_PROFILE").is_some() {
        std::fs::write(&peer, "import runpy,os\nrunpy.run_path(os.environ['SHIJU_NOVA_TEST_FIXTURE'],run_name='__main__')\n").unwrap();
        runtime.block_on(agent_config::runtime::update(json!({"acp":{"nova":{
            "command":"python3","args":[peer,capture],"env":{
                "HOME":std::env::var("SHIJU_NOVA_TEST_HOME").unwrap(),
                "SHIJU_NOVA_TEST_ENTRY":std::env::var("SHIJU_NOVA_TEST_ENTRY").unwrap(),
                "SHIJU_NOVA_TEST_FIXTURE":concat!(env!("CARGO_MANIFEST_DIR"),"/../../vw-acp/tests/fixtures/nova_real_peer.py"),
                "SHIJU_NOVA_TEST_CAPTURE":capture,
                "SHIJU_NOVA_TEST_EXPECTED_MODEL":"claude-sonnet-4-6"
            }
        }}}))).unwrap();
    }
    if backend == "sigit" && std::env::var_os("SHIJU_SIGIT_REAL_ENTRY").is_some() {
        runtime
            .block_on(agent_config::runtime::update(json!({"acp":{"sigit":{
                "command": std::env::var("SHIJU_SIGIT_REAL_ENTRY").unwrap(), "args":["--acp"],
                "env": {
                    "HOME":workspace.join("isolated-home"),
                    "SIGIT_CONFIG_DIR":workspace.join("isolated-state"),
                    "SIGIT_MCP":"off", "ONDE_DISABLE_PULSE":"1",
                    "OPENAI_BASE_URL":std::env::var("SHIJU_SIGIT_REAL_ENDPOINT").unwrap(),
                    "OPENAI_API_KEY":"synthetic-only", "SIGIT_MODEL":"preset-selected-model"
                }
            }}})))
            .unwrap();
    }
    let directory = workspace.to_string_lossy().into_owned();
    let rejects_mcp = matches!(backend, "dirac" | "nova" | "sigit" | "amp-acp");
    let backend = backend.to_owned();
    let binding_directory = directory.clone();
    runtime
        .block_on(H::with_tenant_instance(
            H::current_tenant_scope(),
            directory,
            move || {
                Box::pin(async move {
                    let config = H::config().await.unwrap();
                    assert_eq!(
                        std::path::Path::new(&config.acp[&backend].command)
                            .file_name()
                            .unwrap()
                            .to_string_lossy(),
                        if backend == "sigit" {
                            "sigit"
                        } else if backend == "amp-acp" {
                            "amp-acp"
                        } else {
                            "python3"
                        },
                        "only the owned peer may launch"
                    );
                    if backend == "sigit" {
                        assert_eq!(config.acp[&backend].args, ["--acp"]);
                    } else if backend == "amp-acp" {
                        assert!(config.acp[&backend].args.is_empty());
                    } else {
                        assert!(config.acp[&backend].args[0].ends_with("acceptance-peer.py"));
                    }
                    let key = format!("acp-{backend}");
                    let catalog = agent_config::catalog::merged_agent_catalog(&config.agents);
                    assert!(catalog[&key].builtin && catalog[&key].enabled);
                    assert_eq!(catalog[&key].system_prompt.as_deref(), Some("ACP_ACCEPTANCE_RULE"));
                    let (binding, _) = session_engine::admission::prepare::<H::Admission>(
                        &format!("ses_binding_{}", uuid::Uuid::new_v4()),
                        Some(&key),
                        None,
                        None,
                        std::path::Path::new(&binding_directory),
                    )
                    .await
                    .unwrap();
                    assert_eq!(
                        serde_json::to_value(binding.runtime).unwrap(),
                        json!({"kind":"acp","backend":backend})
                    );
                    Ok(())
                })
            },
        ))
        .unwrap();
    let events = runtime.block_on(send::<H>(
        workspace,
        &format!("ses_acp_acceptance_{}", uuid::Uuid::new_v4()),
        &profile,
    ));
    assert!(events.contains("chat.done"), "{events}");
    assert!(!events.contains("chat.error"), "{events}");
    assert!(events.contains("验收回复"), "{events}");
    assert!(!capture.with_extension("jsonl.denied").exists());
    if profile == "acp-amp-acp" {
        let output = workspace.join("amp-preset-written.txt");
        assert!(!output.exists(), "denied preset must not write");
        runtime
            .block_on(agent_config::runtime::update(json!({"agents":{profile.clone():{
                "defaults":{"permission":{"mode":"inherit"}}
            }}})))
            .unwrap();
        let response = runtime.block_on(H::chat(
            Query(InstanceQuery { directory: Some(workspace.to_string_lossy().into_owned()) }),
            HeaderMap::new(),
            Json(GatewayChatStreamRequest {
                session_id: Some(format!("ses_amp_allowed_{}", uuid::Uuid::new_v4()).into()),
                agent: Some(profile.clone().into()),
                model: Some("model-recovery/original".into()),
                messages: vec![json!({"role":"user","content":"验收宿主写入"})],
                options: Some(json!({"full_access":true,"cwd":workspace,"acp_model_source":"session_model","acp_close_client_after_request":true})),
                ..Default::default()
            }),
        )).unwrap().into_response();
        let allowed = runtime.block_on(async {
            String::from_utf8(
                tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    to_bytes(response.into_body(), 2_000_000),
                )
                .await
                .unwrap()
                .unwrap()
                .to_vec(),
            )
            .unwrap()
        });
        assert!(allowed.contains("chat.done") && !allowed.contains("chat.error"), "{allowed}");
        assert_eq!(std::fs::read_to_string(output).unwrap(), "preset-host-write");
    }
    if profile == "acp-amp-acp"
        || (profile == "acp-sigit" && std::env::var_os("SHIJU_SIGIT_REAL_ENTRY").is_some())
    {
        return; // Native model-wire evidence is owned by the separate loopback fixture.
    }

    let requests: Vec<Value> = std::fs::read_to_string(capture)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let new = requests.iter().find(|r| r["method"] == "session/new").unwrap();
    if profile == "acp-nova" {
        let initialize = requests.iter().find(|r| r["method"] == "initialize").unwrap();
        assert_eq!(
            initialize["params"]["clientCapabilities"]["_meta"]["compass.owner-approval.v1"],
            true
        );
        assert_eq!(new["params"]["_meta"]["compass.owner-approval.v1"], true);
        assert_eq!(new["params"]["_meta"]["compass.peers-disabled"], true);
    }
    if rejects_mcp {
        assert_eq!(new["params"]["mcpServers"], json!([]));
    } else {
        assert_eq!(new["params"]["mcpServers"][0]["name"], "selected");
    }
    let prompt = requests.iter().find(|r| r["method"] == "session/prompt").unwrap();
    assert!(prompt["params"]["prompt"].to_string().contains("ACP_ACCEPTANCE_RULE"));
}
