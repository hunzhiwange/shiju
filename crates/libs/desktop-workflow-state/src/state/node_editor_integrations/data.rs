//! Data node field editing.

use super::*;

impl WorkflowState {
    pub fn edit_node_editor_data_node(&mut self, edit: WorkflowDataNodeEdit) {
        self.update_node_visual_draft(|draft| apply_data_node_edit(draft, edit));
    }

}

fn parse_data_value(input: &str) -> Value {
    serde_yaml::from_str(input).unwrap_or_else(|_| Value::String(input.to_string()))
}

fn apply_data_node_edit(draft: &mut WorkflowNodeVisualDraft, edit: WorkflowDataNodeEdit) -> bool {
    match (draft, edit) {
        (
            WorkflowNodeVisualDraft::VariableAssigner { items },
            WorkflowDataNodeEdit::AssignmentAdd,
        ) => {
            items.push(WorkflowAssignmentDraft {
                raw_item: Value::Mapping(Mapping::new()),
                variable_selector: WorkflowVariableSelectorDraft::default(),
                variable_selector_input: String::new(),
                operation: "over-write".into(),
                input_type: "constant".into(),
                value: Value::Null,
                value_input: String::new(),
            });
        }
        (
            WorkflowNodeVisualDraft::VariableAssigner { items },
            WorkflowDataNodeEdit::AssignmentRemove(index),
        ) => {
            if index < items.len() {
                items.remove(index);
            }
        }
        (
            WorkflowNodeVisualDraft::VariableAssigner { items },
            WorkflowDataNodeEdit::AssignmentTarget(index, value),
        ) => {
            if let Some(item) = items.get_mut(index) {
                item.variable_selector = WorkflowVariableSelectorDraft::from_input(&value);
                item.variable_selector_input = value;
            }
        }
        (
            WorkflowNodeVisualDraft::VariableAssigner { items },
            WorkflowDataNodeEdit::AssignmentOperation(index, value),
        ) => {
            if let Some(item) = items.get_mut(index) {
                item.operation = value;
            }
        }
        (
            WorkflowNodeVisualDraft::VariableAssigner { items },
            WorkflowDataNodeEdit::AssignmentInputType(index, value),
        ) => {
            if let Some(item) = items.get_mut(index) {
                item.input_type = value;
            }
        }
        (
            WorkflowNodeVisualDraft::VariableAssigner { items },
            WorkflowDataNodeEdit::AssignmentValue(index, value),
        ) => {
            if let Some(item) = items.get_mut(index) {
                item.value = parse_data_value(&value);
                item.value_input = value;
            }
        }
        (
            WorkflowNodeVisualDraft::VariableAggregator { variables, .. },
            WorkflowDataNodeEdit::AggregatorAdd,
        ) => variables.push(String::new()),
        (
            WorkflowNodeVisualDraft::VariableAggregator { variables, .. },
            WorkflowDataNodeEdit::AggregatorRemove(index),
        ) => {
            if index < variables.len() {
                variables.remove(index);
            }
        }
        (
            WorkflowNodeVisualDraft::VariableAggregator { variables, .. },
            WorkflowDataNodeEdit::AggregatorVariable(index, value),
        ) => {
            if let Some(item) = variables.get_mut(index) {
                *item = value;
            }
        }
        (
            WorkflowNodeVisualDraft::VariableAggregator { output_type, .. },
            WorkflowDataNodeEdit::AggregatorOutputType(value),
        ) => *output_type = value,
        (
            WorkflowNodeVisualDraft::ListOperator { variable, .. },
            WorkflowDataNodeEdit::ListVariable(value),
        ) => *variable = value,
        (
            WorkflowNodeVisualDraft::ListOperator { filter_enabled, .. },
            WorkflowDataNodeEdit::ListFilterEnabled(value),
        ) => *filter_enabled = value,
        (
            WorkflowNodeVisualDraft::ListOperator { conditions, .. },
            WorkflowDataNodeEdit::ListConditionAdd,
        ) => conditions.push(WorkflowListConditionDraft {
            raw_condition: Value::Mapping(Mapping::new()),
            key: String::new(),
            comparison_operator: "is".into(),
            value: Value::Null,
            value_input: String::new(),
        }),
        (
            WorkflowNodeVisualDraft::ListOperator { conditions, .. },
            WorkflowDataNodeEdit::ListConditionRemove(index),
        ) => {
            if index < conditions.len() {
                conditions.remove(index);
            }
        }
        (
            WorkflowNodeVisualDraft::ListOperator { conditions, .. },
            WorkflowDataNodeEdit::ListConditionKey(index, value),
        ) => {
            if let Some(item) = conditions.get_mut(index) {
                item.key = value;
            }
        }
        (
            WorkflowNodeVisualDraft::ListOperator { conditions, .. },
            WorkflowDataNodeEdit::ListConditionOperator(index, value),
        ) => {
            if let Some(item) = conditions.get_mut(index) {
                item.comparison_operator = value;
            }
        }
        (
            WorkflowNodeVisualDraft::ListOperator { conditions, .. },
            WorkflowDataNodeEdit::ListConditionValue(index, value),
        ) => {
            if let Some(item) = conditions.get_mut(index) {
                item.value = parse_data_value(&value);
                item.value_input = value;
            }
        }
        (
            WorkflowNodeVisualDraft::ListOperator { order_enabled, .. },
            WorkflowDataNodeEdit::ListOrderEnabled(value),
        ) => *order_enabled = value,
        (
            WorkflowNodeVisualDraft::ListOperator { order_key, .. },
            WorkflowDataNodeEdit::ListOrderKey(value),
        ) => *order_key = value,
        (
            WorkflowNodeVisualDraft::ListOperator { order_direction, .. },
            WorkflowDataNodeEdit::ListOrderDirection(value),
        ) => *order_direction = value,
        (
            WorkflowNodeVisualDraft::ListOperator { limit_enabled, .. },
            WorkflowDataNodeEdit::ListLimitEnabled(value),
        ) => *limit_enabled = value,
        (
            WorkflowNodeVisualDraft::ListOperator { limit_size, .. },
            WorkflowDataNodeEdit::ListLimitSize(value),
        ) => *limit_size = value,
        (
            WorkflowNodeVisualDraft::Iteration { iterator_selector, .. },
            WorkflowDataNodeEdit::IterationIterator(value),
        ) => *iterator_selector = value,
        (
            WorkflowNodeVisualDraft::Iteration { output_selector, .. },
            WorkflowDataNodeEdit::IterationOutput(value),
        ) => *output_selector = value,
        (
            WorkflowNodeVisualDraft::Iteration { output_type, .. },
            WorkflowDataNodeEdit::IterationOutputType(value),
        ) => *output_type = value,
        (
            WorkflowNodeVisualDraft::Iteration { is_parallel, .. },
            WorkflowDataNodeEdit::IterationParallel(value),
        ) => *is_parallel = value,
        (
            WorkflowNodeVisualDraft::Iteration { parallel_nums, .. },
            WorkflowDataNodeEdit::IterationParallelNums(value),
        ) => *parallel_nums = value,
        (
            WorkflowNodeVisualDraft::Iteration { error_handle_mode, .. },
            WorkflowDataNodeEdit::IterationErrorMode(value),
        ) => *error_handle_mode = value,
        (
            WorkflowNodeVisualDraft::Loop { variables, .. },
            WorkflowDataNodeEdit::LoopVariableAdd,
        ) => variables.push(WorkflowLoopVariableDraft {
            raw_variable: Value::Mapping(Mapping::new()),
            id: String::new(),
            label: String::new(),
            value_type: "string".into(),
            initial_value: Value::Null,
            initial_value_input: String::new(),
        }),
        (
            WorkflowNodeVisualDraft::Loop { variables, .. },
            WorkflowDataNodeEdit::LoopVariableRemove(index),
        ) => {
            if index < variables.len() {
                variables.remove(index);
            }
        }
        (
            WorkflowNodeVisualDraft::Loop { variables, .. },
            WorkflowDataNodeEdit::LoopVariableId(index, value),
        ) => {
            if let Some(item) = variables.get_mut(index) {
                item.id = value;
            }
        }
        (
            WorkflowNodeVisualDraft::Loop { variables, .. },
            WorkflowDataNodeEdit::LoopVariableLabel(index, value),
        ) => {
            if let Some(item) = variables.get_mut(index) {
                item.label = value;
            }
        }
        (
            WorkflowNodeVisualDraft::Loop { variables, .. },
            WorkflowDataNodeEdit::LoopVariableType(index, value),
        ) => {
            if let Some(item) = variables.get_mut(index) {
                item.value_type = value;
            }
        }
        (
            WorkflowNodeVisualDraft::Loop { variables, .. },
            WorkflowDataNodeEdit::LoopVariableValue(index, value),
        ) => {
            if let Some(item) = variables.get_mut(index) {
                item.initial_value = parse_data_value(&value);
                item.initial_value_input = value;
            }
        }
        (
            WorkflowNodeVisualDraft::Loop { break_conditions, .. },
            WorkflowDataNodeEdit::LoopConditionAdd,
        ) => break_conditions.push(WorkflowLoopConditionDraft {
            raw_condition: Value::Mapping(Mapping::new()),
            selector: String::new(),
            comparison_operator: "is".into(),
            value: Value::Null,
            value_input: String::new(),
        }),
        (
            WorkflowNodeVisualDraft::Loop { break_conditions, .. },
            WorkflowDataNodeEdit::LoopConditionRemove(index),
        ) => {
            if index < break_conditions.len() {
                break_conditions.remove(index);
            }
        }
        (
            WorkflowNodeVisualDraft::Loop { break_conditions, .. },
            WorkflowDataNodeEdit::LoopConditionSelector(index, value),
        ) => {
            if let Some(item) = break_conditions.get_mut(index) {
                item.selector = value;
            }
        }
        (
            WorkflowNodeVisualDraft::Loop { break_conditions, .. },
            WorkflowDataNodeEdit::LoopConditionOperator(index, value),
        ) => {
            if let Some(item) = break_conditions.get_mut(index) {
                item.comparison_operator = value;
            }
        }
        (
            WorkflowNodeVisualDraft::Loop { break_conditions, .. },
            WorkflowDataNodeEdit::LoopConditionValue(index, value),
        ) => {
            if let Some(item) = break_conditions.get_mut(index) {
                item.value = parse_data_value(&value);
                item.value_input = value;
            }
        }
        (
            WorkflowNodeVisualDraft::Loop { logical_operator, .. },
            WorkflowDataNodeEdit::LoopLogicalOperator(value),
        ) => *logical_operator = value,
        (
            WorkflowNodeVisualDraft::Loop { max_loop_count, .. },
            WorkflowDataNodeEdit::LoopMaxCount(value),
        ) => *max_loop_count = value,
        _ => return false,
    }
    true
}
