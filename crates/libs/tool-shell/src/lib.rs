//! Managed shell execution and Windows PowerShell tools.
pub mod shell;
#[cfg(target_os = "windows")]
pub mod powershell;
pub use shell::ShellTool;
#[cfg(target_os = "windows")]
pub use powershell::PowerShellTool;
