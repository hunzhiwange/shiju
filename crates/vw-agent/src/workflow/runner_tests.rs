use super::*;
use crate::providers::traits::TokenUsage;
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use vw_api_types::workflow::WorkflowNodeRunStatus;

fn node(id: &str, node_type: &str, data: Value) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        node_type: node_type.to_string(),
        title: format!("{node_type} title"),
        data,
    }
}

fn graph(nodes: Vec<WorkflowNode>, edges: Vec<WorkflowEdge>, starts: Vec<&str>) -> WorkflowGraph {
    WorkflowGraph {
        nodes: nodes.into_iter().map(|node| (node.id.clone(), node)).collect(),
        edges,
        start_node_ids: starts.into_iter().map(str::to_string).collect(),
    }
}

#[test]
fn workflow_source_and_start_variables_handle_blank_and_fallback_inputs() {
    let missing = WorkflowRunRequest {
        workflow_yaml: Some(" \n ".to_string()),
        ..WorkflowRunRequest::default()
    };
    assert!(resolve_workflow_source(&missing).expect_err("blank workflow").contains("必须提供"));

    let request = WorkflowRunRequest {
        workflow_yaml: Some("workflow source".to_string()),
        inputs: BTreeMap::from([
            ("query".to_string(), Value::String("input query".to_string())),
            ("count".to_string(), Value::Number(3.into())),
        ]),
        ..WorkflowRunRequest::default()
    };
    assert_eq!(resolve_workflow_source(&request).expect("workflow source"), "workflow source");

    let graph = graph(
        vec![node(
            "start",
            "start",
            json!({
                "variables": [
                    {"variable": "query"},
                    {"variable": "count"},
                    {"variable": "missing"},
                    {"label": "ignored"}
                ]
            }),
        )],
        Vec::new(),
        vec!["start", "unknown"],
    );
    let pool = initialize_variable_pool(&graph, &request, "run-431");

    assert_eq!(
        pool.get_selector(&["sys".to_string(), "query".to_string()]),
        Some(&Value::String("input query".to_string()))
    );
    assert_eq!(
        pool.get_selector(&["sys".to_string(), "workflow_run_id".to_string()]),
        Some(&Value::String("run-431".to_string()))
    );
    assert_eq!(
        pool.get_selector(&["start".to_string(), "count".to_string()]),
        Some(&Value::Number(3.into()))
    );
    assert_eq!(
        pool.get_selector(&["start".to_string(), "missing".to_string()]),
        Some(&Value::Null)
    );
}

#[test]
fn workflow_execution_batch_parallelizes_only_llm_nodes() {
    let graph = graph(
        vec![
            node("a", "llm", json!({})),
            node("b", "llm", json!({})),
            node("c", "answer", json!({})),
        ],
        Vec::new(),
        vec!["a", "b", "c"],
    );

    assert_eq!(
        workflow_execution_batch(&graph, vec!["a".into(), "b".into()], 10),
        vec!["a".to_string(), "b".to_string()]
    );
    assert_eq!(
        workflow_execution_batch(&graph, vec!["a".into(), "c".into()], 10),
        vec!["a".to_string()]
    );
    assert_eq!(workflow_execution_batch(&graph, vec!["a".into(), "b".into()], 1), vec!["a"]);
}

#[test]
fn workflow_node_model_qualifies_dify_provider_and_preserves_explicit_references() {
    let split = node(
        "llm",
        "llm",
        json!({"model": {"provider": " deepseek ", "name": " deepseek-v4-pro "}}),
    );
    assert_eq!(workflow_node_model(&split, "runtime/model"), "deepseek/deepseek-v4-pro");

    let qualified =
        node("llm", "llm", json!({"model": {"provider": "shiju", "name": "deepseek/v4"}}));
    assert_eq!(workflow_node_model(&qualified, "runtime/model"), "deepseek/v4");

    let missing = node("llm", "llm", json!({"model": {"provider": "deepseek", "name": " "}}));
    assert_eq!(workflow_node_model(&missing, "runtime/model"), "runtime/model");
}

#[test]
fn readiness_honors_selected_branch_edges() {
    let graph = graph(
        vec![
            node("start", "start", json!({})),
            node("branch", "if-else", json!({})),
            node("yes", "answer", json!({})),
            node("no", "answer", json!({})),
        ],
        vec![
            WorkflowEdge {
                source: "start".into(),
                target: "branch".into(),
                source_handle: Some("source".into()),
            },
            WorkflowEdge {
                source: "branch".into(),
                target: "yes".into(),
                source_handle: Some("yes".into()),
            },
            WorkflowEdge {
                source: "branch".into(),
                target: "no".into(),
                source_handle: Some("false".into()),
            },
        ],
        vec!["start"],
    );
    let mut active = BTreeSet::from(["start".to_string()]);
    let mut activated = BTreeSet::new();
    let mut selected = BTreeMap::new();

    activate_outgoing_edges(&graph, &graph.nodes["start"], &selected, &mut activated, &mut active);
    assert!(node_ready(
        &graph,
        "branch",
        &active,
        &BTreeSet::from(["start".to_string()]),
        &activated,
        &selected
    ));
    assert!(!edge_required(&graph, &graph.edges[1], &active, &BTreeSet::new(), &selected));

    active.insert("branch".into());
    selected.insert("branch".into(), "yes".into());
    activate_outgoing_edges(&graph, &graph.nodes["branch"], &selected, &mut activated, &mut active);

    assert!(active.contains("yes"));
    assert!(!active.contains("no"));
    assert!(node_ready(
        &graph,
        "yes",
        &active,
        &BTreeSet::from(["start".to_string(), "branch".to_string()]),
        &activated,
        &selected
    ));
}

#[test]
fn human_input_helpers_validate_actions_and_required_fields() {
    let node = node(
        "human",
        "human-input",
        json!({
            "form": {
                "fields": [
                    {"name": "email", "required": true},
                    {"variable": "note", "required": false}
                ]
            },
            "actions": [
                {"id": "approve", "label": "Approve"},
                {"action": "reject"},
                {"value": " "}
            ]
        }),
    );

    let pause = human_input_pause(&node, "token".into());
    assert_eq!(pause.form_token, "token");
    assert_eq!(pause.actions.len(), 2);
    assert_eq!(resolve_human_input_action(&node, Some("approve")).expect("action"), "approve");
    assert!(
        resolve_human_input_action(&node, Some("missing")).expect_err("missing").contains("不存在")
    );
    assert!(
        validate_human_input_values(&node, &BTreeMap::new())
            .expect_err("required")
            .contains("email")
    );

    let values = BTreeMap::from([("email".to_string(), Value::String("a@example.com".into()))]);
    assert!(validate_human_input_values(&node, &values).is_ok());
    assert_eq!(
        human_input_resume_outputs(&values, "approve").get("action"),
        Some(&Value::String("approve".into()))
    );
}

#[test]
fn node_execution_helpers_extract_inputs_outputs_and_messages() {
    let mut pool = VariablePool::default();
    pool.insert_selector(
        &["sys".to_string(), "query".to_string()],
        Value::String("fallback query".into()),
    );
    pool.insert_node_output("start", "name", Value::String("Alice".into()));

    let llm = node(
        "llm",
        "llm",
        json!({
            "prompt_template": [
                {"role": "system", "text": "You are {{#start.name#}}"},
                {"role": "assistant", "text": "ready"},
                {"role": "user", "text": "Hi {{#start.name#}}"}
            ]
        }),
    );
    let messages = build_llm_messages(&llm, &pool);
    assert_eq!(
        messages.iter().map(|m| m.role.as_str()).collect::<Vec<_>>(),
        vec!["system", "assistant", "user"]
    );
    assert_eq!(messages[2].content, "Hi Alice");

    let no_prompt = node("llm2", "llm", json!({}));
    assert_eq!(build_llm_messages(&no_prompt, &pool)[0].content, "fallback query");

    // {{#context#}} 必须展开为 knowledge-retrieval 节点 result 数组的 content 拼接。
    let mut context_pool = VariablePool::default();
    context_pool.insert_node_output(
        "kb",
        "result",
        json!([
            {"content": "库存预警开启路径：系统-系统设置", "score": 0.89},
            {"content": "按仓库权限区分预警", "score": 0.71}
        ]),
    );
    let context_llm = node(
        "llm",
        "llm",
        json!({
            "context": {
                "enabled": true,
                "variable_selector": ["kb", "result"]
            },
            "prompt_template": [
                {"role": "system", "text": "<context>{{#context#}}</context>"}
            ]
        }),
    );
    let context_messages = build_llm_messages(&context_llm, &context_pool);
    assert_eq!(
        context_messages[0].content,
        "<context>库存预警开启路径：系统-系统设置\n\n按仓库权限区分预警</context>"
    );

    // context.enabled 为 false 时不注入，{{#context#}} 渲染为空。
    let disabled_llm = node(
        "llm",
        "llm",
        json!({
            "context": {
                "enabled": false,
                "variable_selector": ["kb", "result"]
            },
            "prompt_template": [
                {"role": "system", "text": "<context>{{#context#}}</context>"}
            ]
        }),
    );
    assert_eq!(build_llm_messages(&disabled_llm, &context_pool)[0].content, "<context></context>");

    let mapping_node = node(
        "code",
        "code",
        json!({"variables": [{"variable": "name", "value_selector": ["start", "name"]}, {"value_selector": ["x"]}]}),
    );
    assert_eq!(code_inputs(&mapping_node, &pool).get("name"), Some(&Value::String("Alice".into())));
    assert_eq!(
        template_inputs(&mapping_node, &pool).get("name"),
        Some(&Value::String("Alice".into()))
    );

    let answer = execute_answer_node(
        &node("answer", "answer", json!({"answer": "Hello {{#start.name#}}"})),
        &pool,
    );
    assert_eq!(answer.answer.as_deref(), Some("Hello Alice"));
    assert_eq!(
        execute_start_node(&node("start", "start", json!({})), &pool).outputs.get("name"),
        Some(&Value::String("Alice".into()))
    );
}

#[test]
fn output_and_aggregator_helpers_support_aliases_and_errors() {
    let mut pool = VariablePool::default();
    pool.insert_node_output("a", "value", Value::String("one".into()));
    pool.insert_node_output("b", "value", Value::Number(2.into()));

    let output = execute_output_node(
        &node(
            "out",
            "end",
            json!({"output_variables": [{"key": "result", "value_selector": ["a", "value"]}]}),
        ),
        &pool,
    )
    .expect("output");
    assert_eq!(output.outputs.get("result"), Some(&Value::String("one".into())));

    let aggregate = execute_variable_aggregator_node(
        &node("agg", "variable-aggregator", json!({"groups": [{"name": "value", "selectors": [["missing", "value"], ["a", "value"]]}]})),
        &pool,
    )
    .expect("aggregate");
    assert_eq!(aggregate.outputs.get("value"), Some(&Value::String("one".into())));

    let mismatch = match execute_variable_aggregator_node(
        &node(
            "agg",
            "variable-aggregator",
            json!({"variables": [{"variable": "value", "selectors": [["a", "value"], ["b", "value"]]}]}),
        ),
        &pool,
    ) {
        Ok(_) => panic!("mismatched values should be rejected"),
        Err(error) => error,
    };
    assert!(mismatch.contains("类型不一致"));
    assert_eq!(value_type_name(&Value::Bool(true)), "bool");
}

#[test]
fn list_operator_helpers_filter_sort_and_compare_values() {
    let items = vec![
        json!({"name": "b", "score": 2, "tags": "paid"}),
        json!({"name": "a", "score": 10, "tags": "draft"}),
        json!({"name": "c", "score": 1, "tags": "paid"}),
    ];

    let mut filtered = filter_list_items(
        &items,
        Some(&json!({"field": "tags", "operator": "contains", "value": "paid"})),
    )
    .expect("filter");
    sort_list_items(&mut filtered, Some(&json!({"field": "score", "order": "desc"})))
        .expect("sort");

    assert_eq!(
        filtered.iter().map(|item| item["name"].as_str().unwrap()).collect::<Vec<_>>(),
        vec!["b", "c"]
    );
    assert!(list_filter_matches(&Value::String("x".into()), "in", &json!(["a", "x"])).expect("in"));
    assert!(
        filter_list_items(&items, Some(&json!({"operator": "="})))
            .expect_err("field")
            .contains("field")
    );
    assert!(
        sort_list_items(&mut filtered, Some(&json!({"order": "asc"})))
            .expect_err("sort field")
            .contains("field")
    );
    assert_eq!(
        compare_list_sort_values(&Value::Number(2.into()), &Value::Number(10.into())),
        Ordering::Less
    );
    assert_eq!(value_at_field_path(&json!({"a": {"b": true}}), "a.b"), Some(&Value::Bool(true)));
}

#[test]
fn variable_assigner_operations_cover_numbers_arrays_and_persistence_rejection() {
    assert_eq!(
        apply_variable_assignment("add", Value::Null, Value::Number(2.into())).expect("add"),
        json!(2.0)
    );
    assert_eq!(apply_variable_assignment("-", json!(5), json!(3)).expect("subtract"), json!(2.0));
    assert_eq!(apply_variable_assignment("*", json!(5), json!(3)).expect("multiply"), json!(15.0));
    assert_eq!(apply_variable_assignment("/", json!(6), json!(3)).expect("divide"), json!(2.0));
    assert!(
        apply_variable_assignment("/", json!(6), json!(0)).expect_err("zero").contains("除以 0")
    );

    assert_eq!(
        apply_variable_assignment("append", Value::Null, json!("a")).expect("append"),
        json!(["a"])
    );
    assert_eq!(
        apply_variable_assignment("extend", json!(["a"]), json!(["b"])).expect("extend"),
        json!(["a", "b"])
    );
    assert_eq!(
        apply_variable_assignment("remove_first", json!(["a", "b"]), Value::Null)
            .expect("remove first"),
        json!(["b"])
    );
    assert_eq!(
        apply_variable_assignment("remove_last", json!(["a", "b"]), Value::Null)
            .expect("remove last"),
        json!(["a"])
    );
    assert!(
        apply_variable_assignment("extend", Value::Null, json!("bad"))
            .expect_err("extend type")
            .contains("数组")
    );
    assert!(
        apply_variable_assignment("unknown", Value::Null, Value::Null)
            .expect_err("unknown")
            .contains("不支持")
    );

    assert!(
        reject_persistent_variable_assigner(&node(
            "assign",
            "variable-assigner",
            json!({"persistent": true})
        ))
        .is_err()
    );
    assert!(reject_persistent_assignment(&json!({"persist": true})).is_err());
}

#[test]
fn tool_and_agent_helper_functions_validate_shapes() {
    let mut pool = VariablePool::default();
    pool.insert_node_output("start", "count", Value::Number(3.into()));

    let inputs = workflow_tool_inputs(
        Some(&json!([
            {"variable": "count", "value_selector": ["start", "count"], "type": "number"},
            {"key": "literal", "value": "x", "input_type": "string"}
        ])),
        &pool,
    )
    .expect("tool inputs");
    assert_eq!(inputs.get("count"), Some(&Value::Number(3.into())));
    assert!(
        validate_workflow_tool_input_type("count", &Value::String("bad".into()), "number").is_err()
    );
    assert!(workflow_tool_inputs(Some(&json!("bad")), &pool).is_err());

    let tools = workflow_agent_tools(&json!({
        "tools": [{"provider_name": "demo", "tool": "echo", "tool_action": "run", "credentialId": "cred"}]
    }))
    .expect("agent tools");
    assert_eq!(tools[0].provider, "demo");
    assert_eq!(workflow_agent_tools_value(&tools)[0]["credential_id"], "cred");
    assert_eq!(workflow_agent_max_iterations(&json!({})).expect("default"), 3);
    assert!(workflow_agent_max_iterations(&json!({"max_iterations": 0})).is_err());

    let outputs = workflow_tool_outputs(WorkflowToolResult {
        result: json!({"ok": true}),
        text: None,
        json: None,
        files: vec![json!({"name": "a.txt"})],
    });
    assert_eq!(outputs["json"], json!({"ok": true}));
}

#[test]
fn document_helpers_support_inline_text_and_missing_provider_errors() {
    let files = document_files_from_value(&json!([
        {"name": "note.md", "mimeType": "text/markdown", "text": "# Hi", "size": 4},
        {"name": "image.png", "mime_type": "image/png", "text": "raw"}
    ]))
    .expect("files");

    assert!(can_extract_inline_document(&files[0]));
    assert!(!can_extract_inline_document(&files[1]));
    assert_eq!(extract_inline_document(&files[0]).expect("inline").text, "# Hi");
    assert!(document_extractor_missing_provider_error(&files).contains("image/png"));
    assert!(document_files_from_value(&Value::Null).is_err());
    assert!(
        extract_inline_document(&WorkflowDocumentFile {
            name: "remote.txt".into(),
            mime_type: "text/plain".into(),
            path: Some("/tmp/a".into()),
            url: None,
            size: None,
            raw: json!({"text": "x"})
        })
        .is_err()
    );
}

#[test]
fn loop_and_subgraph_helpers_cover_stop_conditions() {
    let mut pool = VariablePool::default();
    pool.insert_node_output("loop", "done", Value::Bool(true));
    pool.insert_node_output("loop", "count", Value::Number(3.into()));

    assert_eq!(loop_max_count(&json!({})).expect("default"), LOOP_DEFAULT_MAX_COUNT);
    assert!(loop_max_count(&json!({"max_count": 0})).is_err());
    assert!(loop_max_count(&json!({"max_iterations": LOOP_MAX_COUNT as u64 + 1})).is_err());
    assert!(loop_has_stop_condition(&json!({"stop_selector": ["loop", "done"]})));
    assert!(
        loop_should_stop(&json!({"stop_selector": ["loop", "done"]}), &pool).expect("selector")
    );
    assert!(loop_should_stop(
        &json!({"break_conditions": [{"variable_selector": ["loop", "count"], "comparison_operator": ">=", "value": 3}]}),
        &pool
    )
    .expect("conditions"));
    assert!(workflow_value_truthy(&json!([1])));
    assert!(!workflow_value_truthy(&Value::String("false".into())));

    let subgraph = workflow_graph_from_value(
        &json!({"nodes": [{"id": "a", "data": {"type": "answer"}}, {"id": "b"}], "edges": [{"source": "a", "target": "b", "sourceHandle": "source"}]}),
        "test graph",
    )
    .expect("subgraph");
    assert_eq!(subgraph.start_node_ids, vec!["a"]);
    assert!(workflow_graph_from_value(&json!({"nodes": []}), "empty").is_err());
    assert_eq!(
        iteration_error_strategy(&node(
            "iter",
            "iteration",
            json!({"error_handle_mode": "continue"})
        )),
        "continue"
    );
    assert!(
        reject_parallel_iteration(&node("iter", "iteration", json!({"mode": "parallel"}))).is_err()
    );
}

#[test]
fn classifier_and_parameter_helpers_parse_model_outputs() {
    let classes =
        question_classes(&json!({"topics": [{"name": "billing", "description": "Billing"}]}))
            .expect("classes");
    assert_eq!(classes[0].id, "billing");
    assert!(question_classifier_messages("pay", &classes)[0].content.contains("billing"));
    assert_eq!(normalize_classifier_response(r#"{"class_id":"billing"}"#), "billing");
    assert_eq!(normalize_classifier_response("'billing'"), "billing");
    let graph = graph(
        vec![node("q", "question-classifier", json!({}))],
        vec![WorkflowEdge {
            source: "q".into(),
            target: "fallback".into(),
            source_handle: Some("default".into()),
        }],
        vec!["q"],
    );
    assert_eq!(question_classifier_fallback_handle(&graph, "q"), Some("default".into()));
    assert_eq!(
        question_classifier_execution(
            "input",
            "id".into(),
            "Name".into(),
            "raw".into(),
            "id".into()
        )
        .selected_handle,
        Some("id".into())
    );

    let params = parameter_definitions(&json!({"parameters": [
        {"name": "city", "type": "string", "required": true},
        {"variable": "count", "type": "number"}
    ]}))
    .expect("params");
    assert_eq!(parameter_prompt_schema(&params).len(), 2);
    let parsed = parse_parameter_extractor_json(r#"{"city":"Hangzhou"}"#).expect("json");
    let outputs = parameter_extractor_outputs(&params, &parsed);
    assert_eq!(missing_required_parameters(&params, &outputs), Vec::<String>::new());
    assert!(parse_parameter_extractor_json("").is_err());
    assert!(validate_parameter_type("date").is_err());
}

#[test]
fn http_helpers_render_validate_and_redact_values() {
    let mut pool = VariablePool::default();
    pool.insert_node_output("start", "token", Value::String("secret-token".into()));
    let template_values = BTreeMap::from([("q".to_string(), Value::String("hello world".into()))]);
    let http_node = node(
        "http",
        "http-request",
        json!({
            "headers": {"X-Query": "{{ q }}"},
            "authorization": {"Authorization": "Bearer {{#start.token#}}"}
        }),
    );

    assert_eq!(http_request_method("post").expect("method"), reqwest::Method::POST);
    assert!(http_request_method("trace").is_err());
    assert!(validate_http_request_url("https://example.com").is_ok());
    assert!(validate_http_request_url("ftp://example.com").is_err());
    assert_eq!(
        http_request_timeout_secs(Some(&json!(999))).expect("timeout"),
        HTTP_REQUEST_MAX_TIMEOUT_SECS
    );
    assert!(http_request_timeout_secs(Some(&json!(0))).is_err());
    assert_eq!(
        http_request_headers(&http_node, &pool, &template_values).expect("headers")["Authorization"],
        "Bearer secret-token"
    );
    assert_eq!(
        render_http_value(&json!({"q": "{{ q }}"}), &pool, &template_values).expect("render"),
        json!({"q": "hello world"})
    );
    assert!(http_string_map(&json!(["bad"]), "headers").is_err());
    assert!(
        append_http_request_params("https://example.com/a?x=1", &json!({"q": ["a", "b"]}))
            .expect("params")
            .contains("q=a")
    );
    assert!(
        http_query_pairs(&json!([["a", 1]])).expect("pairs").contains(&("a".into(), "1".into()))
    );
    assert!(http_query_pairs(&json!([["a"]])).is_err());

    let redacted = redact_url_for_log("https://user:pass@example.com/path?token=abc&q=ok");
    assert!(redacted.contains("%5BREDACTED%5D") || redacted.contains("[REDACTED]"));
    assert!(!redacted.contains("abc"));
}

#[test]
fn output_delta_token_and_redaction_helpers_cover_edge_cases() {
    let outputs = llm_outputs(r#"{"ok":true}"#.to_string());
    assert_eq!(outputs["json"], json!({"ok": true}));
    let plain_outputs = llm_outputs("plain answer".to_string());
    assert_eq!(plain_outputs["text"], json!("plain answer"));
    assert!(plain_outputs.get("json").is_none());
    assert!(workflow_token_usage(None).is_none());
    assert!(workflow_token_usage(Some(&TokenUsage::default())).is_none());
    let usage = workflow_token_usage(Some(&TokenUsage {
        input_tokens: Some(2),
        output_tokens: Some(3),
        cached_tokens: Some(1),
        reasoning_tokens: Some(4),
    }))
    .expect("usage");
    assert_eq!(usage["total_tokens"], 9);
    assert!(workflow_estimated_token_usage(0).is_none());
    assert_eq!(workflow_estimated_token_usage(5).expect("estimated")["completion_tokens"], 5);

    let answer_execution = NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::new(),
        outputs: BTreeMap::new(),
        selected_handle: None,
        answer: Some("answer text".into()),
        error: None,
        elapsed_ms: 0,
    };
    let delta = workflow_node_delta_from_execution(
        &node("answer", "answer", json!({})),
        1,
        &answer_execution,
    )
    .expect("delta");
    assert_eq!(delta.text, "answer text");
    assert!(
        workflow_node_delta_from_execution(&node("llm", "llm", json!({})), 1, &answer_execution)
            .is_none()
    );
    let output_execution = NodeExecution {
        status: WorkflowNodeRunStatus::Succeeded,
        inputs: BTreeMap::new(),
        outputs: BTreeMap::from([("result".to_string(), Value::String("result text".into()))]),
        selected_handle: None,
        answer: None,
        error: None,
        elapsed_ms: 0,
    };
    assert_eq!(
        workflow_node_delta_from_execution(&node("out", "answer", json!({})), 2, &output_execution)
            .expect("output delta")
            .text,
        "result text"
    );
    assert!(
        truncate_workflow_node_delta(&"x".repeat(WORKFLOW_NODE_DELTA_MAX_CHARS + 10))
            .contains("已截断")
    );
    assert_eq!(truncate_workflow_node_delta("short"), "short");

    let redacted = redact_map(&BTreeMap::from([
        ("api_key".to_string(), Value::String("secret".into())),
        ("nested".to_string(), json!({"password": "pw", "ok": true})),
    ]));
    assert_eq!(redacted["api_key"], Value::String("[REDACTED]".into()));
    assert_eq!(redacted["nested"]["password"], Value::String("[REDACTED]".into()));
    let redacted_headers = redact_string_map(&BTreeMap::from([
        ("Authorization".to_string(), "Bearer secret".to_string()),
        ("X-Trace".to_string(), "trace".to_string()),
    ]));
    assert_eq!(redacted_headers["Authorization"], "[REDACTED]");
    assert_eq!(redacted_headers["X-Trace"], "trace");
    assert!(
        debug_json_value(&json!({"text": "x".repeat(WORKFLOW_DEBUG_MAX_CHARS + 10)}))
            .ends_with("...")
    );
    assert!(!debug_body_text(r#"{"password":"pw"}"#).contains("pw"));
    assert!(debug_body_text("plain body").contains("plain body"));
    assert!(is_sensitive_key("Cookie"));
    assert!(workflow_value_truthy(&json!(1)));
    assert!(!workflow_value_truthy(&json!(0)));
    assert!(workflow_value_truthy(&json!({"ok": true})));
    assert!(!workflow_value_truthy(&json!({})));
}
