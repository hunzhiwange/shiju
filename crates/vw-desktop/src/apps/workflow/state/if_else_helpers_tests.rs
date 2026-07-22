#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("if_else_helpers_tests"));
}

use super::*;
use iced::Point;
use iced::widget::text_editor;

fn mapping_field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value
        .as_mapping()
        .and_then(|map| mapping_value(map, key))
        .unwrap_or_else(|| panic!("expected mapping field {key}"))
}

fn mapping_str<'a>(value: &'a Value, key: &str) -> &'a str {
    mapping_field(value, key).as_str().unwrap_or_else(|| panic!("expected string field {key}"))
}

fn editor_with_visual(visual_draft: Option<WorkflowNodeVisualDraft>) -> WorkflowNodeEditorDraft {
    WorkflowNodeEditorDraft {
        mode: WorkflowNodeEditorMode::Create,
        active_tab: WorkflowNodeEditorTab::Visual,
        block_type: "start".to_string(),
        title: "Start".to_string(),
        description: String::new(),
        description_editor: text_editor::Content::with_text(""),
        position: Point::new(0.0, 0.0),
        visual_draft,
        validation: WorkflowNodeEditorValidation::default(),
        show_raw_data_editor: false,
        raw_data_editor: text_editor::Content::with_text("{}"),
        hovered_start_variable_index: None,
        start_variable_focus_index: None,
        start_variable_editor: None,
    }
}

fn start_variable(label: &str) -> WorkflowStartVariableDraft {
    let mut variable = default_start_variable_draft();
    variable.label = label.to_string();
    variable.variable = label.to_ascii_lowercase();
    variable
}

#[test]
fn default_if_else_condition_draft_matches_raw_condition_value() {
    let draft = default_if_else_condition_draft();

    assert_eq!(draft.variable_selector_input, "");
    assert_eq!(draft.comparison_operator, "contains");
    assert_eq!(draft.compare_value, "");
    assert_eq!(draft.var_type, "string");

    assert_eq!(mapping_str(&draft.raw_condition, "comparison_operator"), "contains");
    assert_eq!(mapping_str(&draft.raw_condition, "value"), "");
    assert_eq!(mapping_str(&draft.raw_condition, "varType"), "string");
    assert_eq!(
        mapping_field(&draft.raw_condition, "variable_selector").as_sequence().unwrap().len(),
        0
    );
    assert!(mapping_str(&draft.raw_condition, "id").starts_with("condition-"));
}

#[test]
fn default_if_else_case_draft_writes_ids_operator_and_initial_condition() {
    let draft = default_if_else_case_draft();

    assert!(draft.case_id.starts_with("case-"));
    assert_eq!(draft.logical_operator, "and");
    assert_eq!(draft.conditions.len(), 1);
    assert_eq!(draft.conditions[0].comparison_operator, "contains");

    assert_eq!(mapping_str(&draft.raw_case, "case_id"), draft.case_id.as_str());
    assert_eq!(mapping_str(&draft.raw_case, "id"), draft.case_id.as_str());
    assert_eq!(mapping_str(&draft.raw_case, "logical_operator"), "and");

    let raw_conditions = mapping_field(&draft.raw_case, "conditions").as_sequence().unwrap();
    assert_eq!(raw_conditions.len(), 1);
    assert_eq!(mapping_str(&raw_conditions[0], "comparison_operator"), "contains");
    assert!(mapping_str(&raw_conditions[0], "id").starts_with("condition-"));
}

#[test]
fn yaml_map_for_state_preserves_entries_and_overwrites_duplicate_keys() {
    let value = yaml_map_for_state(vec![
        ("name", Value::String("old".to_string())),
        ("count", Value::Number(2.into())),
        ("name", Value::String("new".to_string())),
    ]);

    assert_eq!(mapping_str(&value, "name"), "new");
    assert_eq!(mapping_field(&value, "count").as_i64(), Some(2));
    assert_eq!(value.as_mapping().unwrap().len(), 2);
}

#[test]
fn initial_start_variable_focus_only_selects_non_empty_start_visual_draft() {
    assert_eq!(initial_start_variable_focus(None), None);
    assert_eq!(
        initial_start_variable_focus(Some(&WorkflowNodeVisualDraft::Start { variables: vec![] })),
        None
    );

    let start = WorkflowNodeVisualDraft::Start { variables: vec![start_variable("Query")] };
    assert_eq!(initial_start_variable_focus(Some(&start)), Some(0));

    let answer =
        WorkflowNodeVisualDraft::Answer { answer_editor: text_editor::Content::with_text("Done") };
    assert_eq!(initial_start_variable_focus(Some(&answer)), None);
}

#[test]
fn clamp_node_editor_start_variable_focus_clears_or_clamps_by_visual_draft() {
    let mut empty_start =
        editor_with_visual(Some(WorkflowNodeVisualDraft::Start { variables: vec![] }));
    empty_start.start_variable_focus_index = Some(2);
    clamp_node_editor_start_variable_focus(&mut empty_start);
    assert_eq!(empty_start.start_variable_focus_index, None);

    let mut populated_start = editor_with_visual(Some(WorkflowNodeVisualDraft::Start {
        variables: vec![start_variable("Query"), start_variable("Topic")],
    }));
    populated_start.start_variable_focus_index = Some(1);
    clamp_node_editor_start_variable_focus(&mut populated_start);
    assert_eq!(populated_start.start_variable_focus_index, Some(1));

    populated_start.start_variable_focus_index = Some(9);
    clamp_node_editor_start_variable_focus(&mut populated_start);
    assert_eq!(populated_start.start_variable_focus_index, Some(0));

    populated_start.start_variable_focus_index = None;
    clamp_node_editor_start_variable_focus(&mut populated_start);
    assert_eq!(populated_start.start_variable_focus_index, Some(0));

    let mut non_start = editor_with_visual(Some(WorkflowNodeVisualDraft::Answer {
        answer_editor: text_editor::Content::with_text("Done"),
    }));
    non_start.start_variable_focus_index = Some(1);
    clamp_node_editor_start_variable_focus(&mut non_start);
    assert_eq!(non_start.start_variable_focus_index, None);
}

#[test]
fn refresh_node_editor_validation_replaces_existing_validation_errors() {
    let mut editor = editor_with_visual(None);
    editor.block_type = "answer".to_string();
    editor.raw_data_editor = text_editor::Content::with_text("[");

    refresh_node_editor_validation(&mut editor);
    assert!(editor.validation.has_errors());
    assert!(editor.validation.first_error_for("advanced_yaml.raw_data").is_some());

    editor.raw_data_editor = text_editor::Content::with_text("answer: ok\n");
    refresh_node_editor_validation(&mut editor);
    assert!(!editor.validation.has_errors());
}
