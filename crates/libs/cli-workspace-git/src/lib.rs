//! CLI 工作区 Git 查询与项目摘要。

mod session;

pub use session::{build_project_info, collect_git_workspace_status, collect_modified_files};

#[cfg(test)]
#[path = "session_tests.rs"]
mod session_tests;
