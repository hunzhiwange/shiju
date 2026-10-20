//! 分支类节点编辑器状态的测试，覆盖条件分支与知识检索字段更新。
//! 非匹配草稿用例确保专用 setter 不会误改其他节点类型。
use super::*;

fn open_editor(state: &mut WorkflowState, block_type: &str) {
    state
        .open_create_node_editor(block_type, Point::new(12.0, 34.0))
        .expect("node editor should open");
}

fn editor(state: &WorkflowState) -> &WorkflowNodeEditorDraft {
    state.node_editor.as_ref().expect("node editor should be open")
}

fn visual_draft(state: &WorkflowState) -> &WorkflowNodeVisualDraft {
    editor(state).visual_draft.as_ref().expect("visual draft should exist")
}

fn raw_data(state: &WorkflowState) -> String {
    editor(state).raw_data_editor.text()
}

#[test]
fn if_else_editor_adds_cases_and_updates_conditions() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "if-else");

    state.add_node_editor_if_else_case();
    state.set_node_editor_if_else_case_logical_operator(0, "or".to_string());
    state.set_node_editor_if_else_case_logical_operator(99, "and".to_string());
    state.add_node_editor_if_else_condition(0);
    state.add_node_editor_if_else_condition(99);
    state.set_node_editor_if_else_condition_selector(0, 0, "start.query".to_string());
    state.set_node_editor_if_else_condition_operator(0, 0, "is".to_string());
    state.set_node_editor_if_else_condition_value(0, 0, "ready".to_string());
    state.set_node_editor_if_else_condition_var_type(0, 0, "number".to_string());
    state.set_node_editor_if_else_condition_selector(99, 0, "ignored".to_string());
    state.set_node_editor_if_else_condition_operator(0, 99, "ignored".to_string());
    state.set_node_editor_if_else_condition_value(99, 99, "ignored".to_string());
    state.set_node_editor_if_else_condition_var_type(99, 99, "ignored".to_string());

    let WorkflowNodeVisualDraft::IfElse { cases } = visual_draft(&state) else {
        panic!("expected if-else visual draft");
    };
    assert_eq!(cases.len(), 3);
    assert_eq!(cases[0].logical_operator, "or");
    assert_eq!(cases[0].conditions.len(), 1);
    assert_eq!(cases[0].conditions[0].variable_selector_input, "start.query");
    assert_eq!(cases[0].conditions[0].comparison_operator, "is");
    assert_eq!(cases[0].conditions[0].compare_value, "ready");
    assert_eq!(cases[0].conditions[0].var_type, "number");
    assert_eq!(cases[2].logical_operator, "and");
    assert_eq!(cases[2].conditions.len(), 1);
    assert!(raw_data(&state).contains("start"));
    assert!(raw_data(&state).contains("query"));
}

#[test]
fn if_else_editor_removes_conditions_only_for_valid_indexes() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "if-else");

    state.add_node_editor_if_else_condition(0);
    state.add_node_editor_if_else_condition(0);
    state.remove_node_editor_if_else_condition(0, 1);
    state.remove_node_editor_if_else_condition(0, 99);
    state.remove_node_editor_if_else_condition(99, 0);

    let WorkflowNodeVisualDraft::IfElse { cases } = visual_draft(&state) else {
        panic!("expected if-else visual draft");
    };
    assert_eq!(cases.len(), 2);
    assert_eq!(cases[0].conditions.len(), 1);

    state.remove_node_editor_if_else_condition(0, 0);

    let WorkflowNodeVisualDraft::IfElse { cases } = visual_draft(&state) else {
        panic!("expected if-else visual draft");
    };
    assert!(cases[0].conditions.is_empty());
}

#[test]
fn knowledge_editor_updates_all_visual_fields() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "knowledge-retrieval");

    state.set_node_editor_knowledge_query_selector("start.query".to_string());
    state.set_node_editor_knowledge_query_attachment_selector("start.file".to_string());
    state.set_node_editor_knowledge_dataset_ids("dataset-a, dataset-b".to_string());
    state.set_node_editor_knowledge_retrieval_mode("single".to_string());
    state.set_node_editor_knowledge_top_k("12".to_string());
    state.set_node_editor_knowledge_score_threshold_enabled(true);
    state.set_node_editor_knowledge_score_threshold("0.77".to_string());
    state.set_node_editor_knowledge_reranking_enabled(true);
    state.set_node_editor_knowledge_single_model_provider("openai".to_string());
    state.set_node_editor_knowledge_single_model_name("text-embedding-3-large".to_string());
    state.set_node_editor_knowledge_single_model_mode("embedding".to_string());

    let WorkflowNodeVisualDraft::KnowledgeRetrieval {
        query_selector_input,
        query_attachment_selector_input,
        dataset_ids_input,
        retrieval_mode,
        top_k_input,
        score_threshold_enabled,
        score_threshold_input,
        reranking_enable,
        single_model_provider,
        single_model_name,
        single_model_mode,
        ..
    } = visual_draft(&state)
    else {
        panic!("expected knowledge retrieval visual draft");
    };
    assert_eq!(query_selector_input, "start.query");
    assert_eq!(query_attachment_selector_input, "start.file");
    assert_eq!(dataset_ids_input, "dataset-a, dataset-b");
    assert_eq!(retrieval_mode, "single");
    assert_eq!(top_k_input, "12");
    assert!(*score_threshold_enabled);
    assert_eq!(score_threshold_input, "0.77");
    assert!(*reranking_enable);
    assert_eq!(single_model_provider, "openai");
    assert_eq!(single_model_name, "text-embedding-3-large");
    assert_eq!(single_model_mode, "embedding");
    assert!(raw_data(&state).contains("dataset-a"));
    assert!(raw_data(&state).contains("text-embedding-3-large"));
}

#[test]
fn branch_setters_ignore_non_matching_visual_drafts() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "answer");
    let before = raw_data(&state);

    state.add_node_editor_if_else_case();
    state.set_node_editor_if_else_case_logical_operator(0, "or".to_string());
    state.add_node_editor_if_else_condition(0);
    state.remove_node_editor_if_else_condition(0, 0);
    state.set_node_editor_if_else_condition_selector(0, 0, "ignored.selector".to_string());
    state.set_node_editor_if_else_condition_operator(0, 0, "ignored".to_string());
    state.set_node_editor_if_else_condition_value(0, 0, "ignored".to_string());
    state.set_node_editor_if_else_condition_var_type(0, 0, "ignored".to_string());
    state.set_node_editor_knowledge_query_selector("ignored.query".to_string());
    state.set_node_editor_knowledge_query_attachment_selector("ignored.file".to_string());
    state.set_node_editor_knowledge_dataset_ids("ignored-dataset".to_string());
    state.set_node_editor_knowledge_retrieval_mode("single".to_string());
    state.set_node_editor_knowledge_top_k("99".to_string());
    state.set_node_editor_knowledge_score_threshold_enabled(true);
    state.set_node_editor_knowledge_score_threshold("0.99".to_string());
    state.set_node_editor_knowledge_reranking_enabled(true);
    state.set_node_editor_knowledge_single_model_provider("ignored-provider".to_string());
    state.set_node_editor_knowledge_single_model_name("ignored-model".to_string());
    state.set_node_editor_knowledge_single_model_mode("ignored-mode".to_string());

    assert!(matches!(visual_draft(&state), WorkflowNodeVisualDraft::Answer { .. }));
    assert_eq!(raw_data(&state), before);
}
