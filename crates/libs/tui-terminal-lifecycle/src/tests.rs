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

#[test]
fn workflow_req_06_selection_restores_screen_and_mouse_after_success_or_error() {
    for mode in [TerminalMode::Inline, TerminalMode::Alternate] {
        for fail in [false, true] {
            let control = FakeControl::default();
            let observed = control.0.clone();
            *observed.borrow_mut() = State {
                raw: true, paste: true, mouse: true, cursor_hidden: true,
                alternate: mode == TerminalMode::Alternate,
                ..State::default()
            };
            let result = with_selection_control(control, mode, || {
                let state = observed.borrow();
                assert!(!state.mouse);
                assert!(state.raw && state.paste && state.cursor_hidden && state.alternate);
                if fail { Err(io::Error::other("static output failed")) } else { Ok(()) }
            });
            assert_eq!(result.is_err(), fail);
            let state = observed.borrow();
            assert!(state.mouse && state.raw && state.paste && state.cursor_hidden);
            assert_eq!(state.alternate, mode == TerminalMode::Alternate);
            assert_eq!(state.restores, if mode == TerminalMode::Inline {
                vec!["mouse", "screen"]
            } else { vec!["mouse"] });
        }
    }
}

#[test]
fn workflow_req_06_selection_restores_after_panic() {
    let control = FakeControl::default();
    let observed = control.0.clone();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = with_selection_control(control, TerminalMode::Inline, || -> io::Result<()> {
            panic!("static view interrupted");
        });
    }));
    let state = observed.borrow();
    assert!(state.mouse);
    assert!(!state.alternate);
    assert_eq!(state.restores, vec!["mouse", "screen"]);
}

fn editor_roundtrip(mode: TerminalMode, unwind: bool) {
    let control = FakeControl::default();
    let state = control.0.clone();
    let _session = TerminalSession::enter(control.clone(), mode).unwrap();
    let before = state.borrow().clone();
    let operation = || {
        let mut suspension = EditorSuspension::suspend(control, mode).unwrap();
        assert!(!state.borrow().raw);
        assert!(!state.borrow().mouse);
        assert!(!state.borrow().paste);
        assert!(!state.borrow().alternate);
        assert!(!state.borrow().cursor_hidden);
        if unwind { panic!("editor failed"); }
        suspension.restore().unwrap();
        suspension.restore().unwrap();
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
    assert_eq!(result.is_err(), unwind);
    let after = state.borrow();
    assert_eq!(after.raw, before.raw);
    assert_eq!(after.mouse, before.mouse);
    assert_eq!(after.paste, before.paste);
    assert_eq!(after.alternate, before.alternate);
    assert_eq!(after.cursor_hidden, before.cursor_hidden);
}
#[test]
fn terminal_req_07_inline_resume() { editor_roundtrip(TerminalMode::Inline, false); }
#[test]
fn terminal_req_07_alternate_resume() { editor_roundtrip(TerminalMode::Alternate, false); }
#[test]
fn terminal_req_07_inline_unwind() { editor_roundtrip(TerminalMode::Inline, true); }
#[test]
fn terminal_req_07_alternate_unwind() { editor_roundtrip(TerminalMode::Alternate, true); }
