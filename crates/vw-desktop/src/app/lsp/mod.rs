//! 管理桌面端语言服务协议能力。
//! 本模块把编辑器可见的诊断、补全和后台 LSP 生命周期分离。

#![cfg(not(target_arch = "wasm32"))]

pub use iced_code_editor::LspEvent;

pub(crate) mod install;
mod manager;
pub(crate) use manager::LspServiceManager;

pub mod config {
    pub use iced_code_editor::{
        LspCommand, LspLanguage, LspServerConfig, ensure_rust_analyzer_config,
        lsp_language_for_extension, lsp_language_for_path, lsp_server_config, resolve_lsp_command,
    };
}
#[cfg(test)]
mod tests;
