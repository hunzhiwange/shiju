//! Skills 配置类型的兼容导出层。
//!
//! skill 发现、启用和策略字段的类型定义集中在 `vw_config_types`；本模块将它们
//! 重新挂载到 agent 配置 schema 下，保持历史导入路径稳定。

pub use config_skills::*;
