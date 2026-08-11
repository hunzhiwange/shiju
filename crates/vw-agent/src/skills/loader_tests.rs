use super::*;
use crate::app::agent::skills::types::SkillLoadMode;

#[test]
fn markdown_metadata_parser_uses_frontmatter_when_present() {
    let metadata = parse_markdown_skill_metadata(
        "---\nname: custom\ndescription: Front matter description\n---\n# Ignored\n",
    );

    assert_eq!(metadata.display_name.as_deref(), Some("custom"));
    assert_eq!(metadata.description.as_deref(), Some("Front matter description"));
}

#[test]
fn markdown_metadata_parser_handles_body_fallbacks_and_invalid_frontmatter() {
    let fallback = parse_markdown_skill_metadata("# Heading\n\nBody description.\n");
    assert_eq!(fallback.display_name, None);
    assert_eq!(fallback.description.as_deref(), Some("Body description."));

    let invalid = parse_markdown_skill_metadata("---\nname: [broken\n---\n# Title\nFallback\n");
    assert_eq!(invalid.display_name, None);
    assert_eq!(invalid.description.as_deref(), Some("Fallback"));

    let unterminated = parse_markdown_skill_metadata("---\ndescription: no close\n# Body\n");
    assert_eq!(unterminated.description.as_deref(), Some("description: no close"));
}

#[test]
fn read_markdown_skill_metadata_reads_from_disk() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("SKILL.md");
    std::fs::write(&path, "---\nname: disk\ndescription: From disk\n---\n# Ignored\n").unwrap();

    let metadata = read_markdown_skill_metadata(&path).unwrap();

    assert_eq!(metadata.display_name.as_deref(), Some("disk"));
    assert_eq!(metadata.description.as_deref(), Some("From disk"));
}

#[test]
fn open_skill_markdown_metadata_only_omits_prompt_body() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("browser.md");
    std::fs::write(&path, "# Browser\n\nUse browser safely.").unwrap();

    let skill = load_open_skill_md(&path, SkillLoadMode::MetadataOnly).unwrap();
    assert_eq!(skill.name, "browser");
    assert!(skill.prompts.is_empty());
    assert!(skill.tags.contains(&"open-skills".to_string()));
}

#[test]
fn open_skill_markdown_full_loads_prompt_body_and_defaults_description() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("helper.markdown");
    std::fs::write(&path, "# Helper\n").unwrap();

    let skill = load_open_skill_md(&path, SkillLoadMode::Full).unwrap();

    assert_eq!(skill.name, "helper");
    assert_eq!(skill.description, "No description");
    assert_eq!(skill.prompts, vec!["# Helper\n"]);
    assert_eq!(skill.author.as_deref(), Some("besoeasy/open-skills"));
}

#[test]
fn load_skills_from_directory_skips_disabled_and_missing_directories() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("missing");
    assert!(
        load_skills_from_directory(
            &missing,
            SkillLoadMode::Full,
            &vw_config_types::skills::SkillSecurityConfig::default(),
        )
        .is_empty()
    );

    let disabled = dir.path().join("disabled");
    std::fs::create_dir_all(&disabled).unwrap();
    std::fs::write(disabled.join("SKILL.md"), "# Disabled\nShould not load\n").unwrap();
    std::fs::write(crate::app::agent::skills::local_skill_disabled_marker_path(&disabled), "")
        .unwrap();

    assert!(
        load_skills_from_directory(
            dir.path(),
            SkillLoadMode::Full,
            &vw_config_types::skills::SkillSecurityConfig::default(),
        )
        .is_empty()
    );
}

#[test]
fn markdown_description_helpers_return_defaults_and_read_errors() {
    assert_eq!(extract_description("# Only heading\n\n## Another heading"), "No description");

    let dir = tempfile::tempdir().expect("temp dir");
    let heading_only = dir.path().join("SKILL.md");
    std::fs::write(&heading_only, "# Only heading\n").unwrap();
    assert_eq!(extract_description_from_markdown(&heading_only).unwrap(), "No description");

    let missing = dir.path().join("missing.md");
    assert!(extract_description_from_markdown(&missing).is_err());
}

#[test]
fn directory_loader_rejects_file_path_as_skills_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("skills");
    std::fs::write(&file, "not a directory").unwrap();

    assert!(
        load_skills_from_directory(
            &file,
            SkillLoadMode::Full,
            &vw_config_types::skills::SkillSecurityConfig::default(),
        )
        .is_empty()
    );
}

#[test]
fn markdown_loader_uses_unknown_name_without_directory_component() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("SKILL.md");
    std::fs::write(&path, "# Skill\nUseful description.\n").unwrap();

    let skill = load_skill_md(&path, Path::new(""), SkillLoadMode::MetadataOnly).unwrap();

    assert_eq!(skill.name, "unknown");
    assert_eq!(skill.description, "Useful description.");
    assert!(skill.prompts.is_empty());
}

#[test]
fn markdown_loader_keeps_invocation_metadata_in_metadata_only_mode() {
    let dir = tempfile::tempdir().expect("temp dir");
    let skill_dir = dir.path().join("review");
    std::fs::create_dir_all(&skill_dir).unwrap();
    let path = skill_dir.join("SKILL.md");
    std::fs::write(
        &path,
        "---\nallowed-tools: Read\ndisable-model-invocation: true\nuser-invocable: false\npaths: src/*.rs\nargument-hint: '[file]'\n---\n# Review\n",
    )
    .unwrap();

    let skill = load_skill_md(&path, &skill_dir, SkillLoadMode::MetadataOnly).unwrap();

    assert_eq!(skill.allowed_tools, vec!["Read"]);
    assert!(skill.disable_model_invocation);
    assert!(!skill.user_invocable);
    assert_eq!(skill.paths, vec!["src/*.rs"]);
    assert_eq!(skill.argument_hint.as_deref(), Some("[file]"));
    assert!(skill.prompts.is_empty());
}
