//! Desktop Shell selection and local terminal authorization adapter.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use desktop_terminal::panes;
pub use desktop_terminal::{
    TerminalTheme, build_palette, truncate_string_to_limit, truncate_terminal_content,
};
pub type TerminalState = desktop_terminal::TerminalState<Shell>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 桌面终端支持启动的交互式 Shell。
pub enum Shell {
    Bash,
    Zsh,
}

impl Shell {
    pub fn all() -> [Shell; 2] {
        [Shell::Bash, Shell::Zsh]
    }
}

impl std::fmt::Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Shell::Bash => write!(f, "bash"),
            Shell::Zsh => write!(f, "zsh"),
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::Bash
    }
}

impl From<Shell> for &'static str {
    fn from(shell: Shell) -> Self {
        match shell {
            Shell::Bash => "/bin/bash",
            Shell::Zsh => "/bin/zsh",
        }
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod terminal_tests;

/// Open a local directory without interpreting it as shell code or an `open` option.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn open_local_directory(path: &str) -> std::io::Result<()> {
    if !crate::app::config::gateway_client_endpoint().is_loopback() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "远端终端只能通过网关 PTY 打开，不能启动本机终端",
        ));
    }
    desktop_terminal::local::open_local_directory(path)
}
