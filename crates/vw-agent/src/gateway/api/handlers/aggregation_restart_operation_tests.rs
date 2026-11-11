//! Exercise the real task executor after the irreversible handoff, without restarting a client.
use super::*;
#[tokio::test]
async fn aggregation_t45_cancel_after_handoff_retains_real_terminal_result() {
    let state = State { policy: None, editor: Editor::new(None), operations: Default::default() };
    let owner = Owner {
        project: ProjectScope::new(
            TenantScope::new("p", "c").unwrap(),
            vw_api_types::id::ProjectId::from("project"),
        )
        .unwrap(),
        host: "host".into(),
        app: AppId::Codex,
        principal: "actor".into(),
    };
    let (handoff_tx, handoff_rx) = tokio::sync::oneshot::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let initial = state
        .aggregation_restart_start(owner.clone(), "request".into(), [1; 32], move || {
            vw_agent_apps::providers::operations::publication().unwrap();
            handoff_tx.send(()).unwrap();
            finish_rx.recv().unwrap();
            AggregationOutcome::Restarted { result: CodexRestartResult::Restarted }
        })
        .unwrap();
    assert!(state.aggregation_replay(&owner, "request", &[1; 32]).unwrap().is_none());
    handoff_rx.await.unwrap();
    let id = initial["operationId"].as_str().unwrap();
    {
        let registry = state.operations.lock().unwrap();
        let running = registry.get(id).unwrap();
        running.operation.cancel();
        let progress = running.aggregation_progress();
        assert_eq!(progress["status"], "running");
        assert_eq!(progress["cancellationRequested"], true);
        assert_eq!(progress["phase"], "settling");
        assert!(progress.get("aggregationOutcome").is_none());
    }
    finish_tx.send(()).unwrap();
    for _ in 0..200 {
        let p = state.aggregation_restart_replay(&owner, "request", &[1; 32]).unwrap().unwrap();
        if p["status"] == "completed" {
            assert_eq!(p["aggregationOutcome"]["result"], "restarted");
            assert_eq!(p["cancellationRequested"], true);
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("cancelled wait discarded the real command result");
}
