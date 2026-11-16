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

/// 执行 import_prompt_template 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(crate) fn import_prompt_template(
    format: TaskImportPromptFormat,
    selected_priority: u32,
    selected_model: &str,
    selected_executor: Option<&str>,
) -> String {
    draft::import_prompt_template(format, selected_priority, selected_model, selected_executor)
}

/// 对外暴露当前模块需要复用的能力。
pub(crate) use bulk::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use draft::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use execution::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use logs::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use review::*;
/// 对外暴露当前模块需要复用的能力。
pub(crate) use settings::*;
