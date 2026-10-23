//! 模型提供商共享定义模块。
//!
//! 该模块负责承载提供商、模型、能力、路由兼容信息以及从静态模型定义转换到
//! 运行时展示结构的辅助逻辑。
//!
//! # 子模块
//!
//! - `models`：较贴近原始模型清单格式的定义
//! - `state`：从原始清单构造运行时状态的转换函数
//! - `types`：对外暴露的提供商与模型协议类型

pub mod reasoning;
pub mod models;
pub mod state;
pub mod types;

/// provider 状态缓存结构。
pub use state::State;
/// 对外暴露的 provider 公共类型。
pub use types::*;

/// 默认适配器辅助函数。
pub use types::default_adapter;
