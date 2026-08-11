//! `SkillTool` 对模型暴露的提示文本。

use super::{accessible_skills, file_url, skill_file_id, skill_location, xml_escape};
use crate::app::agent::{config, skills};
use std::path::Path;

pub(super) const SKILL_EXECUTION_POLICY: &str = r#"<skill_execution_policy>
严格执行要求：
1. 除非用户明确要求跳过、修改或重排某个具体步骤，否则必须按本技能规定的顺序完整执行全部工作流。
2. 不得静默跳过、合并、替代或虚构任何步骤；不得因为已有部分信息而省略技能要求的验证步骤。
3. 每完成一步，必须核对该步骤的要求和实际结果，再进入下一步。
4. 如果任一步骤无法执行，必须立即停止并明确报告阻塞原因；不得继续后续步骤，也不得声称任务已经完成。
5. 输出最终结果前，必须逐项确认所有必需步骤均已完成，并具有实际执行依据。
</skill_execution_policy>"#;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn build_description_text(workspace_dir: &Path, root_config: &config::Config) -> String {
    let skills = accessible_skills(skills::load_skills_with_config(workspace_dir, root_config));
    if skills.is_empty() {
        return "Load a specialized skill for domain-specific guidance.\n\n<BLOCKING REQUIREMENT>When a task matches an available skill, you MUST call this tool before responding. Do not answer from memory or proceed with the task until the skill has been loaded.</BLOCKING REQUIREMENT>\n\nNo skills are available."
            .to_string();
    }

    let mut lines = vec![
        "Load a specialized skill for domain-specific guidance and workflow.".to_string(),
        String::new(),
        "<BLOCKING REQUIREMENT>When a task matches an available skill, you MUST call this tool before responding. Do not answer from memory or proceed with the task until the skill has been loaded.</BLOCKING REQUIREMENT>".to_string(),
        "The full SKILL.md is injected as a meta user message only after a successful call."
            .to_string(),
        String::new(),
        "<available_skills>".to_string(),
    ];

    for skill in skills {
        lines.push("  <skill>".to_string());
        lines.push(format!("    <name>{}</name>", xml_escape(&skill.name)));
        if let Some(id) = skill_file_id(&skill)
            && id != skill.name
        {
            lines.push(format!("    <id>{}</id>", xml_escape(&id)));
        }
        lines.push(format!("    <description>{}</description>", xml_escape(&skill.description)));
        let location = skill_location(&skill, workspace_dir);
        lines.push(format!("    <location>{}</location>", file_url(&location)));
        lines.push("  </skill>".to_string());
    }
    lines.push("</available_skills>".to_string());
    lines.join("\n")
}

#[cfg(target_arch = "wasm32")]
pub(super) fn build_description_text(
    _workspace_dir: &Path,
    _root_config: &config::Config,
) -> String {
    "Load a specialized skill with domain-specific instructions and workflow context.".to_string()
}

pub(super) fn skill_prompt_content(skill: &skills::Skill, args: Option<&str>) -> String {
    let content = skill
        .prompts
        .iter()
        .filter(|prompt| !prompt.trim().is_empty())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n\n");
    let content = if content.is_empty() { "该技能没有内联指令。" } else { &content };
    format!("{SKILL_EXECUTION_POLICY}\n\n{}", substitute_arguments(content, args))
}

fn substitute_arguments(content: &str, args: Option<&str>) -> String {
    let args = args.unwrap_or_default();
    content.replace("$ARGUMENTS", args)
}
