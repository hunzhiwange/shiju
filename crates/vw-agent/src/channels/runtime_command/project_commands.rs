//! 当前聊天的项目查询和选择；校验全部成功后同步提交，不自动恢复旧上下文。
#[cfg(any(test, target_arch = "wasm32"))]
use super::super::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_runtime::runtime_command::project_commands::{handle_project_command};
