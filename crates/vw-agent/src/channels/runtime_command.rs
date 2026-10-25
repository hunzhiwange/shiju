//! 通道运行时命令处理模块。
//!
//! 该模块保留原有外部接口，并按职责将实现拆分到同名子目录中，
//! 便于分别维护命令解析、审批配置、任务模式和命令执行逻辑。

mod approval_commands;
mod approval_config;
pub(crate) mod catalog;
mod command;
mod handler;
pub(crate) mod project_commands;
mod session_commands;
mod session_lifecycle;
mod task_mode;

#[cfg(test)]
mod handler_tests;
#[cfg(test)]
pub(crate) mod session_commands_tests;
#[cfg(test)]
mod task_mode_tests;

#[allow(unused_imports)]
pub(crate) use approval_config::{
    describe_non_cli_approvals, non_cli_natural_language_mode_label,
    persist_non_cli_approval_to_config, remove_non_cli_approval_from_config,
};
#[allow(unused_imports)]
pub(crate) use command::{
    ChannelRuntimeCommand, approval_target_label, contains_any_fragment,
    extract_runtime_tail_token, is_approval_management_command,
    is_natural_language_all_tools_once_intent, is_runtime_token,
    parse_natural_language_runtime_command, parse_runtime_command,
};
pub(crate) use handler::handle_runtime_command_if_needed;

#[cfg(test)]
#[path = "runtime_command_tests.rs"]
mod runtime_command_tests;

pub(crate) use session_lifecycle::{is_session_lifecycle_command, persist_current_route};
