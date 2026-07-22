//! VibeWindow 配置类型共享库
//!
//! 本库存放所有配置相关的纯数据类型定义，
//! 供 `vw-agent`、`vw-desktop` 及其他模块共同使用。

pub mod agent;
pub mod automation;
pub mod channels;
pub mod config;
pub mod gateway;
pub mod hooks;
pub mod memory;
pub mod observability;
pub mod paths;
pub mod plugins;
pub mod provider;
pub mod proxy;
pub mod reliability;
pub mod routing;
pub mod runtime;
pub mod security;
pub mod skills;
pub mod staff;
pub mod tools;
pub mod transcription;
pub mod ui;
