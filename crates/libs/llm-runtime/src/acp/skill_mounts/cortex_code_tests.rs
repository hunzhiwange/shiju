use super::super::MountedSkills;
use std::fs;

#[test]
fn acp_skill_coexistence_cortex_code_keeps_launch_config_without_selection() {
    let profile = tempfile::tempdir().unwrap();
    let config = profile.path().join("cortex-launch.json");
    fs::write(&config, b"binding").unwrap();

    super::publish(profile.path(), profile.path(), None::<&MountedSkills>).unwrap();

    assert_eq!(fs::read(&config).unwrap(), b"binding");
    assert!(!profile.path().join("session-skills").exists());
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn acp_skill_coexistence_cortex_code_exposes_only_the_verified_workspace_selection() {
    let workspace = tempfile::tempdir().unwrap();
    let profile = tempfile::tempdir().unwrap();
    let other_project = tempfile::tempdir().unwrap();
    let source = workspace.path().join(".shiju/skills/selected");
    let native = workspace.path().join(".claude/skills/native");
    for skill in [&source, &native] {
        fs::create_dir_all(skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            format!(
                "---\nname: {}\ndescription: Fixture\n---\nRead the fixture.\n",
                skill.file_name().unwrap().to_str().unwrap()
            ),
        )
        .unwrap();
    }
    let view = MountedSkills::prepare(
        &[acp_types::SessionSkillMount {
            name: "selected".into(),
            path: source.to_str().unwrap().into(),
        }],
        workspace.path(),
        "cortex-coexistence",
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .unwrap();

    assert!(super::super::validate("cortex-code", true).is_ok());
    let scanner = profile.path().join("session-skills");
    assert!(super::publish(profile.path(), other_project.path(), Some(&view)).is_err());
    assert!(!scanner.exists());

    super::publish(profile.path(), workspace.path(), Some(&view)).unwrap();

    assert_eq!(fs::read_dir(&scanner).unwrap().count(), 1);
    assert_eq!(fs::read_link(scanner.join("selected")).unwrap(), source.canonicalize().unwrap());
    assert!(native.join("SKILL.md").is_file());
}
