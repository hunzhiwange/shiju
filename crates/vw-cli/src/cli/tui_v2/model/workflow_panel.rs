#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowPanelItem {
    pub(crate) uuid: String,
    pub(crate) name: String,
    pub(crate) description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WorkflowPanel {
    pub(crate) items: Vec<WorkflowPanelItem>,
    pub(crate) selected: usize,
    pub(crate) error: Option<String>,
}
