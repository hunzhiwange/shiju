//! 根据配置与终端能力选择 OSC 9 或 BEL 桌面通知后端。

mod bel;
mod osc9;

use std::io;

use bel::BelBackend;
use codex_config::types::NotificationMethod;
use codex_terminal_detection::TerminalInfo;
use codex_terminal_detection::TerminalName;
use codex_terminal_detection::terminal_info;
use osc9::Osc9Backend;

#[derive(Debug)]
/// 已解析的桌面通知后端；自动探测完成后调用方只通过该枚举派发消息。
pub enum DesktopNotificationBackend {
    Osc9(Osc9Backend),
    Bel(BelBackend),
}

impl DesktopNotificationBackend {
    pub fn for_method(method: NotificationMethod) -> Self {
        // 显式配置不做能力降级；只有 Auto 会依据已知终端白名单选择 OSC 9。
        match method {
            NotificationMethod::Auto => {
                if supports_osc9(&terminal_info()) {
                    Self::Osc9(Osc9Backend::new())
                } else {
                    Self::Bel(BelBackend)
                }
            }
            NotificationMethod::Osc9 => Self::Osc9(Osc9Backend::new()),
            NotificationMethod::Bel => Self::Bel(BelBackend),
        }
    }

    pub fn method(&self) -> NotificationMethod {
        match self {
            DesktopNotificationBackend::Osc9(_) => NotificationMethod::Osc9,
            DesktopNotificationBackend::Bel(_) => NotificationMethod::Bel,
        }
    }

    pub fn notify(&mut self, message: &str) -> io::Result<()> {
        // 保留底层 I/O 错误，让上层决定是否提示或忽略通知失败。
        match self {
            DesktopNotificationBackend::Osc9(backend) => backend.notify(message),
            DesktopNotificationBackend::Bel(backend) => backend.notify(message),
        }
    }
}

pub fn detect_backend(method: NotificationMethod) -> DesktopNotificationBackend {
    // 对外保留独立探测入口，避免调用方依赖具体后端构造细节。
    DesktopNotificationBackend::for_method(method)
}

fn supports_osc9(terminal: &TerminalInfo) -> bool {
    // 能力判断采用保守白名单，未知终端自动回退到兼容性更高的 BEL。
    matches!(
        terminal.name,
        TerminalName::Ghostty
            | TerminalName::Iterm2
            | TerminalName::Kitty
            | TerminalName::WarpTerminal
            | TerminalName::WezTerm
    )
}

#[cfg(test)]
mod tests {
    // 测试同时固定显式方法选择和 Auto 模式依赖的终端能力边界。
    use super::detect_backend;
    use super::supports_osc9;
    use codex_config::types::NotificationMethod;
    use codex_terminal_detection::TerminalInfo;
    use codex_terminal_detection::TerminalName;
    use pretty_assertions::assert_eq;

    fn test_terminal(name: TerminalName) -> TerminalInfo {
        TerminalInfo {
            name,
            term_program: None,
            version: None,
            term: None,
            multiplexer: None,
        }
    }

    #[test]
    // 显式 OSC 9 配置必须直接构造 OSC 9 后端。
    fn selects_osc9_method() {
        assert!(matches!(
            detect_backend(NotificationMethod::Osc9),
            super::DesktopNotificationBackend::Osc9(_)
        ));
    }

    #[test]
    // 显式 BEL 配置必须直接构造 BEL 后端。
    fn selects_bel_method() {
        assert!(matches!(
            detect_backend(NotificationMethod::Bel),
            super::DesktopNotificationBackend::Bel(_)
        ));
    }

    #[test]
    // 白名单中的终端均应启用 OSC 9 能力。
    fn supports_osc9_for_supported_terminals() {
        for name in [
            TerminalName::Ghostty,
            TerminalName::Iterm2,
            TerminalName::Kitty,
            TerminalName::WarpTerminal,
            TerminalName::WezTerm,
        ] {
            assert!(
                supports_osc9(&test_terminal(name)),
                "{name:?} should support OSC 9"
            );
        }
    }

    #[test]
    // 其余已知及未知终端保持保守回退，避免输出不受支持的控制序列。
    fn supports_osc9_for_unsupported_terminals() {
        for name in [
            TerminalName::AppleTerminal,
            TerminalName::Alacritty,
            TerminalName::Dumb,
            TerminalName::GnomeTerminal,
            TerminalName::Konsole,
            TerminalName::Unknown,
            TerminalName::VsCode,
            TerminalName::Vte,
            TerminalName::WindowsTerminal,
        ] {
            assert_eq!(
                supports_osc9(&test_terminal(name)),
                false,
                "{name:?} should not support OSC 9"
            );
        }
    }
}
