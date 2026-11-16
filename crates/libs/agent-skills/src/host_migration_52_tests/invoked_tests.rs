//! 验证已调用技能集合的记录、去重与会话隔离行为。
//! 压缩恢复与会话分叉共享调用记录，但重新加载必须读取当前磁盘内容。

use super::{InvokedSkillRecord, SkillListingSessionState};

fn write_skill(path: &std::path::Path, body: &str, allowed_tools: &str, model: &str, effort: &str) {
    std::fs::write(
        path,
        format!(
            "---\nname: restore-skill\ndescription: restore test\nallowed-tools: {allowed_tools}\nmodel: {model}\neffort: {effort}\n---\n\n{body}\n"
        ),
    )
    .expect("write skill");
}

#[test]
fn invoked_skills_restore_after_compaction_resume_and_fork() {
    let temp = tempfile::tempdir().expect("temporary skills root");
    let skill_dir = temp.path().join("skills/restore-skill");
    std::fs::create_dir_all(&skill_dir).expect("skill directory");
    let skill_file = skill_dir.join("SKILL.md");
    write_skill(&skill_file, "INITIAL $ARGUMENTS", "[read, shell]", "provider/model-a", "high");

    let record =
        InvokedSkillRecord::new("restore-skill", &skill_file, Some("argument".to_string()))
            .expect("stable invocation record");
    let mut state = SkillListingSessionState::default();
    assert!(state.register_invoked("main", record.clone()));
    assert!(!state.register_invoked("main", record));

    let restored = state.restore_invoked("main").expect("restore after compaction");
    assert_eq!(restored.len(), 1);
    assert!(restored[0].content.contains("INITIAL argument"));
    assert_eq!(restored[0].allowed_tools, vec!["read", "shell"]);
    assert_eq!(restored[0].model.as_deref(), Some("provider/model-a"));
    assert_eq!(restored[0].effort.as_deref(), Some("high"));

    let resumed: SkillListingSessionState =
        serde_json::from_str(&serde_json::to_string(&state).expect("serialize state"))
            .expect("resume state");
    assert_eq!(resumed.restore_invoked("main").expect("resume replay"), restored);

    let forked = state.fork();
    assert_eq!(forked.restore_invoked("main").expect("fork replay"), restored);
}

#[test]
fn invoked_skill_reload_uses_current_file_state() {
    let temp = tempfile::tempdir().expect("temporary skills root");
    let skill_dir = temp.path().join("skills/restore-skill");
    std::fs::create_dir_all(&skill_dir).expect("skill directory");
    let skill_file = skill_dir.join("SKILL.md");
    write_skill(&skill_file, "OLD BODY", "[read]", "provider/model-old", "low");

    let mut state = SkillListingSessionState::default();
    state.register_invoked(
        "main",
        InvokedSkillRecord::new("restore-skill", &skill_file, None).expect("invocation record"),
    );
    write_skill(&skill_file, "NEW BODY", "[write]", "provider/model-new", "max");

    let replay = state.restore_invoked("main").expect("reload current file");
    assert_eq!(replay.len(), 1);
    assert!(replay[0].content.contains("NEW BODY"));
    assert!(!replay[0].content.contains("OLD BODY"));
    assert_eq!(replay[0].allowed_tools, vec!["write"]);
    assert_eq!(replay[0].model.as_deref(), Some("provider/model-new"));
    assert_eq!(replay[0].effort.as_deref(), Some("max"));

    std::fs::remove_file(&skill_file).expect("remove source");
    let error = state.restore_invoked("main").expect_err("missing source must be explicit");
    assert!(error.contains("不存在"));
}
