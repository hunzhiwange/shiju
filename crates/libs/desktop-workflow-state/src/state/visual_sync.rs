//! # Workflow 可视化同步
//!
//! 该模块将节点原始 YAML 解析成可视化编辑草稿，并将可视化草稿同步回 YAML。

use super::*;

mod data_nodes;
mod input_nodes;
mod integration_nodes;

pub(super) fn apply_visual_draft_to_yaml(
    block_type: &str,
    raw_data_yaml: &str,
    visual_draft: Option<&WorkflowNodeVisualDraft>,
) -> Result<String, desktop_i18n::workflow::Feedback> {
    let Some(visual_draft) = visual_draft else {
        return Ok(raw_data_yaml.to_string());
    };

    let mut data_value = parse_node_data_yaml_value(raw_data_yaml)?;
    let data_map = data_value.as_mapping_mut().ok_or_else(|| {
        desktop_i18n::workflow::Feedback::new("节点 data 必须是对象映射（YAML map）", vec![])
    })?;

    match block_type {
        "start" | "llm" | "answer" | "http-request" => {
            input_nodes::apply(data_map, block_type, visual_draft)?;
        }
        "end" | "template-transform" | "document-extractor" | "variable-assigner"
        | "variable-aggregator" | "list-operator" | "iteration" | "loop"
        | "question-classifier" | "parameter-extractor" | "if-else" => {
            data_nodes::apply(data_map, block_type, visual_draft)?;
        }
        "knowledge-retrieval" | "tool" | "agent" | "code" => {
            integration_nodes::apply(data_map, block_type, visual_draft)?;
        }
        _ => {}
    }

    let yaml = serde_yaml::to_string(&data_value).map_err(|error| {
        desktop_i18n::workflow::Feedback::new(
            "生成节点 data YAML 失败: %s",
            vec![error.to_string()],
        )
    })?;
    Ok(yaml.strip_prefix("---\n").unwrap_or(&yaml).to_string())
}

pub(super) use crate::parse_http_value;

fn named_values_value(values: &[WorkflowNamedValueDraft], include_type: bool) -> Value {
    Value::Sequence(
        values
            .iter()
            .map(|item| {
                let mut map = Mapping::new();
                if !item.id.is_empty() {
                    set_mapping_string(&mut map, "id", &item.id);
                }
                set_mapping_string(&mut map, "variable", &item.name);
                if include_type {
                    set_mapping_string(&mut map, "value_type", &item.value_type);
                }
                map.insert(
                    yaml_key("value_selector"),
                    Value::Sequence(
                        item.selector.path.iter().cloned().map(Value::String).collect(),
                    ),
                );
                Value::Mapping(map)
            })
            .collect(),
    )
}

fn selector_path_value(selector: &WorkflowVariableSelectorDraft) -> Value {
    Value::Sequence(selector.path.iter().cloned().map(Value::String).collect())
}

fn merge_model_value(data_map: &mut Mapping, model: &WorkflowModelDraft) {
    let model_map = ensure_mapping_entry(data_map, "model");
    set_mapping_string(model_map, "provider", &model.provider);
    set_mapping_string(model_map, "name", &model.name);
    set_mapping_string(model_map, "mode", &model.mode);
}

pub(super) use crate::parse_node_data_yaml_value;

pub(super) fn ensure_root_mapping(value: Value) -> Value {
    if value.is_mapping() { value } else { Value::Mapping(Mapping::new()) }
}

pub(super) fn selector_input_from_value(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_sequence)
        .map(|selectors| {
            selectors
                .iter()
                .filter_map(Value::as_sequence)
                .map(|parts| parts.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("."))
                .filter(|selector| !selector.trim().is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

pub(super) fn selector_path_input_from_value(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_sequence)
        .map(|parts| parts.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("."))
        .unwrap_or_default()
}

pub(super) fn selector_value_from_input(input: &str) -> Value {
    let selectors = input
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            Value::Sequence(
                item.split('.')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(|part| Value::String(part.to_string()))
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    Value::Sequence(selectors)
}

pub(super) fn selector_path_value_from_input(input: &str) -> Value {
    Value::Sequence(
        input
            .split('.')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(|part| Value::String(part.to_string()))
            .collect(),
    )
}

fn code_variable_value(variable: &WorkflowCodeVariableDraft) -> Value {
    yaml_map(vec![
        ("variable", Value::String(variable.variable.clone())),
        (
            "value_selector",
            Value::Sequence(
                variable.selector.iter().map(|part| Value::String(part.clone())).collect(),
            ),
        ),
        ("value_type", Value::String(variable.value_type.clone())),
    ])
}

fn code_outputs_value(outputs: &[WorkflowCodeOutputDraft]) -> Value {
    let mut mapping = Mapping::new();

    for output in outputs {
        mapping.insert(
            yaml_key(&output.key),
            yaml_map(vec![
                ("children", Value::Null),
                ("type", Value::String(output.value_type.clone())),
            ]),
        );
    }

    Value::Mapping(mapping)
}

#[cfg(test)]
#[path = "visual_sync_tests.rs"]
mod visual_sync_tests;
