use super::project_skill_settings;
use std::{fs, path::Path};

fn settings(workspace: &Path, source: &str) {
    fs::create_dir_all(workspace.join(".github/copilot")).unwrap();
    fs::write(workspace.join(".github/copilot/settings.json"), source).unwrap();
}

#[test]
fn acp_skill_coexistence_copilot_projects_only_explicit_settings() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::create_dir_all(root.join(".github/skills")).unwrap();
    settings(
        root,
        r#"{
        "skillDirectories":[".github/skills"],
        "disabledSkills":["disabled"],
        "ignoredSkillsLocations":[".agents/skills"]
    }"#,
    );
    let projected = project_skill_settings(root).unwrap();
    let root = root.canonicalize().unwrap();
    assert_eq!(projected.skill_directories, [root.join(".github/skills").to_str().unwrap()]);
    assert_eq!(projected.disabled_skills, ["disabled"]);
    assert_eq!(projected.ignored_skills_locations, [root.join(".agents/skills").to_str().unwrap()]);

    settings(&root, "{}");
    assert!(project_skill_settings(&root).unwrap().skill_directories.is_empty());
}

#[test]
fn acp_skill_coexistence_copilot_rejects_outside_roots_and_settings_overrides() {
    let project = tempfile::tempdir().unwrap();
    for source in [
        r#"{"skillDirectories":["/tmp"]}"#,
        r#"{"skillDirectories":[".shiju/skills"]}"#,
        r#"{"ignoredSkillsLocations":["../other"]}"#,
        r#"{"ignoredSkillsLocations":[".github/skills/subdirectory"]}"#,
        r#"{"allowAllTools":true}"#,
    ] {
        settings(project.path(), source);
        assert!(project_skill_settings(project.path()).is_err());
    }
}

#[cfg(unix)]
#[test]
fn acp_skill_coexistence_copilot_rejects_native_symlinks() {
    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let root = project.path();
    fs::create_dir_all(root.join(".github/skills")).unwrap();
    settings(root, r#"{"skillDirectories":[".github/skills"]}"#);
    std::os::unix::fs::symlink(outside.path(), root.join(".github/skills/linked")).unwrap();
    assert!(project_skill_settings(root).is_err());
}

#[cfg(unix)]
#[test]
fn acp_skill_coexistence_copilot_rejects_linked_settings() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    settings(root, "{}");
    let path = root.join(".github/copilot/settings.json");
    let outside = tempfile::NamedTempFile::new().unwrap();
    fs::write(outside.path(), "{}").unwrap();
    fs::remove_file(&path).unwrap();
    fs::hard_link(outside.path(), &path).unwrap();
    assert!(project_skill_settings(root).is_err());
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(outside.path(), &path).unwrap();
    assert!(project_skill_settings(root).is_err());
}
