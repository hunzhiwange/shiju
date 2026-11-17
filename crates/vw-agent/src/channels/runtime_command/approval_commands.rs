//! 非 CLI 授权运行时命令的具体处理器。
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use channel_runtime::runtime_command::approval_commands::*;
