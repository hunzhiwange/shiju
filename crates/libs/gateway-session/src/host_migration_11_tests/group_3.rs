use super::*;

#[test]
fn streamed_acp_tool_part_retains_display_title_and_input() {
    use session_engine::message::{Part, ToolState};
    use session_engine::processor_events::StreamEvent;
    use session_port::{ToolCallStatus, ToolEvent, ToolEventKind};

    let mut recorder = StreamTrajectoryRecorder::new("assistant".into());
    recorder.record(&StreamEvent::Tool(ToolEvent {
        call_id: "call-execute".into(),
        name: "execute".into(),
        kind: ToolEventKind::Complete,
        status: ToolCallStatus::Completed,
        occurred_ms: 20,
        payload: serde_json::json!({
            "toolCallId": "call-execute",
            "input": r#"{"command":"pwd"}"#,
            "output": "",
            "title": "Inspect workspace"
        }),
    }));
    let mut parts = Vec::new();
    recorder.append_tool_parts("session", &mut parts).unwrap();
    let [Part::Tool(part)] = parts.as_slice() else { panic!("expected one tool part") };
    let ToolState::Completed(state) = &part.state else { panic!("expected completed tool") };
    assert_eq!(part.tool, "execute");
    assert_eq!(state.title, "Inspect workspace");
    assert_eq!(state.input["command"], "pwd");
}

#[test]
fn recovery_event_preserves_status_without_assistant_content() {
    use session_engine::processor_events::StreamEvent;
    use session_port::{ModelRecovery, RecoveryPhase};
    for (phase, name) in [
        (RecoveryPhase::Waiting, "waiting"),
        (RecoveryPhase::Recovered, "recovered"),
        (RecoveryPhase::Stopped, "stopped"),
        (RecoveryPhase::Cancelled, "cancelled"),
    ] {
        let mut text = String::from("answer");
        let mut finish = None;
        let mut model = None;
        let mut current_step_index = None;
        let payload = stream_payload_for_non_done_event(
            StreamEvent::ModelRecovery(ModelRecovery {
                attempt: 2,
                max_attempts: 3,
                wait_ms: 500,
                reason: "temporary".into(),
                phase,
                next_action: "retry".into(),
            }),
            &mut text,
            &mut finish,
            &mut model,
            &mut current_step_index,
            "session-recovery",
        )
        .expect("recovery status must be transmitted");
        assert_eq!(
            payload,
            serde_json::json!({"type":"chat.model_recovery", "session_id":"session-recovery", "attempt":2,"max_attempts":3,"wait_ms":500,"reason":"temporary","phase":name,"next_action":"retry"})
        );
        assert_eq!(text, "answer");
        assert!(finish.is_none() && model.is_none());
        assert_eq!(current_step_index, None);
    }
}

#[test]
fn agcollab_gateway_keeps_inherited_skill_ceiling() {
    use config_agent::DelegateCapabilityPolicy;
    for role in [DelegateCapabilityPolicy::Strict, DelegateCapabilityPolicy::Unrestricted] {
        let options = merge_delegate_capability_policy_options(
            serde_json::json!({"delegate_skill_policy":"strict","delegate_default_allowed_skills":["shared","parent-only"]}),
            Some(vec!["shared".into(), "role-only".into()]),
            Some(role),
            None,
        );
        assert_eq!(options["delegate_skill_policy"], "strict");
        let expected = if role == DelegateCapabilityPolicy::Strict {
            serde_json::json!(["shared"])
        } else {
            serde_json::json!(["shared", "parent-only"])
        };
        assert_eq!(options["delegate_default_allowed_skills"], expected);
    }
}

#[test]
fn todo_continuation_emits_status_without_changing_assistant_text() {
    let mut assistant_text = "Earlier reply".to_string();
    let mut finish_reason = None;
    let mut assistant_model = None;
    let mut current_step_index = None;
    let event =
        processor_event_from_prompt_event(session_port::PromptEvent::TodoContinuation);
    let payload = stream_payload_for_non_done_event(
        event,
        &mut assistant_text,
        &mut finish_reason,
        &mut assistant_model,
        &mut current_step_index,
        "session-id",
    )
    .expect("todo status payload");
    assert_eq!(payload, serde_json::json!({"type": "chat.todo_continuation"}));
    assert_eq!(assistant_text, "Earlier reply");
}

#[tokio::test]
async fn task_pool_recovery_collector_waits_for_producer_shutdown() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tx.send(serde_json::json!({"type":"chat.done"})).unwrap();
    let mut collection = Box::pin(collect_scheduler_gateway_chat_stream_payloads(rx, |_| {}));
    // A ready terminal alone must not complete the scheduler collector.
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), &mut collection).await.is_err()
    );
    drop(tx);
    assert!(collection.await.is_ok());
}

#[tokio::test]
async fn task_pool_recovery_collector_keeps_shutdown_failure_after_done() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tx.send(serde_json::json!({"type":"chat.done"})).unwrap();
    tx.send(serde_json::json!({"type":"chat.error", "error":"execution shutdown unconfirmed"}))
        .unwrap();
    drop(tx);
    assert!(
        collect_scheduler_gateway_chat_stream_payloads(rx, |_| {})
            .await
            .unwrap_err()
            .to_string()
            .contains("execution shutdown unconfirmed")
    );
}
