//! 成功加载技能后构造的 V2 调用结果。

use super::prompt::skill_prompt_content;
use super::{base_dir_url, skill_location};
use crate::app::agent::skills;
use crate::app::agent::tools::traits::{
    ToolCallResult, ToolCallTelemetry, ToolContextUpdate, ToolExtraMessage,
};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn build_invocation_result(
    skill: &skills::Skill,
    workspace_dir: &Path,
    args: Option<&str>,
    explicitly_requested: bool,
) -> Result<ToolCallResult, String> {
    if skill.disable_model_invocation && !explicitly_requested {
        return Err(format!("技能 \"{}\" 禁止模型调用", skill.name));
    }
    if !skill.paths.is_empty() {
        return Err(format!(
            "技能 \"{}\" 使用了未支持的 paths 条件：{}",
            skill.name,
            skill.paths.join(", ")
        ));
    }

    let location = skill_location(skill, workspace_dir);
    let directory = location.parent().ok_or_else(|| "技能路径无效".to_string())?;
    let base_dir = base_dir_url(directory);
    let content = format!(
        "<system-reminder>\nSkill \"{}\" is now active. Its base directory is {}. Relative paths in the skill resolve from that directory.\n\n{}\n</system-reminder>",
        skill.name,
        base_dir,
        skill_prompt_content(skill, args),
    );

    let mut context_updates = vec![
        update("skill.name", Value::String(skill.name.clone())),
        update("skill.base_dir", Value::String(base_dir.clone())),
    ];
    if !skill.allowed_tools.is_empty() {
        context_updates.push(update("skill.allowed_tools", json!(skill.allowed_tools)));
    }
    if let Some(model) = &skill.model {
        context_updates.push(update("skill.model", Value::String(model.clone())));
    }
    if let Some(effort) = &skill.effort {
        context_updates.push(update("skill.effort", Value::String(effort.clone())));
    }
    if let Some(hooks) = &skill.hooks {
        context_updates.push(update("skill.hooks", hooks.clone()));
    }
    if let Some(context) = &skill.context {
        context_updates.push(update("skill.context", Value::String(context.clone())));
        if context == "fork" {
            context_updates.push(update("skill.fork_context", Value::Bool(true)));
        }
    }

    Ok(ToolCallResult {
        data: json!({
            "success": true,
            "skill": skill.name,
            "base_dir": base_dir,
        }),
        model_result: Value::String(format!("Skill \"{}\" loaded.", skill.name)),
        context_updates,
        extra_messages: vec![ToolExtraMessage::meta_user_text(content)],
        telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
        ..ToolCallResult::default()
    })
}

fn update(key: &str, value: Value) -> ToolContextUpdate {
    ToolContextUpdate { key: key.to_string(), value }
}
