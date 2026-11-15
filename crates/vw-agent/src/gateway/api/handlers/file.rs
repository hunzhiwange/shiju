//! 文件操作路由模块
//!
//! 本模块提供与文件系统交互的 HTTP API 路由，包括：
//! - 文本内容搜索（基于 ripgrep）
//! - 文件名搜索
//! - 目录列表
//! - 文件内容读取
//! - Git 状态查询
//!
//! # 路由端点
//!
//! - `GET /find` - 在文件中搜索文本内容
//! - `GET /find/file` - 按文件名搜索文件
//! - `GET /find/symbol` - 符号搜索（未实现）
//! - `GET /file` - 列出目录内容
//! - `GET /file/content` - 读取文件内容
//! - `GET /file/status` - 获取 Git 状态
/// File routes use the host snapshot bootstrap inside the project instance scope.
struct FileHost;

impl gateway_file::Host for FileHost {
    fn bootstrap(worktree: std::path::PathBuf) -> agent_project::BoxFuture<()> {
        Box::pin(async move {
            crate::app::agent::project::instance_bootstrap(worktree).await;
        })
    }
}

pub(crate) fn router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    gateway_file::router::<S, FileHost>()
}

// Production middleware coverage belongs to the composed gateway host.
#[cfg(test)]
#[path = "file_drag_integration_tests.rs"]
mod file_drag_integration_tests;
