//! 通过真实网关流入口验证配置落盘、身份固化和能力撤销。
use crate::gateway::instance::InstanceQuery;
use axum::{Json, body::to_bytes, extract::Query, http::HeaderMap, response::IntoResponse};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use vw_api_types::chat::GatewayChatStreamRequest;

async fn send(workspace: &std::path::Path, session: &str, agent: &str) -> String {
    let response = super::stream::chat_stream(
        Query(InstanceQuery { directory: Some(workspace.to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(session.to_owned().into()),
            agent: Some(agent.into()),
            model: (agent == "acp-amp-acp").then(|| "model-recovery/original".into()),
            messages: vec![json!({"role":"user","content":"验收运行"})],
            options: Some(if agent == "main" { json!({"full_access":false}) } else { json!({"full_access":false,"cwd":workspace,"acp_model_source":if agent == "acp-amp-acp" { "session_model" } else { "cli_local" },"acp_close_client_after_request":agent != "acp-deepagents"}) }),
            ..Default::default()
        }),
    )
    .await
    .unwrap()
    .into_response();
    let bytes = if agent == "acp-amp-acp" {
        use futures_util::StreamExt;
        let mut stream = response.into_body().into_data_stream();
        let mut bytes = Vec::new();
        let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while let Some(chunk) = stream.next().await {
                bytes.extend_from_slice(&chunk.unwrap());
                assert!(bytes.len() < 2_000_000);
            }
        })
        .await;
        assert!(result.is_ok(), "Amp gateway timed out: {}", String::from_utf8_lossy(&bytes));
        bytes
    } else {
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            to_bytes(response.into_body(), 2_000_000),
        )
        .await
        .unwrap()
        .unwrap()
        .to_vec()
    };
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn assert_completed_turn(
    runtime: &tokio::runtime::Runtime,
    workspace: &std::path::Path,
    session: &str,
) {
    let events = runtime.block_on(send(workspace, session, "main"));
    assert!(events.contains("chat.done"), "{events}");
    assert!(events.contains("验收回复"), "{events}");
    assert!(!events.contains("chat.error"), "{events}");
}

#[test]
fn agent_profile_acceptance_runs_selected_profile_end_to_end() {
    crate::env::provide_with(
        std::collections::HashMap::from([(
            "VW_MODEL_RECOVERY_TEST_KEY".into(),
            "fixture-key".into(),
        )]),
        run_profile_acceptance,
    );
}

fn run_profile_acceptance() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    struct HttpMode(Option<std::ffi::OsString>);
    impl Drop for HttpMode {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _http = HttpMode(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    // Reuse the existing test-only real transport switch and restore it on panic.
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    if let Ok(workspace) = std::env::var("SHIJU_ACCEPTANCE_RESTART_WORKSPACE") {
        let session = std::env::var("SHIJU_ACCEPTANCE_RESTART_SESSION").unwrap();
        assert_completed_turn(&runtime, std::path::Path::new(&workspace), &session);
        return;
    }
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let captured = Arc::clone(&requests);
    let router = axum::Router::new().route("/v1/chat/completions", axum::routing::post(move |Json(body): Json<Value>| {
        let captured = Arc::clone(&captured);
        async move {
            captured.lock().unwrap().push(body);
            let chunk = json!({"id":"acceptance","object":"chat.completion.chunk","created":0,"model":"original","choices":[{"index":0,"delta":{"content":"验收回复"},"finish_reason":"stop"}]});
            ([("content-type", "text/event-stream")], format!("data: {chunk}\n\ndata: [DONE]\n\n"))
        }
    }));
    let listener = runtime.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
    let address = listener.local_addr().unwrap();
    let server = runtime.spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            &format!("http://{address}/v1"),
            Some("fixture-key"),
        ));
    let workspace = tempfile::tempdir().unwrap();
    let session = format!("ses_acceptance_{}", uuid::Uuid::new_v4());
    for rule in ["ACCEPTANCE_ORIGINAL_RULE", "ACCEPTANCE_EDITED_RULE"] {
        runtime.block_on(crate::config::update(json!({"agents":{"main":{
            "label":"中文验收助手", "description":"持久化验收", "recommended_prompts":["检查项目"],
            "system_prompt":rule,
            "defaults":{
                "model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}},
                "skills":{"mode":"fixed","value":[]}, "mcp":{"mode":"fixed","value":[]}
            }
        }}}))).unwrap();
        assert_completed_turn(&runtime, workspace.path(), &session);
    }
    // A fresh process cannot reuse an in-memory binding or config cache.
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "gateway::api::handlers::session::agent_profile_acceptance_tests::agent_profile_acceptance_runs_selected_profile_end_to_end", "--nocapture"])
        .env("SHIJU_ACCEPTANCE_RESTART_WORKSPACE", workspace.path())
        .env("SHIJU_ACCEPTANCE_RESTART_SESSION", &session)
        .output().unwrap();
    assert!(
        child.status.success(),
        "restart failed: {}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    assert_acp_profile(&runtime, workspace.path(), "codex");
    server.abort();
    let captured = requests.lock().unwrap();
    assert_eq!(captured.len(), 3, "two live turns plus one fresh-process turn");
    for request in captured.iter() {
        assert_eq!(request["model"], "original");
        let body = request["messages"].to_string();
        assert!(body.contains("ACCEPTANCE_ORIGINAL_RULE"));
        assert!(!body.contains("ACCEPTANCE_EDITED_RULE"));
    }
}

#[tokio::test]
async fn agent_profile_acceptance_rejects_revoked_capabilities() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let workspace = tempfile::tempdir().unwrap();
    let mcp = workspace.path().join(".vwacprc.json");
    std::fs::write(
        &mcp,
        serde_json::to_vec(
            &json!({"mcpServers":[{"name":"revoked","command":"missing-command","enabled":true}]}),
        )
        .unwrap(),
    )
    .unwrap();
    crate::config::update(
        json!({"agents":{"main":{"defaults":{"mcp":{"mode":"fixed","value":["revoked"]}}}}}),
    )
    .await
    .unwrap();
    let session = format!("ses_revoked_{}", uuid::Uuid::new_v4());
    let selected_session = session.clone();
    let directory = workspace.path().to_string_lossy().into_owned();
    super::shared::with_tenant_instance(
        crate::session::ui_store::current_tenant_scope(),
        directory.clone(),
        move || {
            Box::pin(async move {
                let (binding, _) = crate::session::agent_binding::prepare(
                    &selected_session,
                    Some("main"),
                    None,
                    None,
                    std::path::Path::new(&directory),
                )
                .await
                .unwrap();
                assert_eq!(
                    serde_json::to_value(binding.defaults.mcp).unwrap(),
                    json!({"mode":"fixed","value":["revoked"]})
                );
                Ok(())
            })
        },
    )
    .await
    .unwrap();
    std::fs::write(
        &mcp,
        serde_json::to_vec(
            &json!({"mcpServers":[{"name":"revoked","command":"missing-command","enabled":false}]}),
        )
        .unwrap(),
    )
    .unwrap();
    let result = super::stream::chat_stream(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(session.into()),
            agent: Some("main".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    )
    .await;
    assert!(result.is_err(), "revoked MCP must reject before model or command execution");
}

// Keep the real profile resolver and gateway transport; only the external ACP peer is controlled.
fn assert_acp_profile(
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
            "/../vw-acp/src/managed_runtime/contracts.json"
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
        .block_on(crate::config::update(json!({
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
        runtime.block_on(crate::config::update(json!({"acp":{"nova":{
            "command":"python3","args":[peer,capture],"env":{
                "HOME":std::env::var("SHIJU_NOVA_TEST_HOME").unwrap(),
                "SHIJU_NOVA_TEST_ENTRY":std::env::var("SHIJU_NOVA_TEST_ENTRY").unwrap(),
                "SHIJU_NOVA_TEST_FIXTURE":concat!(env!("CARGO_MANIFEST_DIR"),"/../vw-acp/tests/fixtures/nova_real_peer.py"),
                "SHIJU_NOVA_TEST_CAPTURE":capture,
                "SHIJU_NOVA_TEST_EXPECTED_MODEL":"claude-sonnet-4-6"
            }
        }}}))).unwrap();
    }
    if backend == "sigit" && std::env::var_os("SHIJU_SIGIT_REAL_ENTRY").is_some() {
        runtime
            .block_on(crate::config::update(json!({"acp":{"sigit":{
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
        .block_on(super::shared::with_tenant_instance(
            crate::session::ui_store::current_tenant_scope(),
            directory,
            move || {
                Box::pin(async move {
                    let config = crate::config::get_current().await.unwrap();
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
                    let catalog = crate::agent::catalog::merged_agent_catalog(&config.agents);
                    assert!(catalog[&key].builtin && catalog[&key].enabled);
                    assert_eq!(catalog[&key].system_prompt.as_deref(), Some("ACP_ACCEPTANCE_RULE"));
                    let (binding, _) = crate::session::agent_binding::prepare(
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
    let events = runtime.block_on(send(
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
            .block_on(crate::config::update(json!({"agents":{profile.clone():{
                "defaults":{"permission":{"mode":"inherit"}}
            }}})))
            .unwrap();
        let response = runtime.block_on(super::stream::chat_stream(
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

#[tokio::test]
async fn agent_profile_acceptance_catalog_uses_request_tenant() {
    use axum::{body::Body, extract::Extension, http::Request};
    use tower::ServiceExt;
    use vw_api_types::id::TenantScope;

    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    crate::config::update_global_for_tenant(
        &TenantScope::default(),
        json!({
            "agents":{"main":{"label":"DEFAULT_CATALOG_SENTINEL"}}
        }),
    )
    .await
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    for platform in ["acceptance-one", "acceptance-two"] {
        let tenant = TenantScope::new(platform, "validation").unwrap();
        crate::config::update_global_for_tenant(
            &tenant,
            json!({
                "agents":{"main":{"label":platform}}
            }),
        )
        .await
        .unwrap();
        let response = super::super::misc::router::<()>()
            .layer(Extension(tenant))
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/agent?include_disabled=true&directory={}",
                        directory.path().display()
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = to_bytes(response.into_body(), 2_000_000).await.unwrap();
        let agents: Vec<Value> = serde_json::from_slice(&body).unwrap();
        let main = agents.iter().find(|entry| entry["key"] == "main").unwrap();
        assert_eq!(
            main["label"], platform,
            "catalog must use the request tenant, not the ambient default"
        );
    }
}

#[tokio::test]
async fn agent_config_cleanup_global_profile_roundtrips_to_new_session() {
    use axum::{body::Body, extract::Extension, http::Request};
    use tower::ServiceExt;
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for (key, runtime) in
        [("main", json!({"kind":"native"})), ("acp-codex", json!({"kind":"acp","backend":"codex"}))]
    {
        let patch = json!({"agents":{key:{
            "enabled":true,"runtime":runtime,"description":"中文描述\n第二行",
            "system_prompt":"GLOBAL_PROFILE_RULE",
            "defaults":{"full_access":true,"permission":{"mode":"fixed","value":{"*":"deny"}},
                "model":{"mode":"fixed","value":{"provider":"model-recovery","model":"original"}},
                "skills":{"mode":"fixed","value":[]},"mcp":{"mode":"fixed","value":[]}}
        }}});
        let response = super::super::config::router::<()>()
            .layer(Extension(crate::session::ui_store::current_tenant_scope()))
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/config")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&patch).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let saved: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 2_000_000).await.unwrap())
                .unwrap();
        assert_eq!(saved["agents"][key]["defaults"]["full_access"], true);
        for workspace in [first.path(), second.path()] {
            let runtime = runtime.clone();
            let directory = workspace.to_string_lossy().into_owned();
            super::shared::with_tenant_instance(
                crate::session::ui_store::current_tenant_scope(),
                directory.clone(),
                move || {
                    Box::pin(async move {
                        let config = crate::config::get_current().await.unwrap();
                        let profile = &config.agents[key];
                        assert_eq!(profile.description.as_deref(), Some("中文描述\n第二行"));
                        let session = format!("ses_cleanup_{}", uuid::Uuid::new_v4());
                        let (binding, _) = crate::session::agent_binding::prepare(
                            &session,
                            Some(key),
                            None,
                            None,
                            std::path::Path::new(&directory),
                        )
                        .await
                        .unwrap();
                        assert_eq!(binding.system_prompt.as_deref(), Some("GLOBAL_PROFILE_RULE"));
                        assert_eq!(serde_json::to_value(&binding.runtime).unwrap(), runtime);
                        assert_eq!(
                            serde_json::to_value(&binding.defaults).unwrap(),
                            serde_json::to_value(&profile.defaults).unwrap()
                        );
                        assert!(
                            crate::session::agent_binding::resolve_full_access_scoped(
                                &directory,
                                &session,
                                None,
                                binding.defaults.full_access,
                            )
                            .await
                            .unwrap()
                        );
                        assert!(
                            !crate::session::agent_binding::resolve_full_access_scoped(
                                &directory,
                                &session,
                                Some(false),
                                true,
                            )
                            .await
                            .unwrap()
                        );
                        assert!(
                            !crate::session::agent_binding::resolve_full_access_scoped(
                                &directory, &session, None, true,
                            )
                            .await
                            .unwrap()
                        );
                        Ok(())
                    })
                },
            )
            .await
            .unwrap();
        }
    }
}

#[test]
fn acp_onboarding_autohand_builtin_profile_persists_and_runs() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile(&runtime, workspace.path(), "autohand");
    runtime
        .block_on(crate::config::update(json!({"agents":{"acp-autohand":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(super::stream::chat_stream(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-autohand".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

#[test]
#[ignore = "requires reviewed DeepAgents runtime and controlled loopback model"]
fn acp_onboarding_deepagents_builtin_profile_persists_and_runs() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    let entry = std::env::var("SHIJU_DEEPAGENTS_TEST_ENTRY").unwrap();
    vw_acp::verify_reviewed_agent_executable("deepagents", &entry).unwrap();
    let endpoint = std::env::var("SHIJU_DEEPAGENTS_TEST_ENDPOINT").unwrap();
    assert!(endpoint.starts_with("http://127.0.0.1:"));
    let previous = std::env::var_os("VW_RECOVERY_WINDOW_HTTP");
    struct Restore(Option<std::ffi::OsString>);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _restore = Restore(previous);
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    runtime.block_on(crate::config::update(json!({
        "acp":{"deepagents":{
            "command":entry,
            "args":["--model","openai:gpt-4o","--workspace",workspace.path()],
            "env":{"OPENAI_API_KEY":"local-fixture","OPENAI_BASE_URL":endpoint,"LANGSMITH_TRACING":"false","HOME":workspace.path()}
        }},
        "agents":{"acp-deepagents":{
            "enabled":true,"label":"DeepAgents acceptance",
            "defaults":{
                "permission":{"mode":"fixed","value":{"*":"deny"}},
                "skills":{"mode":"fixed","value":[]},
                "mcp":{"mode":"fixed","value":[]}
            }
        }}
    }))).unwrap();
    let directory = workspace.path().to_string_lossy().into_owned();
    runtime
        .block_on(super::shared::with_tenant_instance(
            crate::session::ui_store::current_tenant_scope(),
            directory,
            || {
                Box::pin(async {
                    let config = crate::config::get_current().await.unwrap();
                    let catalog = crate::agent::catalog::merged_agent_catalog(&config.agents);
                    assert!(catalog["acp-deepagents"].builtin && catalog["acp-deepagents"].enabled);
                    assert_eq!(
                        catalog["acp-deepagents"].label.as_deref(),
                        Some("DeepAgents acceptance")
                    );
                    assert_eq!(
                        catalog["acp-deepagents"].runtime,
                        vw_config_types::agent::AgentRuntime::Acp { backend: "deepagents".into() }
                    );
                    Ok(())
                })
            },
        ))
        .unwrap();
    let session = format!("ses_deepagents_acceptance_{}", uuid::Uuid::new_v4());
    for _ in 0..2 {
        let events = runtime.block_on(send(workspace.path(), &session, "acp-deepagents"));
        assert!(events.contains("chat.done"), "{events}");
        assert!(!events.contains("chat.error"), "{events}");
        assert!(events.contains("验收回复"), "{events}");
    }
    runtime
        .block_on(crate::config::update(json!({"agents":{"acp-deepagents":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(super::stream::chat_stream(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            agent: Some("acp-deepagents".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err());
}
#[test]
fn acp_onboarding_dimcode_builtin_profile_persists_and_runs() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile(&runtime, workspace.path(), "dimcode");
    runtime
        .block_on(crate::config::update(json!({"agents":{"acp-dimcode":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(super::stream::chat_stream(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-dimcode".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

#[test]
fn acp_onboarding_dirac_builtin_profile_persists_and_runs() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile(&runtime, workspace.path(), "dirac");
    runtime
        .block_on(crate::config::update(json!({"agents":{"acp-dirac":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(super::stream::chat_stream(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-dirac".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

#[test]
fn acp_onboarding_glm_acp_agent_builtin_profile_persists_and_runs() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile(&runtime, workspace.path(), "glm-acp-agent");
    runtime
        .block_on(crate::config::update(json!({"agents":{"acp-glm-acp-agent":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(super::stream::chat_stream(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-glm-acp-agent".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

#[test]
fn acp_onboarding_grok_builtin_profile_persists_and_runs() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile(&runtime, workspace.path(), "grok");
    runtime
        .block_on(crate::config::update(json!({"agents":{"acp-grok":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(super::stream::chat_stream(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-grok".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

#[test]
fn acp_onboarding_nova_builtin_profile_persists_and_runs() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile(&runtime, workspace.path(), "nova");
    runtime
        .block_on(crate::config::update(json!({"agents":{"acp-nova":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(super::stream::chat_stream(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-nova".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

#[test]
#[ignore = "requires the official Nova package and authorized isolated local consent"]
fn acp_onboarding_nova_real_builtin_profile_persists_and_runs() {
    assert!(std::env::var_os("SHIJU_NOVA_REAL_PROFILE").is_some());
    acp_onboarding_nova_builtin_profile_persists_and_runs();
}

#[test]
fn acp_onboarding_sigit_builtin_profile_persists_and_runs() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            "https://example.test/v1",
            None,
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile(&runtime, workspace.path(), "sigit");
    runtime
        .block_on(crate::config::update(json!({"agents":{"acp-sigit":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(super::stream::chat_stream(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-sigit".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}

#[test]
#[ignore = "requires the reviewed siGit build and an owned loopback model endpoint"]
fn acp_onboarding_sigit_real_builtin_profile_persists_and_runs() {
    assert!(std::env::var_os("SHIJU_SIGIT_REAL_ENTRY").is_some());
    acp_onboarding_sigit_builtin_profile_persists_and_runs();
}

#[test]
#[ignore = "requires the reviewed Amp runtime and an owned loopback model fixture"]
fn acp_onboarding_amp_acp_builtin_profile_persists_and_runs() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _env = runtime.block_on(crate::test_support::global_async_env_lock().lock());
    let _fixture =
        runtime.block_on(crate::session::llm::recovery_test_support::RecoveryConfig::new(
            &format!(
                "{}v1/chat/completions",
                std::env::var("SHIJU_AMP_REAL_ENDPOINT").expect("owned Amp model fixture")
            ),
            Some("fixture-model-token"),
        ));
    struct RealTransport(Option<std::ffi::OsString>);
    impl Drop for RealTransport {
        fn drop(&mut self) {
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("VW_RECOVERY_WINDOW_HTTP", value),
                    None => std::env::remove_var("VW_RECOVERY_WINDOW_HTTP"),
                }
            }
        }
    }
    let _transport = RealTransport(std::env::var_os("VW_RECOVERY_WINDOW_HTTP"));
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    runtime
        .block_on(crate::config::update(
            json!({"providers":{"model-recovery":{"adapter":"openai-compatible","models":{"original":{"tool_call":true}}}}}),
        ))
        .unwrap();
    runtime.block_on(crate::provider::provider::invalidate_cache());
    let entry = std::path::PathBuf::from(
        std::env::var("SHIJU_AMP_REAL_ENTRY").expect("reviewed Amp runtime"),
    );
    let installation =
        entry.parent().and_then(std::path::Path::parent).and_then(std::path::Path::parent).unwrap();
    let directory = std::path::PathBuf::from(std::env::var_os("VIBEWINDOW_CONFIG_DIR").unwrap())
        .join("acp-runtime");
    std::fs::create_dir_all(&directory).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(installation, directory.join("amp-acp-0.12.0-model-source-v1"))
        .unwrap();
    let workspace = tempfile::tempdir().unwrap();
    assert_acp_profile(&runtime, workspace.path(), "amp-acp");
    runtime
        .block_on(crate::config::update(json!({"agents":{"acp-amp-acp":{"enabled":false}}})))
        .unwrap();
    let disabled = runtime.block_on(super::stream::chat_stream(
        Query(InstanceQuery { directory: Some(workspace.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(GatewayChatStreamRequest {
            session_id: Some(format!("ses_disabled_{}", uuid::Uuid::new_v4()).into()),
            agent: Some("acp-amp-acp".into()),
            messages: vec![json!({"role":"user","content":"must reject"})],
            ..Default::default()
        }),
    ));
    assert!(disabled.is_err(), "disabled preset must reject before execution");
}
