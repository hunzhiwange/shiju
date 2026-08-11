use super::invocation::build_invocation_result;
use crate::app::agent::skills::Skill;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn skill() -> (Skill, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "vw-agent-skill-tool-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).expect("system clock").as_nanos(),
    ));
    fs::create_dir_all(&root).expect("create temporary skill directory");
    let location = root.join("SKILL.md");
    fs::write(&location, "# Review\nInspect $ARGUMENTS before responding.\n")
        .expect("write temporary SKILL.md");

    let skill = Skill {
        name: "review".to_string(),
        description: "Review source files".to_string(),
        prompts: vec![fs::read_to_string(&location).expect("read temporary SKILL.md")],
        location: Some(location),
        allowed_tools: vec!["file_read".to_string(), "grep".to_string()],
        model: Some("gpt-5.6".to_string()),
        effort: Some("high".to_string()),
        hooks: Some(json!({"on_complete": "audit"})),
        context: Some("fork".to_string()),
        ..Skill::default()
    };
    (skill, root)
}

#[test]
fn skill_tool_returns_full_skill_as_meta_user_extra_message() {
    let (skill, root) = skill();
    let result = build_invocation_result(
        &skill,
        PathBuf::from("/workspace").as_path(),
        Some("src/lib.rs"),
        false,
    )
    .expect("supported skill should load");

    assert_eq!(result.model_text(), "Skill \"review\" loaded.");
    assert_eq!(result.extra_messages.len(), 1);
    assert_eq!(result.extra_messages[0].role, "user");
    let content = result.extra_messages[0].content.as_str().expect("meta user text");
    assert!(content.contains("<system-reminder>"));
    assert!(content.contains("# Review\nInspect src/lib.rs before responding."));
    assert!(content.contains(&format!("file:///{}", root.to_string_lossy())));
    assert!(!result.model_text().contains("# Review"));
    assert_eq!(
        result
            .context_updates
            .iter()
            .find(|update| update.key == "skill.allowed_tools")
            .map(|u| &u.value),
        Some(&json!(["file_read", "grep"]))
    );
    assert!(result.context_updates.iter().any(|update| update.key == "skill.fork_context"));
    fs::remove_dir_all(root).expect("remove temporary skill directory");
}

#[test]
fn skill_tool_rejects_model_disabled_and_path_scoped_skills() {
    let (mut disabled, disabled_root) = skill();
    disabled.disable_model_invocation = true;
    assert!(
        build_invocation_result(&disabled, PathBuf::from("/workspace").as_path(), None, false,)
            .expect_err("model-disabled skills must not load")
            .contains("禁止模型调用")
    );
    assert!(
        build_invocation_result(&disabled, PathBuf::from("/workspace").as_path(), None, true,)
            .is_ok()
    );

    let (mut path_scoped, path_scoped_root) = skill();
    path_scoped.paths = vec!["src/**/*.rs".to_string()];
    assert!(
        build_invocation_result(&path_scoped, PathBuf::from("/workspace").as_path(), None, false,)
            .expect_err("path-scoped skills need an execution path")
            .contains("未支持的 paths 条件")
    );
    fs::remove_dir_all(disabled_root).expect("remove disabled temporary skill directory");
    fs::remove_dir_all(path_scoped_root).expect("remove path-scoped temporary skill directory");
}
