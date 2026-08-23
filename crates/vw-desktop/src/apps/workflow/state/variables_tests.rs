//! 工作流环境变量与会话变量状态的测试，覆盖面板和编辑器完整生命周期。
//! 用例验证缺失目标、名称生成、秘密值及不同变量类型的提交行为。
use super::*;
use iced::{Point, widget::text_editor};
use serde_yaml::Value;
use std::sync::Arc;

fn env_variable(
    id: &str,
    name: &str,
    value_type: &str,
    value: Value,
) -> WorkflowEnvironmentVariable {
    WorkflowEnvironmentVariable {
        id: id.to_string(),
        name: name.to_string(),
        value_type: value_type.to_string(),
        value,
        description: format!("{name} description"),
        raw_variable: Value::String(format!("raw-{id}")),
    }
}

fn conversation_variable(
    id: &str,
    name: &str,
    value_type: &str,
    value: Value,
) -> WorkflowConversationVariable {
    WorkflowConversationVariable {
        id: id.to_string(),
        name: name.to_string(),
        value_type: value_type.to_string(),
        value,
        description: format!("{name} description"),
        raw_variable: Value::String(format!("raw-{id}")),
    }
}

fn editor_text(state: &WorkflowState) -> String {
    state.variable_editor.as_ref().expect("editor is open").raw_value_editor.text()
}

fn set_editor_value(state: &mut WorkflowState, value: &str) {
    state.variable_editor.as_mut().expect("editor is open").raw_value_editor =
        text_editor::Content::with_text(value);
}

#[test]
fn open_variable_panel_closes_floating_menus() {
    let mut state = WorkflowState {
        context_menu: Some(WorkflowCanvasContextMenu {
            target: WorkflowCanvasContextMenuTarget::Canvas,
            anchor: Point::new(1.0, 2.0),
            world: Point::new(3.0, 4.0),
        }),
        quick_insert_panel_open: true,
        action_menu_open: true,
        zoom_menu_open: true,
        ..WorkflowState::default()
    };

    state.open_variable_panel(WorkflowVariablePanelKind::System);

    assert_eq!(state.variable_panel, Some(WorkflowVariablePanelKind::System));
    assert!(state.context_menu.is_none());
    assert!(!state.quick_insert_panel_open);
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);

    state.close_variable_panel();

    assert!(state.variable_panel.is_none());
}

#[test]
fn open_create_environment_editor_uses_next_available_name() {
    let mut state = WorkflowState {
        context_menu: Some(WorkflowCanvasContextMenu {
            target: WorkflowCanvasContextMenuTarget::Canvas,
            anchor: Point::ORIGIN,
            world: Point::ORIGIN,
        }),
        action_menu_open: true,
        zoom_menu_open: true,
        environment_variables: vec![
            env_variable("env-1", "env_1", "string", Value::String("one".to_string())),
            env_variable("env-2", "env_2", "string", Value::String("two".to_string())),
        ],
        ..WorkflowState::default()
    };

    state.open_create_environment_variable_editor();

    let editor = state.variable_editor.as_ref().expect("editor is open");
    assert_eq!(state.variable_panel, Some(WorkflowVariablePanelKind::Environment));
    assert_eq!(editor.mode, WorkflowVariableEditorMode::CreateEnvironment);
    assert_eq!(editor.name, "env_3");
    assert_eq!(editor.description, "");
    assert_eq!(editor.value_type, "string");
    assert_eq!(editor.raw_value_editor.text(), "");
    assert!(state.context_menu.is_none());
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
}

#[test]
fn open_edit_environment_editor_loads_existing_variable() {
    let mut state = WorkflowState {
        environment_variables: vec![env_variable(
            "env-1",
            "api_key",
            "secret",
            Value::String("token".to_string()),
        )],
        context_menu: Some(WorkflowCanvasContextMenu {
            target: WorkflowCanvasContextMenuTarget::Canvas,
            anchor: Point::ORIGIN,
            world: Point::ORIGIN,
        }),
        action_menu_open: true,
        zoom_menu_open: true,
        ..WorkflowState::default()
    };

    let result = state.open_edit_environment_variable_editor("env-1");

    assert_eq!(result, Ok(()));
    let editor = state.variable_editor.as_ref().expect("editor is open");
    assert_eq!(state.variable_panel, Some(WorkflowVariablePanelKind::Environment));
    assert_eq!(editor.mode, WorkflowVariableEditorMode::EditEnvironment("env-1".to_string()));
    assert_eq!(editor.name, "api_key");
    assert_eq!(editor.description, "api_key description");
    assert_eq!(editor.value_type, "secret");
    assert_eq!(editor.raw_value_editor.text(), "token\n");
    assert!(state.context_menu.is_none());
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
}

#[test]
fn open_edit_environment_editor_reports_missing_variable() {
    let mut state = WorkflowState::default();

    let result = state.open_edit_environment_variable_editor("missing");

    assert_eq!(result, Err("环境变量不存在".to_string()));
    assert!(state.variable_editor.is_none());
}

#[test]
fn open_create_conversation_editor_uses_next_available_name() {
    let mut state = WorkflowState {
        context_menu: Some(WorkflowCanvasContextMenu {
            target: WorkflowCanvasContextMenuTarget::Canvas,
            anchor: Point::ORIGIN,
            world: Point::ORIGIN,
        }),
        action_menu_open: true,
        zoom_menu_open: true,
        conversation_variables: vec![conversation_variable(
            "conversation-1",
            "conversation_1",
            "string",
            Value::String("one".to_string()),
        )],
        ..WorkflowState::default()
    };

    state.open_create_conversation_variable_editor();

    let editor = state.variable_editor.as_ref().expect("editor is open");
    assert_eq!(state.variable_panel, Some(WorkflowVariablePanelKind::Conversation));
    assert_eq!(editor.mode, WorkflowVariableEditorMode::CreateConversation);
    assert_eq!(editor.name, "conversation_2");
    assert_eq!(editor.description, "");
    assert_eq!(editor.value_type, "string");
    assert_eq!(editor.raw_value_editor.text(), "");
    assert!(state.context_menu.is_none());
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
}

#[test]
fn open_edit_conversation_editor_loads_existing_variable() {
    let mut state = WorkflowState {
        conversation_variables: vec![conversation_variable(
            "conversation-1",
            "topic",
            "array",
            Value::Sequence(vec![Value::String("rust".to_string())]),
        )],
        context_menu: Some(WorkflowCanvasContextMenu {
            target: WorkflowCanvasContextMenuTarget::Canvas,
            anchor: Point::ORIGIN,
            world: Point::ORIGIN,
        }),
        action_menu_open: true,
        zoom_menu_open: true,
        ..WorkflowState::default()
    };

    let result = state.open_edit_conversation_variable_editor("conversation-1");

    assert_eq!(result, Ok(()));
    let editor = state.variable_editor.as_ref().expect("editor is open");
    assert_eq!(state.variable_panel, Some(WorkflowVariablePanelKind::Conversation));
    assert_eq!(
        editor.mode,
        WorkflowVariableEditorMode::EditConversation("conversation-1".to_string())
    );
    assert_eq!(editor.name, "topic");
    assert_eq!(editor.description, "topic description");
    assert_eq!(editor.value_type, "array");
    assert_eq!(editor.raw_value_editor.text(), "- rust\n");
    assert!(state.context_menu.is_none());
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
}

#[test]
fn open_edit_conversation_editor_reports_missing_variable() {
    let mut state = WorkflowState::default();

    let result = state.open_edit_conversation_variable_editor("missing");

    assert_eq!(result, Err("会话变量不存在".to_string()));
    assert!(state.variable_editor.is_none());
}

#[test]
fn editor_setters_ignore_missing_editor_and_update_open_editor() {
    let mut state = WorkflowState::default();

    state.set_variable_editor_name("ignored".to_string());
    state.set_variable_editor_description("ignored".to_string());
    state.set_variable_editor_type("ignored".to_string());
    state.variable_editor_action(text_editor::Action::Edit(text_editor::Edit::Insert('x')));
    state.close_variable_editor();

    assert!(state.variable_editor.is_none());

    state.open_create_environment_variable_editor();
    state.set_variable_editor_name(" api_url ".to_string());
    state.set_variable_editor_description(" endpoint ".to_string());
    state.set_variable_editor_type(" SECRET ".to_string());
    state.variable_editor_action(text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(
        "value".to_string(),
    ))));

    let editor = state.variable_editor.as_ref().expect("editor is open");
    assert_eq!(editor.name, " api_url ");
    assert_eq!(editor.description, " endpoint ");
    assert_eq!(editor.value_type, " SECRET ");
    assert_eq!(editor.raw_value_editor.text(), "value");

    state.close_variable_editor();

    assert!(state.variable_editor.is_none());
}

#[test]
fn submit_without_open_editor_is_ok() {
    let mut state = WorkflowState::default();

    let result = state.submit_variable_editor();

    assert_eq!(result, Ok(()));
    assert!(state.environment_variables.is_empty());
    assert!(state.conversation_variables.is_empty());
}

#[test]
fn submit_environment_create_trims_normalizes_and_closes_editor() {
    let mut state = WorkflowState::default();
    state.open_create_environment_variable_editor();
    state.set_variable_editor_name(" api_url ".to_string());
    state.set_variable_editor_description(" endpoint ".to_string());
    state.set_variable_editor_type(" SECRET ".to_string());
    set_editor_value(&mut state, "token");

    let result = state.submit_variable_editor();

    assert_eq!(result, Ok(()));
    assert_eq!(state.environment_variables.len(), 1);
    let variable = &state.environment_variables[0];
    assert!(variable.id.starts_with("env-"));
    assert_eq!(variable.name, "api_url");
    assert_eq!(variable.description, "endpoint");
    assert_eq!(variable.value_type, "secret");
    assert_eq!(variable.value, Value::String("token".to_string()));
    assert_eq!(variable.raw_variable, Value::Null);
    assert_eq!(state.variable_panel, Some(WorkflowVariablePanelKind::Environment));
    assert!(state.variable_editor.is_none());
    assert_eq!(state.status_message, Some("已新增环境变量 api_url".to_string()));
}

#[test]
fn submit_environment_create_rejects_invalid_inputs() {
    let mut state = WorkflowState::default();
    state.open_create_environment_variable_editor();

    state.set_variable_editor_name("   ".to_string());
    assert_eq!(state.submit_variable_editor(), Err("变量名称不能为空".to_string()));

    state.set_variable_editor_name("api_url".to_string());
    set_editor_value(&mut state, "[");
    let yaml_error = state.submit_variable_editor().expect_err("invalid YAML is rejected");
    assert!(yaml_error.starts_with("变量值 YAML 解析失败:"));

    set_editor_value(&mut state, "plain");
    state.set_variable_editor_type("boolean".to_string());
    assert_eq!(
        state.submit_variable_editor(),
        Err("环境变量类型仅支持 string / number / secret".to_string())
    );

    state.set_variable_editor_type("number".to_string());
    assert_eq!(
        state.submit_variable_editor(),
        Err("number 类型环境变量的值必须是数字 YAML 标量".to_string())
    );

    state.set_variable_editor_type("string".to_string());
    set_editor_value(&mut state, "1");
    assert_eq!(
        state.submit_variable_editor(),
        Err("string 类型环境变量的值必须是字符串 YAML 标量".to_string())
    );

    state.set_variable_editor_type("secret".to_string());
    assert_eq!(
        state.submit_variable_editor(),
        Err("secret 类型环境变量的值必须是字符串 YAML 标量".to_string())
    );

    state.environment_variables.push(env_variable(
        "env-existing",
        "api_url",
        "string",
        Value::String("existing".to_string()),
    ));
    state.set_variable_editor_type("string".to_string());
    set_editor_value(&mut state, "new");
    assert_eq!(state.submit_variable_editor(), Err("环境变量名称不能重复".to_string()));
    assert_eq!(state.environment_variables.len(), 1);
    assert!(state.variable_editor.is_some());
}

#[test]
fn submit_environment_edit_preserves_raw_variable_and_allows_same_name() {
    let mut state = WorkflowState {
        environment_variables: vec![
            env_variable("env-1", "api_url", "string", Value::String("old".to_string())),
            env_variable("env-2", "api_key", "secret", Value::String("secret".to_string())),
        ],
        ..WorkflowState::default()
    };
    state.open_edit_environment_variable_editor("env-1").expect("editor opens");
    state.set_variable_editor_name(" api_url ".to_string());
    state.set_variable_editor_description(" updated ".to_string());
    state.set_variable_editor_type(" NUMBER ".to_string());
    set_editor_value(&mut state, "42");

    let result = state.submit_variable_editor();

    assert_eq!(result, Ok(()));
    let variable = state.environment_variable("env-1").expect("variable remains");
    assert_eq!(variable.name, "api_url");
    assert_eq!(variable.description, "updated");
    assert_eq!(variable.value_type, "number");
    assert_eq!(variable.value, serde_yaml::from_str::<Value>("42").expect("number"));
    assert_eq!(variable.raw_variable, Value::String("raw-env-1".to_string()));
    assert_eq!(state.status_message, Some("已更新环境变量 api_url".to_string()));
    assert!(state.variable_editor.is_none());
}

#[test]
fn submit_environment_edit_rejects_duplicate_and_missing_target() {
    let mut state = WorkflowState {
        environment_variables: vec![
            env_variable("env-1", "api_url", "string", Value::String("old".to_string())),
            env_variable("env-2", "api_key", "secret", Value::String("secret".to_string())),
        ],
        ..WorkflowState::default()
    };
    state.open_edit_environment_variable_editor("env-1").expect("editor opens");
    state.set_variable_editor_name("api_key".to_string());
    set_editor_value(&mut state, "new");

    assert_eq!(state.submit_variable_editor(), Err("环境变量名称不能重复".to_string()));

    state.set_variable_editor_name("api_new".to_string());
    state.environment_variables.retain(|variable| variable.id != "env-1");

    assert_eq!(state.submit_variable_editor(), Err("环境变量不存在".to_string()));
}

#[test]
fn submit_conversation_create_accepts_any_non_empty_type() {
    let mut state = WorkflowState::default();
    state.open_create_conversation_variable_editor();
    state.set_variable_editor_name(" topic ".to_string());
    state.set_variable_editor_description(" current topic ".to_string());
    state.set_variable_editor_type(" ARRAY ".to_string());
    set_editor_value(&mut state, "- rust\n- iced");

    let result = state.submit_variable_editor();

    assert_eq!(result, Ok(()));
    assert_eq!(state.conversation_variables.len(), 1);
    let variable = &state.conversation_variables[0];
    assert!(variable.id.starts_with("conversation-"));
    assert_eq!(variable.name, "topic");
    assert_eq!(variable.description, "current topic");
    assert_eq!(variable.value_type, "array");
    assert_eq!(
        variable.value,
        Value::Sequence(vec![Value::String("rust".to_string()), Value::String("iced".to_string())])
    );
    assert_eq!(variable.raw_variable, Value::Null);
    assert_eq!(state.variable_panel, Some(WorkflowVariablePanelKind::Conversation));
    assert!(state.variable_editor.is_none());
    assert_eq!(state.status_message, Some("已新增会话变量 topic".to_string()));
}

#[test]
fn submit_conversation_create_rejects_empty_type_duplicate_and_bad_yaml() {
    let mut state = WorkflowState {
        conversation_variables: vec![conversation_variable(
            "conversation-1",
            "topic",
            "string",
            Value::String("old".to_string()),
        )],
        ..WorkflowState::default()
    };
    state.open_create_conversation_variable_editor();
    state.set_variable_editor_name("status".to_string());
    state.set_variable_editor_type(" ".to_string());

    assert_eq!(state.submit_variable_editor(), Err("会话变量类型不能为空".to_string()));

    state.set_variable_editor_type("string".to_string());
    set_editor_value(&mut state, "[");
    let yaml_error = state.submit_variable_editor().expect_err("invalid YAML is rejected");
    assert!(yaml_error.starts_with("变量值 YAML 解析失败:"));

    state.set_variable_editor_name("topic".to_string());
    set_editor_value(&mut state, "new");
    assert_eq!(state.submit_variable_editor(), Err("会话变量名称不能重复".to_string()));
    assert_eq!(state.conversation_variables.len(), 1);
    assert!(state.variable_editor.is_some());
}

#[test]
fn submit_conversation_edit_preserves_raw_variable_and_allows_same_name() {
    let mut state = WorkflowState {
        conversation_variables: vec![
            conversation_variable("conversation-1", "topic", "string", Value::String("old".into())),
            conversation_variable(
                "conversation-2",
                "status",
                "string",
                Value::String("ready".into()),
            ),
        ],
        ..WorkflowState::default()
    };
    state.open_edit_conversation_variable_editor("conversation-1").expect("editor opens");
    state.set_variable_editor_name(" topic ".to_string());
    state.set_variable_editor_description(" updated ".to_string());
    state.set_variable_editor_type(" OBJECT ".to_string());
    set_editor_value(&mut state, "name: rust");

    let result = state.submit_variable_editor();

    assert_eq!(result, Ok(()));
    let variable = state.conversation_variable("conversation-1").expect("variable remains");
    assert_eq!(variable.name, "topic");
    assert_eq!(variable.description, "updated");
    assert_eq!(variable.value_type, "object");
    assert_eq!(variable.value, serde_yaml::from_str::<Value>("name: rust").expect("map"));
    assert_eq!(variable.raw_variable, Value::String("raw-conversation-1".to_string()));
    assert_eq!(state.status_message, Some("已更新会话变量 topic".to_string()));
    assert!(state.variable_editor.is_none());
}

#[test]
fn submit_conversation_edit_rejects_duplicate_and_missing_target() {
    let mut state = WorkflowState {
        conversation_variables: vec![
            conversation_variable("conversation-1", "topic", "string", Value::String("old".into())),
            conversation_variable(
                "conversation-2",
                "status",
                "string",
                Value::String("ready".into()),
            ),
        ],
        ..WorkflowState::default()
    };
    state.open_edit_conversation_variable_editor("conversation-1").expect("editor opens");
    state.set_variable_editor_name("status".to_string());
    set_editor_value(&mut state, "new");

    assert_eq!(state.submit_variable_editor(), Err("会话变量名称不能重复".to_string()));

    state.set_variable_editor_name("new_topic".to_string());
    state.conversation_variables.retain(|variable| variable.id != "conversation-1");

    assert_eq!(state.submit_variable_editor(), Err("会话变量不存在".to_string()));
}

#[test]
fn delete_environment_variable_updates_panel_status_and_open_editor() {
    let mut state = WorkflowState {
        environment_variables: vec![
            env_variable("env-1", "api_url", "string", Value::String("old".to_string())),
            env_variable("env-2", "api_key", "secret", Value::String("secret".to_string())),
        ],
        ..WorkflowState::default()
    };
    state.open_edit_environment_variable_editor("env-1").expect("editor opens");

    let removed = state.delete_environment_variable("env-1");

    assert!(removed);
    assert_eq!(state.environment_variables.len(), 1);
    assert_eq!(state.environment_variables[0].id, "env-2");
    assert!(state.variable_editor.is_none());
    assert_eq!(state.variable_panel, Some(WorkflowVariablePanelKind::Environment));
    assert_eq!(state.status_message, Some("已删除环境变量 api_url".to_string()));

    assert!(!state.delete_environment_variable("missing"));
}

#[test]
fn delete_environment_variable_keeps_unrelated_editor_open() {
    let mut state = WorkflowState {
        environment_variables: vec![
            env_variable("env-1", "api_url", "string", Value::String("old".to_string())),
            env_variable("env-2", "api_key", "secret", Value::String("secret".to_string())),
        ],
        ..WorkflowState::default()
    };
    state.open_edit_environment_variable_editor("env-2").expect("editor opens");

    assert!(state.delete_environment_variable("env-1"));
    assert_eq!(
        state.variable_editor.as_ref().expect("editor remains").mode,
        WorkflowVariableEditorMode::EditEnvironment("env-2".to_string())
    );
}

#[test]
fn delete_conversation_variable_updates_panel_status_and_open_editor() {
    let mut state = WorkflowState {
        conversation_variables: vec![
            conversation_variable("conversation-1", "topic", "string", Value::String("old".into())),
            conversation_variable(
                "conversation-2",
                "status",
                "string",
                Value::String("ready".into()),
            ),
        ],
        ..WorkflowState::default()
    };
    state.open_edit_conversation_variable_editor("conversation-1").expect("editor opens");

    let removed = state.delete_conversation_variable("conversation-1");

    assert!(removed);
    assert_eq!(state.conversation_variables.len(), 1);
    assert_eq!(state.conversation_variables[0].id, "conversation-2");
    assert!(state.variable_editor.is_none());
    assert_eq!(state.variable_panel, Some(WorkflowVariablePanelKind::Conversation));
    assert_eq!(state.status_message, Some("已删除会话变量 topic".to_string()));

    assert!(!state.delete_conversation_variable("missing"));
}

#[test]
fn delete_conversation_variable_keeps_unrelated_editor_open() {
    let mut state = WorkflowState {
        conversation_variables: vec![
            conversation_variable("conversation-1", "topic", "string", Value::String("old".into())),
            conversation_variable(
                "conversation-2",
                "status",
                "string",
                Value::String("ready".into()),
            ),
        ],
        ..WorkflowState::default()
    };
    state.open_edit_conversation_variable_editor("conversation-2").expect("editor opens");

    assert!(state.delete_conversation_variable("conversation-1"));
    assert_eq!(
        state.variable_editor.as_ref().expect("editor remains").mode,
        WorkflowVariableEditorMode::EditConversation("conversation-2".to_string())
    );
}

#[test]
fn empty_editor_value_is_empty_string_yaml_value() {
    let mut state = WorkflowState::default();
    state.open_create_environment_variable_editor();
    state.set_variable_editor_name("empty_value".to_string());
    set_editor_value(&mut state, "");

    assert_eq!(state.submit_variable_editor(), Ok(()));
    assert_eq!(state.environment_variables[0].value, Value::String(String::new()));
}

#[test]
fn opened_environment_editor_renders_number_value_as_yaml() {
    let mut state = WorkflowState {
        environment_variables: vec![env_variable(
            "env-1",
            "limit",
            "number",
            serde_yaml::from_str::<Value>("5").expect("number"),
        )],
        ..WorkflowState::default()
    };

    state.open_edit_environment_variable_editor("env-1").expect("editor opens");

    assert_eq!(editor_text(&state), "5\n");
}

#[test]
fn create_editors_fill_first_gap_in_default_names() {
    let mut state = WorkflowState {
        environment_variables: vec![
            env_variable("env-1", "env_1", "string", Value::String("one".to_string())),
            env_variable("env-3", "env_3", "string", Value::String("three".to_string())),
        ],
        conversation_variables: vec![
            conversation_variable(
                "conversation-1",
                "conversation_1",
                "string",
                Value::String("one".to_string()),
            ),
            conversation_variable(
                "conversation-3",
                "conversation_3",
                "string",
                Value::String("three".to_string()),
            ),
        ],
        ..WorkflowState::default()
    };

    state.open_create_environment_variable_editor();
    assert_eq!(state.variable_editor.as_ref().expect("editor is open").name, "env_2");

    state.open_create_conversation_variable_editor();
    assert_eq!(state.variable_editor.as_ref().expect("editor is open").name, "conversation_2");
}

#[test]
fn delete_missing_variables_preserves_existing_state() {
    let mut state = WorkflowState {
        environment_variables: vec![env_variable(
            "env-1",
            "api_url",
            "string",
            Value::String("old".to_string()),
        )],
        conversation_variables: vec![conversation_variable(
            "conversation-1",
            "topic",
            "string",
            Value::String("old".to_string()),
        )],
        variable_panel: Some(WorkflowVariablePanelKind::System),
        status_message: Some("unchanged".to_string()),
        ..WorkflowState::default()
    };
    state.open_edit_environment_variable_editor("env-1").expect("editor opens");

    assert!(!state.delete_environment_variable("missing"));
    assert!(!state.delete_conversation_variable("missing"));

    assert_eq!(state.environment_variables.len(), 1);
    assert_eq!(state.conversation_variables.len(), 1);
    assert_eq!(
        state.variable_editor.as_ref().expect("editor remains").mode,
        WorkflowVariableEditorMode::EditEnvironment("env-1".to_string())
    );
    assert_eq!(state.variable_panel, Some(WorkflowVariablePanelKind::Environment));
    assert_eq!(state.status_message, Some("unchanged".to_string()));
}

#[test]
fn delete_missing_variables_without_open_editor_keeps_panel_and_status_unchanged() {
    let mut state = WorkflowState {
        variable_panel: Some(WorkflowVariablePanelKind::System),
        status_message: Some("unchanged".to_string()),
        ..WorkflowState::default()
    };

    assert!(!state.delete_environment_variable("missing-env"));
    assert!(!state.delete_conversation_variable("missing-conversation"));

    assert!(state.variable_editor.is_none());
    assert_eq!(state.variable_panel, Some(WorkflowVariablePanelKind::System));
    assert_eq!(state.status_message, Some("unchanged".to_string()));
}

#[test]
fn submit_environment_number_accepts_float_yaml_scalar() {
    let mut state = WorkflowState::default();
    state.open_create_environment_variable_editor();
    state.set_variable_editor_name("threshold".to_string());
    state.set_variable_editor_type("number".to_string());
    set_editor_value(&mut state, "0.75");

    assert_eq!(state.submit_variable_editor(), Ok(()));
    assert_eq!(
        state.environment_variables[0].value,
        serde_yaml::from_str::<Value>("0.75").expect("float")
    );
}

#[test]
fn submit_conversation_empty_editor_value_uses_empty_string() {
    let mut state = WorkflowState::default();
    state.open_create_conversation_variable_editor();
    state.set_variable_editor_name("note".to_string());
    set_editor_value(&mut state, "");

    assert_eq!(state.submit_variable_editor(), Ok(()));
    assert_eq!(state.conversation_variables[0].value, Value::String(String::new()));
}
