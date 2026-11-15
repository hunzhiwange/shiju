//! 调度计划列表的只读视图模型。
//!
//! 面板保留列表、选中项与最近历史；计划的查询、启停和执行由运行时负责。

/// 一条调度计划的面板快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulePanelItem {
    pub id: String,
    pub name: String,
    pub schedule: String,
    pub next_run: String,
    pub last_status: Option<String>,
    pub enabled: bool,
    pub history: Vec<String>,
    pub history_error: Option<String>,
}

/// 调度计划面板的列表、选中位置和加载错误。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SchedulePanel {
    pub items: Vec<SchedulePanelItem>,
    pub selected: usize,
    pub error: Option<String>,
}
