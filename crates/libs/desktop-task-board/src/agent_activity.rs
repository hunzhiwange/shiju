//! 桌面设置和执行器快照到领域活动投影的映射。
use desktop_settings_agents::state::DelegateAgentSettingsEntry;
use desktop_task_executor::executor::state::{WorktreeSlotSnapshot, WorktreeState};
use desktop_task_model::Task;
pub use desktop_task_model::agent_activity::AgentActivity;
use desktop_task_model::agent_activity::{ActivitySlot, AgentEntry};

pub fn agent_available(key: &str, entries: &[DelegateAgentSettingsEntry]) -> bool {
    let entries = entries
        .iter()
        .map(|e| AgentEntry { key: &e.key, label: &e.label, enabled: e.enabled })
        .collect::<Vec<_>>();
    desktop_task_model::agent_activity::agent_available(key, &entries)
}

pub fn project_activity(
    slots: &[WorktreeSlotSnapshot],
    entries: &[DelegateAgentSettingsEntry],
    tasks: &[Task],
) -> Vec<AgentActivity> {
    let entries = entries
        .iter()
        .map(|e| AgentEntry { key: &e.key, label: &e.label, enabled: e.enabled })
        .collect::<Vec<_>>();
    let slots = slots
        .iter()
        .map(|s| ActivitySlot {
            agent_id: s.agent_id.as_deref(),
            run_id: s.run_id.as_deref(),
            leased_task_id: s.leased_task_id.as_deref(),
            busy: s.state == WorktreeState::Busy,
        })
        .collect::<Vec<_>>();
    desktop_task_model::agent_activity::project_activity(&slots, &entries, tasks)
}
