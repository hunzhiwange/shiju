/// 工作模式右侧资源区的呈现模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkResourceMode {
    /// 展示当前项目与会话摘要。
    #[default]
    Overview,
    /// 展示当前项目的工作空间文件。
    WorkspaceFiles,
    /// 展示内置浏览器承载面。
    Browser,
}

pub use desktop_workspace::views::project::layout::work_sidebar::presentation::{
    WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS, WORK_SIDEBAR_LOAD_MORE_STEP, WORK_SIDEBAR_RAIL_WIDTH,
    WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE, WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP,
    WorkSidebarFilterScope, WorkSidebarFilterStatus, WorkSidebarFilterTime, WorkSidebarTab,
};

#[cfg(test)]
#[path="presentation_tests.rs"]
mod presentation_tests;
