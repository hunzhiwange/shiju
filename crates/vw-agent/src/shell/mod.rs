//! Shell 领域类型重导出层，用于保持 agent crate 现有导入路径稳定。

/// 重导出 shell_command::*，保持外部调用路径稳定。
pub use shell_command::*;
