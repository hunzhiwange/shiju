//! 会话内按代理记录已发送的技能目录项。

use super::{InvokedSkillRecord, InvokedSkillReplay, Skill, listing};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// 可作为元用户消息发送的技能目录增量。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillListingReminder {
    pub(crate) content: String,
    pub(crate) skill_names: Vec<String>,
    pub(crate) estimated_tokens: usize,
    pub(crate) budget_tokens: usize,
}

/// 一个会话的技能目录发送状态；每个代理拥有独立的已发送集合。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct SkillListingSessionState {
    sent_skill_names_by_agent: HashMap<String, HashSet<String>>,
    invoked_skills_by_agent: HashMap<String, Vec<InvokedSkillRecord>>,
}

impl SkillListingSessionState {
    /// 为一个代理准备尚未发送的、受预算约束的目录增量。
    ///
    /// 仅当 Skill Tool 已启用时返回提醒；未能装入本次预算的项不会被标记为已发送。
    pub(crate) fn prepare(
        &mut self,
        agent_id: &str,
        skills: &[Skill],
        skill_tool_enabled: bool,
        context_window_tokens: usize,
    ) -> Option<SkillListingReminder> {
        if !skill_tool_enabled {
            return None;
        }

        let sent_skill_names =
            self.sent_skill_names_by_agent.entry(agent_id.to_string()).or_default();
        let reminder =
            listing::build_skill_listing(skills, sent_skill_names, context_window_tokens)?;
        sent_skill_names.extend(reminder.skill_names.iter().cloned());
        Some(reminder)
    }

    /// 登记一次成功的技能调用。相同稳定来源只保留一次，以免恢复时重复注入。
    pub(crate) fn register_invoked(&mut self, agent_id: &str, record: InvokedSkillRecord) -> bool {
        let records = self.invoked_skills_by_agent.entry(agent_id.to_string()).or_default();
        if records.iter().any(|existing| existing.stable_id() == record.stable_id()) {
            return false;
        }
        records.push(record);
        true
    }

    /// 为压缩、resume 或 fork 重新读取并按原调用顺序重放技能上下文。
    pub(crate) fn restore_invoked(
        &self,
        agent_id: &str,
    ) -> Result<Vec<InvokedSkillReplay>, String> {
        self.invoked_skills_by_agent
            .get(agent_id)
            .map(|records| records.iter().map(InvokedSkillRecord::replay).collect())
            .unwrap_or_else(|| Ok(Vec::new()))
    }

    /// fork 复制会话状态，使新会话可确定性地重放同一组已调用技能。
    pub(crate) fn fork(&self) -> Self {
        self.clone()
    }
}
