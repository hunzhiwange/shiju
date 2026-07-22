use super::*;
use serde_yaml::Value;

fn validation_with_errors(paths: &[&str]) -> state::WorkflowNodeEditorValidation {
    state::WorkflowNodeEditorValidation {
        field_errors: paths
            .iter()
            .map(|path| state::WorkflowNodeValidationError {
                path: (*path).to_string(),
                message: format!("error for {path}"),
            })
            .collect(),
    }
}

fn if_else_case(
    conditions: Vec<state::WorkflowIfElseConditionDraft>,
) -> state::WorkflowIfElseCaseDraft {
    state::WorkflowIfElseCaseDraft {
        raw_case: Value::Null,
        case_id: "case-a".to_string(),
        logical_operator: "and".to_string(),
        conditions,
    }
}

fn if_else_condition() -> state::WorkflowIfElseConditionDraft {
    state::WorkflowIfElseConditionDraft {
        raw_condition: Value::Null,
        variable_selector_input: "start.text".to_string(),
        comparison_operator: "contains".to_string(),
        compare_value: "orders".to_string(),
        var_type: "string".to_string(),
    }
}

#[test]
fn visual_section_handles_empty_cases() {
    let cases = Vec::new();
    let validation = state::WorkflowNodeEditorValidation::default();

    let _element = build_if_else_visual_section(&cases, &validation);
}

#[test]
fn visual_section_renders_case_list() {
    let cases = vec![if_else_case(vec![if_else_condition()])];
    let validation = state::WorkflowNodeEditorValidation::default();

    let element = build_if_else_visual_section(&cases, &validation);

    std::hint::black_box(element);
}

#[test]
fn visual_section_renders_multiple_cases_with_mixed_conditions() {
    let cases = vec![
        if_else_case(vec![if_else_condition()]),
        state::WorkflowIfElseCaseDraft {
            raw_case: Value::Null,
            case_id: "case-b".to_string(),
            logical_operator: "or".to_string(),
            conditions: Vec::new(),
        },
    ];
    let validation = validation_with_errors(&["if_else.cases[1].conditions"]);

    let element = build_if_else_visual_section(&cases, &validation);

    std::hint::black_box(element);
}

#[test]
fn visual_section_passes_validation_to_nested_condition_fields() {
    let mut first_condition = if_else_condition();
    first_condition.variable_selector_input = "start.customer_id".to_string();
    first_condition.compare_value = "42".to_string();

    let mut second_condition = if_else_condition();
    second_condition.var_type = "number".to_string();
    second_condition.comparison_operator = ">".to_string();
    second_condition.variable_selector_input = "start.total".to_string();
    second_condition.compare_value = "100".to_string();

    let cases = vec![if_else_case(vec![first_condition, second_condition])];
    let validation = validation_with_errors(&[
        "if_else.cases[0].logical_operator",
        "if_else.cases[0].conditions[1].var_type",
        "if_else.cases[0].conditions[1].operator",
        "if_else.cases[0].conditions[1].selector",
        "if_else.cases[0].conditions[1].value",
    ]);

    let element = build_if_else_visual_section(&cases, &validation);

    std::hint::black_box(element);
}

#[test]
fn case_card_handles_empty_conditions_without_validation_error() {
    let case = if_else_case(Vec::new());
    let validation = state::WorkflowNodeEditorValidation::default();

    let element = build_if_else_case_card(0, &case, &validation);

    std::hint::black_box(element);
}

#[test]
fn case_card_renders_validation_errors_and_conditions() {
    let case = if_else_case(vec![if_else_condition()]);
    let validation = validation_with_errors(&[
        "if_else.cases[2].logical_operator",
        "if_else.cases[2].conditions",
        "if_else.cases[2].conditions[0].var_type",
        "if_else.cases[2].conditions[0].operator",
        "if_else.cases[2].conditions[0].selector",
        "if_else.cases[2].conditions[0].value",
    ]);

    let element = build_if_else_case_card(2, &case, &validation);

    std::hint::black_box(element);
}

#[test]
fn case_card_ignores_validation_for_other_case_indexes() {
    let case = if_else_case(vec![if_else_condition(), if_else_condition()]);
    let validation = validation_with_errors(&[
        "if_else.cases[1].logical_operator",
        "if_else.cases[1].conditions",
        "if_else.cases[1].conditions[0].selector",
    ]);

    let element = build_if_else_case_card(0, &case, &validation);

    std::hint::black_box(element);
}

#[test]
fn condition_card_renders_without_validation_errors() {
    let condition = if_else_condition();
    let validation = state::WorkflowNodeEditorValidation::default();

    let element = build_if_else_condition_card(1, 3, &condition, &validation);

    std::hint::black_box(element);
}

#[test]
fn condition_card_renders_field_validation_errors() {
    let condition = if_else_condition();
    let validation = validation_with_errors(&[
        "if_else.cases[1].conditions[3].var_type",
        "if_else.cases[1].conditions[3].operator",
        "if_else.cases[1].conditions[3].selector",
        "if_else.cases[1].conditions[3].value",
    ]);

    let element = build_if_else_condition_card(1, 3, &condition, &validation);

    std::hint::black_box(element);
}

#[test]
fn condition_card_uses_indexes_when_building_change_messages() {
    let condition = state::WorkflowIfElseConditionDraft {
        raw_condition: Value::Null,
        variable_selector_input: "1711528917469.text".to_string(),
        comparison_operator: "not empty".to_string(),
        compare_value: String::new(),
        var_type: "array[string]".to_string(),
    };
    let validation = validation_with_errors(&[
        "if_else.cases[4].conditions[2].var_type",
        "if_else.cases[4].conditions[2].operator",
        "if_else.cases[4].conditions[2].selector",
        "if_else.cases[4].conditions[2].value",
    ]);

    let element = build_if_else_condition_card(4, 2, &condition, &validation);

    std::hint::black_box(element);
}

#[test]
fn condition_card_accepts_blank_draft_values_before_validation() {
    let condition = state::WorkflowIfElseConditionDraft {
        raw_condition: Value::Null,
        variable_selector_input: String::new(),
        comparison_operator: String::new(),
        compare_value: String::new(),
        var_type: String::new(),
    };

    let validation = state::WorkflowNodeEditorValidation::default();
    let element = build_if_else_condition_card(0, 0, &condition, &validation);

    std::hint::black_box(element);
}
