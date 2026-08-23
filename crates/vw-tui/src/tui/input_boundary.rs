//! Restore terminal ownership on initialization failures and quarantine protected-screen input.
//! 初始化失败时恢复终端所有权，并隔离安全敏感界面的残留输入。
//! 任何无法确认已排空的控制序列都按拒绝继续处理，避免旧按键越过审批边界。

use std::io;
#[cfg(unix)]
use std::io::IsTerminal;
use std::io::Result;
#[cfg(unix)]
use std::io::stdin;
use std::time::Duration;
use std::time::Instant;

/// Restores terminal modes when initialization fails before ownership is transferred.
/// 在终端所有权尚未成功移交前初始化失败时，恢复已经启用的终端模式。
pub(super) struct TerminalInitializationGuard {
    pub(super) active: bool,
}

impl Drop for TerminalInitializationGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = super::restore_after_exit();
        }
    }
}

/// Drain decoded and unread input before a screen can accept a security-sensitive decision.
///
/// Partial control sequences remain quarantined; an unresolved boundary fails after one second.
/// 在安全敏感界面接受输入前排空已解码及未读输入；残缺控制序列继续隔离，边界一秒内无法确认则失败。
pub(crate) fn discard_pending_terminal_input() -> Result<()> {
    // 同时排空已解码事件与底层队列，避免残留确认键误触发审批决定。
    #[cfg(unix)]
    let _ = crossterm::event::discard_buffered_input()?;

    let deadline = Instant::now() + Duration::from_secs(/*secs*/ 1);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "timed out discarding buffered terminal input",
            ));
        }

        if crossterm::event::poll(remaining.min(Duration::from_millis(/*millis*/ 10)))? {
            let _ = crossterm::event::read()?;
            continue;
        }

        #[cfg(unix)]
        if crossterm::event::discard_buffered_input()?
            != crossterm::event::InputDiscardStatus::Complete
        {
            continue;
        }

        #[cfg(any(unix, windows))]
        match terminal_input_is_readable() {
            Ok(true) => continue,
            Ok(false) => {}
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }

        return Ok(());
    }
}

/// Check Crossterm's terminal input source without consuming input or changing descriptor flags.
/// 在不消费输入且不修改描述符标志的情况下检查 Crossterm 终端输入源。
#[cfg(unix)]
pub(super) fn terminal_input_is_readable() -> Result<bool> {
    use std::os::fd::AsRawFd;

    let terminal = if stdin().is_terminal() {
        None
    } else {
        Some(std::fs::File::open("/dev/tty")?)
    };
    let mut fd = libc::pollfd {
        fd: terminal
            .as_ref()
            .map_or(libc::STDIN_FILENO, AsRawFd::as_raw_fd),
        events: libc::POLLIN,
        revents: 0,
    };
    // Safety: `fd` is valid for the duration of this zero-timeout poll.
    // 安全性：`fd` 在这次零超时 poll 调用期间保持有效。
    let result = unsafe {
        libc::poll(&mut fd, /*nfds*/ 1, /*timeout*/ 0)
    };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    if fd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
        return Err(io::Error::other(format!(
            "terminal input poll failed with events {:#x}",
            fd.revents
        )));
    }
    Ok(result > 0 && fd.revents & libc::POLLIN != 0)
}

/// Check the same Windows console input queue consumed by Crossterm.
/// 检查 Crossterm 实际消费的同一个 Windows 控制台输入队列。
#[cfg(windows)]
fn terminal_input_is_readable() -> Result<bool> {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::GetNumberOfConsoleInputEvents;
    use windows_sys::Win32::System::Console::GetStdHandle;
    use windows_sys::Win32::System::Console::STD_INPUT_HANDLE;

    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    if handle == INVALID_HANDLE_VALUE || handle == 0 {
        return Err(io::Error::other("terminal input handle is unavailable"));
    }

    let mut pending = 0;
    if unsafe { GetNumberOfConsoleInputEvents(handle, &mut pending) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(pending != 0)
}
