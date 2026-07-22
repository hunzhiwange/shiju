// Tests for plan6 task 846.
const SOURCE: &str = include_str!("agent_stream.rs");

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
        SOURCE.contains("stream_task_plan_run"),
        "workflow mode should subscribe to task plan-run SSE"
    );
    assert!(
        !SOURCE.contains("workflow_mode_yaml(&req.query"),
        "workflow mode should no longer build temporary workflow YAML before streaming"
    );

    let stream_body = concat!(
        "data: {\"type\":\"planning_started\",\"task_id\":\"task-1\",\"plan_dir\":\"/tmp/plan\"}\n\n",
        "data: {\"type\":\"plan_created\",\"task_id\":\"task-1\",\"plan_dir\":\"/tmp/plan\",\"plan_md_path\":\"/tmp/plan/plan.md\",\"prompt_md_path\":\"/tmp/plan/prompt.md\",\"tasks_json_path\":\"/tmp/plan/tasks.json\",\"subtask_count\":1}\n\n",
        "data: {\"type\":\"batch_started\",\"task_id\":\"task-1\",\"batch\":1,\"subtask_ids\":[\"subtask-1\"]}\n\n",
        "data: {\"type\":\"subtask_started\",\"task_id\":\"task-1\",\"batch\":1,\"subtask_id\":\"subtask-1\",\"title\":\"实现入口\",\"task_file\":\"1-entry.md\"}\n\n",
        "data: {\"type\":\"subtask_finished\",\"task_id\":\"task-1\",\"batch\":1,\"subtask_id\":\"subtask-1\",\"title\":\"实现入口\",\"task_file\":\"1-entry.md\",\"success\":true,\"output\":\"ok\"}\n\n",
        "data: {\"type\":\"finished\",\"task_id\":\"task-1\",\"plan_dir\":\"/tmp/plan\",\"status\":\"completed\",\"output\":\"all done\"}\n\n",
    )
    .to_string();
    let (endpoint, handle) = spawn_workflow_stream_response("200 OK", stream_body);
    let request = vw_gateway_client::vw_api_types::task::TaskPlanRunRequest {
        query: "请规划并执行".to_string(),
        root: "/tmp/project".to_string(),
        model: Some("demo-model".to_string()),
        agent: Some("main".to_string()),
        acp_agent: Some("codex".to_string()),
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
            think_timing: Vec::new(),
        },
        crate::app::models::ChatMessage {
            role: crate::app::models::ChatRole::User,
            content: "   ".to_string(),
            think_timing: Vec::new(),
        },
        crate::app::models::ChatMessage {
            role: crate::app::models::ChatRole::Assistant,
            content: " answer ".to_string(),
            think_timing: Vec::new(),
        },
        crate::app::models::ChatMessage {
            role: crate::app::models::ChatRole::Tool,
            content: " tool output ".to_string(),
            think_timing: Vec::new(),
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
        think_timing: Vec::new(),
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
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: false,
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
        },
        TaskPlanRunStreamEventDto::PlanCreated {
            task_id: "task-1".to_string(),
            plan_dir: "/plans/task-1".to_string(),
            plan_md_path: "/plans/task-1/plan.md".to_string(),
            prompt_md_path: "/plans/task-1/prompt.md".to_string(),
            tasks_json_path: "/plans/task-1/tasks.json".to_string(),
            subtask_count: 2,
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
        },
        TaskPlanRunStreamEventDto::Blocked {
            task_id: "task-1".to_string(),
            batch: Some(3),
            subtask_id: None,
            reason: "等待输入".to_string(),
        },
        TaskPlanRunStreamEventDto::Finished {
            task_id: "task-1".to_string(),
            plan_dir: "/plans/task-1".to_string(),
            status: "completed".to_string(),
            output: "全部完成".to_string(),
        },
        TaskPlanRunStreamEventDto::Error { task_id: None, message: "连接失败".to_string() },
    ]
}

#[test]
fn plan_run_event_helpers_map_every_variant_and_failure_branch() {
    let events = plan_run_events();
    assert_eq!(
        events.iter().map(super::plan_run_event_type).collect::<Vec<_>>(),
        vec![
            "planning_started",
            "plan_created",
            "batch_started",
            "subtask_started",
            "subtask_finished",
            "subtask_finished",
            "blocked",
            "finished",
            "error",
        ]
    );
    assert_eq!(
        events.iter().map(super::plan_run_event_status).collect::<Vec<_>>(),
        vec![
            "running",
            "completed",
            "running",
            "running",
            "completed",
            "error",
            "error",
            "completed",
            "error",
        ]
    );
    assert_eq!(super::plan_run_event_task_id(&events[8]), "unknown");
    assert_eq!(super::plan_run_tool_call_id(&events[0]), "plan-run-task-1-planning");
    assert_eq!(super::plan_run_tool_call_id(&events[2]), "plan-run-task-1-batch-3");
    assert_eq!(super::plan_run_tool_call_id(&events[3]), "plan-run-task-1-subtask-sub-1");
    assert_eq!(super::plan_run_tool_call_id(&events[6]), "plan-run-task-1-blocked-task");
    assert_eq!(super::plan_run_tool_call_id(&events[8]), "plan-run-unknown-error");

    assert_eq!(super::plan_run_event_summary(&events[1]), "计划已创建 · 2 个子任务");
    assert_eq!(super::plan_run_event_summary(&events[5]), "批次 3 · 补测试 · 失败");
    assert!(super::plan_run_event_output_text(&events[2]).contains("sub-1, sub-2"));
    assert!(super::plan_run_event_output_text(&events[5]).ends_with("执行失败"));
    assert_eq!(super::plan_run_event_output_text(&events[6]), "执行受阻: 等待输入");
}

#[test]
fn plan_run_tool_block_serializes_error_and_metadata() {
    let events = plan_run_events();
    let block = super::plan_run_tool_block(&events[5]);
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
    assert!(payload["error"].as_str().is_some_and(|error| error.contains("执行失败")));

    let output: serde_json::Value =
        serde_json::from_str(payload["output"].as_str().expect("serialized preview output"))
            .expect("preview output json");
    assert!(output["answer"].as_str().is_some_and(|answer| answer.contains("2-tests.md")));
}

#[test]
fn plan_run_blocked_subtask_and_success_without_output_use_stable_payloads() {
    use vw_gateway_client::vw_api_types::task::TaskPlanRunStreamEventDto;

    let blocked = TaskPlanRunStreamEventDto::Blocked {
        task_id: "task-2".to_string(),
        batch: Some(4),
        subtask_id: Some("sub-9".to_string()),
        reason: "需要审批".to_string(),
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
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: true,
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
