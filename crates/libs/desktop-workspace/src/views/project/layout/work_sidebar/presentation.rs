/// 工作模式左栏的项目范围过滤。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkSidebarFilterScope {
    /// 显示所有最近项目中的会话。
    #[default]
    All,
    /// 只显示当前打开项目中的会话。
    CurrentProject,
}

/// 工作模式左栏的会话运行状态过滤。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkSidebarFilterStatus {
    /// 不限制运行状态。
    #[default]
    All,
    /// 当前正在处理请求。
    InProgress,
    /// 没有运行中请求和待处理队列。
    Completed,
    /// 当前未运行、但队列中仍有待处理请求。
    Pending,
}

/// 工作模式左栏的更新时间过滤。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkSidebarFilterTime {
    /// 不限制更新时间。
    #[default]
    All,
    /// 最近 24 小时内更新。
    LastDay,
    /// 最近 7 天内更新。
    LastSevenDays,
    /// 最近 30 天内更新。
    LastThirtyDays,
}

pub const WORK_SIDEBAR_RAIL_WIDTH: f32 = 54.0;
pub const WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS: usize = 10;
pub const WORK_SIDEBAR_LOAD_MORE_STEP: usize = 5;
/// 工作模式空间展开后默认展示的任务条数。
pub const WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE: usize = 20;
/// 工作模式空间任务每次加载更多的条数。
pub const WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP: usize = 20;

/// 工作模式左栏当前展示的列表。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkSidebarTab {
    /// 最近任务。
    #[default]
    Tasks,
    /// 最近空间。
    Spaces,
}
