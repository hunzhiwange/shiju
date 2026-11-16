/// 通道管理子命令枚举。
///
/// 与 CLI 枚举镜像，但避免 clap 依赖。
/// 用于运行时动态处理通道相关的管理命令。
pub use agent_cli_types::ChannelCommands;
