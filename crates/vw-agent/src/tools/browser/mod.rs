//! Browser automation implementation owned by tool-browser.
pub use tool_browser::{BrowserTool, ComputerUseConfig};
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use tool_browser::installation;
