use super::*;

#[derive(Debug, clap::Parser)]
#[command(name = "skill-test")]
struct SkillTestCli {
    #[command(subcommand)]
    command: SkillCommands,
}

#[test]
fn default_download_policy_is_deny_by_default_for_domains() {
    let policy = SkillDownloadPolicy::default();
    assert_eq!(policy.version, 1);
    assert!(policy.trusted_domains.is_empty());
    assert!(policy.blocked_domains.is_empty());
    assert!(policy.aliases.contains_key("create-task-plans"));
}

#[test]
fn skills_sh_source_formats_github_clone_url() {
    let source = SkillsShSource {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        skill: "skill".to_string(),
    };
    assert_eq!(source.github_repo_url(), "https://github.com/owner/repo.git");
}

#[test]
fn runtime_and_skill_config_defaults_are_conservative() {
    let config = SkillsConfig::default();
    assert!(!config.open_skills_enabled);
    assert!(config.open_skills_dir.is_none());
    assert_eq!(config.prompt_injection_mode, SkillsPromptInjectionMode::Full);

    let runtime = SkillRuntimeConfig::default();
    assert_eq!(runtime.workspace_dir, PathBuf::from("."));
    assert!(!runtime.skills.open_skills_enabled);
}

#[test]
fn load_mode_tracks_prompt_injection_mode() {
    assert_eq!(
        SkillLoadMode::from_prompt_mode(SkillsPromptInjectionMode::Full),
        SkillLoadMode::Full
    );
    assert_eq!(
        SkillLoadMode::from_prompt_mode(SkillsPromptInjectionMode::Compact),
        SkillLoadMode::MetadataOnly
    );
}

#[test]
fn skill_manifest_deserializes_defaults_prompts_and_tools() {
    let manifest: SkillManifest = toml::from_str(
        r#"
        prompts = ["Check carefully"]

        [skill]
        name = "lint"
        description = "Lint code"

        [[tools]]
        name = "cargo_check"
        description = "Run cargo check"
        kind = "shell"
        command = "cargo check"
        [tools.args]
        package = "vw-agent"
        "#,
    )
    .unwrap();

    assert_eq!(manifest.skill.version, "0.1.0");
    assert_eq!(manifest.prompts, vec!["Check carefully"]);
    assert_eq!(manifest.tools.len(), 1);
    assert_eq!(manifest.tools[0].args.get("package").map(String::as_str), Some("vw-agent"));
}

#[test]
fn public_skill_serializes_without_runtime_location() {
    let skill = Skill {
        name: "demo".to_string(),
        description: "Demo".to_string(),
        version: "1.2.3".to_string(),
        author: Some("me".to_string()),
        tags: vec!["test".to_string()],
        tools: Vec::new(),
        prompts: vec!["Use it".to_string()],
        location: Some(PathBuf::from("/tmp/secret/SKILL.md")),
    };

    let value = serde_json::to_value(&skill).unwrap();

    assert_eq!(value["name"], "demo");
    assert!(value.get("location").is_none());
}

#[test]
fn download_policy_deserialization_applies_missing_field_defaults() {
    let policy: SkillDownloadPolicy = toml::from_str(
        r#"
        trusted_domains = ["trusted.example"]
        "#,
    )
    .unwrap();

    assert_eq!(policy.version, 1);
    assert_eq!(
        policy.aliases.get("create-task-plans").map(String::as_str),
        Some("bundled:create-task-plans")
    );
    assert_eq!(policy.trusted_domains, vec!["trusted.example".to_string()]);
    assert!(policy.blocked_domains.is_empty());
}

#[test]
fn public_skill_deserialization_defaults_optional_fields_and_ignores_location() {
    let skill: Skill = serde_json::from_value(serde_json::json!({
        "name": "demo",
        "description": "Demo",
        "version": "1.2.3",
        "location": "/tmp/untrusted/SKILL.md"
    }))
    .unwrap();

    assert!(skill.author.is_none());
    assert!(skill.tags.is_empty());
    assert!(skill.tools.is_empty());
    assert!(skill.prompts.is_empty());
    assert!(skill.location.is_none());
}

#[test]
fn public_skill_deserialization_rejects_missing_version() {
    let result = serde_json::from_value::<Skill>(serde_json::json!({
        "name": "demo",
        "description": "Demo"
    }));

    assert!(result.is_err());
}

#[test]
fn metadata_manifest_deserialization_applies_metadata_defaults() {
    let manifest: SkillMetadataManifest = toml::from_str(
        r#"
        [skill]
        name = "metadata-only"
        description = "Metadata only skill"
        "#,
    )
    .unwrap();

    assert_eq!(manifest.skill.name, "metadata-only");
    assert_eq!(manifest.skill.version, "0.1.0");
    assert!(manifest.skill.author.is_none());
    assert!(manifest.skill.tags.is_empty());
}

#[test]
fn metadata_manifest_deserialization_rejects_missing_name() {
    let result = toml::from_str::<SkillMetadataManifest>(
        r#"
        [skill]
        description = "Missing required name"
        "#,
    );

    assert!(result.is_err());
}

#[test]
fn skill_manifest_defaults_absent_prompts_and_tool_args() {
    let manifest: SkillManifest = toml::from_str(
        r#"
        [skill]
        name = "lint"
        description = "Lint code"

        [[tools]]
        name = "check"
        description = "Check code"
        kind = "shell"
        command = "check"
        "#,
    )
    .unwrap();

    assert!(manifest.prompts.is_empty());
    assert_eq!(manifest.tools.len(), 1);
    assert!(manifest.tools[0].args.is_empty());
}

#[test]
fn skill_commands_parse_every_variant() {
    let list = <SkillTestCli as clap::Parser>::try_parse_from(["skill-test", "list"])
        .expect("list command should parse");
    assert!(matches!(list.command, SkillCommands::List));

    let audit =
        <SkillTestCli as clap::Parser>::try_parse_from(["skill-test", "audit", "owner/repo/skill"])
            .expect("audit command should parse");
    let SkillCommands::Audit { source } = audit.command else {
        panic!("expected audit command");
    };
    assert_eq!(source, "owner/repo/skill");

    let install = <SkillTestCli as clap::Parser>::try_parse_from([
        "skill-test",
        "install",
        "owner/repo/skill",
    ])
    .expect("install command should parse");
    let SkillCommands::Install { source } = install.command else {
        panic!("expected install command");
    };
    assert_eq!(source, "owner/repo/skill");

    let remove = <SkillTestCli as clap::Parser>::try_parse_from(["skill-test", "remove", "demo"])
        .expect("remove command should parse");
    let SkillCommands::Remove { name } = remove.command else {
        panic!("expected remove command");
    };
    assert_eq!(name, "demo");
}

#[test]
fn skill_commands_reject_missing_value_and_unknown_variant() {
    let missing = <SkillTestCli as clap::Parser>::try_parse_from(["skill-test", "audit"])
        .expect_err("audit source should be required");
    assert_eq!(missing.kind(), clap::error::ErrorKind::MissingRequiredArgument);

    let unknown = <SkillTestCli as clap::Parser>::try_parse_from(["skill-test", "update"])
        .expect_err("unknown command should be rejected");
    assert_eq!(unknown.kind(), clap::error::ErrorKind::InvalidSubcommand);
}
