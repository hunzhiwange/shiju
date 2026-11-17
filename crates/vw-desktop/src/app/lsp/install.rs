//! 桌面语言服务器安装入口，后端实现由 desktop-lsp 统一持有。
pub use desktop_lsp::install::install;

#[cfg(test)]
pub use desktop_lsp::install::Server;
