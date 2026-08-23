//! Display model for grouped command cells.

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecCommandSource {
    Tool,
    User,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub output: String,
}

impl CommandOutput {
    pub fn new(exit_code: i32, output: String) -> Self {
        Self { exit_code, output }
    }
}

#[derive(Clone, Debug)]
pub struct ExecCall {
    pub call_id: String,
    pub tool: String,
    pub command: String,
    pub output: Option<CommandOutput>,
    pub source: ExecCommandSource,
    pub start_time: Option<Instant>,
    pub duration: Option<Duration>,
}

#[derive(Clone, Debug)]
pub struct ExecCell {
    pub calls: Vec<ExecCall>,
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
        self.calls.iter().any(|call| call.duration.is_none())
    }
}
