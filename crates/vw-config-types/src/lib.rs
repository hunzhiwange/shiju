//! 配置类型的兼容发布门面。
//!
//! 各公开模块再导出对应领域库的唯一类型定义，保持既有发布路径。

pub mod acp_install;
pub mod agent;
pub mod agent_profile;
pub mod automation;
pub mod channels;
pub mod computer_control;
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
pub mod tools;
pub mod ui;
pub mod voice;

pub mod worktree;
