//! 工具输入输出压缩逻辑，负责在日志和 UI 展示前收窄大块或敏感的工具载荷。

pub(crate) use session_engine::tool_output::*;

#[cfg(test)]
#[path = "../../../../../libs/session-engine/tests/host_suites/output_compaction_tests.rs"]
mod output_compaction_tests;
