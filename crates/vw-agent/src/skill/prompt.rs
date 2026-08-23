//! 技能清单的提示词渲染与安全转义。
//!
//! 根据注入模式输出结构化技能元数据，并对 XML 文本和本地路径进行规范化。

use crate::app::agent::skill::types::{Skill, SkillsPromptInjectionMode};
use std::path::{Path, PathBuf};

fn append_xml_escaped(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
}

fn write_xml_text_element(out: &mut String, indent: usize, tag: &str, value: &str) {
    for _ in 0..indent {
        out.push(' ');
    }
    out.push('<');
    out.push_str(tag);
    out.push('>');
    append_xml_escaped(out, value);
    out.push_str("</");
    out.push_str(tag);
    out.push_str(">\n");
}

fn resolve_skill_location(skill: &Skill, workspace_dir: &Path) -> PathBuf {
    skill
        .location
        .clone()
        .unwrap_or_else(|| workspace_dir.join("skills").join(&skill.name).join("SKILL.md"))
}

fn render_skill_location(skill: &Skill, workspace_dir: &Path, prefer_relative: bool) -> String {
    let location = resolve_skill_location(skill, workspace_dir);
    if prefer_relative {
        if let Ok(relative) = location.strip_prefix(workspace_dir) {
            return relative.display().to_string();
        }
    }
    location.display().to_string()
}

/// Build the "Available Skills" system prompt section with full skill instructions.
/// 使用默认兼容模式把技能集合渲染为提示词。
pub fn skills_to_prompt(skills: &[Skill], workspace_dir: &Path) -> String {
    skills_to_prompt_with_mode(skills, workspace_dir, SkillsPromptInjectionMode::Full)
}

/// Build the "Available Skills" system prompt section with configurable verbosity.
/// 按指定注入模式渲染技能元数据。
///
/// XML 模式会转义所有动态文本；路径仅在确实位于工作区内时渲染为相对形式。
pub fn skills_to_prompt_with_mode(
    skills: &[Skill],
    workspace_dir: &Path,
    mode: SkillsPromptInjectionMode,
) -> String {
    use std::fmt::Write;

    if skills.is_empty() {
        return String::new();
    }

    let mut prompt = match mode {
        SkillsPromptInjectionMode::Full => String::from(
            "## Available Skills\n\n\
             Skill instructions and tool metadata are preloaded below.\n\
             Follow these instructions directly; do not read skill files at runtime unless the user asks.\n\
             Unless the user explicitly asks for a specific skill to be executed in a different way, that skill's instructions are mandatory. Follow its documented workflow exactly and completely, including every required sequence, tool, check, and output.\n\
             A user-requested variation applies only to the identified skill and the specifically requested parts; every other instruction remains mandatory. Do not infer an override from general preferences, goals, or suggested shortcuts.\n\
             Without such an explicit request, do not simplify, omit, substitute, short-circuit, or bypass any required step, even if another approach seems faster or equivalent.\n\
             If a required step cannot be executed, stop the skill workflow and report the exact blocker; do not silently improvise around it.\n\n\
             <available_skills>\n",
        ),
        SkillsPromptInjectionMode::Compact => String::from(
            "## Available Skills\n\n\
             Skill summaries are preloaded below to keep context compact.\n\
             Before taking task actions for a skill that applies or is invoked, read its complete instruction file from `location`.\n\
             Unless the user explicitly asks for a specific skill to be executed in a different way, that skill's loaded instructions are mandatory. Follow the documented workflow exactly and completely, including every required sequence, tool, check, and output.\n\
             A user-requested variation applies only to the identified skill and the specifically requested parts; every other instruction remains mandatory. Do not infer an override from general preferences, goals, or suggested shortcuts.\n\
             Without such an explicit request, do not simplify, omit, substitute, short-circuit, or bypass any required step, even if another approach seems faster or equivalent.\n\
             If a required step cannot be executed, stop the skill workflow and report the exact blocker; do not silently improvise around it.\n\n\
             <available_skills>\n",
        ),
    };

    for skill in skills {
        let _ = writeln!(prompt, "  <skill>");
        write_xml_text_element(&mut prompt, 4, "name", &skill.name);
        write_xml_text_element(&mut prompt, 4, "description", &skill.description);
        let location = render_skill_location(
            skill,
            workspace_dir,
            matches!(mode, SkillsPromptInjectionMode::Compact),
        );
        write_xml_text_element(&mut prompt, 4, "location", &location);

        if matches!(mode, SkillsPromptInjectionMode::Full) {
            if !skill.prompts.is_empty() {
                let _ = writeln!(prompt, "    <instructions>");
                for instruction in &skill.prompts {
                    write_xml_text_element(&mut prompt, 6, "instruction", instruction);
                }
                let _ = writeln!(prompt, "    </instructions>");
            }

            if !skill.tools.is_empty() {
                let _ = writeln!(prompt, "    <tools>");
                for tool in &skill.tools {
                    let _ = writeln!(prompt, "      <tool>");
                    write_xml_text_element(&mut prompt, 8, "name", &tool.name);
                    write_xml_text_element(&mut prompt, 8, "description", &tool.description);
                    write_xml_text_element(&mut prompt, 8, "kind", &tool.kind);
                    let _ = writeln!(prompt, "      </tool>");
                }
                let _ = writeln!(prompt, "    </tools>");
            }
        }

        let _ = writeln!(prompt, "  </skill>");
    }

    prompt.push_str("</available_skills>");
    prompt
}

/// Get the skills directory path
/// 返回工作区内约定的 `.agents/skills` 技能目录。
pub fn skills_dir(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("skills")
}
#[cfg(test)]
#[path = "prompt_tests.rs"]
mod prompt_tests;
