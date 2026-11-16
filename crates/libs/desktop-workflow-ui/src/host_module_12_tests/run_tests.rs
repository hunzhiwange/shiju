use super::*;
use crate::controller::run::handle;
use desktop_workflow_state::state::{WorkflowRunInputDraft, WorkflowRunPromptDraft};
#[test]
fn handle_submit_saved_app_run_prompt_noops_without_prompt_and_reports_loading_or_invalid_inputs() {
    let (mut app, _) = Fixture::new();

    assert!(handle(&mut app.context(), WorkflowMessage::SubmitSavedAppRunPrompt).is_some());
    assert!(app.workflow_state.saved_app_run_prompt.is_none());

    app.workflow_state.saved_app_run_prompt = Some(WorkflowRunPromptDraft {
        uuid: "workflow".to_string(),
        loading: true,
        ..WorkflowRunPromptDraft::default()
    });
    assert!(handle(&mut app.context(), WorkflowMessage::SubmitSavedAppRunPrompt).is_some());
    assert_eq!(
        app.workflow_state.saved_app_run_prompt.as_ref().unwrap().error.as_deref(),
        Some("运行参数仍在加载")
    );

    app.workflow_state.saved_app_run_prompt = Some(WorkflowRunPromptDraft {
        uuid: "workflow".to_string(),
        inputs: vec![WorkflowRunInputDraft {
            name: "topic".to_string(),
            label: "主题".to_string(),
            required: true,
            ..WorkflowRunInputDraft::default()
        }],
        ..WorkflowRunPromptDraft::default()
    });
    assert!(handle(&mut app.context(), WorkflowMessage::SubmitSavedAppRunPrompt).is_some());
    assert_eq!(
        app.workflow_state.saved_app_run_prompt.as_ref().unwrap().error.as_deref(),
        Some("主题 必填")
    );
}

#[test]
fn handle_cancel_prompt_and_ignores_finished_event_for_other_uuid() {
    let (mut app, _) = Fixture::new();
    app.workflow_state.saved_app_run_prompt = Some(WorkflowRunPromptDraft {
        uuid: "workflow".to_string(),
        query: "hello".to_string(),
        ..WorkflowRunPromptDraft::default()
    });

    assert!(handle(&mut app.context(), WorkflowMessage::CancelSavedAppRunPrompt).is_some());
    assert!(app.workflow_state.saved_app_run_prompt.is_none());

    app.workflow_state.running_saved_app_uuid = Some("active".to_string());
    assert!(
        handle(
            &mut app.context(),
            WorkflowMessage::SavedAppRunFinished {
                uuid: "other".to_string(),
                result: Err("network down".to_string()),
            },
        )
        .is_some()
    );
    assert_eq!(app.workflow_state.running_saved_app_uuid.as_deref(), Some("active"));
}
