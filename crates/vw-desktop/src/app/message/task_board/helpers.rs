//! 提供任务看板消息处理过程中复用的局部辅助逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;

mod bulk;
mod draft;
mod execution;
mod logs;
mod review;
mod settings;

/// 对外暴露当前模块需要复用的能力。
pub(crate) use bulk::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use draft::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use execution::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use logs::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use settings::*;

#[cfg(test)]
pub(crate) use review::*;
