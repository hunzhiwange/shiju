//! 维护系统设置状态及其按领域拆分的派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

/// 常规配置页内的一级标签。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GeneralSettingsTab {
    #[default]
    Interface,
    Terminal,
    Editor,
    Themes,
}

/// 对外暴露当前模块需要复用的能力。
pub(super) use super::*;

mod agents;
mod channels;
mod infrastructure;
mod provider_model;
mod runtime;
mod staff;
mod staff_resources;

/// 对外暴露当前模块需要复用的能力。
pub(crate) use agents::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use channels::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use infrastructure::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use provider_model::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use runtime::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use staff::*;
/// 对外暴露员工资源状态。
pub(crate) use staff_resources::*;

#[cfg(test)]
#[path = "settings_tests.rs"]
mod settings_tests;
