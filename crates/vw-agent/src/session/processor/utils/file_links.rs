//! 文件链接辅助逻辑，负责从工具输入输出中提取、压缩和生成可展示的文件引用。

pub(crate) use session_engine::tool_file_links::*;

#[cfg(test)]
#[path = "../../../../../libs/session-engine/tests/host_suites/file_links_tests.rs"]
mod file_links_tests;
