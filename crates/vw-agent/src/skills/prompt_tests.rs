use super::*;
use crate::app::agent::config::SkillsPromptInjectionMode;
use crate::app::agent::skills::types::{Skill, SkillTool};
use std::collections::HashMap;

#[test]
fn compact_prompt_escapes_xml_and_omits_full_sections() {
    let skill = Skill {
        name: "docs&review".to_string(),
        description: "Read <docs>".to_string(),
        version: "0.1.0".to_string(),
        author: None,
        tags: Vec::new(),
        tools: Vec::new(),
        prompts: vec!["full instructions".to_string()],
        location: None,
    };
    let prompt = skills_to_prompt_with_mode(
        &[skill],
        std::path::Path::new("/workspace"),
        SkillsPromptInjectionMode::Compact,
    );

    assert!(prompt.contains("docs&amp;review"));
    assert!(prompt.contains("Read &lt;docs&gt;"));
    assert!(prompt.contains("skills/docs&amp;review/SKILL.md"));
    assert!(!prompt.contains("<instructions>"));
}

#[test]
fn full_prompt_includes_tools_instructions_and_absolute_locations() {
    let workspace = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let skill = Skill {
        name: "quote\"skill".to_string(),
        description: "Use 'quotes' safely".to_string(),
        version: "0.1.0".to_string(),
        author: None,
        tags: Vec::new(),
        tools: vec![SkillTool {
            name: "lint<all>".to_string(),
            description: "Run checks > quick".to_string(),
            kind: "shell&script".to_string(),
            command: "cargo test".to_string(),
            args: HashMap::new(),
        }],
        prompts: vec!["Always check \"edge\" cases".to_string()],
        location: Some(outside.path().join("quote-skill").join("SKILL.md")),
    };

    let prompt = skills_to_prompt(&[skill], workspace.path());

    assert!(prompt.contains("Skill instructions and tool metadata are preloaded"));
    assert!(prompt.contains("quote&quot;skill"));
    assert!(prompt.contains("Use &apos;quotes&apos; safely"));
    assert!(prompt.contains("Always check &quot;edge&quot; cases"));
    assert!(prompt.contains("lint&lt;all&gt;"));
    assert!(prompt.contains("Run checks &gt; quick"));
    assert!(prompt.contains("shell&amp;script"));
    assert!(prompt.contains(&outside.path().display().to_string()));
}

#[test]
fn prompt_returns_empty_string_for_no_skills() {
    assert_eq!(skills_to_prompt(&[], std::path::Path::new("/workspace")), "");
}

#[test]
fn compact_prompt_keeps_absolute_location_outside_workspace() {
    let skill = Skill {
        name: "shared".to_string(),
        description: "Shared skill".to_string(),
        version: "0.1.0".to_string(),
        author: None,
        tags: Vec::new(),
        tools: Vec::new(),
        prompts: Vec::new(),
        location: Some(std::path::PathBuf::from("/opt/shared/SKILL.md")),
    };

    let prompt = skills_to_prompt_with_mode(
        &[skill],
        std::path::Path::new("/workspace"),
        SkillsPromptInjectionMode::Compact,
    );

    assert!(prompt.contains("<location>/opt/shared/SKILL.md</location>"));
}
