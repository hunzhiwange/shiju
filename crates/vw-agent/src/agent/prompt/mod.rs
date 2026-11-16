//! 系统提示词规则的兼容出口。
pub use agent_loop::prompt::*;

#[cfg(test)]
#[path = "../../../../libs/agent-loop/src/tests/prompt_host.rs"]
mod mod_tests;
