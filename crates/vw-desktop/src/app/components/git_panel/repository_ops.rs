//! Git 面板的协议类型和界面状态；不执行仓库操作。
pub use vw_gateway_client::vw_api_types::git::{
    GitHistoryDetailDto as HistoryDetail, GitRemoteAction as RemoteAction,
    GitRepositoryDto as RepositorySnapshot,
};

pub(crate) fn remote_action_label(action: RemoteAction) -> &'static str {
    match action {
        RemoteAction::Fetch => "获取",
        RemoteAction::Pull => "拉取",
        RemoteAction::Push => "推送",
    }
}

#[derive(Debug, Default)]
pub(crate) struct RepositoryState {
    pub history: bool,
    pub repo_path: Option<String>,
    pub generation: u64,
    pub loading: bool,
    pub limit: usize,
    pub snapshot: RepositorySnapshot,
    pub selected: Option<String>,
    pub detail: Option<HistoryDetail>,
    pub selected_file: Option<usize>,
    pub file_menu: Option<usize>,
    pub detail_loading: bool,
    pub error: Option<String>,
    // 独立于视图刷新，切换仓库时仍保留正在执行的操作锁。
    pub remote_busy: bool,
    pub remote_action: Option<RemoteAction>,
    pub remote_animation_frame: u16,
}
