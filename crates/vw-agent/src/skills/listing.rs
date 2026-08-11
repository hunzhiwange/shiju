//! 技能目录的预算选择。
//!
//! 此模块只处理技能元数据，绝不读取或输出 `SKILL.md` 正文。

use super::{Skill, session_state::SkillListingReminder};
use std::collections::HashSet;

pub(crate) const MAX_SKILL_DESCRIPTION_CHARS: usize = 250;
const CONTEXT_WINDOW_BUDGET_DIVISOR: usize = 100;
const CHARS_PER_ESTIMATED_TOKEN: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillListingEntry {
    pub(crate) name: String,
    pub(crate) description: String,
}

/// 以确定性顺序选取尚未发送且可由模型调用的技能。
pub(crate) fn build_skill_listing(
    skills: &[Skill],
    sent_skill_names: &HashSet<String>,
    context_window_tokens: usize,
) -> Option<SkillListingReminder> {
    let budget_tokens = context_window_tokens / CONTEXT_WINDOW_BUDGET_DIVISOR;
    if budget_tokens == 0 {
        return None;
    }

    let mut seen_names = HashSet::new();
    let mut candidates = skills
        .iter()
        .filter(|skill| !skill.disable_model_invocation)
        .filter(|skill| !sent_skill_names.contains(&skill.name))
        .filter(|skill| seen_names.insert(skill.name.clone()))
        .map(|skill| SkillListingEntry {
            name: skill.name.clone(),
            description: truncate_description(&skill.description),
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.name.cmp(&right.name));

    let mut selected = Vec::new();
    for entry in candidates {
        let mut candidate = selected.clone();
        candidate.push(entry);
        let content = super::prompt::format_skill_listing_reminder(&candidate);
        if estimate_tokens(&content) > budget_tokens {
            break;
        }
        selected = candidate;
    }

    if selected.is_empty() {
        return None;
    }

    let content = super::prompt::format_skill_listing_reminder(&selected);
    Some(SkillListingReminder {
        skill_names: selected.into_iter().map(|entry| entry.name).collect(),
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
