//! Both public execution paths use the real tool factory and child Gateway loops.
use serde_json::{Value, json};
use gateway_session::child_acceptance::{HttpMode, ParentModel, start_entry_server};
use std::{
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[tokio::test]
async fn agcollab_end_to_end_gateway_and_query_engine_entries() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let http = HttpMode(
        std::env::var_os("VW_RECOVERY_WINDOW_HTTP"),
        agent_env::get("VW_AGCOLLAB_ENTRY_KEY"),
    );
    unsafe {
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    agent_env::set("VW_AGCOLLAB_ENTRY_KEY", "synthetic-key");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        &format!("http://{address}/v1"),
        Some("synthetic-key"),
    )
    .await;
    crate::config::update(json!({"providers":{"model-recovery":{"env":["VW_AGCOLLAB_ENTRY_KEY"],"models":{"original":{"status":"active","tool_call":true}}}}})).await.unwrap();
    crate::provider::provider::invalidate_cache().await;
    let stopped = Arc::new(AtomicBool::new(false));
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let root = Arc::new(Mutex::new(String::new()));
    let server = start_entry_server(listener, stopped.clone(), requests.clone(), root.clone());
    for gateway in [true, false] {
        let workspace = tempfile::tempdir().unwrap();
        crate::config::update_project(workspace.path(),json!({"agents":{
            "main":{"provider":"model-recovery","model":"original","allowed_tools":["AgentTool"],"max_iterations":12},
            "worker":{"provider":"model-recovery","model":"original","allowed_tools":[],"allow_as_subagent":true}
        }})).await.unwrap();
        let directory = workspace.path().to_string_lossy().to_string();
        let parent = gateway_session::child_acceptance::create_collaboration_parent::<
            crate::session::session_io::NativeSessionIo,
        >(directory.clone()).await;
        *root.lock().unwrap() = parent.id.clone();
        let before = requests.lock().unwrap().len();
        let result = if gateway {
            crate::gateway::api::handlers::session::stream::collect_gateway_chat_stream(
                directory.clone(),
                vw_api_chat::chat::GatewayChatStreamRequest {
                    session_id: Some(parent.id.clone().into()),
                    messages: vec![json!({"role":"user","content":"ROOT_ENTRY_WORKFLOW"})],
                    agent: Some("main".into()),
                    allowed_tools: Some(vec!["AgentTool".into()]),
                    model: Some("model-recovery/original".into()),
                    options: Some(json!({"full_access":true})),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .output
        } else {
            let mut config = crate::config::get_project(workspace.path()).await.unwrap();
            config.default_provider = Some("model-recovery".into());
            config.api_url = Some(format!("http://{address}/v1"));
            config.api_key = Some("synthetic-key".into());
            config.default_model = Some("original".into());
            config.workspace_dir = workspace.path().to_path_buf();
            config.autonomy.level = crate::security::AutonomyLevel::Full;
            let mut engine = crate::agent::loop_::query_engine::QueryEngine::new_with_provider(
                config,
                &parent.id,
                Some(Arc::new(ParentModel { root: parent.id.clone() })),
            )
            .await
            .unwrap();
            engine.submit_message("ROOT_ENTRY_WORKFLOW").await.unwrap()
        };
        assert!(result.contains("ENTRY_INTEGRATED_ALPHA_BETA"), "gateway={gateway}: {result}");
        let tasks = crate::session::child_agent::coordinator::global_child_agent_coordinator()
            .snapshots(&parent.id, &[])
            .unwrap();
        assert_eq!(tasks.len(), 2);
        assert!(requests.lock().unwrap().len() >= before + if gateway { 4 } else { 2 });
        for child in tasks {
            crate::session::child_agent::coordinator::global_child_agent_coordinator()
                .cancel(&parent.id, &child.child_session_id)
                .unwrap();
        }
    }
    stopped.store(true, Ordering::SeqCst);
    let _ = std::net::TcpStream::connect(address);
    server.join().unwrap();
    drop(fixture);
    drop(http);
}
