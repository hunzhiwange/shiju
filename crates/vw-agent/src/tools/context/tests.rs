//! 验证工具运行上下文、工作流状态和任务局部作用域。

use super::*;
use crate::app::agent::config::AutonomyConfig;
use crate::app::agent::observability::NoopObserver;

use std::sync::Arc;

#[test]
fn default_workflow_state_is_unbound() {
    let state = WorkflowState::default();
    assert!(state.worktree.directory.is_none());
}

#[test]
fn context_preserves_session_and_root() {
    let context = ToolUseContext::new("session-a", Some("/tmp/work".into()));
    assert_eq!(context.session(), "session-a");
    assert_eq!(context.root(), Some("/tmp/work"));
    assert_eq!(context.root_path().as_deref(), Some(std::path::Path::new("/tmp/work")));
}

#[test]
fn workflow_transitions_reset_worktree() {
    let context = ToolUseContext::new("session-a", Some("/workspace".into()));

    let binding =
        context.bind_worktree("/worktrees/task".into(), "task".into(), "codex/task".into());
    assert_eq!(binding.directory.as_deref(), Some("/worktrees/task"));
    assert_eq!(context.workspace_root().as_deref(), Some(std::path::Path::new("/worktrees/task")));

    let cleared = context.clear_worktree_binding();
    assert!(cleared.directory.is_none());
    assert_eq!(context.workspace_root().as_deref(), Some(std::path::Path::new("/workspace")));
}

#[tokio::test]
async fn task_local_context_is_visible_only_inside_scope() {
    assert!(current_tool_use_context().is_none());
    let context = std::sync::Arc::new(
        ToolUseContext::new("scoped-session", Some("/workspace".into()))
            .with_message_id("message-1")
            .with_tool_call_id("tool-1")
            .with_channel("cli")
            .with_provider("provider-a")
            .with_model("model-a")
            .with_auto_model_selection(true)
            .with_turn_id("turn-1")
            .with_iteration(3)
            .with_full_access_enabled(true)
            .with_bypass_non_cli_approval_for_turn(true),
    );

    scope_tool_use_context(context, async {
        let current = current_tool_use_context().expect("context should be scoped");
        assert_eq!(current.session(), "scoped-session");
        assert_eq!(current.message_id(), Some("message-1"));
        assert_eq!(current.tool_call_id(), Some("tool-1"));
        assert_eq!(current.channel(), Some("cli"));
        assert_eq!(current.provider(), Some("provider-a"));
        assert_eq!(current.model(), Some("model-a"));
        assert!(current.model_selected_by_auto());
        assert_eq!(current.turn_id(), Some("turn-1"));
        assert_eq!(current.iteration(), 3);
        assert!(current.full_access_enabled());
        assert!(current.bypass_non_cli_approval_for_turn());
    })
    .await;

    assert!(current_tool_use_context().is_none());
}

#[test]
fn normalize_path_uses_context_workspace_root() {
    let context = ToolUseContext::new("session-a", Some("/workspace".into()));
    assert_eq!(
        normalize_path_for_context(&context, "src/lib.rs"),
        std::path::PathBuf::from("/workspace/src/lib.rs")
    );
}

#[test]
fn default_and_debug_report_runtime_capability_presence() {
    let context = ToolUseContext::default();

    assert_eq!(context.session(), "default");
    assert!(context.root_path().is_some());
    assert!(context.security().is_none());
    assert!(context.approval_manager().is_none());
    assert!(context.messages_view().is_none());
    assert!(context.observer().is_none());
    assert!(context.progress_tx().is_none());
    assert!(context.abort_token().is_none());
    assert!(context.hook_runner().is_none());
    assert!(context.non_cli_approval_context().is_none());

    let debug = format!("{context:?}");
    assert!(debug.contains("ToolUseContext"));
    assert!(debug.contains("has_security: false"));
    assert!(debug.contains("has_progress_tx: false"));
}

#[test]
fn builder_methods_expose_injected_runtime_capabilities() {
    let (progress_tx, _progress_rx) = tokio::sync::mpsc::channel(1);
    let (prompt_tx, _prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let abort_token = CancellationToken::new();
    let mut security = SecurityPolicy::default();
    security.workspace_dir = PathBuf::from("/secure-workspace");
    let approval = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let hook_runner = Arc::new(HookRunner::new());
    let messages = vec![ChatMessage::user("hello"), ChatMessage::assistant("hi")];

    let context = ToolUseContext::new("session-b", None)
        .with_root(None)
        .with_security(Arc::new(security))
        .with_approval(approval.clone())
        .with_messages_view(messages)
        .with_explicitly_requested_skills(["RustCodeGraph", "review"])
        .with_observer(Arc::new(NoopObserver))
        .with_progress_tx(progress_tx)
        .with_abort_token(abort_token.clone())
        .with_hook_runner(hook_runner.clone())
        .with_non_cli_approval_context(NonCliApprovalContext {
            sender: "sender".to_string(),
            reply_target: "reply".to_string(),
            prompt_tx,
        });

    assert_eq!(context.workspace_root().as_deref(), Some(Path::new("/secure-workspace")));
    assert!(std::ptr::eq(context.approval_manager().expect("approval manager"), approval.as_ref()));
    assert_eq!(context.messages_view().expect("messages").len(), 2);
    assert!(context.is_skill_explicitly_requested("rustcodegraph"));
    assert!(context.is_skill_explicitly_requested("REVIEW"));
    assert_eq!(context.explicitly_requested_skills().len(), 2);
    assert!(context.observer().is_some());
    assert!(context.progress_tx().is_some());
    abort_token.cancel();
    assert!(context.abort_token().expect("abort token").is_cancelled());
    assert!(std::ptr::eq(context.hook_runner().expect("hook runner"), hook_runner.as_ref()));
    let non_cli = context.non_cli_approval_context().expect("non-cli context");
    assert_eq!(non_cli.sender, "sender");
    assert_eq!(non_cli.reply_target, "reply");

    let debug = format!("{context:?}");
    assert!(debug.contains("has_security: true"));
    assert!(debug.contains("has_non_cli_approval_context: true"));
}

#[test]
fn trusted_user_skill_selections_survive_tool_call_context_cloning() {
    let context = ToolUseContext::new("parent", Some("/workspace".into()))
        .with_trusted_user_selected_skills(["Review", " rustcodegraph "]);
    let child_call = context.clone().with_tool_call_id("call-1");

    assert_eq!(
        child_call.trusted_user_selected_skills(),
        &std::collections::BTreeSet::from(["review".to_string(), "rustcodegraph".to_string()])
    );
}

#[test]
fn shared_read_state_and_workflow_handles_are_reused() {
    let read_state = Arc::new(Mutex::new(FileReadStateCache::new(8, 1024)));
    let workflow = Arc::new(Mutex::new(WorkflowState::default()));
    let first = ToolUseContext::new("first", Some("/workspace".into()))
        .with_read_state_handle(read_state.clone())
        .with_workflow_handle(workflow.clone());
    let second = ToolUseContext::new("second", Some("/workspace".into()))
        .with_read_state_handle(read_state)
        .with_workflow_handle(workflow);

    let mut other = FileReadStateCache::new(8, 1024);
    other.note_read(Some(Path::new("/workspace")), "src/lib.rs", 12, false, None, None, None);
    first.merge_read_state(&other);
    let entry = second
        .read_state_snapshot()
        .get(Some(Path::new("/workspace")), "src/lib.rs")
        .expect("merged read state");
    assert_eq!(entry.bytes_read, 12);
}

#[tokio::test]
async fn scoped_read_state_helpers_note_get_and_invalidate_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_string_lossy().to_string();
    let context = Arc::new(ToolUseContext::new("read-state", Some(root)));

    assert!(current_read_state_for_path("src/lib.rs").is_none());
    assert!(note_current_read_state("src/lib.rs", 1, false, None, None, None).is_none());
    assert!(invalidate_current_read_state("src/lib.rs").is_none());

    scope_tool_use_context(context.clone(), async {
        let snapshot = FileSnapshot::from_text("hello");
        let normalized = note_current_read_state(
            "src/lib.rs",
            5,
            true,
            Some(1),
            Some(3),
            Some(snapshot.clone()),
        )
        .expect("scoped note should return normalized path");

        assert_eq!(normalized, temp.path().join("src/lib.rs"));
        let entry = current_read_state_for_path("src/lib.rs").expect("read state entry");
        assert_eq!(entry.path, normalized);
        assert_eq!(entry.bytes_read, 5);
        assert!(entry.partial_view);
        assert_eq!(entry.offset, Some(1));
        assert_eq!(entry.limit, Some(3));
        assert_eq!(entry.snapshot, Some(snapshot.clone()));
        assert!(snapshot.matches_text("hello"));

        let invalidated = invalidate_current_read_state("src/lib.rs").expect("invalidated entry");
        assert_eq!(invalidated.path, normalized);
        assert!(current_read_state_for_path("src/lib.rs").is_none());
    })
    .await;
}
