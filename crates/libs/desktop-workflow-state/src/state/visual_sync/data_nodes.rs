//! Node-specific visual draft serialization.
use super::*;

pub(super) fn apply(
    data_map: &mut Mapping,
    block_type: &str,
    visual_draft: &WorkflowNodeVisualDraft,
) -> Result<(), desktop_i18n::workflow::Feedback> {
    match (block_type, visual_draft) {
        ("end", WorkflowNodeVisualDraft::End { outputs }) => {
            data_map.insert(yaml_key("outputs"), named_values_value(outputs, true));
        }
        (
            "template-transform",
            WorkflowNodeVisualDraft::TemplateTransform { variables, template_editor },
        ) => {
            data_map.insert(yaml_key("variables"), named_values_value(variables, false));
            set_mapping_string(data_map, "template", &template_editor.text());
        }
        (
            "document-extractor",
            WorkflowNodeVisualDraft::DocumentExtractor { variable_selector },
        ) => {
            data_map.insert(
                yaml_key("variable_selector"),
                Value::Sequence(
                    variable_selector.path.iter().cloned().map(Value::String).collect(),
                ),
            );
        }
        ("variable-assigner", WorkflowNodeVisualDraft::VariableAssigner { items }) => {
            data_map.insert(
                yaml_key("items"),
                Value::Sequence(
                    items
                        .iter()
                        .map(|item| {
                            let mut map = item.raw_item.as_mapping().cloned().unwrap_or_default();
                            map.insert(
                                yaml_key("variable_selector"),
                                selector_path_value(&item.variable_selector),
                            );
                            set_mapping_string(&mut map, "operation", &item.operation);
                            set_mapping_string(&mut map, "input_type", &item.input_type);
                            map.insert(yaml_key("value"), item.value.clone());
                            Value::Mapping(map)
                        })
                        .collect(),
                ),
            );
        }
        (
            "variable-aggregator",
            WorkflowNodeVisualDraft::VariableAggregator { variables, output_type },
        ) => {
            data_map.insert(
                yaml_key("variables"),
                Value::Sequence(
                    variables
                        .iter()
                        .map(|item| {
                            selector_path_value(&WorkflowVariableSelectorDraft::from_input(item))
                        })
                        .collect(),
                ),
            );
            set_mapping_string(data_map, "output_type", output_type);
        }
        (
            "list-operator",
            WorkflowNodeVisualDraft::ListOperator {
                variable,
                conditions,
                filter_enabled,
                order_enabled,
                order_key,
                order_direction,
                limit_enabled,
                limit_size,
            },
        ) => {
            data_map.insert(
                yaml_key("variable"),
                selector_path_value(&WorkflowVariableSelectorDraft::from_input(variable)),
            );
            let filter = ensure_mapping_entry(data_map, "filter_by");
            set_mapping_bool(filter, "enabled", *filter_enabled);
            filter.insert(
                yaml_key("conditions"),
                Value::Sequence(
                    conditions
                        .iter()
                        .map(|condition| {
                            let mut map =
                                condition.raw_condition.as_mapping().cloned().unwrap_or_default();
                            set_mapping_string(&mut map, "key", &condition.key);
                            set_mapping_string(
                                &mut map,
                                "comparison_operator",
                                &condition.comparison_operator,
                            );
                            map.insert(yaml_key("value"), condition.value.clone());
                            Value::Mapping(map)
                        })
                        .collect(),
                ),
            );
            let order = ensure_mapping_entry(data_map, "order_by");
            set_mapping_bool(order, "enabled", *order_enabled);
            set_mapping_string(order, "key", order_key);
            set_mapping_string(order, "value", order_direction);
            let limit = ensure_mapping_entry(data_map, "limit");
            set_mapping_bool(limit, "enabled", *limit_enabled);
            let size = limit_size.trim().parse::<u64>().map_err(|_| {
                desktop_i18n::workflow::Feedback::new("列表截取数量必须是非负整数", vec![])
            })?;
            limit.insert(yaml_key("size"), serde_yaml::to_value(size).map_err(|e| e.to_string())?);
        }
        (
            "iteration",
            WorkflowNodeVisualDraft::Iteration {
                iterator_selector,
                output_selector,
                output_type,
                is_parallel,
                parallel_nums,
                error_handle_mode,
            },
        ) => {
            data_map.insert(
                yaml_key("iterator_selector"),
                selector_path_value(&WorkflowVariableSelectorDraft::from_input(iterator_selector)),
            );
            data_map.insert(
                yaml_key("output_selector"),
                selector_path_value(&WorkflowVariableSelectorDraft::from_input(output_selector)),
            );
            set_mapping_string(data_map, "output_type", output_type);
            set_mapping_bool(data_map, "is_parallel", *is_parallel);
            let count = parallel_nums.trim().parse::<u64>().map_err(|_| {
                desktop_i18n::workflow::Feedback::new("并行数量必须是正整数", vec![])
            })?;
            data_map.insert(
                yaml_key("parallel_nums"),
                serde_yaml::to_value(count).map_err(|e| e.to_string())?,
            );
            set_mapping_string(data_map, "error_handle_mode", error_handle_mode);
        }
        (
            "loop",
            WorkflowNodeVisualDraft::Loop {
                variables,
                break_conditions,
                logical_operator,
                max_loop_count,
            },
        ) => {
            data_map.insert(
                yaml_key("loop_variables"),
                Value::Sequence(
                    variables
                        .iter()
                        .map(|item| {
                            let mut map =
                                item.raw_variable.as_mapping().cloned().unwrap_or_default();
                            set_mapping_string(&mut map, "id", &item.id);
                            set_mapping_string(&mut map, "label", &item.label);
                            set_mapping_string(&mut map, "var_type", &item.value_type);
                            map.insert(yaml_key("value"), item.initial_value.clone());
                            Value::Mapping(map)
                        })
                        .collect(),
                ),
            );
            data_map.insert(
                yaml_key("break_conditions"),
                Value::Sequence(
                    break_conditions
                        .iter()
                        .map(|item| {
                            let mut map =
                                item.raw_condition.as_mapping().cloned().unwrap_or_default();
                            map.insert(
                                yaml_key("variable_selector"),
                                selector_path_value(&WorkflowVariableSelectorDraft::from_input(
                                    &item.selector,
                                )),
                            );
                            set_mapping_string(
                                &mut map,
                                "comparison_operator",
                                &item.comparison_operator,
                            );
                            map.insert(yaml_key("value"), item.value.clone());
                            Value::Mapping(map)
                        })
                        .collect(),
                ),
            );
            set_mapping_string(data_map, "logical_operator", logical_operator);
            let count = max_loop_count.trim().parse::<u64>().map_err(|_| {
                desktop_i18n::workflow::Feedback::new("最大循环次数必须是正整数", vec![])
            })?;
            data_map.insert(
                yaml_key("max_loop_count"),
                serde_yaml::to_value(count).map_err(|e| e.to_string())?,
            );
        }
        (
            "question-classifier",
            WorkflowNodeVisualDraft::QuestionClassifier {
                query,
                model,
                classes,
                instruction_editor,
            },
        ) => {
            data_map.insert(yaml_key("query_variable_selector"), selector_path_value(query));
            merge_model_value(data_map, model);
            data_map.insert(
                yaml_key("classes"),
                Value::Sequence(
                    classes
                        .iter()
                        .map(|item| {
                            let mut map = item.raw_class.as_mapping().cloned().unwrap_or_default();
                            set_mapping_string(&mut map, "id", &item.id);
                            set_mapping_string(&mut map, "name", &item.name);
                            Value::Mapping(map)
                        })
                        .collect(),
                ),
            );
            set_mapping_string(data_map, "instruction", &instruction_editor.text());
        }
        (
            "parameter-extractor",
            WorkflowNodeVisualDraft::ParameterExtractor {
                query,
                model,
                parameters,
                reasoning_mode,
                instruction_editor,
            },
        ) => {
            data_map.insert(yaml_key("query"), selector_path_value(query));
            merge_model_value(data_map, model);
            data_map.insert(
                yaml_key("parameters"),
                Value::Sequence(
                    parameters
                        .iter()
                        .map(|item| {
                            let mut map =
                                item.raw_parameter.as_mapping().cloned().unwrap_or_default();
                            set_mapping_string(&mut map, "name", &item.name);
                            set_mapping_string(&mut map, "type", &item.value_type);
                            set_mapping_string(&mut map, "description", &item.description);
                            set_mapping_bool(&mut map, "required", item.required);
                            Value::Mapping(map)
                        })
                        .collect(),
                ),
            );
            set_mapping_string(data_map, "reasoning_mode", reasoning_mode);
            set_mapping_string(data_map, "instruction", &instruction_editor.text());
        }
        ("if-else", WorkflowNodeVisualDraft::IfElse { cases }) => {
            data_map.insert(
                yaml_key("cases"),
                Value::Sequence(
                    cases.iter().map(merge_if_else_case_value).collect::<Result<Vec<_>, _>>()?,
                ),
            );
        }
        _ => {}
    }
    Ok(())
}
