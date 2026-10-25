use super::*;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::AutonomyConfig;
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::Provider;
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

pub(crate) fn channel_message(thread_ts: Option<&str>) -> traits::ChannelMessage {
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
        reliability: Arc::new(crate::app::agent::config::ReliabilityConfig::default()),
        provider_runtime_options: crate::app::agent::providers::ProviderRuntimeOptions::default(),
        workspace_dir: Arc::new(PathBuf::from(".")),
        message_timeout_secs: 30,
        interrupt_on_new_message: false,
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        hooks: None,
        non_cli_excluded_tools: Arc::new(Mutex::new(Vec::new())),
        query_classification: crate::app::agent::config::QueryClassificationConfig::default(),
        model_routes: Vec::new(),
        approval_manager: Arc::new(ApprovalManager::from_config(&AutonomyConfig::default())),
    }
}

fn processor_request(root: &std::path::Path) -> crate::app::agent::session::processor::Request {
    crate::app::agent::session::processor::Request {
        stream: 1,
        session: "channel-session-test".to_string(),
        query: "请读取 docs 文件列表".to_string(),
        root: Some(root.to_string_lossy().to_string()),
        model: None,
        reliability: crate::app::agent::config::ReliabilityConfig::default(),
        options: serde_json::json!({}),
        approval: None,
        channel_name: Some("test".to_string()),
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: Vec::new(),
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools: crate::session::runtime::SessionToolset::default(),
    }
}

#[test]
fn initial_session_title_normalizes_truncates_and_falls_back_to_channel_sender() {
    let mut msg = channel_message(None);

    msg.content = "  hello\n   world\tfrom channel  ".to_string();
    assert_eq!(initial_session_title_for_message(&msg), "hello world from channel");

    msg.content = format!("{} tail", "a".repeat(51));
    assert_eq!(initial_session_title_for_message(&msg), format!("{}...", "a".repeat(50)));

    msg.content = " \n\t ".to_string();
    assert_eq!(initial_session_title_for_message(&msg), "telegram sender-1");
}

#[test]
fn initial_session_title_truncates_on_char_boundary_after_trimming() {
    let mut msg = channel_message(None);
    msg.content = format!("{}   tail", "好".repeat(51));

    let title = initial_session_title_for_message(&msg);

    assert_eq!(title, format!("{}...", "好".repeat(50)));
    assert!(title.ends_with("..."));
}

#[tokio::test]
async fn spawn_channel_session_title_refresh_ignores_empty_title_source() {
    spawn_channel_session_title_refresh(
        "missing-channel-session".to_string(),
        " \n\t ".to_string(),
        Some("provider/model".to_string()),
    );

    tokio::task::yield_now().await;
}

#[tokio::test]
async fn run_session_processor_for_channel_accumulates_and_forwards_docs_output() {
    if isolated_processor_fixture("channels::session::session_tests::run_session_processor_for_channel_accumulates_and_forwards_docs_output").await { return; }
    let workspace = tempfile::tempdir().expect("workspace temp dir");
    let docs_dir = workspace.path().join("docs");
    std::fs::create_dir(&docs_dir).expect("create docs dir");
    std::fs::write(docs_dir.join("guide.md"), "# Guide\n").expect("write docs fixture");
    let (delta_tx, mut delta_rx) = tokio::sync::mpsc::channel(1);

    let output =
        run_session_processor_for_channel(processor_request(workspace.path()), Some(delta_tx))
            .await
            .expect("docs request should complete");

    assert!(output.contains("docs/guide.md"));
    assert_eq!(delta_rx.recv().await.as_deref(), Some(output.as_str()));
    assert_eq!(delta_rx.recv().await, None);
}

#[tokio::test]
async fn run_session_processor_for_channel_propagates_docs_error() {
    if isolated_processor_fixture(
        "channels::session::session_tests::run_session_processor_for_channel_propagates_docs_error",
    )
    .await
    {
        return;
    }
    let workspace = tempfile::tempdir().expect("workspace temp dir");

    let error = run_session_processor_for_channel(processor_request(workspace.path()), None)
        .await
        .expect_err("missing docs directory should fail")
        .to_string();

    assert_eq!(error, "docs 目录不存在");
}

#[test]
fn to_session_history_maps_known_roles_and_skips_unknown_roles() {
    let turns = vec![
        ChatMessage::system("system"),
        ChatMessage::user("user"),
        ChatMessage::assistant("assistant"),
        ChatMessage::tool("tool"),
        ChatMessage { role: "unknown".to_string(), content: "skip".to_string() },
    ];

    let history = to_session_history(&turns);

    assert_eq!(history.len(), 4);
    assert!(matches!(history[0].role, crate::session::ui_types::ChatRole::System));
    assert!(matches!(history[1].role, crate::session::ui_types::ChatRole::User));
    assert!(matches!(history[2].role, crate::session::ui_types::ChatRole::Assistant));
    assert!(matches!(history[3].role, crate::session::ui_types::ChatRole::Tool));
    assert!(history.iter().all(|message| message.think_timing.is_empty()));
}

#[test]
pub(crate) fn channel_project_scope_rooms_have_independent_history() {
    let a = channel_message(None);
    let mut b = a.clone();
    b.reply_target = "room-2".into();
    assert_ne!(conversation_history_key(&a), conversation_history_key(&b));
}

#[test]
pub(crate) fn channel_project_scope_instances_have_independent_history() {
    let a = channel_message(None);
    let mut b = a.clone();
    b.instance_id = "second".into();
    assert_ne!(conversation_history_key(&a), conversation_history_key(&b));
}

#[test]
pub(crate) fn channel_project_scope_threads_have_independent_history() {
    let a = channel_message(None);
    let mut b = a.clone();
    b.thread_ts = Some("thread-2".into());
    assert_ne!(conversation_history_key(&a), conversation_history_key(&b));
}

#[test]
pub(crate) fn channel_project_scope_senders_have_independent_history() {
    let a = channel_message(None);
    let mut b = a.clone();
    b.sender = "sender-2".into();
    assert_ne!(conversation_history_key(&a), conversation_history_key(&b));
}

#[test]
pub(crate) fn channel_project_scope_identity_delimiters_do_not_collide() {
    let mut a = channel_message(None);
    a.channel = "a_b".into();
    a.sender = "c".into();
    let mut b = a.clone();
    b.channel = "a".into();
    b.sender = "b_c".into();
    assert_ne!(conversation_history_key(&a), conversation_history_key(&b));
}

pub(crate) async fn registered_context()
-> (tempfile::TempDir, ChannelRuntimeContext, crate::project::Info) {
    let temp = tempfile::tempdir().unwrap();
    let mut ctx = test_context();
    ctx.workspace_dir = Arc::new(temp.path().to_path_buf());
    let (project, _) = crate::project::from_directory(
        &crate::session::ui_store::current_tenant_scope(),
        temp.path(),
    )
    .await
    .unwrap();
    (temp, ctx, project)
}

pub(crate) async fn channel_project_scope_binding_priority_and_immutable_snapshot() {
    let (_a, ctx, a) = registered_context().await;
    let (_b, _, b) = registered_context().await;
    let msg = channel_message(None);
    let fallback = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_eq!(fallback.scope.project, a.id);
    assert_eq!(fallback.source, ChannelProjectSource::Workspace);
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), b.id.clone());
    let default = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_eq!(default.scope.project, b.id);
    assert_eq!(default.source, ChannelProjectSource::Instance);
    ctx.project_state
        .lock()
        .unwrap()
        .selections
        .insert(ChannelChatScope::from_message(&msg), a.id.clone());
    let explicit = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_eq!(explicit.source, ChannelProjectSource::Chat);
    assert_eq!(explicit.scope.project, a.id);
    assert_eq!(default.scope.project, b.id);
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), "missing".into());
    assert_eq!(resolve_channel_message_snapshot(&ctx, &msg).await.unwrap().scope.project, a.id);
}

pub(crate) async fn channel_project_scope_invalid_binding_preserves_state() {
    let (_a, ctx, _) = registered_context().await;
    let msg = channel_message(None);
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), "missing-project-for-scope".into());
    append_sender_turn(&ctx, "keep", ChatMessage::user("keep"));
    assert!(resolve_channel_message_snapshot(&ctx, &msg).await.is_err());
    assert_eq!(ctx.conversation_histories.lock().unwrap()["keep"].len(), 1);
    assert!(ctx.project_state.lock().unwrap().selections.is_empty());
    ctx.project_state
        .lock()
        .unwrap()
        .selections
        .insert(ChannelChatScope::from_message(&msg), "missing-explicit".into());
    ctx.project_state.lock().unwrap().config.instance_default_projects.clear();
    assert!(resolve_channel_message_snapshot(&ctx, &msg).await.is_err());
}

pub(crate) async fn channel_project_scope_sessions_follow_snapshot_and_isolate_rooms() {
    let (_a, ctx, _) = registered_context().await;
    let (_b, _, b) = registered_context().await;
    let msg = channel_message(None);
    let a = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let session_a = resolve_or_create_sender_session_id(&a, &msg).await.unwrap();
    assert_eq!(session_a, resolve_or_create_sender_session_id(&a, &msg).await.unwrap());
    let info = crate::session::session::get_any(&session_a).await.unwrap();
    assert_eq!(info.project_id, a.scope.project);
    assert_eq!(Path::new(&info.directory), a.directory);
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), b.id);
    let selected_b = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_ne!(session_a, resolve_or_create_sender_session_id(&selected_b, &msg).await.unwrap());
    let mut other = msg.clone();
    other.reply_target = "other-room".into();
    let room = resolve_channel_message_snapshot(&ctx, &other).await.unwrap();
    assert_ne!(
        resolve_or_create_sender_session_id(&selected_b, &msg).await.unwrap(),
        resolve_or_create_sender_session_id(&room, &other).await.unwrap()
    );
    // 旧快照不受后续默认项目更新影响。
    assert_eq!(session_a, resolve_or_create_sender_session_id(&a, &msg).await.unwrap());
}

pub(crate) async fn channel_project_scope_project_history_and_routes_are_independent() {
    let (_a, ctx, _) = registered_context().await;
    let (_b, _, b) = registered_context().await;
    let msg = channel_message(None);
    let a = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), b.id);
    let b = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let mut route = get_route_selection(&ctx, &a.scope.key());
    route.model = "chosen-model".into();
    route.task_mode_enabled = true;
    set_route_selection(&ctx, &a.scope.key(), route);
    assert_ne!(get_route_selection(&ctx, &b.scope.key()).model, "chosen-model");
    assert!(!get_route_selection(&ctx, &b.scope.key()).task_mode_enabled);
    append_sender_turn(&ctx, &a.scope.key(), ChatMessage::user("keep a"));
    clear_sender_history(&ctx, &b.scope.key());
    assert_eq!(ctx.conversation_histories.lock().unwrap()[&a.scope.key()][0].content, "keep a");
}

pub(crate) async fn channel_project_scope_unavailable_directory_does_not_cache_fake_session() {
    let (temp, ctx, _) = registered_context().await;
    let msg = channel_message(None);
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    temp.close().unwrap();
    assert!(resolve_or_create_sender_session_id(&snapshot, &msg).await.is_err());
    assert!(!sender_session_store().lock().unwrap().contains_key(&snapshot.scope));
    assert!(resolve_channel_message_snapshot(&ctx, &msg).await.is_err());
}

pub(crate) async fn channel_project_scope_tenant_cannot_resolve_other_tenant_binding() {
    let (_a, ctx, a) = registered_context().await;
    let msg = channel_message(None);
    ctx.project_state
        .lock()
        .unwrap()
        .config
        .instance_default_projects
        .insert("telegram:default".into(), a.id);
    let first = ChannelChatScope::from_message(&msg);
    crate::session::ui_store::with_tenant_scope(
        vw_api_types::id::TenantScope::new("scope-platform", "scope-company").unwrap(),
        async {
            assert_ne!(first, ChannelChatScope::from_message(&msg));
            ctx.project_state.lock().unwrap().tenant =
                crate::session::ui_store::current_tenant_scope();
            assert!(resolve_channel_message_snapshot(&ctx, &msg).await.is_err());
        },
    )
    .await;
}

pub(crate) async fn channel_project_scope_processor_uses_captured_directory() {
    if isolated_processor_fixture(
        "channels::channel_project_scope_processor_uses_captured_directory",
    )
    .await
    {
        return;
    }
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
        task_mode_enabled: false,
    };
    let result = run_message_execution(
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
        LlmExecutionResult::Completed(Ok(result)) => result.expect("docs processor result"),
        LlmExecutionResult::Completed(Err(err)) => panic!("docs processor timed out: {err}"),
        LlmExecutionResult::Cancelled => panic!("docs processor cancelled"),
    };
    assert!(output.contains("docs/a.md"), "{output}");
    assert!(!output.contains("docs/b.md"));
    let session = resolve_or_create_sender_session_id(&snapshot, &msg).await.unwrap();
    let info = crate::session::session::get_any(&session).await.unwrap();
    assert_eq!(Path::new(&info.directory), snapshot.directory);
    assert_eq!(info.project_id, snapshot.scope.project);
}

pub(crate) async fn channel_project_scope_memory_reads_only_matching_scope() {
    let (temp_a, mut ctx, _) = registered_context().await;
    let (_temp_b, _, b) = registered_context().await;
    ctx.memory = Arc::new(crate::memory::sqlite::SqliteMemory::new(temp_a.path()).unwrap());
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

pub(crate) async fn channel_project_scope_snapshot_rejects_different_sender() {
    let (_temp, ctx, _) = registered_context().await;
    let mut msg = channel_message(None);
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    msg.sender = "another-sender".into();
    assert!(resolve_or_create_sender_session_id(&snapshot, &msg).await.is_err());
    assert!(!sender_session_store().lock().unwrap().contains_key(&snapshot.scope));
}

/// 子进程隔离模型目录与配置数据库，避免修改维护者配置或并行测试的环境。
async fn isolated_processor_fixture(test: &str) -> bool {
    if std::env::var("VW_CHANNEL_SCOPE_PROCESSOR_TEST").as_deref() == Ok(test) {
        crate::config::update(serde_json::json!({
            "providers": {"channel-scope-fixture": {"models": {"scope-model": {"status": "active"}}}}
        })).await.unwrap();
        return false;
    }
    let home = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test, "--nocapture"])
        .env("VW_CHANNEL_SCOPE_PROCESSOR_TEST", test)
        .env("HOME", home.path())
        .env("VIBEWINDOW_CONFIG_DIR", home.path().join("config"))
        .env("VIBEWINDOW_WORKSPACE", home.path().join("workspace"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "isolated processor fixture failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
