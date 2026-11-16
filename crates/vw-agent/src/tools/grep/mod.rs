//! GrepTool 的宿主类型特化。
pub type GrepTool = tool_files::grep::GrepTool<super::context::AgentContextHost>;
// 原测试依赖私有函数和真实宿主，仅测试构建复用迁移后的同一源码。
#[cfg(test)]
use tool_files::{FileToolHost, external_directory};
#[cfg(test)]
mod implementation {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../libs/tool-files/src/grep/mod.rs"));
    mod tests {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../libs/tool-files/src/grep/tests.rs"));
    }
}
