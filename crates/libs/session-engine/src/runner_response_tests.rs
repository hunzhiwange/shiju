//! 验证步骤错误与空响应重试的局部终止规则。

fn unique_session_id(name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    format!("runner-tests-{name}-{nanos}")
}

fn event_texts(events: &[super::StreamEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            super::StreamEvent::LlmRawArtifact(_)
            | super::StreamEvent::ModelToolCalls(_)
            | super::StreamEvent::ModelRecovery(_) => None,
            super::StreamEvent::Delta(text) => Some(format!("delta:{text}")),
            super::StreamEvent::Tool(event) => {
                Some(format!("tool:{}:{}", event.name, event.payload))
            }
            super::StreamEvent::TodoContinuation => Some("todo-continuation".into()),
            super::StreamEvent::Compaction { phase } => Some(format!("compaction:{phase:?}")),
            super::StreamEvent::Error(text) => Some(format!("error:{text}")),
            super::StreamEvent::Done(_) => Some("done".to_string()),
            super::StreamEvent::PostToolRound { step_index } => {
                Some(format!("post-tool-round:{step_index}"))
            }
            super::StreamEvent::StepStart { step_index, .. } => {
                Some(format!("step-start:{step_index}"))
            }
            super::StreamEvent::StepFinish { step_index, .. } => {
                Some(format!("step-finish:{step_index}"))
            }
            super::StreamEvent::ContextLoaded { paths } => {
                Some(format!("context-loaded:{}", paths.join(",")))
            }
        })
        .collect()
}

#[test]
fn handle_step_error_emits_error_for_non_empty_response_failure() {
    let mut session = super::Session::new(unique_session_id("step-error"));
    let mut llm_messages = Vec::new();
    let mut retries = 0usize;
    let mut step = 0;
    let mut events = Vec::new();

    let should_continue = super::handle_step_error(
        &mut session,
        &mut llm_messages,
        &mut retries,
        &mut step,
        &mut |event| {
            events.push(event);
            true
        },
        "provider failed".to_string(),
    );

    assert!(!should_continue);
    assert_eq!(step, 0);
    assert_eq!(retries, 0);
    assert!(llm_messages.is_empty());
    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, super::Role::System);
    assert_eq!(event_texts(&events), vec!["error:provider failed"]);
}

#[test]
fn handle_step_error_retries_empty_llm_responses_until_limit() {
    let mut session = super::Session::new(unique_session_id("empty-step-error"));
    let mut llm_messages = Vec::new();
    let mut retries = 0usize;
    let mut step = 0;
    let mut events = Vec::new();

    let should_continue = super::handle_step_error(
        &mut session,
        &mut llm_messages,
        &mut retries,
        &mut step,
        &mut |event| {
            events.push(event);
            true
        },
        super::EMPTY_RESPONSE_ERR.to_string(),
    );

    assert!(should_continue);
    assert_eq!(retries, 1);
    assert_eq!(step, 1);
    assert_eq!(llm_messages.len(), 1);
    assert!(events.is_empty());
    assert!(session.messages[0].content.contains("自动重试中"));

    retries = super::EMPTY_RESPONSE_RETRY_LIMIT;
    let should_continue = super::handle_step_error(
        &mut session,
        &mut llm_messages,
        &mut retries,
        &mut step,
        &mut |event| {
            events.push(event);
            true
        },
        super::EMPTY_RESPONSE_ERR.to_string(),
    );

    assert!(!should_continue);
    assert_eq!(retries, super::EMPTY_RESPONSE_RETRY_LIMIT + 1);
    assert!(event_texts(&events).last().is_some_and(|text| text.contains("任务终止")));
}

#[test]
fn handle_step_error_recognizes_trimmed_empty_response_marker() {
    let mut session = super::Session::new(unique_session_id("trimmed-empty-step-error"));
    let mut llm_messages = Vec::new();
    let mut retries = 0usize;
    let mut step = 0;
    let mut events = Vec::new();

    let should_continue = super::handle_step_error(
        &mut session,
        &mut llm_messages,
        &mut retries,
        &mut step,
        &mut |event| {
            events.push(event);
            true
        },
        format!(" \n{}\t", super::EMPTY_RESPONSE_ERR),
    );

    assert!(should_continue);
    assert_eq!(retries, 1);
    assert_eq!(step, 1);
    assert_eq!(llm_messages[0]["content"].as_str(), Some(super::EMPTY_RESPONSE_RETRY_PROMPT));
    assert!(events.is_empty());
}

#[test]
fn handle_step_error_saturates_retry_counter_before_terminal_error() {
    let mut session = super::Session::new(unique_session_id("saturated-step-error"));
    let mut llm_messages = Vec::new();
    let mut retries = usize::MAX;
    let mut step = 3;
    let mut events = Vec::new();

    let should_continue = super::handle_step_error(
        &mut session,
        &mut llm_messages,
        &mut retries,
        &mut step,
        &mut |event| {
            events.push(event);
            true
        },
        super::EMPTY_RESPONSE_ERR.to_string(),
    );

    assert!(!should_continue);
    assert_eq!(retries, usize::MAX);
    assert_eq!(step, 3);
    assert!(llm_messages.is_empty());
    assert_eq!(session.messages.len(), 1);
    assert!(event_texts(&events)[0].contains("任务终止"));
}
