use super::*;
use crate::workflow::{WorkflowChildContext, WorkflowNodeFinishedEvent, WorkflowNodeStartedEvent};
use vw_api_types::workflow::WorkflowNodeRunDto;
fn child(event: WorkflowRunEvent) -> WorkflowRunEvent {
    WorkflowRunEvent::Child {
        context: WorkflowChildContext {
            run_id: "run".into(),
            parent_node_id: "planning".into(),
            chunk_index: 0,
            chunk_id: "chunk-0".into(),
            node_id: "model".into(),
            call_id: "model-0".into(),
        },
        event: Box::new(event),
    }
}
fn finish(status: WorkflowNodeRunStatus) -> WorkflowRunEvent {
    child(WorkflowRunEvent::NodeFinished(WorkflowNodeFinishedEvent {
        index: 0,
        node: WorkflowNodeRunDto {
            node_id: "model".into(),
            node_type: "llm".into(),
            title: "model".into(),
            status,
            inputs: Default::default(),
            outputs: Default::default(),
            selected_handle: None,
            error: None,
            elapsed_ms: 1,
        },
    }))
}
#[tokio::test]
async fn video_edit_ui_reverse_chunk_events_keep_failures_and_shared_peak() {
    let budget = crate::workflow::budget::VideoEditRunBudget::default();
    let mut permits = Vec::new();
    for _ in 0..5 {
        permits.push(budget.acquire().await.unwrap());
    }
    drop(permits);
    let mut progress = Progress::default();
    progress.observe(&WorkflowRunEvent::WorkflowStarted { run_id: "run".into() });
    progress.observe(&finish(WorkflowNodeRunStatus::Failed));
    progress.observe(&child(WorkflowRunEvent::NodeStarted(WorkflowNodeStartedEvent {
        node_id: "model".into(),
        node_type: "llm".into(),
        title: "model".into(),
        index: 0,
    })));
    progress.observe(&finish(WorkflowNodeRunStatus::Succeeded));
    let StreamEvent::Tool(event) = progress.event(&budget) else { panic!("tool") };
    let p = &event.payload["metadata"]["video_edit_progress"];
    assert_eq!(p["failed"], 1);
    assert_eq!(p["completed"], 0);
    assert_eq!(p["running"], 0);
    assert_eq!(p["peak"], 5);
    assert_eq!(p["limit"], 5);
    assert_eq!(event.status, ToolCallStatus::Failed);
}
