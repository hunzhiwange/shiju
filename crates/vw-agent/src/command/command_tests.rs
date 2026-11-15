// 项目上下文、技能发现与缓存集成测试保留在宿主。
use super::command::{ExecutedEvent, Source, r#default, event, get, list, publish_executed};
use crate::app::agent::{bus, project::instance};
use std::sync::{Arc, Mutex};

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
