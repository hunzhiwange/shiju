//! 把 crate 内的 VT100 测试后端复用到集成测试，避免维护第二份实现。
//! `#[path]` 保持单一源码来源，公开导出仅供本测试二进制使用。

#[path = "../src/test_backend.rs"]
mod inner;

pub use inner::VT100Backend;
