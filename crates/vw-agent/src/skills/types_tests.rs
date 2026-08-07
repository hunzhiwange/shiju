use super::*;
use crate::app::agent::config::SkillsPromptInjectionMode;

#[test]
fn default_policy_contains_stable_builtin_aliases() {
    let policy = SkillDownloadPolicy::default();
    assert_eq!(policy.version, default_policy_version());
    assert!(policy.aliases.contains_key("create-task-plans"));
    assert!(policy.aliases.contains_key("do-task-plans"));
    assert!(policy.aliases.contains_key("rustcodegraph"));
    assert!(policy.trusted_domains.is_empty());
    assert!(policy.blocked_domains.is_empty());
}

#[test]
fn skill_meta_deserialization_uses_default_version() {
    let manifest: SkillMetadataManifest = toml::from_str(
        r#"
        [skill]
        name = "demo"
        description = "Demo skill"
        "#,
    )
    .unwrap();
    assert_eq!(manifest.skill.version, default_version());
    assert_eq!(manifest.skill.tags, Vec::<String>::new());
}

#[test]
fn full_manifest_deserializes_defaults_and_tools() {
    let manifest: SkillManifest = toml::from_str(
        r#"
        prompts = ["Remember this"]

        [skill]
        name = "demo"
        description = "Demo skill"
        author = "tester"

        [[tools]]
        name = "hello"
        description = "Say hello"
        kind = "shell"
        command = "echo hello"
        "#,
    )
    .unwrap();

    assert_eq!(manifest.skill.version, default_version());
    assert_eq!(manifest.skill.author.as_deref(), Some("tester"));
    assert_eq!(manifest.tools.len(), 1);
    assert!(manifest.tools[0].args.is_empty());
    assert_eq!(manifest.prompts, vec!["Remember this"]);
}

#[test]
fn skill_load_mode_tracks_prompt_injection_mode() {
    assert_eq!(
        SkillLoadMode::from_prompt_mode(SkillsPromptInjectionMode::Full),
        SkillLoadMode::Full
    );
    assert_eq!(
        SkillLoadMode::from_prompt_mode(SkillsPromptInjectionMode::Compact),
        SkillLoadMode::MetadataOnly
    );
}
