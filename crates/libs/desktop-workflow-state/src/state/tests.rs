//! 工作流状态测试，验证编辑状态、选择状态和导入状态的转换行为。

use super::*;
use serde_yaml::{Mapping, Value};

fn mapping_value<'a>(mapping: &'a Mapping, key: &str) -> &'a Value {
    mapping.get(&yaml_key(key)).expect("expected key to exist")
}

#[test]
fn code_visual_draft_generates_default_values_from_outputs() {
    let yaml = r#"
code_language: javascript
code: |
  function main() {
    return { result: "ok", total: 2 }
  }
variables:
  - variable: query
    value_selector: [sys, query]
    value_type: string
  - variable: limit
    value_selector: [start, limit]
    value_type: number
outputs:
  result:
    type: string
    children: null
  total:
    type: number
    children: null
retry_config:
  retry_enabled: true
  max_retries: 5
  retry_interval: 1200
error_strategy: default-value
"#;

    let visual_draft = build_node_visual_draft("code", yaml)
        .expect("code node draft should build")
        .expect("code node should have a visual draft");

    let WorkflowNodeVisualDraft::Code {
        language,
        inputs,
        code_editor,
        outputs,
        retry_config,
        error_strategy,
        default_value_editor,
    } = visual_draft
    else {
        panic!("expected code visual draft");
    };

    assert_eq!(language, "javascript");
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0].variable, "query");
    assert_eq!(inputs[0].selector, vec!["sys".to_string(), "query".to_string()]);
    assert_eq!(inputs[0].value_type, "string");
    assert!(code_editor.text().contains("function main()"));
    assert_eq!(outputs.len(), 2);
    assert_eq!(outputs[0].key, "result");
    assert_eq!(outputs[0].value_type, "string");
    assert_eq!(outputs[1].key, "total");
    assert_eq!(outputs[1].value_type, "number");
    assert!(retry_config.enabled);
    assert_eq!(retry_config.max_retries, 5);
    assert_eq!(retry_config.retry_interval, 1200);
    assert_eq!(error_strategy, "default-value");

    let generated_default_value = parse_code_default_value_yaml(&default_value_editor.text())
        .expect("default value editor should contain generated yaml");
    let generated_items =
        generated_default_value.as_sequence().expect("default value should be a sequence");
    assert_eq!(generated_items.len(), 2);
    assert_eq!(
        mapping_value(
            generated_items[0].as_mapping().expect("first item should be a map"),
            "value",
        )
        .as_str(),
        Some("")
    );
    assert_eq!(
        mapping_value(
            generated_items[1].as_mapping().expect("second item should be a map"),
            "value",
        )
        .as_u64(),
        Some(0)
    );
}

#[test]
fn code_default_value_generation_uses_native_yaml_for_object_and_array() {
    let generated = default_code_default_value_value(&[
        WorkflowCodeOutputDraft { key: "payload".to_string(), value_type: "object".to_string() },
        WorkflowCodeOutputDraft {
            key: "items".to_string(),
            value_type: "array[string]".to_string(),
        },
    ]);

    let items = generated.as_sequence().expect("generated default value should be a sequence");
    assert!(
        mapping_value(items[0].as_mapping().expect("payload item should be a map"), "value")
            .is_mapping()
    );
    assert!(
        mapping_value(items[1].as_mapping().expect("items item should be a map"), "value")
            .is_sequence()
    );
}

#[test]
fn apply_visual_draft_to_yaml_serializes_structured_code_fields() {
    let yaml = r#"
code_language: javascript
code: |
  function main() {
    return { result: "ok", total: 2 }
  }
variables:
  - variable: query
    value_selector: [sys, query]
    value_type: string
outputs:
  result:
    type: string
    children: null
  total:
    type: number
    children: null
retry_config:
  retry_enabled: true
  max_retries: 5
  retry_interval: 1200
error_strategy: default-value
"#;

    let visual_draft = build_node_visual_draft("code", yaml)
        .expect("code node draft should build")
        .expect("code node should have a visual draft");
    let merged_yaml = apply_visual_draft_to_yaml("code", yaml, Some(&visual_draft))
        .expect("code visual draft should serialize back to yaml");
    let merged_value =
        serde_yaml::from_str::<Value>(&merged_yaml).expect("merged yaml should parse");
    let merged_map = merged_value.as_mapping().expect("merged yaml should be a map");

    let variables = mapping_value(merged_map, "variables")
        .as_sequence()
        .expect("variables should be a sequence");
    assert_eq!(variables.len(), 1);
    let first_variable = variables[0].as_mapping().expect("variable should be a map");
    assert_eq!(mapping_value(first_variable, "variable").as_str(), Some("query"));
    assert_eq!(
        mapping_value(first_variable, "value_selector")
            .as_sequence()
            .expect("selector should be a sequence")
            .len(),
        2
    );

    let outputs =
        mapping_value(merged_map, "outputs").as_mapping().expect("outputs should be a map");
    let result_output =
        mapping_value(outputs, "result").as_mapping().expect("result output should be a map");
    let total_output =
        mapping_value(outputs, "total").as_mapping().expect("total output should be a map");
    assert_eq!(mapping_value(result_output, "type").as_str(), Some("string"));
    assert_eq!(mapping_value(total_output, "type").as_str(), Some("number"));

    let retry_config = mapping_value(merged_map, "retry_config")
        .as_mapping()
        .expect("retry_config should be a map");
    assert_eq!(mapping_value(retry_config, "retry_enabled").as_bool(), Some(true));
    assert_eq!(mapping_value(retry_config, "max_retries").as_u64(), Some(5));
    assert_eq!(mapping_value(retry_config, "retry_interval").as_u64(), Some(1200));
    assert_eq!(mapping_value(merged_map, "error_strategy").as_str(), Some("default-value"));

    let default_value = mapping_value(merged_map, "default_value")
        .as_sequence()
        .expect("default_value should be a sequence");
    assert_eq!(default_value.len(), 2);
}
