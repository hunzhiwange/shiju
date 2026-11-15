//! 工具配置与工具目录。

mod browser;
mod catalog;
mod multimodal;
mod network;

pub use browser::{BrowserComputerUseConfig, BrowserConfig};
pub use catalog::{
    BUILTIN_REQUIRED_TOOL_IDS, DEFAULT_ENABLED_TOOL_IDS, is_builtin_required_tool_id,
    is_mcp_dynamic_tool_id,
};
pub use multimodal::{MultimodalConfig, MultimodalOcrConfig};
pub use network::{
    DEFAULT_USER_AGENT, HttpRequestConfig, WebFetchConfig, WebSearchConfig, resolve_user_agent,
};

#[cfg(test)]
mod tools_tests;
#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod multimodal_tests;
#[cfg(test)]
mod browser_tests;
#[cfg(test)]
mod network_tests;
