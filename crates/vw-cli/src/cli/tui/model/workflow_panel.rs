//! 工作流列表的只读视图模型。
//!
//! 此处不解析或执行工作流，只承载面板渲染所需的稳定标识与摘要。

/// 一条工作流的面板摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowPanelItem {
    pub(crate) uuid: String,
    pub(crate) name: String,
    pub(crate) description: String,
}

/// 工作流面板的列表、选中位置和查询错误。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WorkflowPanel {
    pub(crate) items: Vec<WorkflowPanelItem>,
    pub(crate) selected: usize,
    pub(crate) error: Option<String>,
}
