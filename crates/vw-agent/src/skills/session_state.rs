//! 会话内按代理记录已发送的技能目录项。

use super::{InvokedSkillRecord, InvokedSkillReplay, Skill, listing};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 可作为 developer 消息发送的技能目录增量。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillListingReminder {
    pub(crate) content: String,
    pub(crate) skill_names: Vec<String>,
    pub(crate) entries: Vec<listing::SkillListingEntry>,
    pub(crate) deferred_skill_names: Vec<String>,
    pub(crate) estimated_tokens: usize,
    pub(crate) budget_tokens: usize,
}

/// 一个会话的技能目录发送状态；每个代理拥有独立的已发送集合。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct SkillListingSessionState {
    #[serde(default)]
    sent_skill_entries_by_agent: HashMap<String, HashMap<String, listing::SkillListingEntry>>,
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
            self.sent_skill_entries_by_agent.remove(agent_id);
            return None;
        }

        let entries = listing::skill_listing_entries(skills);
        let current = entries.iter().map(|entry| (&entry.name, entry)).collect::<HashMap<_, _>>();
        let sent_entries =
            self.sent_skill_entries_by_agent.entry(agent_id.to_string()).or_default();
        sent_entries.retain(|name, entry| {
            let Some(verified) = current.get(name) else { return false };
            if verified.fingerprint != entry.fingerprint {
                return false;
            }
            // 恢复的状态只决定去重，developer 内容始终取自本次验证的元数据。
            *entry = (*verified).clone();
            true
        });
        let reminder = listing::build_skill_listing(&entries, sent_entries, context_window_tokens)?;
        for entry in &reminder.entries {
            sent_entries.insert(entry.name.clone(), entry.clone());
        }
        Some(reminder)
    }

    /// 重新装配已发送的可信目录，避免临时消息在下一次 provider 请求消失。
    /// 每次 prepare 先移除失效项；预算只限制新加入的增量，完整上下文由请求预算治理。
    pub(crate) fn sent_catalog(&self, agent_id: &str) -> Option<(String, Vec<String>)> {
        let mut entries =
            self.sent_skill_entries_by_agent.get(agent_id)?.values().cloned().collect::<Vec<_>>();
        if entries.is_empty() {
            return None;
        }
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        let names = entries.iter().map(|entry| entry.name.clone()).collect();
        Some((super::prompt::format_skill_listing_reminder(&entries), names))
    }

    /// 登记一次成功的技能调用。相同稳定来源只保留一次，以免恢复时重复注入。
    pub(crate) fn register_invoked(&mut self, agent_id: &str, record: InvokedSkillRecord) -> bool {
        let records = self.invoked_skills_by_agent.entry(agent_id.to_string()).or_default();
        if let Some(existing) =
            records.iter_mut().find(|existing| existing.stable_id() == record.stable_id())
        {
            if *existing == record {
                return false;
            }
            *existing = record;
            return true;
        }
        records.push(record);
        true
    }

    pub(crate) fn invoked_records(&self, agent_id: &str) -> &[InvokedSkillRecord] {
        self.invoked_skills_by_agent.get(agent_id).map(Vec::as_slice).unwrap_or_default()
    }

    /// The persisted source is an identity, not a permission grant.
    pub(crate) fn validate_invoked_sources(
        &self,
        agent_id: &str,
        skills: &[Skill],
    ) -> Result<(), String> {
        for record in self.invoked_records(agent_id) {
            if !skills.iter().any(|skill| record.is_authorized_by(skill)) {
                return Err(format!(
                    "已调用技能不存在、已禁用或不在当前权限范围内：{}",
                    record.skill_name
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn reset_catalog(&mut self, agent_id: &str) {
        self.sent_skill_entries_by_agent.remove(agent_id);
    }

    pub(crate) fn preload_explicit(
        &mut self,
        agent_id: &str,
        skills: &[Skill],
        names: &std::collections::BTreeSet<String>,
    ) -> Result<(), String> {
        for skill in skills
            .iter()
            .filter(|skill| names.contains(&skill.invocation_name().trim().to_lowercase()))
        {
            if !skill.user_invocable {
                return Err(format!("技能已禁止用户调用：{}", skill.name));
            }
            let source =
                skill.location.as_deref().ok_or_else(|| format!("技能缺少来源：{}", skill.name))?;
            let record = InvokedSkillRecord::new(skill.invocation_name(), source, None)?
                .with_explicit_request(true);
            record.replay()?;
            self.register_invoked(agent_id, record);
        }
        Ok(())
    }

    /// 在首个模型请求前登记配置的默认技能。
    ///
    /// 正文仍通过 `InvokedSkillRecord::replay` 从真实来源读取；因此压缩、resume
    /// 和 fork 与普通成功调用共用同一条恢复链。禁止模型调用的技能绝不预加载。
    pub(crate) fn preload_defaults(
        &mut self,
        agent_id: &str,
        skills: &[Skill],
        default_skill_names: &std::collections::BTreeSet<String>,
    ) -> Result<(), String> {
        for name in default_skill_names {
            let mut candidates = skills.iter().filter(|skill| {
                skill.invocation_aliases().iter().any(|alias| alias.trim().to_lowercase() == *name)
            });
            let skill = candidates
                .next()
                .ok_or_else(|| format!("默认技能不存在、已禁用或不在当前权限范围内：{name}"))?;
            if candidates.next().is_some() {
                return Err(format!("默认技能名称有歧义，请使用完整插件身份限定调用：{name}"));
            }
            if skill.disable_model_invocation {
                return Err(format!("默认技能已禁止模型调用：{}", skill.name));
            }
            let Some(source) = skill.location.as_deref() else {
                return Err(format!("默认技能缺少来源：{}", skill.name));
            };
            let record = InvokedSkillRecord::new(skill.invocation_name(), source, None)?;
            record.replay()?;
            self.register_invoked(agent_id, record);
        }
        Ok(())
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
