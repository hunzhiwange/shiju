//! Git 工作区状态数据，不执行本地命令。

/// CLI/TUI 消费的 Git 工作区状态三态。
///
/// - `ReadyClean`：Git 可用且当前没有变更
/// - `ReadyDirty`：Git 可用且存在变更文件
/// - `Unavailable`：当前目录不是 Git 仓库，或 Git 查询失败
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GitWorkspaceStatus {
    ReadyClean,
    ReadyDirty(Vec<String>),
    #[default]
    Unavailable,
}

impl GitWorkspaceStatus {
    pub fn modified_files(&self) -> &[String] {
        match self {
            Self::ReadyDirty(files) => files.as_slice(),
            Self::ReadyClean | Self::Unavailable => &[],
        }
    }
}
