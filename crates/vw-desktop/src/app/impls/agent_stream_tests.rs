// Tests for plan6 task 846.
const SOURCE: &str = include_str!("agent_stream.rs");

const _: () = assert!(
    super::PLAN_RUN_DELTA_FLUSH_INTERVAL.as_secs() == 10
        && super::PLAN_RUN_DELTA_FLUSH_INTERVAL.subsec_nanos() == 0
);

fn source_declares_symbol(name: &str) -> bool {
    let needles = [
        format!("fn {name}"),
        format!("pub fn {name}"),
        format!("struct {name}"),
        format!("pub struct {name}"),
        format!("enum {name}"),
        format!("pub enum {name}"),
        format!("type {name}"),
        format!("pub type {name}"),
        format!("const {name}"),
        format!("pub const {name}"),
        format!("static {name}"),
        format!("pub static {name}"),
        format!("impl {name}"),
    ];

    needles.iter().any(|needle| SOURCE.contains(needle))
}

#[test]
fn agent_stream_tests_keeps_planned_coverage_targets() {
    for name in [
        "agent_stream",
        "gateway_agent_stream",
        "parse_usage",
        "temporary_workflow_yaml",
        "workflow_mode_yaml",
        "stream_workflow_chat_messages",
    ] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}

#[test]
fn plan_run_delta_buffer_coalesces_before_ui_dispatch() {
    use std::time::Duration;

    let mut buffer = super::PlanRunDeltaBuffer::default();
    let mut dispatched = Vec::new();
    let mut send = |batch: &super::PlanRunDeltaBatch| {
        dispatched.push(batch.clone());
        Ok::<(), ()>(())
    };
    let inputs = [
        (0, "session-a", "你", 1),
        (10, "session-a", "好", 2),
        (20, "session-b", "B1", 3),
        (30, "session-b", "B2", 4),
        (40, "session-a", "!", 5),
        (300, "session-a", "done", 6),
    ];

    for (millis, session_id, delta, event_id) in inputs {
        buffer
            .push_delta(
                session_id,
                delta,
                event_id,
                Duration::from_millis(millis),
                &mut send,
            )
            .expect("delta dispatch");
    }
    buffer.flush(&mut send).expect("final flush");

    assert!(dispatched.len() < inputs.len());
    assert_eq!(
        dispatched
            .iter()
            .map(|batch| (batch.session_id.as_str(), batch.delta.as_str()))
            .collect::<Vec<_>>(),
        vec![("session-a", "你好"), ("session-b", "B1B2"), ("session-a", "!done")]
    );
    assert_eq!(
        dispatched.iter().map(|batch| batch.delta.as_str()).collect::<String>(),
        inputs.iter().map(|(_, _, delta, _)| *delta).collect::<String>()
    );
    assert_eq!(dispatched[0].event_count, 2);
    assert_eq!(dispatched[0].first_event_id, 1);
    assert_eq!(dispatched[0].last_event_id, 2);

    let mut window_buffer = super::PlanRunDeltaBuffer::default();
    let mut window_batches = Vec::new();
    window_buffer
        .push_delta(
            "session-a",
            &"x".repeat(360),
            7,
            Duration::ZERO,
            &mut |batch| {
                window_batches.push(batch.clone());
                Ok::<(), ()>(())
            },
        )
        .expect("buffer former soft threshold");
    assert!(window_batches.is_empty(), "360 characters must not flush early");
    window_buffer
        .push_delta(
            "session-a",
            "y",
            8,
            Duration::from_millis(9_999),
            &mut |batch| {
                window_batches.push(batch.clone());
                Ok::<(), ()>(())
            },
        )
        .expect("buffer before ten-second window");
    assert!(window_batches.is_empty(), "9.999 seconds must not flush");
    window_buffer
        .push_delta(
            "session-a",
            "界",
            9,
            Duration::from_secs(10),
            &mut |batch| {
                window_batches.push(batch.clone());
                Ok::<(), ()>(())
            },
        )
        .expect("flush at ten-second window");
    assert_eq!(window_batches.len(), 1);
    assert_eq!(
        window_batches[0].delta,
        format!("{}y", "x".repeat(360))
    );
    assert_eq!(window_batches[0].event_count, 2);
    assert_eq!(window_batches[0].first_event_id, 7);
    assert_eq!(window_batches[0].last_event_id, 8);
    assert_eq!(window_batches[0].cursor_event_id, Some(8));
    window_buffer
        .flush(&mut |batch| {
            window_batches.push(batch.clone());
            Ok::<(), ()>(())
        })
        .expect("flush boundary remainder");
    assert_eq!(
        window_batches.iter().map(|batch| batch.delta.as_str()).collect::<String>(),
        format!("{}y界", "x".repeat(360))
    );

    let mut hard_limit_buffer = super::PlanRunDeltaBuffer::default();
    let mut hard_limit_batches = Vec::new();
    let oversized = "z".repeat(super::PLAN_RUN_DELTA_HARD_LIMIT_BYTES + 1);
    hard_limit_buffer
        .push_delta(
            "session-hard-limit",
            &oversized,
            10,
            Duration::ZERO,
            &mut |batch| {
                hard_limit_batches.push(batch.clone());
                Ok::<(), ()>(())
            },
        )
        .expect("split oversized delta");
    hard_limit_buffer
        .flush(&mut |batch| {
            hard_limit_batches.push(batch.clone());
            Ok::<(), ()>(())
        })
        .expect("flush oversized remainder");
    assert!(
        hard_limit_batches
            .iter()
            .all(|batch| batch.delta.len() <= super::PLAN_RUN_DELTA_HARD_LIMIT_BYTES)
    );
    assert_eq!(
        hard_limit_batches
            .iter()
            .filter(|batch| batch.first_event_id == 10)
            .map(|batch| batch.delta.as_str())
            .collect::<String>(),
        oversized
    );
}

#[test]
fn plan_run_delta_buffer_flushes_before_lifecycle_event() {
    use std::time::Duration;

    #[derive(Debug, PartialEq, Eq)]
    enum Dispatch {
        Delta(String),
        Lifecycle(&'static str),
    }

    let mut buffer = super::PlanRunDeltaBuffer::default();
    let mut dispatched = Vec::new();
    buffer
        .push_delta(
            "session-a",
            "partial ",
            20,
            Duration::ZERO,
            &mut |batch| {
                dispatched.push(Dispatch::Delta(batch.delta.clone()));
                Ok::<(), ()>(())
            },
        )
        .expect("first delta");
    buffer
        .push_delta(
            "session-a",
            "answer",
            21,
            Duration::from_millis(10),
            &mut |batch| {
                dispatched.push(Dispatch::Delta(batch.delta.clone()));
                Ok::<(), ()>(())
            },
        )
        .expect("second delta");

    let flushed_cursor = buffer
        .flush(&mut |batch| {
            dispatched.push(Dispatch::Delta(batch.delta.clone()));
            Ok::<(), ()>(())
        })
        .expect("flush before lifecycle");
    dispatched.push(Dispatch::Lifecycle("session_waiting"));

    assert_eq!(flushed_cursor, Some(21));
    assert_eq!(
        dispatched,
        vec![Dispatch::Delta("partial answer".to_string()), Dispatch::Lifecycle("session_waiting")]
    );
}

#[test]
fn plan_run_delta_buffer_preserves_cursor_on_backpressure() {
    use std::time::Duration;

    let mut buffer = super::PlanRunDeltaBuffer::default();
    let mut delivered = Vec::new();
    let mut resume_cursor = None;
    for (event_id, delta) in [(30, "one"), (31, " two")] {
        buffer
            .push_delta(
                "session-a",
                delta,
                event_id,
                Duration::from_millis(event_id),
                &mut |batch| {
                    delivered.push(batch.clone());
                    Ok::<(), &'static str>(())
                },
            )
            .expect("buffer initial delta");
    }
    resume_cursor = buffer
        .flush(&mut |batch| {
            delivered.push(batch.clone());
            Ok::<(), &'static str>(())
        })
        .expect("initial delivery")
        .or(resume_cursor);
    assert_eq!(resume_cursor, Some(31));

    for (event_id, delta) in [(32, "three"), (33, " four")] {
        buffer
            .push_delta(
                "session-a",
                delta,
                event_id,
                Duration::from_millis(event_id),
                &mut |_batch| Err::<(), _>("full"),
            )
            .expect("no threshold flush");
    }
    let failed = buffer.flush(&mut |_batch| Err::<(), _>("full"));
    assert_eq!(failed, Err("full"));
    assert_eq!(resume_cursor, Some(31), "failed delivery must not advance the cursor");
    assert!(buffer.has_pending());

    resume_cursor = buffer
        .flush(&mut |batch| {
            delivered.push(batch.clone());
            Ok::<(), &'static str>(())
        })
        .expect("retry delivery")
        .or(resume_cursor);
    assert_eq!(resume_cursor, Some(33));
    assert_eq!(delivered.last().expect("retried batch").delta, "three four");
    assert!(!buffer.has_pending());
}

#[test]
fn chat_tool_event_keeps_legacy_tool_content() {
    let payload = serde_json::json!({
        "type": "chat.tool",
        "step_index": 3,
        "call_id": "call-1",
        "name": "shell",
        "kind": "start",
        "status": "running",
        "occurred_ms": 99,
        "payload": {"status": "running", "input": "{}"}
    });

    let message = super::chat_tool_message(7, &payload).expect("desktop tool message");
    let crate::app::Message::Chat(crate::app::message::chat::ChatMessage::AgentToolLifecycle(
        request_id,
        event,
    )) = message
    else {
        panic!("expected dedicated tool lifecycle message");
    };

    assert_eq!(request_id, 7);
    assert_eq!(event.step_index, 3);
    assert_eq!(event.call_id, "call-1");
    assert_eq!(event.legacy_delta(), "tool shell\n{\"input\":\"{}\",\"status\":\"running\"}\n");
}

#[test]
fn gateway_stream_retries_incomplete_attempts_until_terminal() {
    use std::collections::VecDeque;
    use std::time::Duration;

    use vw_gateway_client::GatewayStreamAttemptResult;

    assert_eq!(super::GATEWAY_STREAM_MAX_RETRY_DELAY, Duration::from_secs(3 * 60 * 60));
    assert!(
        !SOURCE.contains("任务异常终止：未收到网关完成信号"),
        "an incomplete gateway attempt must enter the reconnect loop instead of failing"
    );
    assert!(
        SOURCE.contains("stream_chat_resumable")
            && SOURCE.contains("stream_task_plan_run_resumable"),
        "chat and plan-run must both use the resumable gateway APIs"
    );

    #[derive(Clone)]
    struct ScriptedAttempt {
        result: Result<GatewayStreamAttemptResult, String>,
        saw_new_event: bool,
        last_forwarded_event_id: Option<u64>,
        stop_reason: Option<super::GatewayStreamStopReason>,
    }

    let run_until_stopped = |stream_kind: &str, script: Vec<ScriptedAttempt>| {
        let mut state = super::GatewayReconnectState::new(format!("desktop-test-{stream_kind}"));
        let mut attempts = VecDeque::from(script);
        let mut attempt_count = 0;
        let mut waits = Vec::new();
        let mut terminal_messages = 0;

        loop {
            let attempt = attempts.pop_front().expect("scripted attempt");
            attempt_count += 1;
            let next = state.finish_attempt(
                attempt.result,
                attempt.saw_new_event,
                attempt.last_forwarded_event_id,
                attempt.stop_reason,
            );
            match next {
                super::GatewayStreamNext::Retry(delay) => waits.push(delay),
                super::GatewayStreamNext::Terminal => {
                    terminal_messages += 1;
                    break;
                }
                _other => break,
            }
        }

        (state, attempt_count, waits, terminal_messages)
    };

    let incomplete = ScriptedAttempt {
        result: Ok(GatewayStreamAttemptResult::Incomplete),
        saw_new_event: false,
        last_forwarded_event_id: None,
        stop_reason: None,
    };
    let terminal = ScriptedAttempt {
        result: Ok(GatewayStreamAttemptResult::Terminated),
        saw_new_event: true,
        last_forwarded_event_id: Some(7),
        stop_reason: None,
    };
    for stream_kind in ["chat", "plan_run"] {
        let (state, attempt_count, waits, terminal_messages) = run_until_stopped(
            stream_kind,
            vec![incomplete.clone(), incomplete.clone(), terminal.clone()],
        );
        assert_eq!(attempt_count, 3, "{stream_kind} attempt count");
        assert_eq!(
            waits,
            vec![Duration::from_millis(500), Duration::from_secs(1)],
            "{stream_kind} retry sequence"
        );
        assert_eq!(terminal_messages, 1, "{stream_kind} terminal message count");
        assert_eq!(state.last_event_id, Some(7), "{stream_kind} resume cursor");
    }

    let mut reset = super::GatewayReconnectState::new("desktop-test-reset".to_string());
    assert_eq!(
        reset.finish_attempt(Ok(GatewayStreamAttemptResult::Incomplete), false, None, None,),
        super::GatewayStreamNext::Retry(Duration::from_millis(500))
    );
    assert_eq!(
        reset.finish_attempt(Ok(GatewayStreamAttemptResult::Incomplete), false, None, None,),
        super::GatewayStreamNext::Retry(Duration::from_secs(1))
    );
    assert_eq!(
        reset.finish_attempt(Ok(GatewayStreamAttemptResult::Incomplete), true, Some(11), None,),
        super::GatewayStreamNext::Retry(Duration::from_millis(500))
    );
    assert_eq!(reset.resume().last_event_id, Some(11));

    let mut capped = super::GatewayReconnectState::new("desktop-test-cap".to_string());
    let delays = (0..17)
        .map(|_| {
            let super::GatewayStreamNext::Retry(delay) = capped.finish_attempt(
                Ok(GatewayStreamAttemptResult::Incomplete),
                false,
                None,
                None,
            ) else {
                panic!("incomplete attempts must retry");
            };
            delay
        })
        .collect::<Vec<_>>();
    assert_eq!(delays[0], Duration::from_millis(500));
    assert_eq!(delays[1], Duration::from_secs(1));
    assert_eq!(delays[14], Duration::from_secs(8_192));
    assert_eq!(delays[15], Duration::from_secs(3 * 60 * 60));
    assert_eq!(delays[16], Duration::from_secs(3 * 60 * 60));

    let mut cancelled = super::GatewayReconnectState::new("desktop-test-cancel".to_string());
    assert_eq!(
        cancelled.finish_attempt(
            Ok(GatewayStreamAttemptResult::CallerStopped),
            false,
            None,
            Some(super::GatewayStreamStopReason::Cancelled),
        ),
        super::GatewayStreamNext::Cancelled
    );
    assert_eq!(cancelled.resume().last_event_id, None);

    let mut backpressured =
        super::GatewayReconnectState::new("desktop-test-backpressure".to_string());
    assert_eq!(
        backpressured.finish_attempt(
            Ok(GatewayStreamAttemptResult::CallerStopped),
            true,
            None,
            Some(super::GatewayStreamStopReason::Backpressure),
        ),
        super::GatewayStreamNext::Retry(Duration::from_millis(500))
    );
    assert_eq!(
        backpressured.resume().last_event_id,
        None,
        "a full UI channel must not advance an unforwarded event cursor"
    );

    let mut terminal_error =
        super::GatewayReconnectState::new("desktop-test-terminal-error".to_string());
    assert_eq!(
        terminal_error.finish_attempt(
            Ok(GatewayStreamAttemptResult::Terminated),
            true,
            Some(3),
            None,
        ),
        super::GatewayStreamNext::Terminal
    );

    let mut rejected = super::GatewayReconnectState::new("desktop-test-rejected".to_string());
    assert_eq!(
        rejected.finish_attempt(
            Err("gateway stream failed: 400 Bad Request".to_string()),
            false,
            None,
            None,
        ),
        super::GatewayStreamNext::Failed("gateway stream failed: 400 Bad Request".to_string())
    );

    let request = super::AgentRequest {
        id: 91,
        session: "private-session".to_string(),
        query: "private request body".to_string(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: Vec::new(),
    };
    let stable_id = super::desktop_stream_request_id(&request);
    assert_eq!(stable_id, super::desktop_stream_request_id(&request));
    assert!(!stable_id.contains(&request.session));
    assert!(!stable_id.contains(&request.query));
}

#[test]
fn subagent_mode_request_gateway_options_sets_requirement_and_preserves_tools() {
    let mut request = super::AgentRequest {
        id: 92,
        session: "subagent-session".to_string(),
        query: "delegate".to_string(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: Some(vec!["AgentTool".to_string(), "shell".to_string()]),
        require_agent_tool: true,
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: Vec::new(),
    };

    let options = super::gateway_request_options(
        &request,
        Some(crate::app::message::chat::session::SUBAGENT_MODE_SYSTEM_PROMPT),
        "desktop-test-92",
        "Asia/Shanghai",
    )
    .expect("subagent options");
    assert_eq!(options["require_agent_tool"], true);
    assert_eq!(options["allowed_tools"], serde_json::json!(["AgentTool", "shell"]));
    assert_eq!(
        options["chat_system_prompt"],
        crate::app::message::chat::session::SUBAGENT_MODE_SYSTEM_PROMPT
    );

    request.require_agent_tool = false;
    request.allowed_tools = Some(vec!["shell".to_string()]);
    let disabled =
        super::gateway_request_options(&request, None, "desktop-test-92", "Asia/Shanghai")
            .expect("restricted options");
    assert!(disabled.get("require_agent_tool").is_none());
    assert_eq!(disabled["allowed_tools"], serde_json::json!(["shell"]));
}

#[test]
fn temporary_workflow_yaml_contains_required_chat_nodes() {
    let yaml = super::temporary_workflow_yaml(
        "请帮我实现 chat 面板里的工作流执行进度展示",
        Some("demo-model"),
    )
    .expect("workflow yaml");
    let value: serde_yaml::Value = serde_yaml::from_str(&yaml).expect("valid yaml");
    let nodes = value
        .get("workflow")
        .and_then(|workflow| workflow.get("graph"))
        .and_then(|graph| graph.get("nodes"))
        .and_then(serde_yaml::Value::as_sequence)
        .expect("nodes");

    let ids = nodes
        .iter()
        .filter_map(|node| node.get("id").and_then(serde_yaml::Value::as_str))
        .collect::<Vec<_>>();
    assert!(ids.contains(&"start"));
    assert!(ids.contains(&"task_1"));
    assert!(ids.contains(&"answer"));
    assert!(!ids.contains(&"parallel_context"));

    let edges = value
        .get("workflow")
        .and_then(|workflow| workflow.get("graph"))
        .and_then(|graph| graph.get("edges"))
        .and_then(serde_yaml::Value::as_sequence)
        .expect("edges");
    assert!(edges.iter().any(|edge| {
        edge.get("source").and_then(serde_yaml::Value::as_str) == Some("start")
            && edge.get("target").and_then(serde_yaml::Value::as_str) == Some("task_1")
    }));
    assert!(edges.iter().any(|edge| {
        edge.get("source").and_then(serde_yaml::Value::as_str) == Some("task_1")
            && edge.get("target").and_then(serde_yaml::Value::as_str) == Some("answer")
    }));

    let task_title = nodes
        .iter()
        .find(|node| node.get("id").and_then(serde_yaml::Value::as_str) == Some("task_1"))
        .and_then(|node| node.get("data"))
        .and_then(|data| data.get("title"))
        .and_then(serde_yaml::Value::as_str)
        .expect("task title");
    assert_eq!(task_title, "实现 chat 面板里的工作流执行进度展示");
}

#[test]
fn workflow_mode_yaml_keeps_temporary_workflow_entrypoint_compatible() {
    let query = "请先规划，再实现，再验证";
    let direct = super::temporary_workflow_yaml(query, Some("demo-model")).expect("direct yaml");
    let entrypoint = super::workflow_mode_yaml(query, Some("demo-model")).expect("entrypoint yaml");

    assert_eq!(entrypoint, direct);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn workflow_mode_streams_task_plan_run_events_instead_of_temporary_workflow_yaml() {
    assert!(
        SOURCE.contains("TASK_PLAN_RUN_STREAM_PATH"),
        "workflow mode should declare the task plan-run stream endpoint"
    );
    assert!(
        SOURCE.contains("parent_session_id"),
        "workflow mode should pass the main session as parent_session_id"
    );
    assert!(
        SOURCE.contains("blocked_reason") && SOURCE.contains("工作流执行受阻"),
        "workflow mode must treat Blocked as a terminal stream signal"
    );
    assert!(
        !SOURCE.contains("workflow_mode_yaml(&req.query"),
        "workflow mode should no longer build temporary workflow YAML before streaming"
    );

    let stream_body = concat!(
        "data: {\"type\":\"planning_started\",\"task_id\":\"task-1\",\"plan_dir\":\"/tmp/plan\",\"session_id\":\"task-1-plan\"}\n\n",
        "data: {\"type\":\"plan_created\",\"task_id\":\"task-1\",\"plan_dir\":\"/tmp/plan\",\"plan_md_path\":\"/tmp/plan/plan.md\",\"prompt_md_path\":\"/tmp/plan/prompt.md\",\"tasks_json_path\":\"/tmp/plan/tasks.json\",\"subtask_count\":1,\"session_id\":\"task-1-plan\"}\n\n",
        "data: {\"type\":\"batch_started\",\"task_id\":\"task-1\",\"batch\":1,\"subtask_ids\":[\"subtask-1\"]}\n\n",
        "data: {\"type\":\"subtask_started\",\"task_id\":\"task-1\",\"batch\":1,\"subtask_id\":\"subtask-1\",\"title\":\"实现入口\",\"task_file\":\"1-entry.md\",\"session_id\":\"task-1-subtask-1\"}\n\n",
        "data: {\"type\":\"subtask_finished\",\"task_id\":\"task-1\",\"batch\":1,\"subtask_id\":\"subtask-1\",\"title\":\"实现入口\",\"task_file\":\"1-entry.md\",\"success\":true,\"output\":\"ok\",\"session_id\":\"task-1-subtask-1\"}\n\n",
        "data: {\"type\":\"finished\",\"task_id\":\"task-1\",\"plan_dir\":\"/tmp/plan\",\"status\":\"completed\",\"output\":\"all done\",\"session_id\":\"task-1-plan\"}\n\n",
    )
    .to_string();
    let (endpoint, handle) = spawn_workflow_stream_response("200 OK", stream_body);
    let request = vw_gateway_client::vw_api_types::task::TaskPlanRunRequest {
        query: "请规划并执行".to_string(),
        root: "/tmp/project".to_string(),
        model: Some("demo-model".to_string()),
        agent: Some("main".to_string()),
        acp_agent: Some("codex".to_string()),
        parent_session_id: Some("main-session".to_string()),
    };
    let mut events = Vec::new();

    let result = super::stream_task_plan_run(&endpoint, &request, |event| {
        events.push(event);
        true
    })
    .await;
    let raw_request = handle.join().expect("server thread should finish");

    assert!(result.is_ok());
    assert!(raw_request.starts_with("POST /v1/task-plan-run/stream "));
    assert_eq!(events.len(), 6);
    assert!(matches!(
        events[0],
        vw_gateway_client::vw_api_types::task::TaskPlanRunStreamEventDto::PlanningStarted { .. }
    ));
    assert!(matches!(
        events[5],
        vw_gateway_client::vw_api_types::task::TaskPlanRunStreamEventDto::Finished { .. }
    ));

    let block = super::plan_run_tool_block(&events[1]);
    assert!(block.contains("\"canonical_tool_id\":\"workflow_node\""));
    assert!(block.contains("\"plan_run\":true"));
    assert!(block.contains("\"event_type\":\"plan_created\""));
    assert!(block.contains("tasks.json"));
}

#[test]
fn temporary_workflow_yaml_generates_different_graphs_by_requirement() {
    let coding =
        super::temporary_workflow_yaml("先梳理需求，然后实现 Rust API，最后验证接口", None)
            .expect("workflow yaml");
    let writing =
        super::temporary_workflow_yaml("写一封产品发布邮件", None).expect("workflow yaml");

    assert!(coding.contains("梳理需求"));
    assert!(coding.contains("实现 Rust API"));
    assert!(coding.contains("验证接口"));
    assert!(!coding.contains("Requirement Analysis"));
    assert!(writing.contains("写一封产品发布邮件"));
    assert_ne!(coding, writing);
}

#[test]
fn temporary_workflow_yaml_sets_dify_node_layout() {
    let yaml = super::temporary_workflow_yaml("为什么 workflow 会超时并卡死", None)
        .expect("workflow yaml");
    let value: serde_yaml::Value = serde_yaml::from_str(&yaml).expect("valid yaml");
    let nodes = value
        .get("workflow")
        .and_then(|workflow| workflow.get("graph"))
        .and_then(|graph| graph.get("nodes"))
        .and_then(serde_yaml::Value::as_sequence)
        .expect("nodes");

    let mut positions = Vec::new();
    for node in nodes {
        let position =
            node.get("position").and_then(serde_yaml::Value::as_mapping).expect("node position");
        let x = position
            .get(&serde_yaml::Value::String("x".to_string()))
            .and_then(serde_yaml::Value::as_i64)
            .expect("x");
        let y = position
            .get(&serde_yaml::Value::String("y".to_string()))
            .and_then(serde_yaml::Value::as_i64)
            .expect("y");
        assert_eq!(node.get("type").and_then(serde_yaml::Value::as_str), Some("custom"));
        assert!(node.get("positionAbsolute").is_some());
        assert!(node.get("sourcePosition").is_some());
        assert!(node.get("targetPosition").is_some());
        positions.push((x, y));
    }

    positions.sort_unstable();
    positions.dedup();
    assert_eq!(positions.len(), nodes.len());
}

#[test]
fn parse_workflow_chat_messages_event_maps_dify_payloads() {
    let delta = super::parse_workflow_chat_messages_event(serde_json::json!({
        "event": "message",
        "answer": "hello"
    }));
    assert!(matches!(delta, super::WorkflowChatMessagesEvent::Delta(text) if text == "hello"));

    let done = super::parse_workflow_chat_messages_event(serde_json::json!({
        "event": "message_end",
        "message_id": "msg-workflow",
        "metadata": {
            "usage": {
                "prompt_tokens": 1,
                "completion_tokens": 2,
                "total_tokens": 3
            }
        }
    }));
    match done {
        super::WorkflowChatMessagesEvent::Done { usage, message_id } => {
            assert_eq!(message_id.as_deref(), Some("msg-workflow"));
            assert_eq!(
                usage
                    .as_ref()
                    .and_then(|usage| usage.get("total_tokens"))
                    .and_then(serde_json::Value::as_i64),
                Some(3)
            );
        }
        _ => panic!("message_end should parse as Done"),
    }

    let node_started = super::parse_workflow_chat_messages_event(serde_json::json!({
        "event": "node_started",
        "data": {
            "node_id": "llm",
            "node_type": "llm",
            "title": "Run Task",
            "index": 2
        }
    }));
    assert!(matches!(
        node_started,
        super::WorkflowChatMessagesEvent::NodeStarted(event)
            if event.node_id == "llm" && event.title == "Run Task" && event.index == 2
    ));

    let node_finished = super::parse_workflow_chat_messages_event(serde_json::json!({
        "event": "node_finished",
        "data": {
            "node_id": "llm",
            "node_type": "llm",
            "title": "Run Task",
            "index": 2,
            "status": "succeeded",
            "outputs": {"answer": "ok"}
        }
    }));
    assert!(matches!(
        node_finished,
        super::WorkflowChatMessagesEvent::NodeFinished(event)
            if event.node_id == "llm" && event.status.as_deref() == Some("succeeded")
    ));

    let text_chunk = super::parse_workflow_chat_messages_event(serde_json::json!({
        "event": "text_chunk",
        "data": {
            "text": "{\"rows\":1}",
            "from_variable_selector": ["answer", "text"]
        }
    }));
    assert!(
        matches!(text_chunk, super::WorkflowChatMessagesEvent::Delta(text) if text == "{\"rows\":1}")
    );
}

#[test]
fn workflow_node_tool_block_marks_running_and_finished_states() {
    let event = super::WorkflowNodeStreamEvent {
        node_id: "llm".to_string(),
        node_type: "llm".to_string(),
        title: "Run Task".to_string(),
        index: 2,
        status: Some("succeeded".to_string()),
        elapsed_time: Some(1.25),
        error: None,
        outputs: Some(serde_json::json!({"answer": "ok"})),
    };

    let workflow_yaml = "kind: app\nworkflow:\n  graph:\n    nodes: []\n    edges: []\n";
    let running = super::workflow_node_tool_block(&event, true, workflow_yaml);
    assert!(running.contains("\"status\":\"running\""));
    assert!(running.contains("\"canonical_tool_id\":\"workflow_node\""));
    assert!(running.contains("workflow_yaml"));
    assert!(running.contains("\\\"answer\\\":\\\"ok\\\""));

    let finished = super::workflow_node_tool_block(&event, false, workflow_yaml);
    assert!(finished.contains("\"status\":\"completed\""));
    assert!(finished.contains("\\\"answer\\\":\\\"ok\\\""));
}

#[test]
fn workflow_node_tool_block_exposes_usage_metadata() {
    let event = super::WorkflowNodeStreamEvent {
        node_id: "llm".to_string(),
        node_type: "llm".to_string(),
        title: "Run Task".to_string(),
        index: 2,
        status: Some("succeeded".to_string()),
        elapsed_time: Some(1.25),
        error: None,
        outputs: Some(serde_json::json!({
            "answer": "ok",
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        })),
    };

    let block = super::workflow_node_tool_block(&event, false, "kind: app\n");
    assert!(block.contains("\"usage\""));
    assert!(block.contains("\"total_tokens\":15"));
}

#[test]
fn workflow_history_context_filters_empty_messages_and_keeps_order() {
    let history = vec![
        crate::app::models::ChatMessage {
            role: crate::app::models::ChatRole::System,
            content: " system rules ".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        crate::app::models::ChatMessage {
            role: crate::app::models::ChatRole::User,
            content: "   ".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        crate::app::models::ChatMessage {
            role: crate::app::models::ChatRole::Assistant,
            content: " answer ".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        crate::app::models::ChatMessage {
            role: crate::app::models::ChatRole::Tool,
            content: " tool output ".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    assert_eq!(
        super::workflow_history_context(&history),
        "System: system rules\n\nAssistant: answer\n\nTool: tool output"
    );
}

#[test]
fn workflow_history_context_truncates_large_context_from_tail() {
    let history = vec![crate::app::models::ChatMessage {
        role: crate::app::models::ChatRole::User,
        content: "x".repeat(super::WORKFLOW_HISTORY_MAX_CHARS + 64),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    let context = super::workflow_history_context(&history);

    assert!(context.starts_with("...[truncated]\n"));
    assert!(context.len() <= super::WORKFLOW_HISTORY_MAX_CHARS + "...[truncated]\n".len());
    assert!(context.ends_with("xxxxxxxx"));
}

#[test]
fn temporary_workflow_task_titles_follow_user_requirement() {
    assert_eq!(
        super::temporary_workflow_task_titles("先读取 CSV，然后统计金额，最后生成报表"),
        vec!["读取 CSV", "统计金额", "生成报表"]
    );
    assert_eq!(
        super::temporary_workflow_task_titles("1. 设计接口\n2. 实现 API\n3. 补充测试"),
        vec!["设计接口", "实现 API", "补充测试"]
    );
}

#[test]
fn temporary_workflow_task_titles_strip_prefixes_dedup_and_limit_long_lists() {
    let titles = super::temporary_workflow_task_titles(
        "一、请帮我分析需求\n二、请帮我分析需求\n3）实现功能\n4: 补测试\n5. 写文档\n6. 发版\n7. 监控\n8. 复盘\n9. 归档\n10. 清理",
    );

    assert_eq!(titles.len(), super::TEMPORARY_WORKFLOW_MAX_TASKS);
    assert_eq!(titles[0], "分析需求");
    assert_eq!(titles[1], "实现功能");
    assert_eq!(titles[7], "完成剩余任务：归档；清理");
}

#[test]
fn temporary_workflow_task_titles_falls_back_for_empty_requirement() {
    assert_eq!(super::temporary_workflow_task_titles("   "), vec!["完成用户需求"]);
}

#[test]
fn temporary_workflow_topology_uses_requirement_language() {
    assert_eq!(
        super::temporary_workflow_topology("同时生成标题", 1),
        super::TemporaryWorkflowTopology::Serial
    );
    assert_eq!(
        super::temporary_workflow_topology("同时生成标题并且生成摘要", 2),
        super::TemporaryWorkflowTopology::Parallel
    );
    assert_eq!(
        super::temporary_workflow_topology("先生成标题，然后写摘要", 2),
        super::TemporaryWorkflowTopology::Serial
    );
}

#[test]
fn temporary_workflow_yaml_trims_empty_model_and_sets_non_empty_model() {
    let without_model = super::temporary_workflow_yaml("普通任务", Some("   ")).expect("yaml");
    assert!(!without_model.contains("provider: vibewindow"));

    let with_model = super::temporary_workflow_yaml("普通任务", Some(" gpt-local ")).expect("yaml");
    assert!(with_model.contains("name: gpt-local"));
    assert!(with_model.contains("provider: vibewindow"));
}

#[test]
fn temporary_workflow_plan_serial_edges_chain_to_final_node() {
    let plan = super::temporary_workflow_plan("先读取文件，然后总结内容");

    assert_eq!(plan.topology, super::TemporaryWorkflowTopology::Serial);
    assert_eq!(plan.final_node_id, "task_2");
    assert_eq!(plan.edges[0].source, "start");
    assert_eq!(plan.edges[0].target, "task_1");
    assert_eq!(plan.edges[1].source, "task_1");
    assert_eq!(plan.edges[1].target, "task_2");
    assert!(plan.steps[1].system_prompt.contains("final task node"));
}

#[test]
fn temporary_workflow_yaml_builds_dynamic_parallel_tasks_when_requested() {
    let yaml = super::temporary_workflow_yaml("同时生成标题并且生成摘要", None).expect("yaml");
    let value: serde_yaml::Value = serde_yaml::from_str(&yaml).expect("valid yaml");
    let nodes = value
        .get("workflow")
        .and_then(|workflow| workflow.get("graph"))
        .and_then(|graph| graph.get("nodes"))
        .and_then(serde_yaml::Value::as_sequence)
        .expect("nodes");
    let titles = nodes
        .iter()
        .filter_map(|node| node.get("data"))
        .filter_map(|data| data.get("title"))
        .filter_map(serde_yaml::Value::as_str)
        .collect::<Vec<_>>();

    assert!(titles.contains(&"生成标题"));
    assert!(titles.contains(&"生成摘要"));
    assert!(titles.contains(&"汇总并交付最终结果"));
    assert!(!yaml.contains("Parallel Context"));
}

#[test]
fn workflow_node_helpers_build_stable_keys_and_delta_preview_event() {
    let delta = super::WorkflowNodeDeltaStreamEvent {
        node_id: "node-a".to_string(),
        node_type: "llm".to_string(),
        title: "Draft".to_string(),
        index: 7,
        text: "partial".to_string(),
        replace: false,
    };
    let mut output = super::WorkflowNodeOutputBuffer::new();
    output.push("partial", false);

    let event = super::workflow_node_stream_event_from_delta(&delta, &output);

    assert_eq!(super::workflow_node_delta_event_key(&delta), "7-node-a");
    assert_eq!(super::workflow_node_stream_event_key(&event), "7-node-a");
    assert_eq!(super::workflow_node_tool_call_id(&event), "workflow-node-7-node-a");
    assert_eq!(event.status.as_deref(), Some("running"));
    assert_eq!(
        event
            .outputs
            .as_ref()
            .and_then(|outputs| outputs.get("answer"))
            .and_then(serde_json::Value::as_str),
        Some("partial")
    );
}

#[test]
fn workflow_node_parsers_accept_top_level_payloads_and_defaults() {
    let node = super::parse_workflow_node_event(&serde_json::json!({
        "node_id": "top",
        "node_type": "tool",
        "title": "Top Level",
        "index": 4,
        "elapsed_time": 0.5,
        "error": "failed",
        "outputs": null
    }));
    assert_eq!(node.node_id, "top");
    assert_eq!(node.node_type, "tool");
    assert_eq!(node.title, "Top Level");
    assert_eq!(node.index, 4);
    assert_eq!(node.elapsed_time, Some(0.5));
    assert_eq!(node.error.as_deref(), Some("failed"));
    assert_eq!(node.outputs, Some(serde_json::Value::Null));

    let delta = super::parse_workflow_node_delta_event(&serde_json::json!({
        "node_id": "delta",
        "answer": "fallback answer"
    }));
    assert_eq!(delta.node_id, "delta");
    assert_eq!(delta.text, "fallback answer");
    assert!(!delta.replace);
}

#[test]
fn workflow_node_output_buffer_flushes_by_force_replace_and_size() {
    let mut output = super::WorkflowNodeOutputBuffer::new();
    output.push("abc", false);
    assert!(!output.should_flush(false));
    assert!(output.should_flush(true));

    output.push(&"x".repeat(super::WORKFLOW_NODE_STREAM_FLUSH_CHARS), false);
    assert!(output.should_flush(false));

    output.push("replacement", true);
    assert_eq!(output.text, "replacement");
    assert!(output.should_flush(true));
}

#[test]
fn compact_workflow_text_preserves_short_text_and_truncates_long_text() {
    let (short, short_truncated) = super::compact_workflow_text("hello");
    assert_eq!(short, "hello");
    assert!(!short_truncated);

    let (long, long_truncated) =
        super::compact_workflow_text(&"字".repeat(super::WORKFLOW_NODE_PREVIEW_MAX_CHARS + 24));
    assert!(long_truncated);
    assert!(long.contains("已截断 24 字符"));

    let mut in_place = "y".repeat(super::WORKFLOW_NODE_PREVIEW_MAX_CHARS + 1);
    assert!(super::compact_workflow_text_in_place(&mut in_place));
    assert!(in_place.contains("已截断 1 字符"));
}

#[test]
fn workflow_output_preview_text_uses_preferred_keys_or_json_fallback() {
    assert_eq!(
        super::workflow_output_preview_text(&serde_json::json!({"answer": "  final answer  "})),
        "final answer"
    );
    assert_eq!(
        super::workflow_output_preview_text(&serde_json::json!({"text": " text answer "})),
        "text answer"
    );
    assert!(
        super::workflow_output_preview_text(&serde_json::json!({"items": [1, 2]}))
            .contains("\"items\"")
    );
}

#[test]
fn workflow_node_preview_outputs_skips_null_and_uses_completion_fallback() {
    let null_event = super::WorkflowNodeStreamEvent {
        node_id: "n".to_string(),
        node_type: "llm".to_string(),
        title: "Null".to_string(),
        index: 1,
        status: None,
        elapsed_time: None,
        error: None,
        outputs: Some(serde_json::Value::Null),
    };
    assert!(super::workflow_node_preview_outputs(&null_event).is_none());

    let block = super::workflow_node_tool_block(&null_event, false, "");
    assert!(block.contains("节点执行完成"));
}

#[test]
fn workflow_node_usage_inline_yaml_and_preview_fallbacks_cover_absent_outputs() {
    assert_eq!(
        super::workflow_node_usage(Some(&serde_json::json!({
            "usage": {"total_tokens": 7}
        })))
        .and_then(|usage| usage.get("total_tokens").cloned())
        .and_then(|value| value.as_i64()),
        Some(7)
    );
    assert!(super::workflow_node_usage(Some(&serde_json::json!({"answer": "ok"}))).is_none());
    assert!(super::workflow_node_usage(None).is_none());

    let short_yaml = "kind: app\n";
    assert_eq!(super::workflow_node_inline_yaml(short_yaml), Some(short_yaml));
    assert!(
        super::workflow_node_inline_yaml(
            &"x".repeat(super::WORKFLOW_NODE_INLINE_YAML_MAX_CHARS + 1)
        )
        .is_none()
    );

    assert_eq!(
        super::workflow_output_preview_text(&serde_json::json!({"result": " done "})),
        "done"
    );
    assert_eq!(
        super::workflow_output_preview_text(&serde_json::json!({"output": " output "})),
        "output"
    );
    assert_eq!(super::workflow_output_preview_text(&serde_json::json!({"data": " data "})), "data");

    let running_event = super::WorkflowNodeStreamEvent {
        node_id: "n".to_string(),
        node_type: String::new(),
        title: String::new(),
        index: 0,
        status: None,
        elapsed_time: None,
        error: None,
        outputs: None,
    };
    let block = super::workflow_node_tool_block(&running_event, true, short_yaml);
    assert!(block.contains("正在执行工作流节点"));
    assert!(block.contains("\"input\":\"\""));
}

#[test]
fn workflow_preview_outputs_marks_truncation_and_preserves_usage() {
    let outputs = super::workflow_preview_outputs(
        &"x".repeat(super::WORKFLOW_NODE_PREVIEW_MAX_CHARS + 8),
        Some(serde_json::json!({"total_tokens": 9})),
        false,
    );

    assert_eq!(outputs.get("truncated").and_then(serde_json::Value::as_bool), Some(true));
    assert_eq!(
        outputs
            .get("usage")
            .and_then(|usage| usage.get("total_tokens"))
            .and_then(serde_json::Value::as_i64),
        Some(9)
    );
}

#[test]
fn workflow_node_tool_block_marks_errors_and_omits_large_yaml() {
    let event = super::WorkflowNodeStreamEvent {
        node_id: "node-b".to_string(),
        node_type: String::new(),
        title: "Failed Node".to_string(),
        index: 3,
        status: Some("failed".to_string()),
        elapsed_time: None,
        error: Some("boom".to_string()),
        outputs: None,
    };

    let block = super::workflow_node_tool_block(
        &event,
        false,
        &"x".repeat(super::WORKFLOW_NODE_INLINE_YAML_MAX_CHARS + 1),
    );

    assert!(block.contains("\"status\":\"error\""));
    assert!(block.contains("\"input\":\"Failed Node\""));
    assert!(block.contains("\"workflow_yaml\":null"));
}

#[test]
fn take_next_workflow_stream_payload_handles_lf_crlf_and_multiline_data() {
    let mut buffer = "event: message\ndata: {\"a\":1}\ndata: {\"b\":2}\n\nrest".to_string();
    assert_eq!(
        super::take_next_workflow_stream_payload(&mut buffer),
        Some("{\"a\":1}\n{\"b\":2}".to_string())
    );
    assert_eq!(buffer, "rest");

    let mut crlf = "data: done\r\n\r\n".to_string();
    assert_eq!(super::take_next_workflow_stream_payload(&mut crlf), Some("done".to_string()));
    assert!(crlf.is_empty());

    let mut no_payload = ": keepalive\n\n".to_string();
    assert!(super::take_next_workflow_stream_payload(&mut no_payload).is_none());
    assert!(no_payload.is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn apply_workflow_auth_sends_skey_as_bearer() {
    let client = reqwest::Client::new();
    let endpoint = vw_gateway_client::GatewayEndpoint::new("127.0.0.1", 1)
        .with_auth(vw_gateway_client::GatewayAuth { skey: Some(" key ".to_string()) });

    let request = super::apply_workflow_auth(client.post("http://127.0.0.1/"), &endpoint)
        .build()
        .expect("request should build");

    assert_eq!(
        request.headers().get(reqwest::header::AUTHORIZATION).and_then(|v| v.to_str().ok()),
        Some("Bearer  key ")
    );
    assert!(!request.headers().contains_key("x-skey"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn apply_workflow_auth_leaves_request_untouched_without_auth() {
    let client = reqwest::Client::new();
    let endpoint = vw_gateway_client::GatewayEndpoint::new("127.0.0.1", 1);

    let request = super::apply_workflow_auth(client.post("http://127.0.0.1/"), &endpoint)
        .build()
        .expect("request should build");

    assert!(!request.headers().contains_key(reqwest::header::AUTHORIZATION));
    assert!(!request.headers().contains_key("x-skey"));
}

#[test]
fn parse_workflow_chat_messages_event_covers_error_other_and_delta_variants() {
    assert!(matches!(
        super::parse_workflow_chat_messages_event(serde_json::json!({
            "event": "workflow_finished",
            "data": {"status": "failed", "error": "bad workflow"}
        })),
        super::WorkflowChatMessagesEvent::Error(error) if error == "bad workflow"
    ));
    assert!(matches!(
        super::parse_workflow_chat_messages_event(serde_json::json!({
            "event": "workflow_finished",
            "data": {"status": "succeeded"}
        })),
        super::WorkflowChatMessagesEvent::Other
    ));
    assert!(matches!(
        super::parse_workflow_chat_messages_event(serde_json::json!({
            "event": "error",
            "message": "bad request"
        })),
        super::WorkflowChatMessagesEvent::Error(error) if error == "bad request"
    ));
    assert!(matches!(
        super::parse_workflow_chat_messages_event(serde_json::json!({
            "event": "node_delta",
            "data": {"node_id": "n", "answer": "chunk", "replace": true}
        })),
        super::WorkflowChatMessagesEvent::NodeDelta(event)
            if event.node_id == "n" && event.text == "chunk" && event.replace
    ));
}

#[test]
fn parse_usage_supports_gateway_and_desktop_shapes() {
    let gateway = super::parse_usage(Some(&serde_json::json!({
        "prompt_tokens": 11,
        "completion_tokens": 12,
        "total_tokens": 23
    })));
    assert_eq!(gateway.input_tokens, 11);
    assert_eq!(gateway.output_tokens, 12);
    assert_eq!(gateway.cached_tokens, 0);

    let desktop = super::parse_usage(Some(&serde_json::json!({
        "input_tokens": 3,
        "output_tokens": 4,
        "cached_tokens": 5,
        "reasoning_tokens": 6
    })));
    assert_eq!(desktop.input_tokens, 3);
    assert_eq!(desktop.output_tokens, 4);
    assert_eq!(desktop.cached_tokens, 5);
    assert_eq!(desktop.reasoning_tokens, 6);

    assert_eq!(super::parse_usage(None), crate::app::models::TokenUsage::default());
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_workflow_stream_response(
    status: &'static str,
    body: String,
) -> (vw_gateway_client::GatewayEndpoint, std::thread::JoinHandle<String>) {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let addr = listener.local_addr().expect("test server address");
    let handle = std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return String::new();
        };
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1024];
        while let Ok(read) = stream.read(&mut chunk) {
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
        String::from_utf8_lossy(&request).to_string()
    });

    (vw_gateway_client::GatewayEndpoint::new("127.0.0.1", addr.port()), handle)
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn stream_workflow_chat_messages_sends_auth_query_and_parses_events() {
    let stream_body = concat!(
        "data: {\"event\":\"message\",\"answer\":\"hello\"}\n\n",
        "data: {\"event\":\"node_delta\",\"data\":{\"node_id\":\"n1\",\"node_type\":\"llm\",\"title\":\"Run\",\"index\":1,\"text\":\"partial\"}}\n\n",
        "data: {\"event\":\"message_end\",\"message_id\":\"done\",\"metadata\":{\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2}}}\n\n"
    )
    .to_string();
    let (endpoint, handle) = spawn_workflow_stream_response("200 OK", stream_body);
    let endpoint =
        endpoint.with_auth(vw_gateway_client::GatewayAuth { skey: Some("skey-value".to_string()) });
    let mut events = Vec::new();

    let result = super::stream_workflow_chat_messages(
        &endpoint,
        Some("/tmp/project"),
        &serde_json::json!({"query": "hello"}),
        |event| {
            events.push(event);
            true
        },
    )
    .await;
    let request = handle.join().expect("server thread should finish");

    assert!(result.is_ok());
    assert!(
        request.starts_with(
            "POST /v1/workflow/applications/chat-messages?directory=%2Ftmp%2Fproject "
        )
    );
    assert!(request.contains("authorization: Bearer skey-value"));
    assert!(!request.contains("x-skey:"));
    assert!(
        matches!(&events[0], super::WorkflowChatMessagesEvent::Delta(delta) if delta == "hello")
    );
    assert!(
        matches!(&events[1], super::WorkflowChatMessagesEvent::NodeDelta(event) if event.node_id == "n1")
    );
    assert!(matches!(
        &events[2],
        super::WorkflowChatMessagesEvent::Done {
            message_id: Some(message_id),
            ..
        } if message_id == "done"
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn stream_workflow_chat_messages_returns_status_errors() {
    let (endpoint, handle) =
        spawn_workflow_stream_response("500 Internal Server Error", "broken".to_string());

    let result = super::stream_workflow_chat_messages(
        &endpoint,
        None,
        &serde_json::json!({"query": "hello"}),
        |_| true,
    )
    .await;
    let _ = handle.join().expect("server thread should finish");

    let error = result.expect_err("status error should be returned");
    assert!(error.contains("workflow chat messages failed: 500 Internal Server Error broken"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn agent_stream_wrapper_returns_stream_without_polling() {
    let req = super::AgentRequest {
        id: 42,
        session: "session".to_string(),
        query: "hello".to_string(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: Vec::new(),
    };

    let stream = super::agent_stream(&req);
    drop(stream);
}

#[test]
fn temporary_workflow_helpers_build_stable_metadata_and_prompts() {
    assert_eq!(super::TemporaryWorkflowTopology::Serial.key(), "dynamic");
    assert_eq!(super::TemporaryWorkflowTopology::Serial.description(), "demand-driven workflow");
    assert_eq!(super::TemporaryWorkflowTopology::Parallel.key(), "dynamic-parallel");
    assert_eq!(
        super::TemporaryWorkflowTopology::Parallel.description(),
        "demand-driven parallel workflow"
    );

    let first = super::first_workflow_user_prompt("分析需求", false);
    assert!(first.contains("{{#start.conversation#}}"));
    assert!(first.contains("{{#start.query#}}"));
    assert!(first.contains("Return only the useful result for downstream task nodes."));

    let next = super::next_workflow_user_prompt("task_1", "分析需求", "实现功能", true);
    assert!(next.contains("Previous task title:\n分析需求"));
    assert!(next.contains("{{#task_1.answer#}}"));
    assert!(next.contains("Return the final user-facing answer."));

    let final_prompt = super::temporary_workflow_task_system_prompt("交付结果", true);
    assert!(final_prompt.contains("final task node"));
    let intermediate_prompt = super::temporary_workflow_task_system_prompt("分析需求", false);
    assert!(intermediate_prompt.contains("next task node"));
}

#[test]
fn temporary_workflow_title_normalization_handles_prefixes_and_unicode_limit() {
    assert_eq!(
        super::normalize_temporary_workflow_task_title("* 请你：  分析  数据！"),
        Some("分析 数据".to_string())
    );
    assert_eq!(
        super::normalize_temporary_workflow_task_title("八、最后归档结果。"),
        Some("归档结果".to_string())
    );
    assert_eq!(super::normalize_temporary_workflow_task_title("。。。；"), None);

    let long_title = "界".repeat(super::TEMPORARY_WORKFLOW_TITLE_MAX_CHARS + 1);
    assert_eq!(
        super::compact_temporary_workflow_task_title(&long_title),
        format!("{}...", "界".repeat(super::TEMPORARY_WORKFLOW_TITLE_MAX_CHARS))
    );
}

#[test]
fn temporary_workflow_segment_helpers_strip_nested_prefixes_and_connectors() {
    assert!(super::contains_any("please run in parallel", &["parallel", "serial"]));
    assert!(!super::contains_any("plain request", &["parallel", "serial"]));
    assert_eq!(super::strip_temporary_workflow_order_prefix("12：整理数据"), "整理数据");
    assert_eq!(
        super::strip_temporary_workflow_request_prefix("请帮我先然后再最后生成报告"),
        "生成报告"
    );
    assert_eq!(
        super::split_temporary_workflow_segment("1. 读取数据，然后清洗数据，并且生成报表"),
        vec!["读取数据", "清洗数据", "生成报表"]
    );
    assert_eq!(
        super::split_temporary_workflow_segment("prepare data, then train model, finally report"),
        vec!["prepare data", "train model", "report"]
    );
}

#[test]
fn temporary_workflow_parallel_plan_fans_out_and_final_prompt_references_each_task() {
    let plan = super::temporary_workflow_plan("同时生成标题以及生成摘要以及生成标签");

    assert_eq!(plan.topology, super::TemporaryWorkflowTopology::Parallel);
    assert_eq!(plan.final_node_id, "task_4");
    assert_eq!(plan.steps.len(), 4);
    assert_eq!(plan.steps[0].lane, -1);
    assert_eq!(plan.steps[1].lane, 0);
    assert_eq!(plan.steps[2].lane, 1);
    assert_eq!(plan.steps[3].column, 2);
    assert!(plan.edges.iter().any(|edge| edge.source == "start" && edge.target == "task_1"));
    assert!(plan.edges.iter().any(|edge| edge.source == "task_3" && edge.target == "task_4"));
    assert!(plan.steps[3].system_prompt.contains("final task node"));
    assert!(plan.steps[3].user_prompt.contains("Task output:\n{{#task_1.answer#}}"));
    assert!(plan.steps[3].user_prompt.contains("Task output:\n{{#task_2.answer#}}"));
    assert!(plan.steps[3].user_prompt.contains("Task output:\n{{#task_3.answer#}}"));
}

#[test]
fn temporary_workflow_graph_helpers_encode_layout_types_and_model() {
    let step = super::workflow_step(
        "task_1".to_string(),
        "分析".to_string(),
        "执行分析".to_string(),
        2,
        -1,
        false,
    );
    let llm_data = super::temporary_workflow_llm_data(&step, Some(" local-model "));
    assert_eq!(llm_data["model"]["name"], "local-model");
    assert_eq!(llm_data["model"]["provider"], "vibewindow");
    assert!(super::temporary_workflow_llm_data(&step, Some("  ")).get("model").is_none());

    let node = super::temporary_workflow_node("task_1", llm_data, 2, -1);
    assert_eq!(node["height"], 126);
    assert_eq!(node["position"]["x"], 480);
    assert_eq!(node["position"]["y"], -30);
    assert_eq!(node["position"], node["positionAbsolute"]);

    let answer = super::temporary_workflow_node(
        "answer",
        super::temporary_workflow_answer_data("task_1"),
        3,
        0,
    );
    assert_eq!(answer["height"], 98);
    assert_eq!(answer["data"]["answer"], "{{#task_1.answer#}}");

    let edge = super::temporary_workflow_edge("task_1", "llm", "answer", "answer");
    assert_eq!(edge["id"], "task_1-source-answer-target");
    assert_eq!(edge["data"]["sourceType"], "llm");
    assert_eq!(edge["data"]["targetType"], "answer");
}

#[test]
fn temporary_workflow_node_assigns_start_template_and_default_heights() {
    let cases = [("start", 116), ("template", 118), ("template-transform", 118), ("tool", 100)];

    for (node_type, expected_height) in cases {
        let node = super::temporary_workflow_node(
            node_type,
            serde_json::json!({ "type": node_type }),
            0,
            0,
        );
        assert_eq!(node["height"], expected_height);
    }
}

#[test]
fn temporary_workflow_fingerprint_ignores_outer_whitespace_only() {
    let base = super::temporary_workflow_query_fingerprint("分析需求");

    assert_eq!(base, super::temporary_workflow_query_fingerprint("  分析需求\n"));
    assert_ne!(base, super::temporary_workflow_query_fingerprint("分析需求并实现"));
    assert_eq!(base.len(), 8);
    assert!(base.chars().all(|ch| ch.is_ascii_hexdigit()));
}

fn plan_run_events() -> Vec<vw_gateway_client::vw_api_types::task::TaskPlanRunStreamEventDto> {
    use vw_gateway_client::vw_api_types::task::TaskPlanRunStreamEventDto;

    vec![
        TaskPlanRunStreamEventDto::PlanningStarted {
            task_id: "task-1".to_string(),
            plan_dir: "/plans/task-1".to_string(),
            session_id: "task-1-plan".to_string(),
        },
        TaskPlanRunStreamEventDto::SessionDelta {
            task_id: "task-1".to_string(),
            session_id: "task-1-plan".to_string(),
            delta: "正在分析".to_string(),
        },
        TaskPlanRunStreamEventDto::SessionWaiting {
            task_id: "task-1".to_string(),
            session_id: "task-1-plan".to_string(),
            active_descendant_session_ids: vec!["task-1-plan-child".to_string()],
        },
        TaskPlanRunStreamEventDto::PlanCreated {
            task_id: "task-1".to_string(),
            plan_dir: "/plans/task-1".to_string(),
            plan_md_path: "/plans/task-1/plan.md".to_string(),
            prompt_md_path: "/plans/task-1/prompt.md".to_string(),
            tasks_json_path: "/plans/task-1/tasks.json".to_string(),
            subtask_count: 2,
            session_id: "task-1-plan".to_string(),
        },
        TaskPlanRunStreamEventDto::BatchStarted {
            task_id: "task-1".to_string(),
            batch: 3,
            subtask_ids: vec!["sub-1".to_string(), "sub-2".to_string()],
        },
        TaskPlanRunStreamEventDto::SubtaskStarted {
            task_id: "task-1".to_string(),
            batch: 3,
            subtask_id: "sub-1".to_string(),
            title: "实现入口".to_string(),
            task_file: "1-entry.md".to_string(),
            session_id: "task-1-sub-1".to_string(),
        },
        TaskPlanRunStreamEventDto::SubtaskFinished {
            task_id: "task-1".to_string(),
            batch: 3,
            subtask_id: "sub-1".to_string(),
            title: "实现入口".to_string(),
            task_file: "1-entry.md".to_string(),
            success: true,
            output: Some("完成".to_string()),
            error: None,
            session_id: "task-1-sub-1".to_string(),
        },
        TaskPlanRunStreamEventDto::SubtaskFinished {
            task_id: "task-1".to_string(),
            batch: 3,
            subtask_id: "sub-2".to_string(),
            title: "补测试".to_string(),
            task_file: "2-tests.md".to_string(),
            success: false,
            output: None,
            error: None,
            session_id: "task-1-sub-2".to_string(),
        },
        TaskPlanRunStreamEventDto::BatchFinished {
            task_id: "task-1".to_string(),
            batch: 3,
            subtask_ids: vec!["sub-1".to_string(), "sub-2".to_string()],
            success: false,
        },
        TaskPlanRunStreamEventDto::Blocked {
            task_id: "task-1".to_string(),
            batch: Some(3),
            subtask_id: None,
            reason: "等待输入".to_string(),
            session_id: "task-1-plan".to_string(),
        },
        TaskPlanRunStreamEventDto::Finished {
            task_id: "task-1".to_string(),
            plan_dir: "/plans/task-1".to_string(),
            status: "completed".to_string(),
            output: "全部完成".to_string(),
            session_id: Some("task-1-plan".to_string()),
        },
        TaskPlanRunStreamEventDto::Error { task_id: None, message: "连接失败".to_string() },
    ]
}

#[test]
fn plan_run_tool_block_exposes_plan_or_task_path() {
    let events = plan_run_events();
    let task_paths = events
        .iter()
        .map(super::plan_run_tool_block)
        .map(|block| {
            let payload = block
                .strip_prefix("tool workflow_node\n")
                .and_then(|value| value.strip_suffix('\n'))
                .expect("tool block payload");
            serde_json::from_str::<serde_json::Value>(payload).expect("tool block json")
        })
        .map(|payload| {
            payload["metadata"]["task_path"].as_str().map(str::to_string)
        })
        .collect::<Vec<_>>();

    assert_eq!(task_paths[0].as_deref(), Some("/plans/task-1"));
    assert_eq!(task_paths[1], None, "session delta has no new path metadata");
    assert_eq!(task_paths[3].as_deref(), Some("/plans/task-1"));
    assert_eq!(task_paths[5].as_deref(), Some("1-entry.md"));
    assert_eq!(task_paths[6].as_deref(), Some("1-entry.md"));
}

#[test]
fn goal_mode_stream_continues_after_negative_evaluation() {
    use vw_gateway_client::vw_api_types::task::TaskPlanRunStreamEventDto;

    let rejected = TaskPlanRunStreamEventDto::GoalEvaluated {
        task_id: "task-goal".to_string(),
        turn: 1,
        achieved: false,
        reason: "验证未通过".to_string(),
        session_id: "task-goal-goal-eval-1".to_string(),
    };
    let finished = TaskPlanRunStreamEventDto::Finished {
        task_id: "task-goal".to_string(),
        plan_dir: "/plans/task-goal".to_string(),
        status: "completed".to_string(),
        output: "验证通过".to_string(),
        session_id: Some("task-goal-goal-2".to_string()),
    };

    assert!(!super::plan_run_event_is_terminal(&rejected));
    assert!(super::plan_run_event_is_terminal(&finished));
    assert_eq!(super::plan_run_event_status(&rejected), "running");
}

#[test]
fn desktop_plan_run_waiting_and_cancelled_events_keep_session_lifecycle_consistent() {
    use vw_gateway_client::vw_api_types::task::TaskPlanRunStreamEventDto;

    let waiting = TaskPlanRunStreamEventDto::SessionWaiting {
        task_id: "task-2".to_string(),
        session_id: "task-2-sub-9".to_string(),
        active_descendant_session_ids: vec!["task-2-sub-9-child".to_string()],
    };
    let cancelled = TaskPlanRunStreamEventDto::Cancelled {
        task_id: "task-2".to_string(),
        session_id: Some("task-2-sub-9".to_string()),
    };

    assert_eq!(super::plan_run_event_status(&waiting), "waiting");
    assert!(!super::plan_run_event_is_terminal(&waiting));
    assert_eq!(super::plan_run_session_finished_id(&waiting), None);

    assert_eq!(super::plan_run_event_status(&cancelled), "cancelled");
    assert!(super::plan_run_event_is_terminal(&cancelled));
    assert_eq!(super::plan_run_session_finished_id(&cancelled), None);
    assert_eq!(super::plan_run_session_cancelled_id(&cancelled), Some("task-2-sub-9"));

    let waiting_block = super::plan_run_tool_block(&waiting);
    let cancelled_block = super::plan_run_tool_block(&cancelled);
    assert!(waiting_block.contains("\"status\":\"waiting\""));
    assert!(cancelled_block.contains("\"status\":\"cancelled\""));
    assert!(!cancelled_block.contains("\"status\":\"completed\""));
}

#[test]
fn plan_run_event_helpers_map_every_variant_and_failure_branch() {
    let events = plan_run_events();
    assert_eq!(
        events.iter().map(super::plan_run_event_type).collect::<Vec<_>>(),
        vec![
            "planning_started",
            "session_delta",
            "session_waiting",
            "plan_created",
            "batch_started",
            "subtask_started",
            "subtask_finished",
            "subtask_finished",
            "batch_finished",
            "blocked",
            "finished",
            "error",
        ]
    );
    assert_eq!(
        events.iter().map(super::plan_run_event_status).collect::<Vec<_>>(),
        vec![
            "running",
            "running",
            "waiting",
            "completed",
            "running",
            "running",
            "completed",
            "error",
            "error",
            "error",
            "completed",
            "error",
        ]
    );
    assert_eq!(super::plan_run_event_task_id(&events[11]), "unknown");
    assert_eq!(super::plan_run_tool_call_id(&events[0]), "plan-run-task-1-planning");
    assert_eq!(super::plan_run_tool_call_id(&events[1]), "plan-run-task-1-session-task-1-plan");
    assert_eq!(super::plan_run_tool_call_id(&events[2]), "plan-run-task-1-session-task-1-plan");
    assert_eq!(super::plan_run_tool_call_id(&events[4]), "plan-run-task-1-batch-3");
    assert_eq!(super::plan_run_tool_call_id(&events[8]), "plan-run-task-1-batch-3");
    assert_eq!(super::plan_run_tool_call_id(&events[5]), "plan-run-task-1-subtask-sub-1");
    assert_eq!(super::plan_run_tool_call_id(&events[9]), "plan-run-task-1-blocked-task");
    assert_eq!(super::plan_run_tool_call_id(&events[11]), "plan-run-unknown-error");

    assert_eq!(super::plan_run_event_summary(&events[1]), "规划会话");
    assert_eq!(super::plan_run_event_output_text(&events[1]), "正在分析");
    assert_eq!(super::plan_run_event_summary(&events[2]), "等待后代智能体");
    assert!(super::plan_run_event_output_text(&events[2]).contains("task-1-plan-child"));
    assert_eq!(super::plan_run_event_summary(&events[3]), "计划已创建 · 2 个子任务");
    assert_eq!(super::plan_run_event_summary(&events[7]), "批次 3 · 补测试 · 失败");
    assert!(super::plan_run_event_output_text(&events[4]).contains("sub-1, sub-2"));
    assert!(super::plan_run_event_output_text(&events[7]).ends_with("执行失败"));
    assert!(super::plan_run_event_output_text(&events[8]).contains("批次 3 执行失败"));
    assert_eq!(super::plan_run_event_output_text(&events[9]), "执行受阻: 等待输入");
}

#[test]
fn completed_batch_reuses_started_node_and_marks_it_completed() {
    use vw_gateway_client::vw_api_types::task::TaskPlanRunStreamEventDto;

    let started = TaskPlanRunStreamEventDto::BatchStarted {
        task_id: "task-2".to_string(),
        batch: 4,
        subtask_ids: vec!["sub-9".to_string()],
    };
    let finished = TaskPlanRunStreamEventDto::BatchFinished {
        task_id: "task-2".to_string(),
        batch: 4,
        subtask_ids: vec!["sub-9".to_string()],
        success: true,
    };

    assert_eq!(super::plan_run_tool_call_id(&started), super::plan_run_tool_call_id(&finished));
    assert_eq!(super::plan_run_event_status(&finished), "completed");
    assert_eq!(super::plan_run_event_summary(&finished), "批次 4");
    assert!(super::plan_run_event_output_text(&finished).contains("批次 4 执行完毕"));
}

#[test]
fn plan_run_tool_block_serializes_error_and_metadata() {
    let events = plan_run_events();
    let block = super::plan_run_tool_block(&events[6]);
    let payload_text = block
        .strip_prefix("tool workflow_node\n")
        .and_then(|value| value.strip_suffix('\n'))
        .expect("tool block payload");
    let payload: serde_json::Value = serde_json::from_str(payload_text).expect("tool block json");

    assert_eq!(payload["status"], "error");
    assert_eq!(payload["metadata"]["canonical_tool_id"], "workflow_node");
    assert_eq!(payload["metadata"]["plan_run"], true);
    assert_eq!(payload["metadata"]["event_type"], "subtask_finished");
    assert_eq!(payload["metadata"]["task_id"], "task-1");
    assert_eq!(payload["metadata"]["session_id"], "task-1-sub-2");
    assert_eq!(payload["metadata"]["title"], "补测试");
    assert!(payload["error"].as_str().is_some_and(|error| error.contains("执行失败")));

    let output: serde_json::Value =
        serde_json::from_str(payload["output"].as_str().expect("serialized preview output"))
            .expect("preview output json");
    assert!(output["answer"].as_str().is_some_and(|answer| answer.contains("2-tests.md")));
}

#[test]
fn plan_run_tool_block_uses_explicit_title_fallback_for_blank_subtask_title() {
    use vw_gateway_client::vw_api_types::task::TaskPlanRunStreamEventDto;

    let event = TaskPlanRunStreamEventDto::SubtaskStarted {
        task_id: "task-2".to_string(),
        batch: 4,
        subtask_id: "sub-9".to_string(),
        title: "   ".to_string(),
        task_file: "9-verify.md".to_string(),
        session_id: "task-2-sub-9".to_string(),
    };
    let block = super::plan_run_tool_block(&event);
    let payload_text = block
        .strip_prefix("tool workflow_node\n")
        .and_then(|value| value.strip_suffix('\n'))
        .expect("tool block payload");
    let payload: serde_json::Value = serde_json::from_str(payload_text).expect("tool block json");

    assert_eq!(payload["metadata"]["title"], "子任务 · sub-9");
}

#[test]
fn plan_run_started_messages_register_planning_and_execution_sessions() {
    use vw_gateway_client::vw_api_types::task::TaskPlanRunStreamEventDto;

    let planning = TaskPlanRunStreamEventDto::PlanningStarted {
        task_id: "task-2".to_string(),
        plan_dir: "/tmp/plan".to_string(),
        session_id: "task-2-plan".to_string(),
    };
    let subtask = TaskPlanRunStreamEventDto::SubtaskStarted {
        task_id: "task-2".to_string(),
        batch: 4,
        subtask_id: "sub-9".to_string(),
        title: "   ".to_string(),
        task_file: "9-verify.md".to_string(),
        session_id: "task-2-sub-9".to_string(),
    };

    let planning_message =
        super::plan_run_session_started_message(&planning, 42, "main", "/tmp/project", "需求")
            .expect("planning registration message");
    let subtask_message =
        super::plan_run_session_started_message(&subtask, 42, "main", "/tmp/project", "需求")
            .expect("subtask registration message");

    match planning_message {
        super::Message::Chat(super::message::ChatMessage::PlanRunSessionStarted {
            parent_request_id,
            parent_session_id,
            session_id,
            title,
            directory,
            query,
        }) => {
            assert_eq!(parent_request_id, 42);
            assert_eq!(parent_session_id, "main");
            assert_eq!(session_id, "task-2-plan");
            assert_eq!(title, "规划 · task-2");
            assert_eq!(directory, "/tmp/project");
            assert_eq!(query, "需求");
        }
        other => panic!("expected planning registration, got {other:?}"),
    }

    match subtask_message {
        super::Message::Chat(super::message::ChatMessage::PlanRunSessionStarted {
            session_id,
            title,
            query,
            ..
        }) => {
            assert_eq!(session_id, "task-2-sub-9");
            assert_eq!(title, "子任务 · sub-9");
            assert_eq!(query, "子任务 · sub-9");
        }
        other => panic!("expected subtask registration, got {other:?}"),
    }
}

#[test]
fn plan_run_blocked_subtask_and_success_without_output_use_stable_payloads() {
    use vw_gateway_client::vw_api_types::task::TaskPlanRunStreamEventDto;

    let blocked = TaskPlanRunStreamEventDto::Blocked {
        task_id: "task-2".to_string(),
        batch: Some(4),
        subtask_id: Some("sub-9".to_string()),
        reason: "需要审批".to_string(),
        session_id: "task-2-sub-9".to_string(),
    };
    assert_eq!(super::plan_run_tool_call_id(&blocked), "plan-run-task-2-blocked-sub-9");
    assert_eq!(super::plan_run_event_summary(&blocked), "执行受阻");
    assert_eq!(super::plan_run_event_output_text(&blocked), "执行受阻: 需要审批");

    let success_without_output = TaskPlanRunStreamEventDto::SubtaskFinished {
        task_id: "task-2".to_string(),
        batch: 4,
        subtask_id: "sub-10".to_string(),
        title: "验证".to_string(),
        task_file: "10-verify.md".to_string(),
        success: true,
        output: None,
        error: Some("ignored".to_string()),
        session_id: "task-2-sub-10".to_string(),
    };
    let output = super::plan_run_event_output_text(&success_without_output);
    assert!(output.starts_with("子任务完成，任务文件已处理: 10-verify.md\n"));
    assert!(!output.contains("ignored"));

    let block = super::plan_run_tool_block(&blocked);
    let payload_text = block
        .strip_prefix("tool workflow_node\n")
        .and_then(|value| value.strip_suffix('\n'))
        .expect("tool block payload");
    let payload: serde_json::Value = serde_json::from_str(payload_text).expect("tool block json");
    assert_eq!(payload["status"], "error");
    assert_eq!(payload["metadata"]["event_type"], "blocked");
    assert_eq!(payload["metadata"]["session_id"], "task-2-sub-9");
    assert!(payload["error"].as_str().is_some_and(|error| error.contains("需要审批")));
}

#[test]
fn workflow_event_defaults_and_incomplete_frames_are_safe() {
    assert!(matches!(
        super::parse_workflow_chat_messages_event(serde_json::json!({
            "event": "workflow_finished",
            "data": {"status": "failed"}
        })),
        super::WorkflowChatMessagesEvent::Error(error) if error == "workflow failed"
    ));
    assert!(matches!(
        super::parse_workflow_chat_messages_event(serde_json::json!({"event": "error"})),
        super::WorkflowChatMessagesEvent::Error(error) if error == "workflow stream failed"
    ));
    assert!(matches!(
        super::parse_workflow_chat_messages_event(serde_json::json!({
            "event": "text_chunk",
            "answer": "fallback"
        })),
        super::WorkflowChatMessagesEvent::Delta(text) if text == "fallback"
    ));

    let mut incomplete = "data: partial".to_string();
    assert!(super::take_next_workflow_stream_payload(&mut incomplete).is_none());
    assert_eq!(incomplete, "data: partial");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn apply_workflow_auth_ignores_blank_skey() {
    let client = reqwest::Client::new();
    let endpoint = vw_gateway_client::GatewayEndpoint::new("127.0.0.1", 1)
        .with_auth(vw_gateway_client::GatewayAuth { skey: Some("   ".to_string()) });
    let request = super::apply_workflow_auth(client.post("http://127.0.0.1/"), &endpoint)
        .build()
        .expect("request should build");
    assert!(!request.headers().contains_key(reqwest::header::AUTHORIZATION));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn workflow_mode_without_project_root_emits_error_without_network() {
    use iced::futures::StreamExt;

    let req = super::AgentRequest {
        id: 43,
        session: "workflow-session".to_string(),
        query: "规划并执行任务".to_string(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: true,
        goal_mode_enabled: false,
        history: Vec::new(),
    };

    let mut stream = super::agent_stream(&req);
    let message = stream.next().await.expect("workflow root validation message");

    match message {
        super::Message::Chat(super::message::ChatMessage::AgentStreamError(id, error)) => {
            assert_eq!(id, 43);
            assert_eq!(error, "启用工作流执行需要先选择项目根目录");
        }
        _ => panic!("missing project root should emit AgentStreamError"),
    }
}

fn sample_agent_request(
    workflow_mode_enabled: bool,
    goal_mode_enabled: bool,
    root: Option<&str>,
) -> super::AgentRequest {
    super::AgentRequest {
        id: 77,
        session: "route-session".to_string(),
        query: "do the work".to_string(),
        root: root.map(str::to_string),
        model: Some("demo-model".to_string()),
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled,
        goal_mode_enabled,
        history: Vec::new(),
    }
}

#[test]
fn gateway_request_options_include_input_source() {
    let request = sample_agent_request(false, true, None);
    let rpc_id = super::desktop_stream_request_id(&request);
    let options = super::gateway_request_options(&request, None, &rpc_id, "America/New_York")
        .expect("request options");

    assert_eq!(
        options["_vw_input_source"],
        serde_json::json!({
            "kind": "user",
            "rpcId": rpc_id,
            "clientTimeZone": "America/New_York",
        })
    );
}

#[test]
fn goal_only_does_not_activate_task_plan_run() {
    assert!(!super::task_plan_run_active(false, false), "goal alone must not enter task_plan_run");
    let request = sample_agent_request(false, true, None);
    let plan_run = super::build_task_plan_run_request(&request)
        .expect("goal-only without root must not demand project root");
    assert!(plan_run.is_none(), "goal-only must stay on ordinary chat");
    assert!(super::lightweight_goal_active(false, false));
    assert!(!super::lightweight_goal_active(true, false));
}

#[test]
fn lightweight_goal_gate_is_structural_and_language_agnostic() {
    let mut request = sample_agent_request(false, true, None);
    for query in ["帮我分析一下", "Please take a look", "これを見て", "حلّل هذا"]
    {
        request.query = query.to_string();
        assert!(
            super::lightweight_goal_enabled_for_request(&request),
            "goal mode must be enabled independently of query language: {query}"
        );
    }

    request.goal_mode_enabled = false;
    assert!(!super::lightweight_goal_enabled_for_request(&request));
    request.goal_mode_enabled = true;
    request.workflow_mode_enabled = true;
    assert!(!super::lightweight_goal_enabled_for_request(&request));
    request.workflow_mode_enabled = false;
    request.resume_history_only = true;
    assert!(!super::lightweight_goal_enabled_for_request(&request));
}

#[test]
fn lightweight_goal_mode_finishes_without_secondary_evaluation() {
    let request = sample_agent_request(false, true, Some("/tmp/project"));
    let options =
        super::gateway_request_options(&request, None, "desktop-goal-request", "Asia/Shanghai")
            .expect("goal mode options");

    assert_eq!(options.get("goal_mode").and_then(|value| value.as_bool()), Some(true));
    assert_eq!(
        SOURCE.matches("GatewayClient::stream_chat").count(),
        1,
        "ordinary goal mode must send only the main chat request"
    );
    assert!(SOURCE.contains("AgentStreamDone"));
    assert!(
        !SOURCE.contains("\"goal_evaluation\": true")
            && !SOURCE.contains("独立会话决策器")
            && !SOURCE.contains("build_lightweight_goal_continuation_prompt"),
        "ordinary goal mode must not send an evaluator request or a continuation prompt"
    );
}

#[cfg(any())]
#[test]
fn lightweight_goal_continues_until_evaluation_passes() {
    let rejected = super::resolve_lightweight_goal_turn(
        "已改代码，但还没跑测试",
        r#"{"status":"continue","reason":"缺少验证退出码证据"}"#,
    );
    assert!(matches!(
        rejected,
        super::LightweightGoalNext::Continue { reason } if reason.contains("缺少验证退出码")
    ));

    let continuation =
        super::build_lightweight_goal_continuation_prompt("修复登录超时", "缺少验证退出码证据");
    assert!(continuation.contains("目标续轮"));
    assert!(continuation.contains("修复登录超时"));
    assert!(!continuation.contains("编号任务文件"));

    let passed = super::resolve_lightweight_goal_turn(
        "验证通过，exit 0",
        r#"{"status":"complete","reason":"验证退出码为 0"}"#,
    );
    assert!(matches!(
        passed,
        super::LightweightGoalNext::Complete { reason } if reason.contains("验证退出码")
    ));

    let fenced = super::resolve_lightweight_goal_turn(
        "验证通过，exit 0",
        "```json\n{\"status\":\"complete\",\"reason\":\"验证退出码为 0\"}\n```",
    );
    assert!(matches!(
        fenced,
        super::LightweightGoalNext::Complete { reason } if reason.contains("验证退出码")
    ));

    let needs_input = super::resolve_lightweight_goal_turn(
        "请上传需要分析的图片。",
        r#"{"status":"needs_input","reason":"缺少用户要分析的图片"}"#,
    );
    assert!(matches!(
        needs_input,
        super::LightweightGoalNext::NeedsInput { reason } if reason.contains("图片")
    ));

    let empty = super::resolve_lightweight_goal_turn("已执行一轮", "");
    assert!(matches!(
        empty,
        super::LightweightGoalNext::Continue { reason }
            if reason.contains("独立验收未返回结论")
                && !reason.contains("目标验收基础设施错误")
    ));

    let mut last_error = None;
    let mut repeated = 0_u8;
    let first = super::lightweight_goal_infrastructure_continue(
        "gateway stream failed: 500",
        &mut last_error,
        &mut repeated,
    );
    assert!(matches!(
        first,
        super::LightweightGoalNext::Continue { reason }
            if reason.contains("目标验收基础设施错误：")
    ));
    let second = super::lightweight_goal_infrastructure_continue(
        "gateway stream failed: 500",
        &mut last_error,
        &mut repeated,
    );
    assert!(matches!(second, super::LightweightGoalNext::Continue { .. }));
    let third = super::lightweight_goal_infrastructure_continue(
        "gateway stream failed: 500",
        &mut last_error,
        &mut repeated,
    );
    assert!(matches!(
        third,
        super::LightweightGoalNext::Blocked { reason }
            if reason.contains("连续三轮出现相同基础设施错误")
    ));

    let mut last_evaluation_error = None;
    let mut repeated_evaluation_errors = 0_u8;
    let skipped_evaluation = super::resolve_lightweight_goal_turn_with_infrastructure_guard(
        "已执行一轮",
        "模型返回了说明文字而不是 JSON",
        &mut last_evaluation_error,
        &mut repeated_evaluation_errors,
    );
    assert!(matches!(
        skipped_evaluation,
        super::LightweightGoalNext::Skipped { reason } if reason.contains("目标验收已跳过")
    ));
    assert_eq!(repeated_evaluation_errors, 0);

    for _ in 0..2 {
        let next = super::resolve_lightweight_goal_turn_with_infrastructure_guard(
            "已执行一轮",
            "```json\n{\"status\":\"unknown\",\"reason\":\"无效状态\"}\n```",
            &mut last_evaluation_error,
            &mut repeated_evaluation_errors,
        );
        assert!(matches!(
            next,
            super::LightweightGoalNext::Continue { reason }
                if reason.contains("目标评估器 status 无效")
        ));
    }
    let blocked_evaluation = super::resolve_lightweight_goal_turn_with_infrastructure_guard(
        "已执行一轮",
        "```json\n{\"status\":\"unknown\",\"reason\":\"无效状态\"}\n```",
        &mut last_evaluation_error,
        &mut repeated_evaluation_errors,
    );
    assert!(matches!(
        blocked_evaluation,
        super::LightweightGoalNext::Blocked { reason }
            if reason.contains("连续三轮出现相同基础设施错误")
    ));

    assert!(
        SOURCE.contains("desktop_stream_request_id_for_turn")
            && SOURCE.contains("lightweight_goal_infrastructure_continue")
            && SOURCE.contains("resolve_lightweight_goal_turn_with_infrastructure_guard")
            && SOURCE.contains("collect_lightweight_goal_evaluation")
            && SOURCE.contains("\"disable_tools\": true")
            && SOURCE.contains("\"max_output_tokens\": 256")
            && SOURCE.contains("goal_terminal_sent")
            && !SOURCE.contains("[目标续轮] {reason}")
            && !SOURCE.contains("目标验收失败：{error}"),
        "goal eval must guard invalid JSON, disable tools, and always emit a terminal event"
    );

    let request = sample_agent_request(false, true, Some("/tmp/project"));
    let options =
        super::gateway_request_options(&request, None, "desktop-goal-request", "Asia/Shanghai")
            .expect("goal-only options");
    assert_eq!(options.get("goal_mode").and_then(|value| value.as_bool()), Some(true));
    assert!(
        super::build_task_plan_run_request(&request).expect("goal-only").is_none(),
        "goal-only continuation must not create task_plan_run / plan directory"
    );
}

#[cfg(any())]
#[test]
fn lightweight_goal_stops_on_explicit_blocker() {
    let blocked = super::resolve_lightweight_goal_turn(
        "状态：已阻塞\n阻塞原因：缺少部署凭据，需要用户输入\n",
        r#"{"status":"continue","reason":"不应再评估"}"#,
    );
    assert!(matches!(
        blocked,
        super::LightweightGoalNext::Blocked { reason } if reason.contains("缺少部署凭据")
    ));

    assert!(matches!(
        super::lightweight_goal_next_action(
            super::LightweightGoalEvaluation::Continue,
            "还差一步",
            Some("需要用户提供 API key")
        ),
        super::LightweightGoalNext::Blocked { reason } if reason.contains("API key")
    ));
}

#[test]
fn workflow_enabled_builds_plan_run_request_without_mode() {
    assert!(super::task_plan_run_active(true, false));
    assert!(!super::task_plan_run_active(true, true));
    assert!(!super::task_plan_run_active(false, false));

    let with_goal = sample_agent_request(true, true, Some("/tmp/project"));
    let plan_run = super::build_task_plan_run_request(&with_goal)
        .expect("workflow+goal with root should build plan-run")
        .expect("workflow+goal must construct TaskPlanRunRequest");
    assert_eq!(plan_run.query, with_goal.query);
    assert_eq!(plan_run.root, "/tmp/project");

    let without_goal = sample_agent_request(true, false, Some("/tmp/project"));
    let plan_run = super::build_task_plan_run_request(&without_goal)
        .expect("workflow with root should build plan-run")
        .expect("workflow must construct TaskPlanRunRequest");
    assert_eq!(plan_run.query, without_goal.query);
    assert_eq!(plan_run.root, "/tmp/project");
}

#[test]
fn parse_usage_defaults_invalid_and_partial_values() {
    let partial = super::parse_usage(Some(&serde_json::json!({
        "prompt_tokens": "invalid",
        "total_tokens": 99
    })));
    assert_eq!(partial.input_tokens, 0);
    assert_eq!(partial.output_tokens, 0);
    assert_eq!(partial.cached_tokens, 0);
    assert_eq!(partial.reasoning_tokens, 0);

    let invalid_desktop = super::parse_usage(Some(&serde_json::json!({"input_tokens": "invalid"})));
    assert_eq!(invalid_desktop, crate::app::models::TokenUsage::default());
}
