//! 重新导出共享格式化工具。
//! 该薄层让代理 crate 继续使用本地 util 路径，同时避免复制格式化实现。

pub use display_format::format_duration;
