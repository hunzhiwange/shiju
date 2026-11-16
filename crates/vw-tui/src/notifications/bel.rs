//! 通过终端 BEL 控制字符触发最基础的桌面提醒后端。

use std::fmt;
use std::io;
use std::io::stdout;

use crossterm::Command;
use ratatui::crossterm::execute;

#[derive(Debug, Default)]
pub struct BelBackend;

impl BelBackend {
    /// 发出 BEL；该协议不携带消息正文，因此参数仅用于统一后端接口。
    pub fn notify(&mut self, _message: &str) -> io::Result<()> {
        execute!(stdout(), PostNotification)
    }
}

/// Command that emits a BEL desktop notification.
/// 发出 BEL 桌面提醒的终端命令。
#[derive(Debug, Clone)]
pub struct PostNotification;

impl Command for PostNotification {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        write!(f, "\x07")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> io::Result<()> {
        // Windows 也必须走 ANSI 控制字符路径，不能把 BEL 误派发为 WinAPI 命令。
        Err(std::io::Error::other(
            "tried to execute PostNotification using WinAPI; use ANSI instead",
        ))
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        true
    }
}
