//! Terminal keyboard enhancement setup and teardown helpers.
//!
//! The TUI uses crossterm's keyboard enhancement stack while it owns the terminal, but
//! process exit gets a stronger reset so the parent shell does not inherit enhanced key
//! reporting if a terminal misses the normal stack pop.
//!
//! TUI 持有终端时使用 Crossterm 键盘增强栈；进程退出还会发送更强复位，
//! 防止终端漏处理普通 pop 时把增强按键报告遗留给父 shell。

use std::fmt;
use std::io::stdout;

use codex_terminal_detection::TerminalName;
use codex_terminal_detection::terminal_info;
use crossterm::Command;
use crossterm::event::KeyboardEnhancementFlags;
use crossterm::event::PopKeyboardEnhancementFlags;
use crossterm::event::PushKeyboardEnhancementFlags;
use ratatui::crossterm::execute;

const DISABLE_KEYBOARD_ENHANCEMENT_ENV_VAR: &str = "CODEX_TUI_DISABLE_KEYBOARD_ENHANCEMENT";

/// 综合显式环境配置与 WSL+VS Code 兼容性判断是否禁用增强键盘。
pub(super) fn keyboard_enhancement_disabled() -> bool {
    let disable_env = std::env::var(DISABLE_KEYBOARD_ENHANCEMENT_ENV_VAR).ok();
    let is_wsl = running_in_wsl();
    let is_vscode_terminal = is_wsl && running_in_vscode_terminal();
    keyboard_enhancement_disabled_for(disable_env.as_deref(), is_wsl, is_vscode_terminal)
}

fn keyboard_enhancement_disabled_for(
    disable_env: Option<&str>,
    is_wsl: bool,
    is_vscode_terminal: bool,
) -> bool {
    if let Some(disabled) = parse_bool_env(disable_env) {
        return disabled;
    }

    // VS Code running a WSL shell can hide TERM_PROGRAM from the Linux process
    // environment, so `running_in_vscode_terminal` also probes the Windows-side
    // environment through WSL interop.
    // WSL 内的 VS Code 可能不向 Linux 进程暴露 TERM_PROGRAM，因此还会通过 WSL 互操作探测 Windows 环境。
    is_wsl && is_vscode_terminal
}

/// 只接受常见真/假拼写，未知值不覆盖自动兼容策略。
fn parse_bool_env(value: Option<&str>) -> Option<bool> {
    match value.map(str::trim) {
        Some("1") => Some(true),
        Some(value) if value.eq_ignore_ascii_case("true") => Some(true),
        Some(value) if value.eq_ignore_ascii_case("yes") => Some(true),
        Some("0") => Some(false),
        Some(value) if value.eq_ignore_ascii_case("false") => Some(false),
        Some(value) if value.eq_ignore_ascii_case("no") => Some(false),
        _ => None,
    }
}

fn running_in_wsl() -> bool {
    #[cfg(target_os = "linux")]
    {
        crate::clipboard_paste::is_probably_wsl()
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// 同时检查 Linux 与 Windows 侧 TERM_PROGRAM，以识别 WSL 中的 VS Code 终端。
pub(super) fn running_in_vscode_terminal() -> bool {
    vscode_terminal_detected(
        std::env::var("TERM_PROGRAM").ok().as_deref(),
        windows_term_program().as_deref(),
    )
}

fn vscode_terminal_detected(
    linux_term_program: Option<&str>,
    windows_term_program: Option<&str>,
) -> bool {
    term_program_is_vscode(linux_term_program) || term_program_is_vscode(windows_term_program)
}

fn term_program_is_vscode(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.eq_ignore_ascii_case("vscode"))
}

fn windows_term_program() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        static WINDOWS_TERM_PROGRAM: std::sync::OnceLock<Option<String>> =
            std::sync::OnceLock::new();
        WINDOWS_TERM_PROGRAM
            .get_or_init(read_windows_term_program)
            .clone()
    }

    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
fn read_windows_term_program() -> Option<String> {
    let output = std::process::Command::new("cmd.exe")
        .args(["/d", "/s", "/c", "set TERM_PROGRAM"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| {
            line.trim_end_matches('\r')
                .strip_prefix("TERM_PROGRAM=")
                .map(str::to_string)
        })
        .filter(|value| !value.trim().is_empty())
}

/// 启用终端支持的增强按键标志，并按 tmux csi-u 能力选择 modifyOtherKeys。
pub(super) fn enable_keyboard_enhancement() {
    if keyboard_enhancement_disabled() {
        return;
    }

    let running_in_tmux_session = running_in_tmux_session();
    let tmux_extended_keys_format = if running_in_tmux_session {
        read_tmux_extended_keys_format()
    } else {
        None
    };

    let _ = execute!(
        stdout(),
        DisableModifyOtherKeys,
        PushKeyboardEnhancementFlags(keyboard_enhancement_flags(
            terminal_info().name,
            tmux_extended_keys_format.as_deref()
        ))
    );

    if tmux_should_enable_modify_other_keys_for(
        running_in_tmux_session,
        tmux_extended_keys_format.as_deref(),
    ) {
        let _ = execute!(stdout(), EnableModifyOtherKeys);
    }
}

fn keyboard_enhancement_flags(
    terminal_name: TerminalName,
    tmux_extended_keys_format: Option<&str>,
) -> KeyboardEnhancementFlags {
    let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS;

    // iTerm and Ghostty can leak shortcut release events that the terminal consumes.
    // tmux's xterm key format also loses Shift-Enter when event types are
    // reported. Preserve repeat classification on transports that support it.
    // iTerm 与 Ghostty 可能泄漏快捷键释放事件；tmux 的 xterm 格式也会丢失 Shift-Enter，故这些传输不请求事件类型。
    if matches!(terminal_name, TerminalName::Ghostty | TerminalName::Iterm2)
        || matches!(tmux_extended_keys_format, Some("xterm"))
    {
        flags
    } else {
        flags | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
    }
}

fn running_in_tmux_session() -> bool {
    tmux_session_detected(
        std::env::var("TMUX").ok().as_deref(),
        std::env::var("TMUX_PANE").ok().as_deref(),
    )
}

fn tmux_session_detected(tmux: Option<&str>, tmux_pane: Option<&str>) -> bool {
    tmux.is_some() || tmux_pane.is_some()
}

fn tmux_should_enable_modify_other_keys_for(
    running_in_tmux_session: bool,
    extended_keys_format: Option<&str>,
) -> bool {
    // Only request mode 2 when tmux confirms csi-u formatting. Older tmux
    // versions do not expose this option and may emit xterm-style sequences,
    // which crossterm does not parse consistently for modified keys.
    // 仅在 tmux 明确报告 csi-u 时请求 mode 2；旧版本可能输出 Crossterm 无法稳定解析的 xterm 序列。
    running_in_tmux_session && matches!(extended_keys_format, Some("csi-u"))
}

/// 优先使用新式 tmux 查询，失败后回退到全局选项读取扩展键格式。
fn read_tmux_extended_keys_format() -> Option<String> {
    for args in [
        ["display-message", "-p", "#{extended-keys-format}"],
        ["show-options", "-gqv", "extended-keys-format"],
    ] {
        let output = std::process::Command::new("tmux")
            .args(args)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;

        if !output.status.success() {
            continue;
        }

        if let Some(value) = String::from_utf8(output.stdout)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            return Some(value);
        }
    }

    None
}

/// 暂时释放终端时弹出一层增强标志并关闭 modifyOtherKeys。
pub(super) fn restore_keyboard_enhancement_stack() {
    let _ = execute!(
        stdout(),
        PopKeyboardEnhancementFlags,
        DisableModifyOtherKeys
    );
}

/// 进程退出时同时 pop、强制复位增强报告并关闭 modifyOtherKeys。
pub(super) fn reset_keyboard_reporting_after_exit() {
    let _ = execute!(
        stdout(),
        PopKeyboardEnhancementFlags,
        ResetKeyboardEnhancementFlags,
        DisableModifyOtherKeys
    );
}

/// 生成 Kitty 键盘协议“重置全部标志”控制序列。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResetKeyboardEnhancementFlags;

impl Command for ResetKeyboardEnhancementFlags {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        f.write_str("\x1b[<u")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "keyboard enhancement reset is not implemented for the legacy Windows API",
        ))
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        false
    }
}

/// 生成 xterm modifyOtherKeys mode 2 启用序列。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EnableModifyOtherKeys;

impl Command for EnableModifyOtherKeys {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        f.write_str("\x1b[>4;2m")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "modifyOtherKeys enable is not implemented for the legacy Windows API",
        ))
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        false
    }
}

/// 生成关闭 modifyOtherKeys 的复位序列。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DisableModifyOtherKeys;

impl Command for DisableModifyOtherKeys {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        f.write_str("\x1b[>4;0m")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "modifyOtherKeys reset is not implemented for the legacy Windows API",
        ))
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::DisableModifyOtherKeys;
    use super::EnableModifyOtherKeys;
    use super::ResetKeyboardEnhancementFlags;
    use super::keyboard_enhancement_disabled_for;
    use super::keyboard_enhancement_flags;
    use super::parse_bool_env;
    use super::tmux_session_detected;
    use super::tmux_should_enable_modify_other_keys_for;
    use super::vscode_terminal_detected;
    use codex_terminal_detection::TerminalName;
    use crossterm::Command;
    use crossterm::event::PushKeyboardEnhancementFlags;
    use pretty_assertions::assert_eq;

    fn ansi_for(command: impl Command) -> String {
        let mut out = String::new();
        command.write_ansi(&mut out).unwrap();
        out
    }

    #[test]
    // 验证 iTerm 不请求可能泄漏的按键释放事件。
    fn keyboard_enhancement_suppresses_release_reporting_for_iterm() {
        assert_eq!(
            ansi_for(PushKeyboardEnhancementFlags(keyboard_enhancement_flags(
                TerminalName::Iterm2,
                /*tmux_extended_keys_format*/ None
            ))),
            "\x1b[>5u"
        );
    }

    #[test]
    // 验证 Ghostty 采用与 iTerm 相同的释放事件保护。
    fn keyboard_enhancement_suppresses_release_reporting_for_ghostty() {
        assert_eq!(
            ansi_for(PushKeyboardEnhancementFlags(keyboard_enhancement_flags(
                TerminalName::Ghostty,
                /*tmux_extended_keys_format*/ None
            ))),
            "\x1b[>5u"
        );
    }

    #[test]
    // 验证 Kitty 仍保留重复事件分类。
    fn keyboard_enhancement_preserves_repeat_reporting_for_kitty() {
        assert_eq!(
            ansi_for(PushKeyboardEnhancementFlags(keyboard_enhancement_flags(
                TerminalName::Kitty,
                /*tmux_extended_keys_format*/ None
            ))),
            "\x1b[>7u"
        );
    }

    #[test]
    // 验证 tmux csi-u 格式保留重复事件报告。
    fn keyboard_enhancement_preserves_repeat_reporting_for_csi_u_tmux() {
        assert_eq!(
            ansi_for(PushKeyboardEnhancementFlags(keyboard_enhancement_flags(
                TerminalName::Kitty,
                Some("csi-u")
            ))),
            "\x1b[>7u"
        );
    }

    #[test]
    // 验证 tmux xterm 格式不因事件类型报告丢失 Shift-Enter。
    fn keyboard_enhancement_preserves_shift_enter_for_xterm_tmux() {
        assert_eq!(
            ansi_for(PushKeyboardEnhancementFlags(keyboard_enhancement_flags(
                TerminalName::Kitty,
                Some("xterm")
            ))),
            "\x1b[>5u"
        );
    }

    #[test]
    // 验证未知终端采用支持重复事件的默认增强集合。
    fn keyboard_enhancement_preserves_repeat_reporting_for_unknown_terminals() {
        assert_eq!(
            ansi_for(PushKeyboardEnhancementFlags(keyboard_enhancement_flags(
                TerminalName::Unknown,
                /*tmux_extended_keys_format*/ None
            ))),
            "\x1b[>7u"
        );
    }

    #[test]
    // 验证环境开关接受常见布尔拼写并拒绝未知值。
    fn keyboard_enhancement_env_flag_parses_common_values() {
        assert_eq!(parse_bool_env(Some("1")), Some(true));
        assert_eq!(parse_bool_env(Some("true")), Some(true));
        assert_eq!(parse_bool_env(Some("YES")), Some(true));
        assert_eq!(parse_bool_env(Some("0")), Some(false));
        assert_eq!(parse_bool_env(Some("false")), Some(false));
        assert_eq!(parse_bool_env(Some("NO")), Some(false));
        assert_eq!(parse_bool_env(Some("unexpected")), None);
        assert_eq!(parse_bool_env(/*value*/ None), None);
    }

    #[test]
    // 验证 WSL 中的 VS Code 终端会自动禁用增强键盘。
    fn keyboard_enhancement_auto_disables_for_vscode_in_wsl() {
        assert!(keyboard_enhancement_disabled_for(
            /*disable_env*/ None, /*is_wsl*/ true, /*is_vscode_terminal*/ true
        ));
    }

    #[test]
    // 验证自动禁用必须同时满足 WSL 与 VS Code 两个条件。
    fn keyboard_enhancement_auto_disable_requires_wsl_and_vscode() {
        assert!(!keyboard_enhancement_disabled_for(
            /*disable_env*/ None, /*is_wsl*/ true, /*is_vscode_terminal*/ false
        ));
        assert!(!keyboard_enhancement_disabled_for(
            /*disable_env*/ None, /*is_wsl*/ false, /*is_vscode_terminal*/ true
        ));
    }

    #[test]
    // 验证显式环境配置优先于自动探测。
    fn keyboard_enhancement_env_flag_overrides_auto_detection() {
        assert!(!keyboard_enhancement_disabled_for(
            Some("0"),
            /*is_wsl*/ true,
            /*is_vscode_terminal*/ true
        ));
        assert!(keyboard_enhancement_disabled_for(
            Some("1"),
            /*is_wsl*/ false,
            /*is_vscode_terminal*/ false
        ));
    }

    #[test]
    // 验证 VS Code 识别同时接受 Linux 与 Windows 侧 TERM_PROGRAM。
    fn vscode_terminal_detection_uses_linux_and_windows_term_program() {
        assert!(vscode_terminal_detected(
            Some("vscode"),
            /*windows_term_program*/ None
        ));
        assert!(vscode_terminal_detected(
            /*linux_term_program*/ None,
            Some("vscode")
        ));
        assert!(!vscode_terminal_detected(
            /*linux_term_program*/ None,
            Some("WindowsTerminal")
        ));
        assert!(!vscode_terminal_detected(
            /*linux_term_program*/ None, /*windows_term_program*/ None
        ));
    }

    #[test]
    // 验证 TMUX 或 TMUX_PANE 任一存在即可识别会话。
    fn tmux_session_detection_accepts_tmux_or_tmux_pane() {
        assert!(tmux_session_detected(
            Some("/tmp/tmux-501/default,1,0"),
            /*tmux_pane*/ None
        ));
        assert!(tmux_session_detected(/*tmux*/ None, Some("%0")));
        assert!(!tmux_session_detected(
            /*tmux*/ None, /*tmux_pane*/ None
        ));
    }

    #[test]
    // 验证只有明确 csi-u 格式才启用 modifyOtherKeys。
    fn tmux_modify_other_keys_only_requests_confirmed_csi_u_format() {
        assert!(tmux_should_enable_modify_other_keys_for(
            /*running_in_tmux_session*/ true,
            Some("csi-u")
        ));
        assert!(!tmux_should_enable_modify_other_keys_for(
            /*running_in_tmux_session*/ true, /*extended_keys_format*/ None
        ));
        assert!(!tmux_should_enable_modify_other_keys_for(
            /*running_in_tmux_session*/ true,
            Some("xterm")
        ));
        assert!(!tmux_should_enable_modify_other_keys_for(
            /*running_in_tmux_session*/ true,
            Some("")
        ));
        assert!(!tmux_should_enable_modify_other_keys_for(
            /*running_in_tmux_session*/ false,
            Some("csi-u")
        ));
    }

    #[test]
    // 验证强复位命令清除所有已压入的增强层级。
    fn reset_keyboard_enhancement_flags_clears_all_pushed_levels() {
        assert_eq!(ansi_for(ResetKeyboardEnhancementFlags), "\x1b[<u");
    }

    #[test]
    // 验证启用命令写出 xterm mode 2 控制序列。
    fn enable_modify_other_keys_requests_xterm_keyboard_reporting() {
        assert_eq!(ansi_for(EnableModifyOtherKeys), "\x1b[>4;2m");
    }

    #[test]
    // 验证禁用命令写出 xterm 报告复位序列。
    fn disable_modify_other_keys_resets_xterm_keyboard_reporting() {
        assert_eq!(ansi_for(DisableModifyOtherKeys), "\x1b[>4;0m");
    }
}
