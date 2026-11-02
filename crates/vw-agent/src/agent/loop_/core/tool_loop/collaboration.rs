//! Channel adapter for the shared coordinator's model boundaries.
use super::*;
use crate::session::child_agent::coordinator::global_child_agent_coordinator;

pub(super) fn consume(context: &ToolUseContext, history: &mut Vec<ChatMessage>) -> Result<bool> {
    let mut events = Vec::new();
    let delivered = global_child_agent_coordinator().deliver_model_events(context, &mut events)?;
    history.extend(
        events
            .into_iter()
            .map(|event| ChatMessage::user(event["content"].as_str().unwrap_or_default())),
    );
    Ok(delivered)
}

pub(super) async fn integrate(
    context: &ToolUseContext,
    history: &mut Vec<ChatMessage>,
    cancellation: Option<&CancellationToken>,
    budget_exhausted: bool,
) -> Result<bool> {
    let coordinator = global_child_agent_coordinator();
    loop {
        if cancellation.is_some_and(CancellationToken::is_cancelled) {
            coordinator.cancel_descendants(context.session(), &[])?;
            return Err(ToolLoopCancelled.into());
        }
        if budget_exhausted
            && coordinator
                .descendant_snapshots(context.session(), &[])?
                .iter()
                .any(|task| !task.state.is_terminal())
        {
            anyhow::bail!("Maximum tool iterations reached: child tasks remain unfinished");
        }
        let outcome = coordinator
            .wait_for_descendants(context.session(), &[], std::time::Duration::from_millis(100))
            .await?;
        if !outcome.timed_out {
            break;
        }
    }
    consume(context, history)
}
