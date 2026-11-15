//! 通道运行时的 provider 选择与懒加载。
//!
//! 该模块把模型 provider 的创建、缓存和工具规格过滤集中在一处，避免每条
//! 通道消息重复执行可能阻塞的初始化逻辑。

pub use channel_runtime::manager::provider::*;
