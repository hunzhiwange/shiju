use super::distribute_skill_folder;
use config_skills::SkillSecurityConfig;
use std::fs;

#[test]
fn distribution_copies_resources_without_environment_files_or_overwriting() {
    let fixture = tempfile::tempdir().unwrap();
    let source = fixture.path().join("demo");
    let target = fixture.path().join("target");
    fs::create_dir_all(source.join("references")).unwrap();
    fs::write(
        source.join("SKILL.md"),
        "---\nname: demo\ndescription: A demo skill\n---\n\n# Demo\n",
    )
    .unwrap();
    fs::write(source.join(".env"), "fixture").unwrap();
    fs::write(source.join("references/.env.local"), "fixture").unwrap();
    fs::write(source.join("references/example.md"), "A resource.").unwrap();

    let security = SkillSecurityConfig::default();
    distribute_skill_folder(&source, &target, &security).unwrap();

    let installed = target.join("demo");
    assert!(!installed.join(".env").exists());
    assert!(!installed.join("references/.env.local").exists());
    assert_eq!(fs::read_to_string(installed.join("references/example.md")).unwrap(), "A resource.");

    fs::write(source.join("references/example.md"), "New content.").unwrap();
    assert!(distribute_skill_folder(&source, &target, &security).is_err());
    assert_eq!(fs::read_to_string(installed.join("references/example.md")).unwrap(), "A resource.");
}
