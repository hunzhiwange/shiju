//! TUI 单元测试共享的确定性终端构造辅助函数。
//! 固定 80×24 几何与原点光标，避免测试依赖真实终端状态。

use std::io;
use std::io::stdout;

use ratatui::backend::CrosstermBackend;
use ratatui::layout::Position;
use ratatui::layout::Size;

use super::Tui;
use super::terminal_stderr::TerminalStderrGuard;
use crate::custom_terminal::Terminal;

/// 创建使用 Crossterm 后端、固定尺寸和受保护 stderr 的测试 TUI。
pub(crate) fn make_test_tui() -> io::Result<Tui> {
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::with_screen_size_and_cursor_position_for_test(
        backend,
        Size {
            width: 80,
            height: 24,
        },
        Position { x: 0, y: 0 },
    );
    let stderr_guard = TerminalStderrGuard::install()?;
    // 测试不启用增强键盘协议，输入语义保持为最小公共集合。
    Ok(Tui::new(
        terminal,
        /*enhanced_keys_supported*/ false,
        stderr_guard,
    ))
}
