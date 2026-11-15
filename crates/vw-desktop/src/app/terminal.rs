//! Desktop Shell selection and local terminal authorization adapter.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use desktop_terminal::panes;
pub use desktop_terminal::{
    TerminalTheme, build_palette, truncate_string_to_limit, truncate_terminal_content,
};
pub type TerminalState = desktop_terminal::TerminalState<Shell>;

pub use desktop_terminal::Shell;


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
