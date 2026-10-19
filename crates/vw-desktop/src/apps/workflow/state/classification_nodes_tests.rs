//! 分类与参数提取节点的导入、编辑、校验及分支一致性回归测试。

use super::*;
use crate::apps::workflow::model::{WorkflowDocument, WorkflowEdge};
use iced::{Point, widget::text_editor};

fn classifier_node(id: &str, classes: &str) -> WorkflowNode {
    let base =
        create_node_from_type("question-classifier", id.to_string(), Point::new(0.0, 0.0), 1.0)
            .expect("classifier should be registered");
    let yaml = format!(
        "query_variable_selector: [start, query]\nmodel: {{ provider: openai, name: gpt, mode: chat }}\nclasses:\n{classes}instruction: classify\nunknown: kept\n"
    );
    rebuild_node_from_parts(&base, "分类", "", &yaml).expect("classifier should rebuild")
}

#[test]
fn classifier_updates_handles_without_dangling_edges() {
    let classifier = classifier_node(
        "classifier",
        "  - id: class-2\n    name: 售后\n  - id: class-9\n    name: 订单\n",
    );
    assert_eq!(
        classifier.source_handles.iter().map(|handle| handle.id.as_str()).collect::<Vec<_>>(),
        vec!["class-2", "class-9"]
    );

    let mut state = WorkflowState {
        document: WorkflowDocument {
            nodes: vec![classifier],
            edges: vec![WorkflowEdge {
                id: "class-9-edge".to_string(),
                source: "classifier".to_string(),
                target: "answer".to_string(),
                source_handle: Some("class-9".to_string()),
                target_handle: Some("target".to_string()),
                source_type: "question-classifier".to_string(),
                target_type: "answer".to_string(),
                selected: false,
                z_index: 1.0,
                raw_edge: Value::Null,
            }],
            ..WorkflowDocument::default()
        },
        ..WorkflowState::default()
    };

    state.open_edit_node_editor(Some("classifier")).expect("editor should open");
    state.remove_node_editor_classifier_class(1);
    state.submit_node_editor().expect("classifier should save");

    let classifier = state.document.node("classifier").expect("classifier remains");
    assert_eq!(
        classifier.source_handles.iter().map(|handle| handle.id.as_str()).collect::<Vec<_>>(),
        vec!["class-2"]
    );
    assert!(state.document.edges.is_empty());
    let data: Value = serde_yaml::from_str(&node_data_yaml(classifier).unwrap()).unwrap();
    assert_eq!(
        mapping_value(data.as_mapping().unwrap(), "unknown").and_then(Value::as_str),
        Some("kept")
    );
    assert_eq!(state.status_message.as_deref(), Some("已更新 问题分类 节点，并移除 1 条失效连线"));
}

#[test]
fn parameter_extractor_round_trips_parameters() {
    let raw_yaml = "query: [start, query]\nmodel: { provider: openai, name: gpt, mode: chat, completion_params: { temperature: 0.2 } }\nparameters:\n  - name: city\n    type: string\n    description: 城市\n    required: true\n    x-extra: kept\nreasoning_mode: prompt\ninstruction: extract\nmemory: { enabled: true }\n";
    let mut draft = build_node_visual_draft("parameter-extractor", raw_yaml)
        .expect("draft should parse")
        .expect("extractor should have a visual draft");

    let WorkflowNodeVisualDraft::ParameterExtractor { parameters, instruction_editor, .. } =
        &mut draft
    else {
        panic!("expected parameter extractor draft");
    };
    assert_eq!(parameters[0].name, "city");
    parameters.push(WorkflowParameterDraft {
        name: "days".to_string(),
        value_type: "number".to_string(),
        description: "天数".to_string(),
        required: false,
        ..WorkflowParameterDraft::default()
    });
    *instruction_editor = text_editor::Content::with_text("extract precisely");

    let validation =
        validate_node_editor_draft("parameter-extractor", "", "", raw_yaml, Some(&draft));
    assert!(validation.field_errors.is_empty(), "{:?}", validation.field_errors);
    let synced = apply_visual_draft_to_yaml("parameter-extractor", raw_yaml, Some(&draft))
        .expect("extractor should sync");
    let value: Value = serde_yaml::from_str(&synced).expect("synced YAML");
    let map = value.as_mapping().expect("map");
    assert_eq!(mapping_value(map, "memory").and_then(Value::as_mapping).is_some(), true);
    assert_eq!(mapping_value(map, "parameters").and_then(Value::as_sequence).unwrap().len(), 2);
    let parameters = mapping_value(map, "parameters").and_then(Value::as_sequence).unwrap();
    assert_eq!(
        mapping_value(parameters[0].as_mapping().unwrap(), "x-extra").and_then(Value::as_str),
        Some("kept")
    );
    let model = mapping_value(map, "model").and_then(Value::as_mapping).unwrap();
    assert_eq!(
        mapping_value(model, "completion_params")
            .and_then(Value::as_mapping)
            .and_then(|params| mapping_value(params, "temperature"))
            .and_then(Value::as_f64),
        Some(0.2)
    );

    if let WorkflowNodeVisualDraft::ParameterExtractor { parameters, .. } = &mut draft {
        parameters[1].name = "__is_success".to_string();
    }
    let validation =
        validate_node_editor_draft("parameter-extractor", "", "", raw_yaml, Some(&draft));
    assert!(validation.first_error_for("parameter_extractor.parameters[1].name").is_some());
}
