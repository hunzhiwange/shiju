use super::*;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn public_skill_queries_load_state_and_return_missing_skill_as_none() {
    let missing_name = format!(
        "__vibewindow_missing_skill_for_unit_test_{}__",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after unix epoch")
            .as_nanos()
    );

    assert!(get(&missing_name).await.is_none());

    let all_skills = all().await;
    let skill_dirs = dirs().await;

    assert!(all_skills.iter().all(|skill| !skill.name.trim().is_empty()));
    assert!(skill_dirs.iter().all(|dir| !dir.trim().is_empty()));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
// 状态加载只接纳有效技能，并显式记录重名来源而不是依赖目录枚举顺序覆盖。
async fn load_state_scans_workspace_skills_skips_invalid_files_and_tracks_duplicate_names() {
    let workspace = tempfile::tempdir().expect("workspace dir");
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after unix epoch")
        .as_nanos();
    let skill_name = format!("workspace-skill-{unique_suffix}");
    let primary_dir = workspace.path().join(".vibewindow/skills/primary");
    let duplicate_dir = workspace.path().join("skills/duplicate");
    let invalid_dir = workspace.path().join(".vibewindow/skills/invalid");
    std::fs::create_dir_all(&primary_dir).expect("primary skill dir");
    std::fs::create_dir_all(&duplicate_dir).expect("duplicate skill dir");
    std::fs::create_dir_all(&invalid_dir).expect("invalid skill dir");
    std::fs::write(
        primary_dir.join("SKILL.md"),
        format!("---\nname: {skill_name}\ndescription: first\n---\nFirst body\n"),
    )
    .expect("write primary skill");
    std::fs::write(
        duplicate_dir.join("SKILL.md"),
        format!("---\nname: {skill_name}\ndescription: second\n---\nSecond body\n"),
    )
    .expect("write duplicate skill");
    std::fs::write(invalid_dir.join("SKILL.md"), "---\ndescription: missing name\n---\nBody")
        .expect("write invalid skill");

    let state =
        instance::provide(workspace.path(), None, || Box::pin(async move { load_state().await }))
            .await
            .expect("project context");

    let loaded = state.skills.get(&skill_name).expect("workspace skill loaded");
    assert_eq!(loaded.name, skill_name);
    assert!(matches!(loaded.description.as_str(), "first" | "second"));
    assert_eq!(state.skills.values().filter(|skill| skill.name == loaded.name).count(), 1);
    assert!(state.dirs.iter().any(|dir| dir.ends_with(".vibewindow/skills/primary")));
    assert!(state.dirs.iter().any(|dir| dir.ends_with("skills/duplicate")));
    assert!(!state.dirs.iter().any(|dir| dir.ends_with(".vibewindow/skills/invalid")));
}
