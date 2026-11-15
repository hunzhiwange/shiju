// 本文件验证命令模板提示、来源序列化、执行事件字段与状态索引。

use super::command::{
    ExecutedEvent, Info, Source, State, r#default, event, hints,
};

#[test]
// 占位符提示去重但保留首次顺序，使补全结果可预测。
fn hints_extracts_unique_template_placeholders_in_order() {
    assert_eq!(hints("run $1 then $2 then $1"), vec!["$1".to_string(), "$2".to_string()]);
}

#[test]
fn hints_sorts_numeric_placeholders_and_appends_arguments_hint() {
    assert_eq!(
        hints("use $10 with $2 and $ARGUMENTS"),
        vec!["$10".to_string(), "$2".to_string(), "$ARGUMENTS".to_string()]
    );
}

#[test]
fn hints_ignores_non_numeric_dollar_words_except_arguments() {
    assert!(hints("price is $abc and $$ and $").is_empty());
    assert_eq!(hints("$ARGUMENTS_ONLY $ARGUMENTS"), vec!["$ARGUMENTS".to_string()]);
}

#[test]
fn hints_accepts_zero_leading_zero_and_adjacent_numeric_placeholders() {
    assert_eq!(
        hints("$2$01 then $0 and $10 then $2"),
        vec!["$0".to_string(), "$01".to_string(), "$10".to_string(), "$2".to_string()]
    );
}

#[test]
fn source_serializes_as_lowercase() {
    assert_eq!(serde_json::to_value(Source::Command).unwrap(), "command");
    assert_eq!(serde_json::to_value(Source::Mcp).unwrap(), "mcp");
    assert_eq!(serde_json::to_value(Source::Skill).unwrap(), "skill");
}

#[test]
fn source_deserializes_lowercase_and_rejects_unknown_variants() {
    let source: Source = serde_json::from_str(r#""mcp""#).unwrap();

    assert!(matches!(source, Source::Mcp));
    assert!(serde_json::from_str::<Source>(r#""plugin""#).is_err());
}

#[test]
fn executed_event_uses_frontend_field_names() {
    let value = serde_json::to_value(ExecutedEvent {
        name: "init".to_string(),
        session_id: "session-1".to_string(),
        arguments: "--force".to_string(),
        message_id: "message-1".to_string(),
    })
    .unwrap();

    assert_eq!(value["sessionID"], "session-1");
    assert_eq!(value["messageID"], "message-1");
    assert!(value.get("session_id").is_none());
    assert!(value.get("message_id").is_none());
}

#[test]
fn executed_event_deserializes_frontend_field_names() {
    let event: ExecutedEvent = serde_json::from_value(serde_json::json!({
        "name": "review",
        "sessionID": "session-2",
        "arguments": "branch feature",
        "messageID": "message-2"
    }))
    .unwrap();

    assert_eq!(event.name, "review");
    assert_eq!(event.session_id, "session-2");
    assert_eq!(event.arguments, "branch feature");
    assert_eq!(event.message_id, "message-2");
}

#[test]
fn info_serialization_omits_absent_optional_fields() {
    let value = serde_json::to_value(Info {
        name: "minimal".to_string(),
        description: None,
        agent: None,
        model: None,
        source: None,
        template: "do work".to_string(),
        subtask: None,
        hints: Vec::new(),
    })
    .unwrap();

    assert_eq!(
        value,
        serde_json::json!({
            "name": "minimal",
            "template": "do work",
            "hints": []
        })
    );
}

#[test]
fn info_deserialization_restores_all_fields() {
    let info: Info = serde_json::from_value(serde_json::json!({
        "name": "ship",
        "description": "Ship changes",
        "agent": "builder",
        "model": "model-1",
        "source": "skill",
        "template": "ship $1",
        "subtask": true,
        "hints": ["$1"]
    }))
    .unwrap();

    assert_eq!(info.name, "ship");
    assert_eq!(info.description.as_deref(), Some("Ship changes"));
    assert_eq!(info.agent.as_deref(), Some("builder"));
    assert_eq!(info.model.as_deref(), Some("model-1"));
    assert!(matches!(info.source, Some(Source::Skill)));
    assert_eq!(info.template, "ship $1");
    assert_eq!(info.subtask, Some(true));
    assert_eq!(info.hints, vec!["$1".to_string()]);
}

#[test]
fn default_command_names_and_event_type_are_stable() {
    assert_eq!(r#default::INIT, "init");
    assert_eq!(r#default::REVIEW, "review");
    assert_eq!(event::EXECUTED.r#type, "command.executed");
}

#[test]
fn state_stores_command_info_by_name() {
    let mut state = State::default();
    let info = Info {
        name: "review".to_string(),
        description: Some("Review changes".to_string()),
        agent: None,
        model: None,
        source: Some(Source::Command),
        template: "review $1".to_string(),
        subtask: None,
        hints: vec!["$1".to_string()],
    };

    state.commands.insert(info.name.clone(), info);

    assert!(state.commands.contains_key("review"));
}
