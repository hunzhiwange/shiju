//! 输出与文本节点的导入、可视化编辑、校验和无损 YAML 合并回归测试。

use super::*;

fn yaml_value(text: &str) -> Value {
    serde_yaml::from_str(text).expect("test YAML should parse")
}

fn map_value<'a>(value: &'a Value, key: &str) -> &'a Value {
    mapping_value(value.as_mapping().expect("value should be a map"), key)
        .unwrap_or_else(|| panic!("missing key {key}"))
}

#[test]
fn output_and_text_nodes_round_trip_visual_edits() {
    let cases = [
        (
            "end",
            "outputs:\n  - variable: old\n    value_type: string\n    value_selector: [start, query]\nunknown: kept\n",
        ),
        (
            "template-transform",
            "template: old template\nvariables:\n  - variable: old\n    value_selector: [start, query]\nunknown: kept\n",
        ),
        ("document-extractor", "variable_selector: [start, files]\nunknown: kept\n"),
    ];

    for (block_type, raw_yaml) in cases {
        let mut draft = build_node_visual_draft(block_type, raw_yaml)
            .expect("draft should parse")
            .expect("node should have a visual draft");

        match &mut draft {
            WorkflowNodeVisualDraft::End { outputs } => {
                outputs[0].name = "result".to_string();
                outputs[0].value_type = "number".to_string();
                outputs[0].selector = WorkflowVariableSelectorDraft::from_input("code.total");
            }
            WorkflowNodeVisualDraft::TemplateTransform { variables, template_editor } => {
                variables[0].name = "question".to_string();
                variables[0].selector = WorkflowVariableSelectorDraft::from_input("start.question");
                *template_editor = text_editor::Content::with_text("Hello {{ question }}");
            }
            WorkflowNodeVisualDraft::DocumentExtractor { variable_selector } => {
                *variable_selector = WorkflowVariableSelectorDraft::from_input("upload.document");
            }
            _ => panic!("unexpected draft for {block_type}"),
        }

        let validation =
            validate_node_editor_draft(block_type, "title", "", raw_yaml, Some(&draft));
        assert!(validation.field_errors.is_empty(), "{block_type}: {:?}", validation.field_errors);

        let synced = apply_visual_draft_to_yaml(block_type, raw_yaml, Some(&draft))
            .expect("visual edit should sync");
        let value = yaml_value(&synced);
        assert_eq!(map_value(&value, "unknown").as_str(), Some("kept"));

        match block_type {
            "end" => {
                let outputs = map_value(&value, "outputs").as_sequence().expect("outputs sequence");
                let output = outputs[0].as_mapping().expect("output map");
                assert_eq!(
                    mapping_value(output, "variable").and_then(Value::as_str),
                    Some("result")
                );
                assert_eq!(
                    mapping_value(output, "value_type").and_then(Value::as_str),
                    Some("number")
                );
                assert_eq!(
                    mapping_value(output, "value_selector").and_then(Value::as_sequence).unwrap()
                        [1]
                    .as_str(),
                    Some("total")
                );
            }
            "template-transform" => {
                assert_eq!(map_value(&value, "template").as_str(), Some("Hello {{ question }}"));
                let variables =
                    map_value(&value, "variables").as_sequence().expect("variables sequence");
                let variable = variables[0].as_mapping().expect("variable map");
                assert_eq!(
                    mapping_value(variable, "variable").and_then(Value::as_str),
                    Some("question")
                );
                assert_eq!(
                    mapping_value(variable, "value_selector").and_then(Value::as_sequence).unwrap()
                        [1]
                    .as_str(),
                    Some("question")
                );
            }
            "document-extractor" => {
                let selector = map_value(&value, "variable_selector")
                    .as_sequence()
                    .expect("selector sequence");
                assert_eq!(selector[0].as_str(), Some("upload"));
                assert_eq!(selector[1].as_str(), Some("document"));
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn output_and_text_nodes_defaults_build_visual_drafts() {
    for block_type in ["end", "template-transform", "document-extractor"] {
        let raw_yaml =
            default_node_data_yaml(block_type).expect("registered node should have defaults");
        assert!(
            build_node_visual_draft(block_type, &raw_yaml)
                .expect("default YAML should parse")
                .is_some(),
            "{block_type} should have a visual draft"
        );
    }
}
