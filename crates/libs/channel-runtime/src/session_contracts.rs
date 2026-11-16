//! 会话领域断言由真实宿主端口驱动；不模拟会话处理器。
use channel_support::{runtime_state::ChannelRuntimeContext, scope::*, routing::ChannelRouteSelection, session::sender_session_store};
use channel_types as traits;
use crate::host::ChannelHost;
use crate::manager::{message_execution::run_message_execution, message_helpers::{maybe_auto_save_message, load_prior_turns}};
use crate::manager::message_result::LlmExecutionResult;
use model_types::ChatMessage;
use std::{path::Path, future::Future, pin::Pin};
use tokio_util::sync::CancellationToken;
pub type Request = session_engine::runner_types::Request<agent_approval::ApprovalManager, agent_skills::session_sources::SessionSkillSources>;
pub type Processor = fn(Request, Option<tokio::sync::mpsc::Sender<String>>) -> Pin<Box<dyn Future<Output=anyhow::Result<String>> + Send>>;
async fn resolve_channel_message_snapshot(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> anyhow::Result<ChannelMessageSnapshot> {
    channel_support::scope::resolve_channel_message_snapshot(&ctx.project_state, ctx.message_snapshot.as_ref(), &ctx.workspace_dir, msg).await
}
fn append_sender_turn(ctx: &ChannelRuntimeContext, key: &str, turn: ChatMessage) {
    channel_support::history::append_sender_turn(&ctx.conversation_histories, key, turn);
}
use agent_approval::ApprovalManager;
use agent_config::runtime_schema::AutonomyConfig;
use agent_memory::NoneMemory;
use agent_observability::NoopObserver;
use model_types::Provider;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("ok".to_string())
    }
}

pub fn channel_message(thread_ts: Option<&str>) -> traits::ChannelMessage {
    traits::ChannelMessage {
        instance_id: "default".to_string(),
        id: "msg-1".to_string(),
        sender: "sender-1".to_string(),
        reply_target: "room-1".to_string(),
        content: "hello".to_string(),
        channel: "telegram".to_string(),
        timestamp: 1,
        thread_ts: thread_ts.map(str::to_string),
    }
}

fn test_context() -> ChannelRuntimeContext {
    let provider: Arc<dyn Provider> = Arc::new(StaticProvider);
    ChannelRuntimeContext {
        project_state: Default::default(),
        message_snapshot: None,
        session_full_access: true,
        channels_by_name: Arc::new(HashMap::new()),
        provider: Arc::clone(&provider),
        default_provider: Arc::new("provider".to_string()),
        memory: Arc::new(NoneMemory::new()),
        tools_registry: Arc::new(Vec::new()),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("system".to_string()),
        model: Arc::new("model".to_string()),
        temperature: 0.0,
        auto_save_memory: false,
        max_iterations: 1,
        min_relevance_score: 0.0,
        conversation_histories: Arc::new(Mutex::new(HashMap::new())),
        provider_cache: Arc::new(Mutex::new(HashMap::new())),
        route_overrides: Arc::new(Mutex::new(HashMap::new())),
        api_key: None,
        api_url: None,
        reliability: Arc::new(agent_config::runtime_schema::ReliabilityConfig::default()),
        provider_runtime_options: agent_providers::ProviderRuntimeOptions::default(),
        workspace_dir: Arc::new(PathBuf::from(".")),
        message_timeout_secs: 30,
        interrupt_on_new_message: false,
        multimodal: agent_config::runtime_schema::MultimodalConfig::default(),
        hooks: None,
        non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),
        query_classification: agent_config::runtime_schema::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&AutonomyConfig::default())),
    }
}

fn processor_request(root: &std::path::Path) -> Request {
    Request {
        stream: 1,
        session: "channel-session-test".to_string(),
        query: "请读取 docs 文件列表".to_string(),
        root: Some(root.to_string_lossy().to_string()),
        model: None,
        reliability: agent_config::runtime_schema::ReliabilityConfig::default(),
        options: serde_json::json!({}),
        approval: None,
        channel_name: Some("test".to_string()),
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: Vec::new(),
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools: Default::default(),
        session_skills: Default::default(),
    }
}

pub async fn run_session_processor_for_channel_accumulates_and_forwards_docs_output(processor: Processor) {
    let workspace = tempfile::tempdir().expect("workspace temp dir");
    let docs_dir = workspace.path().join("docs");
    std::fs::create_dir(&docs_dir).expect("create docs dir");
    std::fs::write(docs_dir.join("guide.md"), "# Guide\n").expect("write docs fixture");
    let (delta_tx, mut delta_rx) = tokio::sync::mpsc::channel(1);

    let output =
        processor(processor_request(workspace.path()), Some(delta_tx))
            .await
            .expect("docs request should complete");

    assert!(output.contains("docs/guide.md"));
    assert_eq!(delta_rx.recv().await.as_deref(), Some(output.as_str()));
    assert_eq!(delta_rx.recv().await, None);
}

pub async fn run_session_processor_for_channel_propagates_docs_error(processor: Processor) {
    let workspace = tempfile::tempdir().expect("workspace temp dir");

    let error = processor(processor_request(workspace.path()), None)
        .await
        .expect_err("missing docs directory should fail")
        .to_string();

    assert_eq!(error, "docs 目录不存在");
}

pub async fn registered_context()
-> (tempfile::TempDir, ChannelRuntimeContext, agent_project::Info) {
    let temp = tempfile::tempdir().unwrap();
    let mut ctx = test_context();
    ctx.workspace_dir = Arc::new(temp.path().to_path_buf());
    let (project, _) = agent_project::from_directory(
        &session_store::ui_store::current_tenant_scope(),
        temp.path(),
    )
    .await
    .unwrap();
    (temp, ctx, project)
}



pub async fn channel_project_scope_sessions_follow_snapshot_and_isolate_rooms(host: &'static dyn ChannelHost) {
    let (_a, ctx, _) = registered_context().await;
    let (_b, _, b) = registered_context().await;
    let msg = channel_message(None);
    let a = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let session_a = host.session_id(&a, &msg, None).await.unwrap();
    assert_eq!(session_a, host.session_id(&a, &msg, None).await.unwrap());
    let info = host.get_session(&session_a).await.unwrap();
    assert_eq!(info.project_id, a.scope.project);
    assert_eq!(Path::new(&info.directory), a.directory);
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), b.id);
    let selected_b = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_ne!(
        session_a,
        host.session_id(&selected_b, &msg, None).await.unwrap()
    );
    let mut other = msg.clone();
    other.reply_target = "other-room".into();
    let room = resolve_channel_message_snapshot(&ctx, &other).await.unwrap();
    assert_ne!(
        host.session_id(&selected_b, &msg, None).await.unwrap(),
        host.session_id(&room, &other, None).await.unwrap()
    );
    // 旧快照不受后续默认项目更新影响。
    assert_eq!(session_a, host.session_id(&a, &msg, None).await.unwrap());
}


pub async fn channel_project_scope_unavailable_directory_does_not_cache_fake_session(host: &'static dyn ChannelHost) {
    let (temp, ctx, _) = registered_context().await;
    let msg = channel_message(None);
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    temp.close().unwrap();
    assert!(host.session_id(&snapshot, &msg, None).await.is_err());
    assert!(!sender_session_store().lock().unwrap().contains_key(&snapshot.scope));
    assert!(resolve_channel_message_snapshot(&ctx, &msg).await.is_err());
}


pub async fn channel_project_scope_processor_uses_captured_directory(host: &'static dyn ChannelHost) {
    let (temp_a, ctx, _) = registered_context().await;
    let (temp_b, _, b) = registered_context().await;
    for (temp, name) in [(&temp_a, "a.md"), (&temp_b, "b.md")] {
        std::fs::create_dir(temp.path().join("docs")).unwrap();
        std::fs::write(temp.path().join("docs").join(name), "# Guide").unwrap();
    }
    let mut msg = channel_message(None);
    msg.content = "请读取 docs 文件列表".into();
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), b.id);
    let route = ChannelRouteSelection {
        provider: "channel-scope-fixture".into(),
        model: "scope-model".into(),
        reasoning_effort: None,
        task_mode_enabled: false,
    };
    let result = run_message_execution(
        host,
        &ctx,
        &msg,
        &route,
        &snapshot,
        Vec::new(),
        None,
        30,
        &CancellationToken::new(),
    )
    .await;
    let output = match result {
        LlmExecutionResult::Completed(Ok(result)) => result.expect("docs processor result").text,
        LlmExecutionResult::Completed(Err(err)) => panic!("docs processor timed out: {err}"),
        LlmExecutionResult::Cancelled => panic!("docs processor cancelled"),
    };
    assert!(output.contains("docs/a.md"), "{output}");
    assert!(!output.contains("docs/b.md"));
    let session = host.session_id(&snapshot, &msg, None).await.unwrap();
    let info = host.get_session(&session).await.unwrap();
    assert_eq!(Path::new(&info.directory), snapshot.directory);
    assert_eq!(info.project_id, snapshot.scope.project);
}

pub async fn channel_project_scope_memory_reads_only_matching_scope() {
    let (temp_a, mut ctx, _) = registered_context().await;
    let (_temp_b, _, b) = registered_context().await;
    ctx.memory = Arc::new(agent_memory::sqlite::SqliteMemory::new(temp_a.path()).unwrap());
    ctx.auto_save_memory = true;
    let mut msg = channel_message(None);
    let a = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), b.id);
    let b = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let mut scoped = ctx.clone();
    scoped.message_snapshot = Some(a.clone());
    msg.content = "shared memory alpha private project detail".into();
    maybe_auto_save_message(&scoped, &msg).await;
    scoped.message_snapshot = Some(b.clone());
    msg.content = "shared memory beta private project detail".into();
    maybe_auto_save_message(&scoped, &msg).await;
    for (snapshot, own, other) in [(a, "alpha", "beta"), (b, "beta", "alpha")] {
        scoped.message_snapshot = Some(snapshot.clone());
        append_sender_turn(&scoped, &snapshot.scope.key(), ChatMessage::user("shared memory"));
        let turns = load_prior_turns(&scoped, &snapshot.scope.key(), "shared memory", false).await;
        assert!(turns[0].content.contains(own), "{}", turns[0].content);
        assert!(!turns[0].content.contains(other), "{}", turns[0].content);
    }
}

pub async fn channel_project_scope_snapshot_rejects_different_sender(host: &'static dyn ChannelHost) {
    let (_temp, ctx, _) = registered_context().await;
    let mut msg = channel_message(None);
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    msg.sender = "another-sender".into();
    assert!(host.session_id(&snapshot, &msg, None).await.is_err());
    assert!(!sender_session_store().lock().unwrap().contains_key(&snapshot.scope));
}
