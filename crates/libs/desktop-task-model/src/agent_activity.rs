//! 员工只是既有逻辑工位及智能体运行的展示投影，不分配执行资源。
use crate::{TASK_AGENT_MAIN, Task};
/// 活动投影实际读取的智能体设置。
pub struct AgentEntry<'a> {
    pub key: &'a str,
    pub label: &'a str,
    pub enabled: bool,
}

/// 活动投影实际读取的逻辑槽位。
pub struct ActivitySlot<'a> {
    pub agent_id: Option<&'a str>,
    pub run_id: Option<&'a str>,
    pub leased_task_id: Option<&'a str>,
    pub busy: bool,
}

#[derive(Debug)]
pub struct AgentActivity {
    pub key: Option<String>,
    pub label: String,
    pub available: bool,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub current_task_count: usize,
}

pub fn agent_available(key: &str, entries: &[AgentEntry<'_>]) -> bool {
    !config_agent::is_retired_builtin_agent_key(key)
        && (key == TASK_AGENT_MAIN || entries.iter().any(|e| e.key == key && e.enabled))
}

pub fn project_activity(
    slots: &[ActivitySlot<'_>],
    entries: &[AgentEntry<'_>],
    tasks: &[Task],
) -> Vec<AgentActivity> {
    slots
        .iter()
        .map(|slot| {
            let task = slot
                .leased_task_id
                .as_deref()
                .and_then(|id| tasks.iter().find(|task| task.id == id));
            // A recorded run keeps its original key even when its definition disappears.
            let key = slot.agent_id.map(str::to_owned).or_else(|| task.and_then(|t| t.agent.clone()));
            let available = key.as_deref().is_some_and(|key| agent_available(key, entries));
            let label = match key.as_deref() {
                Some(TASK_AGENT_MAIN) => "Main".to_string(),
                Some(key) => entries
                    .iter()
                    .find(|e| e.key == key)
                    .map(|e| e.label.to_string())
                    .unwrap_or_else(|| key.to_string()),
                None => String::new(),
            };
            AgentActivity {
                key,
                label,
                available,
                task_id: slot.leased_task_id.map(str::to_owned),
                run_id: slot.run_id.map(str::to_owned),
                current_task_count: usize::from(
                    slot.busy && slot.leased_task_id.is_some(),
                ),
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "agent_activity_tests.rs"]
mod tests;
