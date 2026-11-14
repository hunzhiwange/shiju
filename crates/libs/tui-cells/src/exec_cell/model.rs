//! 分组命令单元的展示模型。
//! Display model for grouped command cells.

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecCommandSource {
    /// 由工具调用发起，展示时包含工具名称。
    Tool,
    /// 由用户直接发起，展示时使用 shell 提示符。
    User,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandOutput {
    /// 进程退出码；零值表示成功。
    pub exit_code: i32,
    /// 已收集的标准输出与错误输出文本。
    pub output: String,
}

impl CommandOutput {
    pub fn new(exit_code: i32, output: String) -> Self {
        Self { exit_code, output }
    }
}

#[derive(Clone, Debug)]
pub struct ExecCall {
    /// 协议调用标识，用于把后续结果关联到本次执行。
    pub call_id: String,
    /// 发起执行的工具名称。
    pub tool: String,
    /// 面向历史记录展示的命令文本。
    pub command: String,
    /// 缺失表示调用仍在运行。
    pub output: Option<CommandOutput>,
    pub source: ExecCommandSource,
    pub start_time: Option<Instant>,
    pub duration: Option<Duration>,
}

#[derive(Clone, Debug)]
pub struct ExecCell {
    /// 可合并展示的一组命令调用。
    pub calls: Vec<ExecCall>,
    /// 是否允许活动状态使用动画指示器。
    pub animations_enabled: bool,
}

impl ExecCell {
    pub fn pending(call_id: String, tool: String, command: String) -> Self {
        Self {
            calls: vec![ExecCall {
                call_id,
                tool,
                command,
                output: None,
                source: ExecCommandSource::Tool,
                start_time: Some(Instant::now()),
                duration: None,
            }],
            animations_enabled: false,
        }
    }

    pub fn completed(
        call_id: String,
        tool: String,
        command: String,
        output: CommandOutput,
        duration: Duration,
    ) -> Self {
        let mut cell = Self::pending(call_id, tool, command);
        let call = &mut cell.calls[0];
        call.output = Some(output);
        call.duration = Some(duration);
        call.start_time = None;
        cell
    }

    pub fn is_active(&self) -> bool {
        // 任一调用尚无耗时即说明其结束事件还未落入模型。
        self.calls.iter().any(|call| call.duration.is_none())
    }
}
