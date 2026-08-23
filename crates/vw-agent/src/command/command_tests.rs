use super::command::{
    ExecutedEvent, Info, Source, State, r#default, event, get, hints, list, publish_executed,
};
use crate::app::agent::{bus, project::instance};
use std::sync::{Arc, Mutex};

#[test]
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

#[tokio::test]
async fn list_loads_builtin_commands_with_project_path() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join(".git")).expect("create git dir");
    let worktree = temp.path().to_string_lossy().to_string();
    let commands = instance::provide(temp.path(), None, || Box::pin(async { list().await }))
        .await
        .expect("project instance");

    let init = commands.iter().find(|command| command.name == r#default::INIT).unwrap();
    assert_eq!(init.description.as_deref(), Some("Create/update AGENTS.md"));
    assert!(matches!(init.source.as_ref(), Some(Source::Command)));
    assert_eq!(init.subtask, None);
    assert!(init.template.contains(&worktree));
    assert!(!init.template.contains("${path}"));
    assert_eq!(init.hints, vec!["$ARGUMENTS".to_string()]);

    let review = commands.iter().find(|command| command.name == r#default::REVIEW).unwrap();
    assert_eq!(
        review.description.as_deref(),
        Some("Review changes [commit|branch|pr], defaults to uncommitted")
    );
    assert!(matches!(review.source.as_ref(), Some(Source::Command)));
    assert_eq!(review.subtask, Some(true));
    assert!(!review.template.contains("${path}"));
    assert_eq!(review.hints, vec!["$ARGUMENTS".to_string()]);
}

#[tokio::test]
async fn get_returns_command_by_name_and_none_for_missing() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let (found, missing) = instance::provide(temp.path(), None, || {
        Box::pin(async { (get(r#default::INIT).await, get("missing-command").await) })
    })
    .await
    .expect("project instance");

    assert_eq!(found.unwrap().name, r#default::INIT);
    assert!(missing.is_none());
}

#[tokio::test]
async fn list_loads_skill_commands_without_overriding_builtins() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    write_skill(temp.path(), "vw-command-test-skill", "Ship plan", "Ship $1");
    write_skill(temp.path(), r#default::INIT, "Shadow init", "shadow");

    let commands = instance::provide(temp.path(), None, || Box::pin(async { list().await }))
        .await
        .expect("project instance");

    let skill = commands.iter().find(|command| command.name == "vw-command-test-skill").unwrap();
    assert_eq!(skill.description.as_deref(), Some("Ship plan"));
    assert!(matches!(skill.source.as_ref(), Some(Source::Skill)));
    assert_eq!(skill.template, "Ship $1");
    assert_eq!(skill.subtask, None);
    assert!(skill.hints.is_empty());

    let init = commands.iter().find(|command| command.name == r#default::INIT).unwrap();
    assert_eq!(init.description.as_deref(), Some("Create/update AGENTS.md"));
    assert!(matches!(init.source.as_ref(), Some(Source::Command)));
    assert_ne!(init.template, "shadow");
}

#[tokio::test]
async fn publish_executed_dispatches_command_event_with_instance_directory() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let expected_dir = temp.path().to_string_lossy().to_string();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let slot = Arc::clone(&seen);
    let unsubscribe = bus::global_subscribe(move |evt| {
        if evt.payload["type"] == event::EXECUTED.r#type {
            slot.lock().unwrap().push((evt.directory.clone(), evt.payload.clone()));
        }
    });

    instance::provide(temp.path(), None, || {
        Box::pin(async {
            publish_executed(ExecutedEvent {
                name: r#default::REVIEW.to_string(),
                session_id: "session-1".to_string(),
                arguments: "branch main".to_string(),
                message_id: "message-1".to_string(),
            })
            .unwrap();
        })
    })
    .await
    .expect("project instance");
    unsubscribe();

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0.as_deref(), Some(expected_dir.as_str()));
    assert_eq!(seen[0].1["properties"]["name"], r#default::REVIEW);
    assert_eq!(seen[0].1["properties"]["sessionID"], "session-1");
    assert_eq!(seen[0].1["properties"]["arguments"], "branch main");
    assert_eq!(seen[0].1["properties"]["messageID"], "message-1");
}

fn write_skill(root: &std::path::Path, name: &str, description: &str, content: &str) {
    let dir = root.join(".vibewindow").join("skills").join(name);
    std::fs::create_dir_all(&dir).expect("skill dir");
    std::fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n\n{content}\n"),
    )
    .expect("skill file");
}
// 本文件验证命令模板提示、来源序列化、执行事件字段与状态索引。
