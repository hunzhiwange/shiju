//! VibeWindow terminal lifecycle built from the copied Codex TUI boundary.
//!
//! The rendering backend remains in `custom_terminal`; this module owns only the
//! process terminal modes that must always be restored, including error and panic paths.
//!
//! v3 终端生命周期建立在复制的 Codex TUI 边界之上；绘制后端仍属于 `custom_terminal`，
//! 本模块只负责原始模式、光标、括号粘贴、鼠标和屏幕缓冲区的成对启停，错误与 panic 路径也必须恢复。

use std::io;
use std::io::Stdout;
use std::io::Write;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

/// Selects whether v3 uses terminal-native scrollback or an alternate screen.
/// 选择 v3 使用保留原生回滚的内联模式，还是隔离内容的替代屏幕模式。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TerminalMode {
    #[default]
    Inline,
    Alternate,
}

/// 抽象终端模式控制，生产调用 Crossterm，测试记录严格的启停顺序。
trait TerminalControl {
    fn enable_raw_mode(&mut self) -> io::Result<()>;
    fn disable_raw_mode(&mut self) -> io::Result<()>;
    fn enter_alternate_screen(&mut self) -> io::Result<()>;
    fn leave_alternate_screen(&mut self) -> io::Result<()>;
    fn enable_bracketed_paste(&mut self) -> io::Result<()>;
    fn disable_bracketed_paste(&mut self) -> io::Result<()>;
    fn enable_mouse_capture(&mut self) -> io::Result<()>;
    fn disable_mouse_capture(&mut self) -> io::Result<()>;
    fn hide_cursor(&mut self) -> io::Result<()>;
    fn show_cursor(&mut self) -> io::Result<()>;
}

/// 对给定 writer 执行 Crossterm 终端控制命令。
struct CrosstermControl<W> {
    writer: W,
}

impl<W> CrosstermControl<W> {
    fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: Write> TerminalControl for CrosstermControl<W> {
    fn enable_raw_mode(&mut self) -> io::Result<()> {
        enable_raw_mode()
    }

    fn disable_raw_mode(&mut self) -> io::Result<()> {
        disable_raw_mode()
    }

    fn enter_alternate_screen(&mut self) -> io::Result<()> {
        execute!(self.writer, EnterAlternateScreen)
    }

    fn leave_alternate_screen(&mut self) -> io::Result<()> {
        execute!(self.writer, LeaveAlternateScreen)
    }

    fn enable_bracketed_paste(&mut self) -> io::Result<()> {
        execute!(self.writer, EnableBracketedPaste)
    }

    fn disable_bracketed_paste(&mut self) -> io::Result<()> {
        execute!(self.writer, DisableBracketedPaste)
    }

    fn enable_mouse_capture(&mut self) -> io::Result<()> {
        execute!(self.writer, EnableMouseCapture)
    }

    fn disable_mouse_capture(&mut self) -> io::Result<()> {
        execute!(self.writer, DisableMouseCapture)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        execute!(self.writer, Hide)
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        execute!(self.writer, Show)
    }
}

/// RAII session that tracks exactly which terminal modes were enabled.
/// 精确记录已成功启用模式的 RAII 会话，只恢复真正取得的资源。
struct TerminalSession<C: TerminalControl> {
    control: C,
    raw_mode: bool,
    alternate_screen: bool,
    bracketed_paste: bool,
    mouse_capture: bool,
    cursor_hidden: bool,
    restored: bool,
}

impl<C: TerminalControl> TerminalSession<C> {
    /// 按固定顺序取得终端所有权，任一步失败都会恢复此前已启用的模式。
    fn enter(control: C, mode: TerminalMode) -> io::Result<Self> {
        let mut session = Self {
            control,
            raw_mode: false,
            alternate_screen: false,
            bracketed_paste: false,
            mouse_capture: false,
            cursor_hidden: false,
            restored: false,
        };
        session.control.enable_raw_mode()?;
        session.raw_mode = true;
        if mode == TerminalMode::Alternate {
            session.control.enter_alternate_screen()?;
            session.alternate_screen = true;
        }
        session.control.enable_bracketed_paste()?;
        session.bracketed_paste = true;
        session.control.enable_mouse_capture()?;
        session.mouse_capture = true;
        session.control.hide_cursor()?;
        session.cursor_hidden = true;
        Ok(session)
    }

    /// 逆序恢复全部已启用模式，并保留遇到的首个错误继续完成其余清理。
    fn restore(&mut self) -> io::Result<()> {
        if self.restored {
            return Ok(());
        }
        let mut first_error = None;
        if self.cursor_hidden {
            record_restore_result(&mut first_error, self.control.show_cursor());
            self.cursor_hidden = false;
        }
        if self.mouse_capture {
            record_restore_result(&mut first_error, self.control.disable_mouse_capture());
            self.mouse_capture = false;
        }
        if self.bracketed_paste {
            record_restore_result(&mut first_error, self.control.disable_bracketed_paste());
            self.bracketed_paste = false;
        }
        if self.alternate_screen {
            record_restore_result(&mut first_error, self.control.leave_alternate_screen());
            self.alternate_screen = false;
        }
        if self.raw_mode {
            record_restore_result(&mut first_error, self.control.disable_raw_mode());
            self.raw_mode = false;
        }
        self.restored = true;
        first_error.map_or(Ok(()), Err)
    }
}

impl<C: TerminalControl> Drop for TerminalSession<C> {
    // Drop 是 panic 与提前返回的最后防线；显式恢复后再次调用保持幂等。
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// 记录首个恢复错误，同时允许后续恢复步骤继续执行。
fn record_restore_result(slot: &mut Option<io::Error>, result: io::Result<()>) {
    if let Err(error) = result
        && slot.is_none()
    {
        *slot = Some(error);
    }
}

/// 在 RAII 终端会话中执行操作，并合并操作错误与恢复错误的优先级。
fn run_with_control<C, T>(
    control: C,
    mode: TerminalMode,
    operation: impl FnOnce() -> io::Result<T>,
) -> io::Result<T>
where
    C: TerminalControl,
{
    let mut session = TerminalSession::enter(control, mode)?;
    let operation_result = operation();
    let restore_result = session.restore();
    match operation_result {
        Err(error) => Err(error),
        Ok(value) => restore_result.map(|()| value),
    }
}

/// Runs a v3 terminal operation and restores raw mode, cursor, paste and screen state.
/// 执行一次 v3 终端操作，并恢复原始模式、光标、粘贴及屏幕状态。
pub fn run_terminal<T>(
    mode: TerminalMode,
    operation: impl FnOnce() -> io::Result<T>,
) -> io::Result<T> {
    run_with_control(CrosstermControl::new(std::io::stdout()), mode, operation)
}

/// Concrete custom terminal used by the CLI host once v3 becomes the default entry.
/// v3 成为默认入口后由 CLI 宿主使用的具体自定义终端类型。
pub type Terminal = crate::custom_terminal::Terminal<ratatui::backend::CrosstermBackend<Stdout>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    struct State {
        raw: bool,
        alternate: bool,
        paste: bool,
        mouse: bool,
        cursor_hidden: bool,
        restores: Vec<&'static str>,
    }

    #[derive(Clone, Default)]
    struct FakeControl(Rc<RefCell<State>>);

    impl TerminalControl for FakeControl {
        fn enable_raw_mode(&mut self) -> io::Result<()> {
            self.0.borrow_mut().raw = true;
            Ok(())
        }
        fn disable_raw_mode(&mut self) -> io::Result<()> {
            let mut state = self.0.borrow_mut();
            state.raw = false;
            state.restores.push("raw");
            Ok(())
        }
        fn enter_alternate_screen(&mut self) -> io::Result<()> {
            self.0.borrow_mut().alternate = true;
            Ok(())
        }
        fn leave_alternate_screen(&mut self) -> io::Result<()> {
            let mut state = self.0.borrow_mut();
            state.alternate = false;
            state.restores.push("screen");
            Ok(())
        }
        fn enable_bracketed_paste(&mut self) -> io::Result<()> {
            self.0.borrow_mut().paste = true;
            Ok(())
        }
        fn disable_bracketed_paste(&mut self) -> io::Result<()> {
            let mut state = self.0.borrow_mut();
            state.paste = false;
            state.restores.push("paste");
            Ok(())
        }
        fn enable_mouse_capture(&mut self) -> io::Result<()> {
            self.0.borrow_mut().mouse = true;
            Ok(())
        }
        fn disable_mouse_capture(&mut self) -> io::Result<()> {
            let mut state = self.0.borrow_mut();
            state.mouse = false;
            state.restores.push("mouse");
            Ok(())
        }
        fn hide_cursor(&mut self) -> io::Result<()> {
            self.0.borrow_mut().cursor_hidden = true;
            Ok(())
        }
        fn show_cursor(&mut self) -> io::Result<()> {
            let mut state = self.0.borrow_mut();
            state.cursor_hidden = false;
            state.restores.push("cursor");
            Ok(())
        }
    }

    #[test]
    // 验证操作成功和失败两条路径都按逆序完整恢复终端状态。
    fn v3_terminal_restores_state_on_success_and_error() {
        for mode in [TerminalMode::Inline, TerminalMode::Alternate] {
            for should_fail in [false, true] {
                let control = FakeControl::default();
                let observed = control.0.clone();
                let result = run_with_control(control, mode, || {
                    if should_fail { Err(io::Error::other("operation failed")) } else { Ok(()) }
                });

                let restores = match mode {
                    TerminalMode::Inline => vec!["cursor", "mouse", "paste", "raw"],
                    TerminalMode::Alternate => {
                        vec!["cursor", "mouse", "paste", "screen", "raw"]
                    }
                };
                assert_eq!(result.is_err(), should_fail);
                assert_eq!(*observed.borrow(), State { restores, ..State::default() });
            }
        }
    }

    #[test]
    // 验证 panic 展开触发 Drop 时仍恢复已取得的终端模式。
    fn terminal_drop_restores_state_during_panic() {
        let control = FakeControl::default();
        let observed = control.0.clone();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _session = TerminalSession::enter(control, TerminalMode::Alternate).unwrap();
            panic!("simulated panic");
        }));

        assert_eq!(
            *observed.borrow(),
            State {
                restores: vec!["cursor", "mouse", "paste", "screen", "raw"],
                ..State::default()
            }
        );
    }
}
