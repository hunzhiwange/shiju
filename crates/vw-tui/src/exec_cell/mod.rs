//! 汇总命令执行单元的数据模型与 Ratatui 渲染入口。
//! 对外只暴露构造、输出截断参数和展示所需的稳定类型。

mod model;
mod render;

pub use model::{CommandOutput, ExecCall, ExecCell, ExecCommandSource};
pub use render::{OutputLinesParams, TOOL_CALL_MAX_LINES, new_active_exec_command, output_lines};
