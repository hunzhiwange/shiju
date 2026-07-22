use super::*;
use crate::app::agent::skill::types::{Skill, SkillTool, SkillsPromptInjectionMode};
use std::collections::HashMap;

#[test]
fn prompt_escapes_xml_and_compact_uses_relative_location() {
    let skill = Skill {
        name: "review&fix".to_string(),
        description: "Use <care>".to_string(),
        version: "0.1.0".to_string(),
        author: None,
        tags: Vec::new(),
        tools: vec![SkillTool {
            name: "private-tool".to_string(),
            description: "Must stay compact".to_string(),
            kind: "shell".to_string(),
            command: "unused".to_string(),
            args: HashMap::new(),
        }],
        prompts: vec!["Keep \"scope\" tight".to_string()],
        location: None,
    };
    let prompt = skills_to_prompt_with_mode(
        &[skill],
        std::path::Path::new("/workspace"),
        SkillsPromptInjectionMode::Compact,
    );

    assert!(prompt.contains("review&amp;fix"));
    assert!(prompt.contains("Use &lt;care&gt;"));
    assert!(prompt.contains("skills/review&amp;fix/SKILL.md"));
    assert!(!prompt.contains("<instructions>"));
    assert!(!prompt.contains("Keep &quot;scope&quot; tight"));
    assert!(!prompt.contains("<tools>"));
    assert!(!prompt.contains("private-tool"));
}

#[test]
fn xml_escape_handles_every_metacharacter_and_preserves_plain_text() {
    let mut escaped = String::new();

    append_xml_escaped(&mut escaped, "plain & < > \" ' 中文");

    assert_eq!(escaped, "plain &amp; &lt; &gt; &quot; &apos; 中文");
}

#[test]
fn xml_text_element_applies_indentation_escaping_and_newline() {
    let mut element = String::new();

    write_xml_text_element(&mut element, 3, "label", "A&B");

    assert_eq!(element, "   <label>A&amp;B</label>\n");
}

#[test]
fn empty_skill_list_produces_empty_prompt() {
    assert_eq!(skills_to_prompt(&[], std::path::Path::new("/workspace")), "");
}

#[test]
fn full_prompt_includes_instructions_tools_and_absolute_location() {
    let workspace = tempfile::tempdir().expect("workspace");
    let location = workspace.path().join("custom/SKILL.md");
    let skill = Skill {
        name: "build".to_string(),
        description: "Build things".to_string(),
        version: "0.1.0".to_string(),
        author: None,
        tags: Vec::new(),
        tools: vec![SkillTool {
            name: "fmt".to_string(),
            description: "Format".to_string(),
            kind: "shell".to_string(),
            command: "cargo fmt".to_string(),
            args: HashMap::new(),
        }],
        prompts: vec!["Run checks".to_string()],
        location: Some(location.clone()),
    };

    let prompt = skills_to_prompt(&[skill], workspace.path());

    assert!(prompt.contains("<instructions>"));
    assert!(prompt.contains("<tools>"));
    assert!(prompt.contains(location.to_string_lossy().as_ref()));
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

#[test]
fn full_prompt_omits_empty_instruction_and_tool_containers() {
    let skill = Skill {
        name: "metadata-only".to_string(),
        description: "No detailed content".to_string(),
        version: "0.1.0".to_string(),
        author: None,
        tags: Vec::new(),
        tools: Vec::new(),
        prompts: Vec::new(),
        location: None,
    };

    let prompt = skills_to_prompt(&[skill], std::path::Path::new("/workspace"));

    assert!(prompt.contains("Skill instructions and tool metadata are preloaded below."));
    assert!(prompt.contains("<location>/workspace/skills/metadata-only/SKILL.md</location>"));
    assert!(!prompt.contains("<instructions>"));
    assert!(!prompt.contains("<tools>"));
}

#[test]
fn full_prompt_preserves_multiple_skills_in_input_order() {
    let skill = |name: &str| Skill {
        name: name.to_string(),
        description: format!("{name} description"),
        version: "0.1.0".to_string(),
        author: None,
        tags: Vec::new(),
        tools: Vec::new(),
        prompts: Vec::new(),
        location: None,
    };

    let prompt =
        skills_to_prompt(&[skill("first"), skill("second")], std::path::Path::new("/workspace"));

    let first = prompt.find("<name>first</name>").expect("first skill");
    let second = prompt.find("<name>second</name>").expect("second skill");
    assert!(first < second);
    assert_eq!(prompt.matches("  <skill>\n").count(), 2);
    assert_eq!(prompt.matches("  </skill>\n").count(), 2);
    assert!(prompt.ends_with("</available_skills>"));
}

#[test]
fn skills_dir_appends_skills_component() {
    assert_eq!(
        skills_dir(std::path::Path::new("/workspace")),
        std::path::Path::new("/workspace/skills")
    );
}
