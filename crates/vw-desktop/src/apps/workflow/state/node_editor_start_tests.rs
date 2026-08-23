//! 开始节点编辑器状态的测试，覆盖输入变量创建、编辑、删除和提交。
//! 用例验证校验时机、焦点移动及缺失编辑器时的安全空操作。
use super::*;
use iced::widget::text_editor::{Action, Edit};

fn state_with_start_editor() -> WorkflowState {
    let mut state = WorkflowState::default();
    state.open_create_node_editor("start", Point::new(10.0, 20.0)).unwrap();
    state
}

fn state_with_node(block_type: &str) -> WorkflowState {
    let mut state = WorkflowState::default();
    state.insert_node_immediately(block_type, Point::new(100.0, 120.0)).unwrap();
    state
}

fn editor(state: &WorkflowState) -> &WorkflowNodeEditorDraft {
    state.node_editor.as_ref().unwrap()
}

fn editor_mut(state: &mut WorkflowState) -> &mut WorkflowNodeEditorDraft {
    state.node_editor.as_mut().unwrap()
}

fn start_variables(state: &WorkflowState) -> &[WorkflowStartVariableDraft] {
    match editor(state).visual_draft.as_ref().unwrap() {
        WorkflowNodeVisualDraft::Start { variables } => variables,
        _ => panic!("expected start visual draft"),
    }
}

fn start_variable_editor(state: &WorkflowState) -> &WorkflowStartVariableEditorDraft {
    editor(state).start_variable_editor.as_ref().unwrap()
}

fn fill_start_variable_editor(state: &mut WorkflowState, label: &str, name: &str) {
    state.open_node_editor_start_variable_create();
    state.set_node_editor_start_variable_editor_label(label.to_string());
    state.set_node_editor_start_variable_editor_name(name.to_string());
}

fn submit_text_variable(state: &mut WorkflowState, label: &str, name: &str) {
    fill_start_variable_editor(state, label, name);
    state.submit_node_editor_start_variable_editor().unwrap();
}

fn action_insert(text: &str) -> Action {
    Action::Edit(Edit::Paste(text.to_string().into()))
}

#[test]
fn empty_state_methods_are_noops_or_expected_errors() {
    let mut state = WorkflowState::default();

    state.set_node_editor_start_variable_hovered(Some(1));
    state.set_node_editor_title("ignored".to_string());
    state.set_node_editor_description("ignored".to_string());
    state.node_editor_description_action(action_insert("ignored"));
    state.node_editor_action(action_insert("ignored"));
    state.set_node_editor_show_raw_data_editor(true);
    state.add_node_editor_start_variable();
    state.open_node_editor_start_variable_edit(0);
    state.close_node_editor_start_variable_editor();
    state.set_node_editor_start_variable_editor_label("ignored".to_string());
    state.set_node_editor_start_variable_editor_name("ignored".to_string());
    state.set_node_editor_start_variable_editor_type("number".to_string());
    state.set_node_editor_start_variable_editor_required(true);
    state.set_node_editor_start_variable_editor_hidden(true);
    state.set_node_editor_start_variable_editor_default("1".to_string());
    state.node_editor_start_variable_editor_default_action(action_insert("2"));
    state.set_node_editor_start_variable_editor_max_length("3".to_string());
    state.add_node_editor_start_variable_editor_option();
    state.remove_node_editor_start_variable_editor_option(0);
    state.set_node_editor_start_variable_editor_option(0, "ignored".to_string());
    state.toggle_node_editor_start_variable_editor_file_type("image".to_string());
    state.set_node_editor_start_variable_editor_extensions("md".to_string());
    state.set_node_editor_start_variable_editor_upload_method("local_file".to_string());
    state.open_node_editor_start_variable_editor_default_file_url_input();
    state.close_node_editor_start_variable_editor_default_file_url_input();
    state.set_node_editor_start_variable_editor_default_file_url_input("url".to_string());
    assert_eq!(state.submit_node_editor_start_variable_editor_default_file_url(), Ok(()));
    assert_eq!(
        state.set_node_editor_start_variable_editor_default_file_path("path".to_string()),
        Ok(())
    );
    state.remove_node_editor_start_variable_editor_default_file(0);
    assert_eq!(state.submit_node_editor_start_variable_editor(), Ok(()));
    state.remove_node_editor_start_variable(0);
    state.set_node_editor_start_variable_focus(0);
    state.set_node_editor_start_variable_label(0, "ignored".to_string());
    state.set_node_editor_start_variable_name(0, "ignored".to_string());
    state.set_node_editor_start_variable_type(0, "ignored".to_string());
    state.set_node_editor_start_variable_required(0, true);
    state.set_node_editor_start_variable_default(0, "ignored".to_string());
    state.set_node_editor_start_variable_placeholder(0, "ignored".to_string());
    state.set_node_editor_start_variable_hint(0, "ignored".to_string());
    state.set_node_editor_start_variable_max_length(0, "ignored".to_string());

    assert!(state.node_editor.is_none());
    assert_eq!(state.delete_edge_by_id("missing"), false);
    assert_eq!(state.delete_node_by_id("missing"), false);
    assert_eq!(state.focus_node("missing", (800.0, 600.0)), Err("目标节点不存在".to_string()));
}

#[test]
fn title_description_raw_editor_and_visibility_defer_validation_until_submit() {
    let mut state = state_with_start_editor();

    state.set_node_editor_title(String::new());
    assert_eq!(editor(&state).validation.first_error_for("title"), None);

    state.set_node_editor_title("Start here".to_string());
    state.set_node_editor_description("first".to_string());
    state.node_editor_description_action(action_insert(" line"));
    assert_eq!(editor(&state).title, "Start here");
    assert_eq!(editor(&state).description, " linefirst");
    assert!(editor(&state).validation.first_error_for("title").is_none());

    state.set_node_editor_show_raw_data_editor(true);
    assert!(editor(&state).show_raw_data_editor);
    state.node_editor_action(action_insert("\nnot: [valid"));
    assert!(!editor(&state).validation.has_errors());

    assert_eq!(
        state.submit_node_editor(),
        Err("请先修正节点表单中的错误字段，再保存。".to_string())
    );
    assert!(editor(&state).validation.has_errors());
}

#[test]
fn create_edit_close_and_submit_text_start_variable() {
    let mut state = state_with_start_editor();

    state.add_node_editor_start_variable();
    assert_eq!(start_variable_editor(&state).mode, WorkflowStartVariableEditorMode::Create);
    state.set_node_editor_start_variable_editor_label("User Name".to_string());
    state.set_node_editor_start_variable_editor_name("user_name".to_string());
    state.set_node_editor_start_variable_editor_default("Ada".to_string());
    state.node_editor_start_variable_editor_default_action(action_insert(" Lovelace"));
    state.set_node_editor_start_variable_editor_max_length("80".to_string());
    state.set_node_editor_start_variable_editor_required(true);
    assert!(!start_variable_editor(&state).variable.hidden);

    state.submit_node_editor_start_variable_editor().unwrap();
    assert_eq!(start_variables(&state).len(), 3);
    assert_eq!(start_variables(&state)[0].default_value, "");
    assert_eq!(editor(&state).start_variable_focus_index, Some(2));
    assert!(editor(&state).start_variable_editor.is_none());

    state.open_node_editor_start_variable_edit(0);
    assert_eq!(editor(&state).hovered_start_variable_index, Some(0));
    assert_eq!(start_variable_editor(&state).mode, WorkflowStartVariableEditorMode::Edit(0));
    state.set_node_editor_start_variable_editor_hidden(true);
    assert!(!start_variable_editor(&state).variable.required);
    state.set_node_editor_start_variable_editor_label("Hidden User".to_string());
    state.submit_node_editor_start_variable_editor().unwrap();
    assert_eq!(start_variables(&state)[0].label, "Hidden User");
    assert!(start_variables(&state)[0].hidden);

    state.open_node_editor_start_variable_edit(0);
    state.close_node_editor_start_variable_editor();
    assert!(editor(&state).start_variable_editor.is_none());
}

#[test]
fn open_start_variable_edit_ignores_missing_index_without_replacing_current_editor() {
    let mut state = state_with_start_editor();
    state.open_node_editor_start_variable_create();
    state.set_node_editor_start_variable_editor_label("Draft".to_string());
    let hovered = editor(&state).hovered_start_variable_index;
    let focused = editor(&state).start_variable_focus_index;

    state.open_node_editor_start_variable_edit(99);

    assert_eq!(start_variable_editor(&state).mode, WorkflowStartVariableEditorMode::Create);
    assert_eq!(start_variable_editor(&state).variable.label, "Draft");
    assert_eq!(editor(&state).hovered_start_variable_index, hovered);
    assert_eq!(editor(&state).start_variable_focus_index, focused);
}

#[test]
fn submit_start_variable_reports_validation_errors_and_restores_editor() {
    let mut state = state_with_start_editor();

    state.open_node_editor_start_variable_create();
    assert_eq!(state.submit_node_editor_start_variable_editor(), Ok(()));
    assert!(editor(&state).start_variable_editor.is_none());

    state.set_node_editor_start_variable_editor_label("Bad".to_string());
    state.set_node_editor_start_variable_editor_name("1bad".to_string());
    assert_eq!(state.submit_node_editor_start_variable_editor(), Ok(()));

    state.set_node_editor_start_variable_editor_name("amount".to_string());
    state.set_node_editor_start_variable_editor_type("number".to_string());
    state.set_node_editor_start_variable_editor_default("NaN text".to_string());
    assert_eq!(state.submit_node_editor_start_variable_editor(), Ok(()));
}

#[test]
fn duplicate_select_options_and_edit_missing_index_are_rejected() {
    let mut state = state_with_start_editor();
    submit_text_variable(&mut state, "First", "first");

    fill_start_variable_editor(&mut state, "Duplicate", "first");
    assert_eq!(state.submit_node_editor_start_variable_editor(), Err("变量名不能重复".to_string()));

    state.set_node_editor_start_variable_editor_name("choice".to_string());
    state.set_node_editor_start_variable_editor_type("select".to_string());
    state.add_node_editor_start_variable_editor_option();
    state.set_node_editor_start_variable_editor_option(0, "alpha".to_string());
    state.add_node_editor_start_variable_editor_option();
    state.set_node_editor_start_variable_editor_option(1, "alpha".to_string());
    assert_eq!(
        state.submit_node_editor_start_variable_editor(),
        Err("下拉选项不能重复".to_string())
    );

    state.set_node_editor_start_variable_editor_option(1, "beta".to_string());
    state.remove_node_editor_start_variable_editor_option(2);
    state.remove_node_editor_start_variable_editor_option(1);
    state.set_node_editor_start_variable_editor_default("alpha".to_string());
    state.submit_node_editor_start_variable_editor().unwrap();
    assert_eq!(start_variables(&state).len(), 4);

    editor_mut(&mut state).start_variable_editor = Some(build_start_variable_editor_draft(
        WorkflowStartVariableEditorMode::Edit(99),
        start_variables(&state)[0].clone(),
    ));
    assert_eq!(state.submit_node_editor_start_variable_editor(), Err("变量名不能重复".to_string()));
}

#[test]
fn file_variable_default_url_and_type_controls_are_normalized() {
    let mut state = state_with_start_editor();

    fill_start_variable_editor(&mut state, "Upload", "upload");
    state.set_node_editor_start_variable_editor_type("file".to_string());
    state.toggle_node_editor_start_variable_editor_file_type("custom".to_string());
    state.set_node_editor_start_variable_editor_extensions(" PDF, .md txt ".to_string());
    state.set_node_editor_start_variable_editor_upload_method("remote_url".to_string());
    state.open_node_editor_start_variable_editor_default_file_url_input();
    assert!(start_variable_editor(&state).show_default_file_url_input);
    state.set_node_editor_start_variable_editor_default_file_url_input(
        " https://example.test/a.pdf ".to_string(),
    );
    state.submit_node_editor_start_variable_editor_default_file_url().unwrap();
    assert_eq!(
        start_variable_editor(&state).variable.default_file_values,
        vec!["https://example.test/a.pdf"]
    );
    assert!(!start_variable_editor(&state).show_default_file_url_input);

    state.open_node_editor_start_variable_editor_default_file_url_input();
    state.close_node_editor_start_variable_editor_default_file_url_input();
    assert!(!start_variable_editor(&state).show_default_file_url_input);
    state.toggle_node_editor_start_variable_editor_file_type("custom".to_string());
    assert!(start_variable_editor(&state).variable.allowed_file_extensions.is_empty());
    state.toggle_node_editor_start_variable_editor_file_type("image".to_string());
    state.toggle_node_editor_start_variable_editor_file_type("image".to_string());
    state.toggle_node_editor_start_variable_editor_file_type("document".to_string());
    state.set_node_editor_start_variable_editor_upload_method("local_file".to_string());
    state.set_node_editor_start_variable_editor_upload_method("both".to_string());
    state.submit_node_editor_start_variable_editor().unwrap();

    assert_eq!(start_variables(&state)[0].input_type, "paragraph");
    assert_eq!(start_variables(&state)[0].default_value, "");
}

#[test]
fn removing_single_file_default_clears_url_input() {
    let mut state = state_with_start_editor();
    fill_start_variable_editor(&mut state, "Upload", "upload");
    state.set_node_editor_start_variable_editor_type("file".to_string());
    state
        .set_node_editor_start_variable_editor_default_file_path(
            "https://example.test/file.txt".to_string(),
        )
        .expect("single file default should be accepted");

    state.remove_node_editor_start_variable_editor_default_file(0);

    let editor = start_variable_editor(&state);
    assert!(editor.variable.default_file_values.is_empty());
    assert!(editor.variable.default_value.is_empty());
    assert!(editor.default_file_url_input.is_empty());
}

#[test]
fn default_file_append_ignores_blank_and_updates_plain_text_variables() {
    let mut state = state_with_start_editor();

    fill_start_variable_editor(&mut state, "Topic", "topic");
    state.set_node_editor_start_variable_editor_default("before".to_string());
    state
        .set_node_editor_start_variable_editor_default_file_path("   ".to_string())
        .expect("blank value should be ignored");
    assert_eq!(start_variable_editor(&state).variable.default_value, "before");

    state
        .set_node_editor_start_variable_editor_default_file_path(" after ".to_string())
        .expect("plain text append path should update default value");
    assert_eq!(start_variable_editor(&state).variable.default_value, "after");
}

#[test]
fn file_list_default_files_respect_max_count_and_removal() {
    let mut state = state_with_start_editor();

    fill_start_variable_editor(&mut state, "Attachments", "attachments");
    state.set_node_editor_start_variable_editor_type("file-list".to_string());
    state.set_node_editor_start_variable_editor_max_length("2".to_string());
    state
        .set_node_editor_start_variable_editor_default_file_path(" /tmp/a.txt ".to_string())
        .unwrap();
    state.open_node_editor_start_variable_editor_default_file_url_input();
    state.set_node_editor_start_variable_editor_default_file_url_input("/tmp/b.txt".to_string());
    state.submit_node_editor_start_variable_editor_default_file_url().unwrap();
    assert_eq!(
        state.set_node_editor_start_variable_editor_default_file_path("/tmp/c.txt".to_string()),
        Err("默认文件最多只能添加 2 个".to_string())
    );

    state.remove_node_editor_start_variable_editor_default_file(9);
    state.remove_node_editor_start_variable_editor_default_file(0);
    assert_eq!(start_variable_editor(&state).variable.default_file_values, vec!["/tmp/b.txt"]);
    state.submit_node_editor_start_variable_editor().unwrap();

    assert_eq!(start_variables(&state)[0].default_file_values, Vec::<String>::new());
}

#[test]
fn start_variable_create_and_edit_ignore_non_start_visual_drafts() {
    let mut state = WorkflowState::default();
    state.open_create_node_editor("answer", Point::new(0.0, 0.0)).unwrap();

    state.open_node_editor_start_variable_create();
    state.open_node_editor_start_variable_edit(0);

    assert!(editor(&state).start_variable_editor.is_none());
}

#[test]
fn submit_variable_on_non_start_editor_is_rejected_and_editor_is_restored() {
    let mut state = WorkflowState::default();
    state.open_create_node_editor("answer", Point::new(0.0, 0.0)).unwrap();
    editor_mut(&mut state).start_variable_editor = Some(build_start_variable_editor_draft(
        WorkflowStartVariableEditorMode::Create,
        default_start_variable_draft(),
    ));

    assert_eq!(
        state.submit_node_editor_start_variable_editor(),
        Err("当前节点不支持开始变量编辑".to_string())
    );
    assert!(editor(&state).start_variable_editor.is_some());
}

#[test]
fn inline_start_variable_updates_sync_raw_yaml_and_ignore_bad_indexes() {
    let mut state = state_with_start_editor();
    submit_text_variable(&mut state, "First", "first");

    state.set_node_editor_start_variable_focus(3);
    assert_eq!(editor(&state).start_variable_focus_index, Some(2));

    state.set_node_editor_start_variable_label(0, "Label".to_string());
    state.set_node_editor_start_variable_name(0, "renamed".to_string());
    state.set_node_editor_start_variable_type(0, "paragraph".to_string());
    state.set_node_editor_start_variable_required(0, true);
    state.set_node_editor_start_variable_default(0, "Default".to_string());
    state.set_node_editor_start_variable_placeholder(0, "Placeholder".to_string());
    state.set_node_editor_start_variable_hint(0, "Hint".to_string());
    state.set_node_editor_start_variable_max_length(0, "120".to_string());
    state.set_node_editor_start_variable_label(9, "ignored".to_string());
    state.set_node_editor_start_variable_name(9, "ignored".to_string());
    state.set_node_editor_start_variable_type(9, "ignored".to_string());
    state.set_node_editor_start_variable_required(9, false);
    state.set_node_editor_start_variable_default(9, "ignored".to_string());
    state.set_node_editor_start_variable_placeholder(9, "ignored".to_string());
    state.set_node_editor_start_variable_hint(9, "ignored".to_string());
    state.set_node_editor_start_variable_max_length(9, "ignored".to_string());

    let variable = &start_variables(&state)[0];
    assert_eq!(variable.label, "Label");
    assert_eq!(variable.variable, "renamed");
    assert_eq!(variable.input_type, "paragraph");
    assert!(editor(&state).raw_data_editor.text().contains("renamed"));
}

#[test]
fn removing_start_variables_clamps_focus() {
    let mut state = state_with_start_editor();
    submit_text_variable(&mut state, "One", "one");
    submit_text_variable(&mut state, "Two", "two");
    submit_text_variable(&mut state, "Three", "three");

    state.set_node_editor_start_variable_focus(2);
    state.remove_node_editor_start_variable(1);
    assert_eq!(editor(&state).start_variable_focus_index, Some(1));
    state.remove_node_editor_start_variable(1);
    assert_eq!(editor(&state).start_variable_focus_index, Some(1));
    state.remove_node_editor_start_variable(0);
    assert_eq!(editor(&state).start_variable_focus_index, Some(0));
    state.remove_node_editor_start_variable(0);
    assert_eq!(start_variables(&state).len(), 1);
}

#[test]
fn focus_node_selects_existing_node_and_resets_transient_state() {
    let mut state = state_with_node("answer");
    let node_id = state.document.nodes[0].id.clone();
    state.node_editor = Some({
        state.open_create_node_editor("start", Point::new(0.0, 0.0)).unwrap();
        state.node_editor.take().unwrap()
    });
    state.selected_edge_id = Some("edge".to_string());
    state.connection_draft = Some(WorkflowConnectionDraft {
        from: WorkflowConnectionEndpoint {
            node_id: node_id.clone(),
            handle_id: "source".to_string(),
            kind: WorkflowHandleKind::Source,
        },
        cursor_world: Point::new(1.0, 1.0),
    });

    state.focus_node(&node_id, (900.0, 700.0)).unwrap();

    assert_eq!(state.selected_node_id, Some(node_id));
    assert_eq!(state.selected_edge_id, None);
    assert!(state.node_editor.is_none());
    assert!(state.connection_draft.is_none());
    assert!(state.status_message.as_ref().unwrap().starts_with("已定位到节点"));
}

#[test]
fn delete_node_and_edge_by_id_select_the_target_before_delete() {
    let mut state = WorkflowState::default();
    state.insert_node_immediately("start", Point::new(0.0, 0.0)).unwrap();
    let start_id = state.document.nodes[0].id.clone();
    state.insert_downstream_node(&start_id, "answer").unwrap();
    let answer_id =
        state.document.nodes.iter().find(|node| node.id != start_id).unwrap().id.clone();
    let edge_id = state.document.edges[0].id.clone();

    assert!(state.delete_edge_by_id(&edge_id));
    assert!(state.document.edges.is_empty());
    assert_eq!(state.selected_edge_id, None);

    assert!(state.delete_node_by_id(&answer_id));
    assert!(state.document.node(&answer_id).is_none());
    assert_eq!(state.selected_node_id, None);
}

#[test]
fn editor_field_setters_keep_dependent_flags_and_buffers_in_sync() {
    let mut state = state_with_start_editor();

    fill_start_variable_editor(&mut state, "Consent", "consent");
    state.set_node_editor_start_variable_editor_required(true);
    assert!(start_variable_editor(&state).variable.required);
    assert!(!start_variable_editor(&state).variable.hidden);

    state.set_node_editor_start_variable_editor_hidden(true);
    assert!(start_variable_editor(&state).variable.hidden);
    assert!(!start_variable_editor(&state).variable.required);

    state.set_node_editor_start_variable_editor_default("yes".to_string());
    assert_eq!(start_variable_editor(&state).default_value_editor.text(), "yes");
    assert_eq!(start_variable_editor(&state).default_file_url_input, "yes");

    state.set_node_editor_start_variable_editor_type("checkbox".to_string());
    assert_eq!(start_variable_editor(&state).variable.input_type, "checkbox");
    assert_eq!(start_variable_editor(&state).default_value_editor.text(), "yes");
    assert!(!start_variable_editor(&state).show_default_file_url_input);

    state.open_node_editor_start_variable_editor_default_file_url_input();
    state.set_node_editor_start_variable_editor_upload_method("local_file".to_string());
    assert!(!start_variable_editor(&state).show_default_file_url_input);

    state.set_node_editor_start_variable_editor_upload_method("remote_url".to_string());
    assert_eq!(
        start_variable_editor(&state).variable.allowed_file_upload_methods,
        vec!["remote_url"]
    );
    state.set_node_editor_start_variable_editor_upload_method("unexpected".to_string());
    assert!(
        start_variable_editor(&state)
            .variable
            .allowed_file_upload_methods
            .iter()
            .any(|method| method == "remote_url")
    );
}

#[test]
fn submit_errors_restore_editor_for_merge_failure_and_missing_edit_target() {
    let mut state = state_with_start_editor();

    fill_start_variable_editor(&mut state, "Amount", "amount");
    state.set_node_editor_start_variable_editor_type("number".to_string());
    state.set_node_editor_start_variable_editor_default("not a number".to_string());
    assert_eq!(
        state.submit_node_editor_start_variable_editor(),
        Err("数字类型默认值必须是数字".to_string())
    );
    assert_eq!(start_variable_editor(&state).mode, WorkflowStartVariableEditorMode::Create);
    assert_eq!(start_variable_editor(&state).variable.default_value, "not a number");

    let mut missing_target = start_variables(&state)[0].clone();
    missing_target.variable = "missing_target".to_string();
    missing_target.label = "Missing target".to_string();
    editor_mut(&mut state).start_variable_editor = Some(build_start_variable_editor_draft(
        WorkflowStartVariableEditorMode::Edit(99),
        missing_target,
    ));

    assert_eq!(state.submit_node_editor_start_variable_editor(), Err("目标变量不存在".to_string()));
    assert_eq!(start_variable_editor(&state).mode, WorkflowStartVariableEditorMode::Edit(99));
}

#[test]
fn raw_editor_valid_yaml_rebuilds_visual_draft_and_clamps_start_focus() {
    let mut state = state_with_start_editor();
    submit_text_variable(&mut state, "Extra", "extra");
    editor_mut(&mut state).start_variable_focus_index = Some(99);
    state.set_node_editor_show_raw_data_editor(true);

    let yaml = r#"variables:
  - label: Only
    variable: only
    type: text-input
"#;
    editor_mut(&mut state).raw_data_editor = text_editor::Content::with_text("");
    state.node_editor_action(action_insert(yaml));

    assert_eq!(start_variables(&state).len(), 1);
    assert_eq!(start_variables(&state)[0].variable, "only");
    assert_eq!(editor(&state).start_variable_focus_index, Some(0));
    assert!(!editor(&state).validation.has_errors());
}

#[test]
fn inline_visual_updates_noop_without_visual_draft_or_start_variables() {
    let mut state = state_with_start_editor();
    editor_mut(&mut state).visual_draft = None;
    let raw_before = editor(&state).raw_data_editor.text();

    state.set_node_editor_start_variable_label(0, "ignored".to_string());
    state.set_node_editor_start_variable_name(0, "ignored".to_string());
    state.set_node_editor_start_variable_type(0, "ignored".to_string());
    state.set_node_editor_start_variable_required(0, true);
    state.set_node_editor_start_variable_default(0, "ignored".to_string());
    state.set_node_editor_start_variable_placeholder(0, "ignored".to_string());
    state.set_node_editor_start_variable_hint(0, "ignored".to_string());
    state.set_node_editor_start_variable_max_length(0, "ignored".to_string());

    assert_eq!(editor(&state).raw_data_editor.text(), raw_before);

    let mut state = WorkflowState::default();
    state.open_create_node_editor("answer", Point::new(0.0, 0.0)).unwrap();
    let raw_before = editor(&state).raw_data_editor.text();

    state.set_node_editor_start_variable_label(0, "ignored".to_string());
    state.set_node_editor_start_variable_name(0, "ignored".to_string());
    state.set_node_editor_start_variable_type(0, "ignored".to_string());
    state.set_node_editor_start_variable_required(0, true);
    state.set_node_editor_start_variable_default(0, "ignored".to_string());
    state.set_node_editor_start_variable_placeholder(0, "ignored".to_string());
    state.set_node_editor_start_variable_hint(0, "ignored".to_string());
    state.set_node_editor_start_variable_max_length(0, "ignored".to_string());

    assert_eq!(editor(&state).raw_data_editor.text(), raw_before);
}
