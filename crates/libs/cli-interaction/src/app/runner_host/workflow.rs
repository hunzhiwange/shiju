//! 工作流恢复结果的归并与会话持久化。

use crate::app::{ActiveRunner, V3GatewayApp, push_runner_error};

impl ActiveRunner {
    pub(super) fn finish_workflow_resume(
        &mut self,
        app: &mut V3GatewayApp,
        pause_request_id: String,
        result: Result<Box<vw_api_workflow::WorkflowRunResponse>, String>,
    ) {
        self.interaction_in_flight = false;
        match result {
            Ok(response) => {
                let projected = self
                    .workflow_run
                    .as_mut()
                    .ok_or_else(|| "workflow resume has no owned run state".to_string())
                    .and_then(|ownership| {
                        if ownership.pause_request_id().as_deref()
                            != Some(pause_request_id.as_str())
                        {
                            return Err(
                                "workflow resume result does not match the current pause"
                                    .to_string(),
                            );
                        }
                        let events = ownership.project_resume_response(*response)?;
                        let question = ownership.pause_question()?;
                        Ok((events, question))
                    });
                match projected {
                    Ok((events, question)) => {
                        for event in events {
                            app.accept_runtime_event(event);
                        }
                        app.replace_workflow_pause_question(question);
                        if let Err(error) = app.persist_session_snapshot() {
                            push_runner_error(
                                app,
                                format!("session save failed: {error}"),
                            );
                        }
                    }
                    Err(error) => app.record_interaction_error(error),
                }
            }
            Err(error) => app.record_interaction_error(format!(
                "failed to resume Workflow pause {pause_request_id}: {error}"
            )),
        }
        app.show_next_interaction();
    }
}
