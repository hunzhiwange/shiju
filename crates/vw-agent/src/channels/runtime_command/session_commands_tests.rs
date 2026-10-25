// 本文件验证会话提供者和模型的展示、切换、持久化与历史清理。

use super::super::*;
use super::session_commands::{
    handle_clear_session, handle_new_session, handle_set_model, handle_set_provider,
    handle_show_model, handle_task_mode, with_test_provider_init_error,
};
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::{AutonomyConfig, ReliabilityConfig};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::{ChatMessage, Provider, ProviderRuntimeOptions};
use crate::app::agent::tools::Tool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(message.to_string())
    }
}

fn route(provider: &str, model: &str, task_mode_enabled: bool) -> ChannelRouteSelection {
    ChannelRouteSelection {
        provider: provider.to_string(),
        model: model.to_string(),
        reasoning_effort: None,
        task_mode_enabled,
    }
}

fn context(
    workspace_dir: &Path,
    default_provider: &str,
    default_model: &str,
) -> ChannelRuntimeContext {
    ChannelRuntimeContext {
        project_state: Default::default(),
        message_snapshot: None,
        channels_by_name: Arc::new(HashMap::new()),
        provider: Arc::new(StaticProvider),
        default_provider: Arc::new(default_provider.to_string()),
        memory: Arc::new(NoneMemory::new()),
        tools_registry: Arc::new(Vec::<Box<dyn Tool>>::new()),
        observer: Arc::new(NoopObserver),
        system_prompt: Arc::new("system".to_string()),
        model: Arc::new(default_model.to_string()),
        temperature: 0.2,
        auto_save_memory: false,
        max_iterations: 4,
        min_relevance_score: 0.0,
        conversation_histories: Arc::new(Mutex::new(HashMap::<String, Vec<ChatMessage>>::new())),
        provider_cache: Arc::new(Mutex::new(HashMap::new())),
        route_overrides: Arc::new(Mutex::new(HashMap::new())),
        api_key: None,
        api_url: None,
        reliability: Arc::new(ReliabilityConfig::default()),
        provider_runtime_options: ProviderRuntimeOptions::default(),
        workspace_dir: Arc::new(PathBuf::from(workspace_dir)),
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

fn message(content: &str) -> traits::ChannelMessage {
    traits::ChannelMessage {
        instance_id: "default".to_string(),
        id: "msg-1".to_string(),
        sender: "sender-1".to_string(),
        reply_target: "room-1".to_string(),
        content: content.to_string(),
        channel: "telegram".to_string(),
        timestamp: 1,
        thread_ts: Some("thread-1".to_string()),
    }
}

#[test]
fn handle_show_model_includes_current_model() {
    let current = route("openai", "gpt-4.1", false);

    let response = handle_show_model(&current, Path::new("/tmp"));

    assert!(response.contains("gpt-4.1"));
}

#[tokio::test]
async fn handle_set_model_rejects_blank_model_without_mutating_route() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "default-provider", "default-model");
    let mut current = route("default-provider", "default-model", false);

    let response = handle_set_model(&ctx, "sender", &mut current, "  `  `  ".to_string()).await;

    assert_eq!(response, "Model ID cannot be empty. Use `/model <model-id>`.");
    assert_eq!(current, route("default-provider", "default-model", false));
    assert!(ctx.route_overrides.lock().unwrap().is_empty());
}

#[tokio::test]
// 模型切换同步持久化路由并清空旧历史，避免上下文串用。
async fn handle_set_model_trims_backticks_persists_route_and_clears_history() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "catalog", "default-model");
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::user("old turn")]);
    let mut current = route("catalog", "default-model", false);

    let response = set_catalog_model(&ctx, "sender", &mut current, "  `gpt-5`  ".to_string()).await;

    assert!(response.contains("Model switched to `gpt-5`"));
    assert_eq!(current, route("catalog", "gpt-5", false));
    assert_eq!(get_route_selection(&ctx, "sender"), current);
    assert!(!ctx.conversation_histories.lock().unwrap().contains_key("sender"));
}

#[tokio::test]
async fn handle_set_model_updates_only_target_sender_and_preserves_task_mode() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "catalog", "default-model");
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("other-sender".to_string(), vec![ChatMessage::user("other turn")]);
    set_route_selection(&ctx, "other-sender", route("other-provider", "other-model", false));
    let mut current = route("catalog", "default-model", true);

    let response =
        set_catalog_model(&ctx, "target-sender", &mut current, "gpt-5".to_string()).await;

    assert!(response.contains("Model switched to `gpt-5`"));
    assert_eq!(current, route("catalog", "gpt-5", true));
    assert_eq!(
        get_route_selection(&ctx, "other-sender"),
        route("other-provider", "other-model", false)
    );
    assert!(ctx.conversation_histories.lock().unwrap().contains_key("other-sender"));
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_set_provider_reports_unknown_provider_without_mutating_state() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "default-provider", "default-model");
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::user("old turn")]);
    let mut current = route("default-provider", "default-model", false);

    let raw_provider = "missing-provider-for-session-command-test";
    let response =
        handle_set_provider(&ctx, "sender", &mut current, raw_provider.to_string()).await;

    assert_eq!(
        response,
        format!("Unknown provider `{raw_provider}`. Use `/providers` to list valid providers.")
    );
    assert_eq!(current, route("default-provider", "default-model", false));
    assert!(ctx.conversation_histories.lock().unwrap().contains_key("sender"));
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_set_provider_uses_default_provider_without_initializing_new_provider() {
    let Some(provider_name) = available_provider_ids().into_iter().next() else {
        return;
    };
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), &provider_name, "default-model");
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::user("old turn")]);
    let mut current = route("old-provider", "custom-model", false);

    let response =
        handle_set_provider(&ctx, "sender", &mut current, provider_name.to_ascii_uppercase()).await;

    assert!(response.contains(&format!("Provider switched to `{provider_name}`")));
    assert_eq!(current, route(&provider_name, "custom-model", false));
    assert_eq!(get_route_selection(&ctx, "sender"), current);
    assert!(!ctx.conversation_histories.lock().unwrap().contains_key("sender"));
    assert!(ctx.provider_cache.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_set_provider_leaves_history_when_provider_is_already_selected() {
    let Some(provider_name) = available_provider_ids().into_iter().next() else {
        return;
    };
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), &provider_name, "default-model");
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::user("old turn")]);
    let mut current = route(&provider_name, "default-model", false);

    let response = handle_set_provider(&ctx, "sender", &mut current, provider_name.clone()).await;

    assert!(response.contains(&format!("Provider switched to `{provider_name}`")));
    assert_eq!(current, route(&provider_name, "default-model", false));
    assert!(ctx.conversation_histories.lock().unwrap().contains_key("sender"));
    assert!(ctx.route_overrides.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_set_provider_reports_sanitized_init_error_without_mutating_state() {
    let Some(provider_name) = available_provider_ids().into_iter().next() else {
        return;
    };
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "default-provider", "default-model");
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert("sender".to_string(), vec![ChatMessage::user("old turn")]);
    let mut current = route("default-provider", "default-model", false);
    let injected_error = "provider init failed Authorization: Bearer sk-secret-value";

    let response = with_test_provider_init_error(
        injected_error,
        handle_set_provider(&ctx, "sender", &mut current, provider_name.clone()),
    )
    .await;

    assert!(response.contains(&format!("Failed to initialize provider `{provider_name}`")));
    assert!(response.contains("Route unchanged."));
    assert!(response.contains("[REDACTED]"));
    assert!(!response.contains("sk-secret-value"));
    assert_eq!(current, route("default-provider", "default-model", false));
    assert!(ctx.conversation_histories.lock().unwrap().contains_key("sender"));
    assert!(ctx.route_overrides.lock().unwrap().is_empty());
}

#[tokio::test]
async fn handle_clear_session_clears_session_history_and_disables_task_mode() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "default-provider", "default-model");
    let msg = message("/new");
    let sender_key = conversation_history_key(&msg);
    crate::project::from_directory(&crate::session::ui_store::current_tenant_scope(), temp.path())
        .await
        .unwrap();
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let session_key = snapshot.scope;
    sender_session_store().lock().unwrap().insert(session_key.clone(), "session-1".to_string());
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert(sender_key.clone(), vec![ChatMessage::user("old turn")]);
    set_route_selection(&ctx, &sender_key, route("default-provider", "default-model", true));

    let response = handle_clear_session(&ctx, &msg, &sender_key).await;

    assert_eq!(response, "会话已重置. 开始新会话.");
    assert!(!sender_session_store().lock().unwrap().contains_key(&session_key));
    assert!(!ctx.conversation_histories.lock().unwrap().contains_key(&sender_key));
    assert!(!get_route_selection(&ctx, &sender_key).task_mode_enabled);
}

#[tokio::test]
async fn handle_task_mode_clears_session_history_and_enables_task_mode() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ctx = context(temp.path(), "default-provider", "default-model");
    let msg = message("/task");
    let sender_key = conversation_history_key(&msg);
    crate::project::from_directory(&crate::session::ui_store::current_tenant_scope(), temp.path())
        .await
        .unwrap();
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let session_key = snapshot.scope;
    sender_session_store().lock().unwrap().insert(session_key.clone(), "session-1".to_string());
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert(sender_key.clone(), vec![ChatMessage::user("old turn")]);

    let response = handle_task_mode(&ctx, &msg, &sender_key).await;

    assert_eq!(response, "我进入了任务模式。");
    assert!(!sender_session_store().lock().unwrap().contains_key(&session_key));
    assert!(!ctx.conversation_histories.lock().unwrap().contains_key(&sender_key));
    assert!(get_route_selection(&ctx, &sender_key).task_mode_enabled);
}

async fn set_catalog_model(
    ctx: &ChannelRuntimeContext,
    key: &str,
    current: &mut ChannelRouteSelection,
    model: String,
) -> String {
    crate::config::update_project_for_tenant(&crate::session::ui_store::current_tenant_scope(), ctx.workspace_dir.as_ref(), serde_json::json!({"providers": {"catalog": {"api": "https://example.test/v1", "models": {"gpt-5": {"status": "active"}}}}})).await.unwrap();
    let (project, _) = crate::project::from_directory(
        &crate::session::ui_store::current_tenant_scope(),
        ctx.workspace_dir.as_ref(),
    )
    .await
    .unwrap();
    let context = crate::project::instance::Context {
        directory: ctx.workspace_dir.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    crate::project::instance::provide_context(context, handle_set_model(ctx, key, current, model))
        .await
}

async fn lifecycle_fixture() -> (tempfile::TempDir, ChannelRuntimeContext, traits::ChannelMessage) {
    let temp = tempfile::tempdir().unwrap();
    let mut ctx = context(temp.path(), "default-provider", "default-model");
    let msg = message("/new");
    crate::config::update_project_for_tenant(
        &crate::session::ui_store::current_tenant_scope(),
        &std::fs::canonicalize(temp.path()).unwrap(),
        serde_json::json!({"providers": {"default-provider": {"api": "https://example.test/v1", "models": {"default-model": {"status": "active"}}}}}),
    ).await.unwrap();
    crate::project::from_directory(&crate::session::ui_store::current_tenant_scope(), temp.path())
        .await
        .unwrap();
    ctx.message_snapshot = Some(resolve_channel_message_snapshot(&ctx, &msg).await.unwrap());
    (temp, ctx, msg)
}

pub(crate) async fn channel_session_lifecycle_new_preserves_old() {
    let (_temp, ctx, msg) = lifecycle_fixture().await;
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    let old = resolve_or_create_sender_session_id(snapshot, &msg).await.unwrap();
    let response = handle_new_session(&ctx, &msg, &snapshot.scope.key()).await;
    let next = sender_session_store()
        .lock()
        .unwrap()
        .get(&snapshot.scope)
        .cloned()
        .expect("/new must create immediately");
    assert_ne!(old, next);
    assert!(response.contains(&next));
    assert!(crate::session::session::get_any(&old).await.is_ok());
}

pub(crate) fn channel_session_lifecycle_lists_are_parsed() {
    for text in ["/sessions", "/resume", "/continue"] {
        assert!(
            !matches!(parse_runtime_command("lark", text), Some(ChannelRuntimeCommand::Error(_))),
            "{text}"
        );
    }
}

pub(crate) fn channel_session_lifecycle_mutations_are_parsed() {
    for text in ["/resume ses-owned", "/rename 新标题", "/archive", "/archive confirm ses-owned"]
    {
        assert!(
            !matches!(parse_runtime_command("qq", text), Some(ChannelRuntimeCommand::Error(_))),
            "{text}"
        );
    }
}

pub(crate) fn channel_session_lifecycle_clear_is_separate() {
    assert_ne!(parse_runtime_command("lark", "/clear"), parse_runtime_command("lark", "/new"));
    assert_eq!(parse_runtime_command("lark", "/session"), parse_runtime_command("lark", "/new"));
}

pub(crate) async fn channel_session_lifecycle_new_clears_cache() {
    let (_temp, ctx, msg) = lifecycle_fixture().await;
    let key = ctx.message_snapshot.as_ref().unwrap().scope.key();
    ctx.conversation_histories.lock().unwrap().insert(key.clone(), vec![ChatMessage::user("old")]);
    set_route_selection(&ctx, &key, route("default-provider", "default-model", true));
    let result = handle_new_session(&ctx, &msg, &key).await;
    assert!(result.contains("ses_"), "{result}");
    assert!(!ctx.conversation_histories.lock().unwrap().contains_key(&key));
    assert!(!get_route_selection(&ctx, &key).task_mode_enabled);
}

pub(crate) fn channel_session_lifecycle_help_explains_clear() {
    let help = super::catalog::help_messages("clear", true).join("\n");
    assert!(help.contains("重置"));
    assert!(super::catalog::help_messages("sessions", true).join("\n").contains("/resume"));
}

async fn lifecycle_call(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    text: &str,
) -> String {
    let mut msg = msg.clone();
    msg.content = text.into();
    let command = parse_runtime_command(&msg.channel, text).expect("parsed command");
    super::session_lifecycle::handle_session_lifecycle_command(ctx, &msg, command).await
}

fn lifecycle_id(ctx: &ChannelRuntimeContext) -> String {
    sender_session_store()
        .lock()
        .unwrap()
        .get(&ctx.message_snapshot.as_ref().unwrap().scope)
        .cloned()
        .expect("bound session")
}

async fn lifecycle_save_history(ctx: &ChannelRuntimeContext, id: &str, content: &str) {
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    let tenant = snapshot.scope.chat.tenant.clone();
    let project = snapshot.scope.project.clone();
    let session = crate::session::ui_types::ChatSession {
        id: id.into(),
        title: "历史".into(),
        messages: to_session_history(&[
            ChatMessage::user(content),
            ChatMessage::assistant("answer"),
        ]),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    };
    crate::global_database::run_isolated(move || {
        crate::session::ui_store::save_session_scoped_for_tenant(&tenant, &session, Some(&project))
            .expect("persist UI history");
    })
    .await
    .unwrap();
}

pub(crate) async fn channel_session_lifecycle_resume_reloads_persisted_state() {
    let (_temp, ctx, msg) = lifecycle_fixture().await;
    lifecycle_call(&ctx, &msg, "/new").await;
    let old = lifecycle_id(&ctx);
    lifecycle_save_history(&ctx, &old, "persisted old history").await;
    lifecycle_call(&ctx, &msg, "/new").await;
    let key = ctx.message_snapshot.as_ref().unwrap().scope.key();
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert(key.clone(), vec![ChatMessage::user("wrong cache")]);
    set_route_selection(&ctx, &key, route("wrong", "wrong", true));
    let result = lifecycle_call(&ctx, &msg, &format!("/resume {old}")).await;
    assert!(result.contains("已恢复"), "{result}");
    assert_eq!(lifecycle_id(&ctx), old);
    assert_eq!(
        ctx.conversation_histories.lock().unwrap()[&key][0].content,
        "persisted old history"
    );
    assert_eq!(get_route_selection(&ctx, &key), route("default-provider", "default-model", false));
    // 模拟重启后映射和路由均丢失，持久化所有权仍提供候选。
    sender_session_store().lock().unwrap().remove(&ctx.message_snapshot.as_ref().unwrap().scope);
    ctx.route_overrides.lock().unwrap().clear();
    let list = lifecycle_call(&ctx, &msg, "/resume").await;
    assert!(list.contains(&old));
    assert!(lifecycle_call(&ctx, &msg, &format!("/resume {old}")).await.contains("已恢复"));
}

pub(crate) async fn channel_session_lifecycle_resume_rejects_unowned_and_invalid_ids() {
    let (_temp, ctx, msg) = lifecycle_fixture().await;
    lifecycle_call(&ctx, &msg, "/new").await;
    let old = lifecycle_id(&ctx);
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    let context = crate::project::instance::Context {
        directory: snapshot.directory.to_string_lossy().into(),
        worktree: snapshot.project.worktree.clone(),
        project: snapshot.project.clone(),
    };
    let unowned = crate::project::instance::provide_context(
        context,
        crate::session::session::create_next(crate::session::session::CreateInput {
            directory: snapshot.directory.to_string_lossy().into(),
            ..Default::default()
        }),
    )
    .await
    .unwrap();
    for id in [&unowned.id, "missing", "../../outside"] {
        let result = lifecycle_call(&ctx, &msg, &format!("/resume {id}")).await;
        assert!(result.contains("失败"), "{result}");
        assert_eq!(lifecycle_id(&ctx), old);
    }
    assert!(!lifecycle_call(&ctx, &msg, "/sessions").await.contains(&unowned.id));
}

pub(crate) async fn channel_session_lifecycle_resume_isolates_all_chat_dimensions() {
    let (_temp, ctx, msg) = lifecycle_fixture().await;
    lifecycle_call(&ctx, &msg, "/new").await;
    let id = lifecycle_id(&ctx);
    for dimension in 0..5 {
        let mut other = msg.clone();
        match dimension {
            0 => other.channel = "qq".into(),
            1 => other.instance_id = "another".into(),
            2 => other.reply_target = "another".into(),
            3 => other.thread_ts = Some("another".into()),
            _ => other.sender = "another".into(),
        }
        let mut candidate = ctx.clone();
        candidate.message_snapshot = None;
        let result = lifecycle_call(&candidate, &other, &format!("/resume {id}")).await;
        assert!(result.contains("失败"), "dimension={dimension}: {result}");
        assert!(!lifecycle_call(&candidate, &other, "/sessions").await.contains(&id));
        assert_eq!(lifecycle_id(&ctx), id);
    }
}

pub(crate) async fn channel_session_lifecycle_archive_requires_matching_confirmation_and_preserves_history()
 {
    let (_temp, ctx, msg) = lifecycle_fixture().await;
    lifecycle_call(&ctx, &msg, "/new").await;
    let id = lifecycle_id(&ctx);
    lifecycle_save_history(&ctx, &id, "keep history").await;
    assert!(lifecycle_call(&ctx, &msg, &format!("/archive confirm {id}")).await.contains("失败"));
    assert!(
        lifecycle_call(&ctx, &msg, "/archive").await.contains(&format!("/archive confirm {id}"))
    );
    assert!(lifecycle_call(&ctx, &msg, "/archive confirm wrong").await.contains("失败"));
    assert_eq!(lifecycle_id(&ctx), id);
    assert!(lifecycle_call(&ctx, &msg, &format!("/archive confirm {id}")).await.contains("已归档"));
    assert!(crate::session::session::get_any(&id).await.unwrap().time.archived.is_some());
    assert!(!lifecycle_call(&ctx, &msg, "/sessions").await.contains(&id));
    assert!(lifecycle_call(&ctx, &msg, &format!("/resume {id}")).await.contains("失败"));
    let project = ctx.message_snapshot.as_ref().unwrap().scope.project.clone();
    let loaded = crate::global_database::run_isolated(move || {
        crate::session::ui_store::load_session_scoped(&id, Some(&project))
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(loaded.messages[0].content, "keep history");
}

pub(crate) async fn channel_session_lifecycle_archive_rejects_stale_and_busy_state() {
    let (_temp, ctx, msg) = lifecycle_fixture().await;
    lifecycle_call(&ctx, &msg, "/new").await;
    let old = lifecycle_id(&ctx);
    lifecycle_call(&ctx, &msg, "/archive").await;
    lifecycle_call(&ctx, &msg, "/rename changed").await;
    assert!(lifecycle_call(&ctx, &msg, &format!("/archive confirm {old}")).await.contains("失败"));
    lifecycle_call(&ctx, &msg, "/archive").await;
    lifecycle_call(&ctx, &msg, "/new").await;
    let next = lifecycle_id(&ctx);
    assert!(lifecycle_call(&ctx, &msg, &format!("/archive confirm {old}")).await.contains("失败"));
    let gate = channel_chat_gate(&ctx, &msg);
    let guard = gate.read().await;
    for command in ["/new", "/clear", "/rename blocked", "/archive"] {
        assert!(lifecycle_call(&ctx, &msg, command).await.contains("繁忙"));
        assert_eq!(lifecycle_id(&ctx), next);
    }
    drop(guard);
    crate::session::status::set(&next, crate::session::status::Info::Busy);
    assert!(lifecycle_call(&ctx, &msg, "/archive").await.contains("繁忙"));
    crate::session::status::set(&next, crate::session::status::Info::Idle);
    assert!(crate::session::session::get_any(&old).await.unwrap().time.archived.is_none());
}

pub(crate) async fn channel_session_lifecycle_continue_and_rename_use_owned_sessions() {
    let (_temp, ctx, msg) = lifecycle_fixture().await;
    assert!(lifecycle_call(&ctx, &msg, "/continue").await.contains("失败"));
    lifecycle_call(&ctx, &msg, "/new").await;
    let old = lifecycle_id(&ctx);
    assert!(lifecycle_call(&ctx, &msg, "/rename 调试 记录").await.contains("已重命名"));
    assert_eq!(crate::session::session::get_any(&old).await.unwrap().title, "调试 记录");
    lifecycle_call(&ctx, &msg, "/new").await;
    let next = lifecycle_id(&ctx);
    lifecycle_call(&ctx, &msg, "/archive").await;
    lifecycle_call(&ctx, &msg, &format!("/archive confirm {next}")).await;
    let result = lifecycle_call(&ctx, &msg, "/continue").await;
    assert!(result.contains("已恢复"), "{result}");
    assert_eq!(lifecycle_id(&ctx), old);
}

#[derive(Default)]
struct LifecycleChannel(tokio::sync::Mutex<Vec<SendMessage>>);
#[async_trait::async_trait]
impl Channel for LifecycleChannel {
    fn name(&self) -> &str {
        "lark"
    }
    async fn send(&self, message: &SendMessage) -> anyhow::Result<()> {
        self.0.lock().await.push(message.clone());
        Ok(())
    }
    async fn listen(
        &self,
        _tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

async fn lifecycle_dispatch(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
    text: &str,
) -> String {
    let channel = Arc::new(LifecycleChannel::default());
    let target: Arc<dyn Channel> = channel.clone();
    let mut msg = msg.clone();
    msg.content = text.into();
    let mut ctx = ctx.clone();
    ctx.channels_by_name =
        Arc::new(HashMap::from([(channel_instance_key(&msg.channel, &msg.instance_id), target)]));
    Box::pin(super::super::manager::process_channel_message(
        Arc::new(ctx),
        msg.clone(),
        tokio_util::sync::CancellationToken::new(),
    ))
    .await;
    let sent = channel.0.lock().await;
    assert_eq!(sent.len(), 1, "each command sends a response");
    assert_eq!(sent[0].recipient, msg.reply_target);
    assert_eq!(sent[0].thread_ts, msg.thread_ts);
    sent[0].content.clone()
}

pub(crate) async fn channel_session_lifecycle_pipeline_persists_and_refreshes_model_options() {
    let (_temp, mut ctx, msg) =
        super::super::channel_command_catalog_tests::catalog_context().await;
    ctx.message_snapshot = Some(resolve_channel_message_snapshot(&ctx, &msg).await.unwrap());
    assert!(lifecycle_dispatch(&ctx, &msg, "/new").await.contains("已创建"));
    let old = lifecycle_id(&ctx);
    assert!(lifecycle_dispatch(&ctx, &msg, "/model known").await.contains("Model switched"));
    assert!(lifecycle_dispatch(&ctx, &msg, "/effort high").await.contains("已切换"));
    lifecycle_save_history(&ctx, &old, "persisted model history").await;
    lifecycle_dispatch(&ctx, &msg, "/new").await;
    let key = ctx.message_snapshot.as_ref().unwrap().scope.key();
    assert_eq!(get_route_selection(&ctx, &key).reasoning_effort.as_deref(), Some("high"));
    lifecycle_dispatch(&ctx, &msg, "/model other").await;
    let result = lifecycle_dispatch(&ctx, &msg, &format!("/resume {old}")).await;
    assert!(result.contains("已恢复"), "{result}");
    let route = get_route_selection(&ctx, &key);
    assert_eq!(route.model, "known");
    assert_eq!(route.reasoning_effort.as_deref(), Some("high"));
    assert_eq!(
        ctx.conversation_histories.lock().unwrap()[&key][0].content,
        "persisted model history"
    );
}

pub(crate) async fn channel_session_lifecycle_resume_isolates_projects_and_tenants() {
    let (_temp, ctx, msg) = lifecycle_fixture().await;
    lifecycle_call(&ctx, &msg, "/new").await;
    let id = lifecycle_id(&ctx);
    let (_other_temp, mut other, mut other_msg) = lifecycle_fixture().await;
    other_msg.channel = msg.channel.clone();
    other_msg.sender = msg.sender.clone();
    other_msg.reply_target = msg.reply_target.clone();
    other_msg.thread_ts = msg.thread_ts.clone();
    other.message_snapshot = None;
    assert!(lifecycle_call(&other, &other_msg, &format!("/resume {id}")).await.contains("失败"));
    let tenant = vw_api_types::id::TenantScope::new(
        format!("lifecycle-{}", std::process::id()),
        format!("user-{id}"),
    )
    .unwrap();
    crate::project::from_directory(&tenant, ctx.workspace_dir.as_ref()).await.unwrap();
    let mut foreign = ctx.clone();
    foreign.project_state = Arc::new(Mutex::new(Default::default()));
    foreign.project_state.lock().unwrap().tenant = tenant;
    foreign.message_snapshot = None;
    assert!(lifecycle_call(&foreign, &msg, &format!("/resume {id}")).await.contains("失败"));
    assert!(!lifecycle_call(&foreign, &msg, "/sessions").await.contains(&id));
    assert_eq!(lifecycle_id(&ctx), id);
}

pub(crate) async fn channel_session_lifecycle_failed_creation_and_rename_preserve_binding() {
    let (temp, ctx, msg) = lifecycle_fixture().await;
    lifecycle_call(&ctx, &msg, "/new").await;
    let id = lifecycle_id(&ctx);
    let key = ctx.message_snapshot.as_ref().unwrap().scope.key();
    ctx.conversation_histories.lock().unwrap().insert(key.clone(), vec![ChatMessage::user("keep")]);
    let result = lifecycle_call(&ctx, &msg, &format!("/rename {}", "长".repeat(101))).await;
    assert!(result.contains("失败"));
    assert_eq!(lifecycle_id(&ctx), id);
    temp.close().unwrap();
    assert!(lifecycle_call(&ctx, &msg, "/new").await.contains("失败"));
    assert_eq!(lifecycle_id(&ctx), id);
    assert_eq!(ctx.conversation_histories.lock().unwrap()[&key][0].content, "keep");
}

pub(crate) async fn channel_session_lifecycle_invalid_saved_model_preserves_current_state() {
    let (_temp, ctx, msg) = lifecycle_fixture().await;
    lifecycle_call(&ctx, &msg, "/new").await;
    let old = lifecycle_id(&ctx);
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    save_channel_session_route(snapshot, &old, &route("missing-provider", "missing-model", false))
        .await
        .unwrap();
    // 移除当前映射，避免离开旧会话时写回其当前合法设置。
    sender_session_store().lock().unwrap().remove(&snapshot.scope);
    lifecycle_call(&ctx, &msg, "/new").await;
    let current = lifecycle_id(&ctx);
    let key = snapshot.scope.key();
    ctx.conversation_histories
        .lock()
        .unwrap()
        .insert(key.clone(), vec![ChatMessage::user("retain")]);
    let before = get_route_selection(&ctx, &key);
    let response = lifecycle_call(&ctx, &msg, &format!("/resume {old}")).await;
    assert!(response.contains("失败"), "{response}");
    assert_eq!(lifecycle_id(&ctx), current);
    assert_eq!(get_route_selection(&ctx, &key), before);
    assert_eq!(ctx.conversation_histories.lock().unwrap()[&key][0].content, "retain");
}

pub(crate) async fn channel_session_lifecycle_lists_are_stable_bounded_and_owned() {
    let (_temp, ctx, msg) = lifecycle_fixture().await;
    let snapshot = ctx.message_snapshot.as_ref().unwrap();
    let mut ids = Vec::new();
    for _ in 0..22 {
        lifecycle_call(&ctx, &msg, "/new").await;
        ids.push(lifecycle_id(&ctx));
    }
    let first = lifecycle_call(&ctx, &msg, "/sessions").await;
    assert_eq!(first, lifecycle_call(&ctx, &msg, "/sessions").await);
    assert_eq!(first.lines().filter(|line| line.starts_with("ses_")).count(), 20);
    assert!(first.contains("最多显示 20 项"));
    assert!(first.chars().count() < 3000);
    let list = owned_channel_sessions(snapshot).await.unwrap();
    let recent = list.first().unwrap().id.clone();
    sender_session_store().lock().unwrap().remove(&snapshot.scope);
    assert!(lifecycle_call(&ctx, &msg, "/continue").await.contains(&recent));
    assert_eq!(lifecycle_id(&ctx), recent);
}

pub(crate) async fn channel_session_lifecycle_new_rejects_invalid_model_options() {
    let (_temp, ctx, msg) = lifecycle_fixture().await;
    lifecycle_call(&ctx, &msg, "/new").await;
    let old = lifecycle_id(&ctx);
    let key = ctx.message_snapshot.as_ref().unwrap().scope.key();
    for mut invalid in [
        route("missing-provider", "missing-model", true),
        route("default-provider", "default-model", true),
    ] {
        invalid.reasoning_effort = Some("unsupported".into());
        set_route_selection(&ctx, &key, invalid.clone());
        ctx.conversation_histories
            .lock()
            .unwrap()
            .insert(key.clone(), vec![ChatMessage::user("keep invalid-route context")]);
        let result = lifecycle_call(&ctx, &msg, "/new").await;
        assert!(result.contains("失败"), "{result}");
        assert_eq!(lifecycle_id(&ctx), old);
        assert_eq!(get_route_selection(&ctx, &key), invalid);
        assert_eq!(
            ctx.conversation_histories.lock().unwrap()[&key][0].content,
            "keep invalid-route context"
        );
    }
}
