//! WebView 工具栏资源、安全 IPC 与纯布局。

mod ipc;
mod layout;
mod resources;

pub use ipc::{ToolbarAction, parse_toolbar_ipc_message};
pub use layout::{ViewRegion, window_regions};
pub use resources::{toolbar_address_script, toolbar_html};

#[cfg(test)]
mod toolbar_tests;
