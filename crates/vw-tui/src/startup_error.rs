use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
#[error(
    "failed to initialize sqlite local db at {}: {detail}",
    database_path.display()
)]
pub struct LocalStateDbStartupError {
    /// 初始化失败的数据库路径，用于面向用户的恢复提示。
    database_path: PathBuf,
    detail: String,
}

impl LocalStateDbStartupError {
    /// 构造不会丢失底层错误详情的启动错误。
    pub fn new(database_path: PathBuf, detail: String) -> Self {
        Self {
            database_path,
            detail,
        }
    }

    pub fn database_path(&self) -> &Path {
        self.database_path.as_path()
    }

    pub fn state_db_path(&self) -> &Path {
        // 保留旧调用方使用的语义化访问器，并与数据库路径保持同一来源。
        self.database_path()
    }

    pub fn detail(&self) -> &str {
        self.detail.as_str()
    }
}
//! 表示本地状态数据库初始化失败，并保留可诊断的路径与错误详情。
