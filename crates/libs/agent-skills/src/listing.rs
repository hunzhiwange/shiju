//! 技能目录的预算选择。
//!
//! 此模块只处理技能元数据，绝不读取或输出 `SKILL.md` 正文。

use crate::types::Skill;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

pub const MAX_SKILL_DESCRIPTION_CHARS: usize = 250;
const CONTEXT_WINDOW_BUDGET_DIVISOR: usize = 100;
const CHARS_PER_ESTIMATED_TOKEN: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillListingEntry {
    pub name: String,
    pub description: String,
    pub location: String,
    pub fingerprint: String,
}

/// 输入由加载器和调用方能力策略验证；目录本身只解析来源路径，不读取正文。
pub fn skill_listing_entries(skills: &[Skill]) -> Vec<SkillListingEntry> {
    let mut seen_names = HashSet::new();
    let mut entries = skills
        .iter()
        // 保留加载器给出的来源优先级，不让后面的同名项覆盖禁用的高优先级项。
        .filter(|skill| seen_names.insert(skill.invocation_name()))
        .filter(|skill| !skill.disable_model_invocation)
        .filter_map(|skill| {
            let location = skill.location.as_ref()?.canonicalize().ok()?;
            let location = location.to_str()?.to_string();
            let metadata = serde_json::json!({
                "name": skill.invocation_name(), "description": skill.description,
                "location": location, "version": skill.version,
                "when_to_use": skill.when_to_use, "allowed_tools": skill.allowed_tools,
                "model": skill.model, "user_invocable": skill.user_invocable,
                "hooks": skill.hooks, "context": skill.context, "paths": skill.paths,
                "effort": skill.effort, "argument_hint": skill.argument_hint,
            });
            Some(SkillListingEntry {
                name: skill.invocation_name(),
                description: truncate_description(&skill.description),
                location,
                fingerprint: hex::encode(Sha256::digest(metadata.to_string().as_bytes())),
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

/// 选择本请求能容纳的元数据增量；超出预算的项保持未发送，供下一请求继续。
pub fn build_skill_listing(
    entries: &[SkillListingEntry],
    sent_entries: &HashMap<String, SkillListingEntry>,
    context_window_tokens: usize,
) -> Option<SkillListingReminder> {
    let budget_tokens = context_window_tokens / CONTEXT_WINDOW_BUDGET_DIVISOR;
    if budget_tokens == 0 {
        return None;
    }

    let mut selected = Vec::new();
    let mut deferred_skill_names = Vec::new();
    for entry in entries.iter().filter(|entry| !sent_entries.contains_key(&entry.name)) {
        let mut candidate = selected.clone();
        candidate.push(entry.clone());
        let content = format_skill_listing_reminder(&candidate);
        if estimate_tokens(&content) > budget_tokens {
            deferred_skill_names.push(entry.name.clone());
            continue;
        }
        selected = candidate;
    }

    if selected.is_empty() {
        return None;
    }

    let content = format_skill_listing_reminder(&selected);
    Some(SkillListingReminder {
        skill_names: selected.iter().map(|entry| entry.name.clone()).collect(),
        entries: selected,
        deferred_skill_names,
        estimated_tokens: estimate_tokens(&content),
        budget_tokens,
        content,
    })
}

fn truncate_description(description: &str) -> String {
    description.chars().take(MAX_SKILL_DESCRIPTION_CHARS).collect()
}

fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(CHARS_PER_ESTIMATED_TOKEN)
}

/// 可作为 developer 消息发送的技能目录增量。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillListingReminder {
    pub content: String,
    pub skill_names: Vec<String>,
    pub entries: Vec<SkillListingEntry>,
    pub deferred_skill_names: Vec<String>,
    pub estimated_tokens: usize,
    pub budget_tokens: usize,
}


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

/// 写入一个带有缩进的 XML 文本元素
///
/// 此函数生成格式化的 XML 元素，包含指定的缩进、标签名和文本内容。
/// 文本内容会自动进行 XML 特殊字符转义。
///
/// # 参数
///
/// - `out`：输出字符串的可变引用
/// - `indent`：缩进空格数，用于格式化输出
/// - `tag`：XML 标签名称
/// - `value`：元素的文本内容（将被自动转义）
///
/// # 输出格式
///
/// ```xml
/// [indent个空格]<tag>转义后的value</tag>\n
/// ```
///
/// # 示例
///
/// ```ignore
/// let mut output = String::new();
/// write_xml_text_element(&mut output, 4, "name", "Test & Demo");
/// // 输出: "    <name>Test &amp; Demo</name>\n"
/// ```
fn write_xml_text_element(out: &mut String, indent: usize, tag: &str, value: &str) {
    // 添加缩进空格
    for _ in 0..indent {
        out.push(' ');
    }
    // 写入开始标签
    out.push('<');
    out.push_str(tag);
    out.push('>');
    // 写入转义后的文本内容
    append_xml_escaped(out, value);
    // 写入结束标签
    out.push_str("</");
    out.push_str(tag);
    out.push_str(">\n");
}

/// 渲染可信 developer 目录；完整正文仍由 skill 工具按需提供为 user 上下文。
pub fn format_skill_listing_reminder(entries: &[SkillListingEntry]) -> String {
    let mut reminder = String::from(
        "<skills_instructions>\n\
         Use the skill tool to load a skill's complete instructions before executing it.\n\
         If the user explicitly names a skill, you must use it. Select skills when their description matches the task.\n\
         Follow the loaded workflow; resolve relative resources from the skill file's directory.\n\
         The catalog contains metadata only, not preloaded instructions.\n\
         <available_skills>\n",
    );
    for entry in entries {
        reminder.push_str("  <skill>\n");
        write_xml_text_element(&mut reminder, 4, "name", &entry.name);
        write_xml_text_element(&mut reminder, 4, "description", &entry.description);
        write_xml_text_element(&mut reminder, 4, "location", &entry.location);
        reminder.push_str("  </skill>\n");
    }
    reminder.push_str("</available_skills>\n</skills_instructions>");
    reminder
}
