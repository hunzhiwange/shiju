//! Budget and cancellation use the real coordinator; host tenant delivery stays native.
use super::host::DomainHost;
use crate::core::errors::ToolLoopCancelled;
use tokio_util::sync::CancellationToken;
type ToolUseContext = tool_context::context::ToolUseContext<DomainHost>;

#[tokio::test]
async fn agcollab_channel_cancellation_and_budget_reject_unfinished_children() {
    let mut services = super::services::services();
    services.cancel_descendants = |session, children| session_engine::child_agent::coordinator::global_child_agent_coordinator().cancel_descendants(session, children);
    services.descendant_snapshots = |session, children| session_engine::child_agent::coordinator::global_child_agent_coordinator().descendant_snapshots(session, children);
    // Both cases must fail before waiting or consuming any model events.
    services.wait_for_descendants = |_, _, _| Box::pin(async { panic!("negative collaboration case unexpectedly waited") });
    services.deliver_model_events = |_, _| panic!("negative collaboration case unexpectedly delivered events");
    for cancelled in [false, true] {
        let root = format!("channel-negative-{}", uuid::Uuid::new_v4());
        let coordinator =
            session_engine::child_agent::coordinator::global_child_agent_coordinator();
        let child = format!("{root}-child");
        coordinator.register(&root, &child, "worker", "research").unwrap();
        let token = CancellationToken::new();
        if cancelled {
            token.cancel();
        }
        let error = crate::core::tool_loop::integrate(
            &services,
            &ToolUseContext::new(&root, None),
            &mut Vec::new(),
            Some(&token),
            true,
        )
        .await
        .unwrap_err();
        if cancelled {
            assert!(error.is::<ToolLoopCancelled>());
        } else {
            assert!(error.to_string().contains("unfinished"));
        }
        coordinator.cancel_descendants(&root, &[]).unwrap();
    }
}
