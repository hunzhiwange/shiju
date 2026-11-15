//! 自动化配置 schema 兼容导出。
//!
//! 本模块将 `config_automation` 中定义的自动化配置类型重新导出到代理配置命名空间。
//! 这样调用方可以继续通过 `crate::app::agent::config::schema::automation` 访问类型，
//! 同时保持真实 schema 定义集中在自动调度配置 crate 中。

pub use config_automation::*;
