//! 暴露在编译期嵌入的 CLI 包版本。

/// 当前 Codex CLI 在编译期嵌入的版本号。
/// The current Codex CLI version as embedded at compile time.
pub const CODEX_CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
