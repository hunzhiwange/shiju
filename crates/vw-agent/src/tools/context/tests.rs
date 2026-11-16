use super::*;
use crate::app::agent::config::AutonomyConfig;
use crate::app::agent::observability::NoopObserver;
use model_types::ChatMessage;
use tokio_util::sync::CancellationToken;
use std::sync::Mutex;
#[test]
fn builder_methods_expose_injected_runtime_capabilities() {
    let (progress_tx, _progress_rx) = tokio::sync::mpsc::channel(1);
    let (prompt_tx, _prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let abort_token = CancellationToken::new();
    let mut security = SecurityPolicy::default();
    security.workspace_dir = PathBuf::from("/secure-workspace");
    let approval = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let hook_runner = Arc::new(crate::hooks::new_hook_runner());
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
