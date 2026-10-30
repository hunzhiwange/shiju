//! 技能目录的预算选择。
//!
//! 此模块只处理技能元数据，绝不读取或输出 `SKILL.md` 正文。

use super::{Skill, session_state::SkillListingReminder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

pub(crate) const MAX_SKILL_DESCRIPTION_CHARS: usize = 250;
const CONTEXT_WINDOW_BUDGET_DIVISOR: usize = 100;
const CHARS_PER_ESTIMATED_TOKEN: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SkillListingEntry {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) location: String,
    pub(crate) fingerprint: String,
}

/// 输入由加载器和调用方能力策略验证；目录本身只解析来源路径，不读取正文。
pub(crate) fn skill_listing_entries(skills: &[Skill]) -> Vec<SkillListingEntry> {
    let mut seen_names = HashSet::new();
    let mut entries = skills
        .iter()
        // 保留加载器给出的来源优先级，不让后面的同名项覆盖禁用的高优先级项。
        .filter(|skill| seen_names.insert(skill.name.clone()))
        .filter(|skill| !skill.disable_model_invocation)
        .filter_map(|skill| {
            let location = skill.location.as_ref()?.canonicalize().ok()?;
            let location = location.to_str()?.to_string();
            let metadata = serde_json::json!({
                "name": skill.name, "description": skill.description,
                "location": location, "version": skill.version,
                "when_to_use": skill.when_to_use, "allowed_tools": skill.allowed_tools,
                "model": skill.model, "user_invocable": skill.user_invocable,
                "hooks": skill.hooks, "context": skill.context, "paths": skill.paths,
                "effort": skill.effort, "argument_hint": skill.argument_hint,
            });
            Some(SkillListingEntry {
                name: skill.name.clone(),
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
pub(crate) fn build_skill_listing(
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
        let content = super::prompt::format_skill_listing_reminder(&candidate);
        if estimate_tokens(&content) > budget_tokens {
            deferred_skill_names.push(entry.name.clone());
            continue;
        }
        selected = candidate;
    }

    if selected.is_empty() {
        return None;
    }

    let content = super::prompt::format_skill_listing_reminder(&selected);
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
