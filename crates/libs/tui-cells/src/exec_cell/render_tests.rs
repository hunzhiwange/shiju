use super::*;
use crate::exec_cell::CommandOutput;

#[test]
// 验证已完成单元同时保留命令、输出和成功状态，不只覆盖布局构造。
fn completed_exec_cell_keeps_command_output_and_status() {
    let cell = ExecCell::completed(
        "call-1".into(),
        "shell".into(),
        "printf ok".into(),
        CommandOutput::new(0, "ok".into()),
        Duration::from_millis(8),
    );
    let lines = cell.display_lines(40);
    assert!(lines.iter().any(|line| line.to_string().contains("shell printf ok")));
    assert!(lines.iter().any(|line| line.to_string().contains("printf ok")));
    assert!(lines.iter().any(|line| line.to_string().contains('✓')));
}
