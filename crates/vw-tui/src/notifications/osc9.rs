//! 通过 OSC 9 控制序列发送带正文的桌面通知，并兼容 tmux 透传。

use std::fmt;
use std::io;
use std::io::stdout;

use codex_terminal_detection::Multiplexer;
use codex_terminal_detection::terminal_info;
use crossterm::Command;
use ratatui::crossterm::execute;

#[derive(Debug)]
pub struct Osc9Backend {
    // 后端创建时缓存复用器信息，避免每次通知重复探测终端环境。
    dcs_passthrough: bool,
}

impl Default for Osc9Backend {
    fn default() -> Self {
        Self::new()
    }
}

impl Osc9Backend {
    pub fn new() -> Self {
        Self {
            dcs_passthrough: matches!(terminal_info().multiplexer, Some(Multiplexer::Tmux { .. })),
        }
    }

    pub fn notify(&mut self, message: &str) -> io::Result<()> {
        // 所有输出都走 crossterm 命令，以复用其 ANSI/Windows 分派契约。
        execute!(
            stdout(),
            PostNotification {
                message: message.to_string(),
                dcs_passthrough: self.dcs_passthrough,
            }
        )
    }
}

/// Command that emits an OSC 9 desktop notification with a message.
/// 发出带消息正文的 OSC 9 桌面通知命令。
#[derive(Debug, Clone)]
pub struct PostNotification {
    pub message: String,
    pub dcs_passthrough: bool,
}

impl Command for PostNotification {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        // tmux 中需用 DCS 包裹 OSC 序列；直连终端则输出标准 OSC 9。
        if self.dcs_passthrough {
            let escaped_message = escape_tmux_dcs_passthrough_payload(&self.message);
            write!(f, "\x1bPtmux;\x1b\x1b]9;{escaped_message}\x07\x1b\\")
        } else {
            write!(f, "\x1b]9;{}\x07", self.message)
        }
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> io::Result<()> {
        Err(std::io::Error::other(
            "tried to execute PostNotification using WinAPI; use ANSI instead",
        ))
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        true
    }
}

fn escape_tmux_dcs_passthrough_payload(message: &str) -> String {
    // tmux DCS 透传要求载荷内的 ESC 加倍，否则消息可能提前终止外层序列。
    message.replace('\u{1b}', "\u{1b}\u{1b}")
}

#[cfg(test)]
mod tests {
    // 字节级断言固定直连与 tmux 两条编码路径，防止控制序列边界回归。
    use crossterm::Command;
    use pretty_assertions::assert_eq;

    use super::PostNotification;

    #[test]
    // 直连终端输出标准 OSC 9，并以 BEL 结束消息。
    fn post_notification_writes_plain_osc9_sequence() {
        let mut ansi = String::new();
        let command = PostNotification {
            message: "hello".to_string(),
            dcs_passthrough: false,
        };

        command
            .write_ansi(&mut ansi)
            .expect("OSC 9 command should format");

        assert_eq!(ansi, "\u{1b}]9;hello\u{7}");
    }

    #[test]
    // tmux 路径应添加 DCS 透传外壳及正确的收尾序列。
    fn post_notification_writes_tmux_dcs_wrapped_osc9_sequence() {
        let mut ansi = String::new();
        let command = PostNotification {
            message: "done".to_string(),
            dcs_passthrough: true,
        };

        command
            .write_ansi(&mut ansi)
            .expect("OSC 9 command should format");

        assert_eq!(ansi, "\u{1b}Ptmux;\u{1b}\u{1b}]9;done\u{7}\u{1b}\\");
    }

    #[test]
    // 消息内 ESC 必须加倍，不能逃逸出 tmux 透传载荷。
    fn post_notification_escapes_escape_bytes_inside_tmux_payload() {
        let mut ansi = String::new();
        let command = PostNotification {
            message: "danger\u{1b}[31m".to_string(),
            dcs_passthrough: true,
        };

        command
            .write_ansi(&mut ansi)
            .expect("OSC 9 command should format");

        assert_eq!(
            ansi,
            "\u{1b}Ptmux;\u{1b}\u{1b}]9;danger\u{1b}\u{1b}[31m\u{7}\u{1b}\\"
        );
    }
}
