//! 调度计划列表的只读视图模型。
//!
//! 面板保留列表、选中项与最近历史；计划的查询、启停和执行由运行时负责。

/// 一条调度计划的面板快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SchedulePanelItem {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) schedule: String,
    pub(crate) next_run: String,
    pub(crate) last_status: Option<String>,
    pub(crate) enabled: bool,
    pub(crate) history: Vec<String>,
    pub(crate) history_error: Option<String>,
}

/// 调度计划面板的列表、选中位置和加载错误。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SchedulePanel {
    pub(crate) items: Vec<SchedulePanelItem>,
    pub(crate) selected: usize,
    pub(crate) error: Option<String>,
}
