//! 安全配置类型的兼容导出层。
//!
//! 安全策略相关的结构体和枚举由 `config_security` 统一定义；本模块只负责在
//! agent 配置 schema 中暴露稳定路径，便于配置加载、校验和 UI 侧共享同一份类型。

pub use config_security::*;
