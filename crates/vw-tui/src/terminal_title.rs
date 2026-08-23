//! TUI 的终端标题输出和写入前清理辅助。
//!
//! 调用方决定标题何时变化及空标题策略，本模块只负责底层 OSC 写入和清理，不尝试读取或恢复
//! 不可移植的旧标题。标题可能来自模型、线程名、路径或配置，必须移除控制字符、双向/不可见
//! 格式字符和冗余空白，防止终止转义序列或制造视觉误导。
//! Terminal-title output helpers for the TUI.
//!
//! This module owns the low-level OSC title write path and the sanitization
//! that happens immediately before we emit it. It is intentionally narrow:
//! callers decide when the title should change and whether an empty title means
//! "leave the old title alone" or "clear the title Codex last wrote".
//! This module does not attempt to read or restore the terminal's previous
//! title because that is not portable across terminals.
//!
//! Sanitization is necessary because title content is assembled from untrusted
//! text sources such as model output, thread names, project paths, and config.
//! Before we place that text inside an OSC sequence, we strip:
//! - control characters that could terminate or reshape the escape sequence
//! - bidi/invisible formatting codepoints that can visually reorder or hide
//!   text (the same family of issues discussed in Trojan Source writeups)
//! - redundant whitespace that would make titles noisy or hard to scan

use std::fmt;
use std::io;
use std::io::IsTerminal;
use std::io::stdout;

use crossterm::Command;
use ratatui::crossterm::execute;

/// 以 Rust `char` 计的实用标题长度上限；240 为 OSC 框架留出余量并保持标签栏可读。
/// Practical upper bound on title length, measured in Rust `char`s.
///
/// Most terminals silently truncate titles beyond a few hundred characters.
/// 240 leaves headroom for the OSC framing bytes while keeping titles
/// readable in tab bars and window managers.
const MAX_TERMINAL_TITLE_CHARS: usize = 240;

/// `set_terminal_title` 调用结果。
/// Outcome of a [`set_terminal_title`] call.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum SetTerminalTitleResult {
    /// 已写入清理后的标题，或 stdout 非终端而无需写入。
    /// A sanitized title was written, or stdout is not a terminal so no write was needed.
    Applied,
    /// 清理移除了全部可见字符，因此未发送标题；这不同于清除标题，具体策略由上层决定。
    /// Sanitization removed every visible character, so no title was emitted.
    ///
    /// This is distinct from clearing the title. Callers decide whether an
    /// empty post-sanitization value should result in no-op behavior, clearing
    /// the title Codex manages, or some other fallback.
    NoVisibleContent,
}

/// 把清理后的 OSC 窗口标题序列写入 stdout；不可信输入会去除控制/不可见字符、折叠空白并限长。
/// 清理后无可见内容时返回 `NoVisibleContent`，不擅自清除标题。
/// Writes a sanitized OSC window-title sequence to stdout.
///
/// The input is treated as untrusted display text: control characters,
/// invisible formatting characters, and redundant whitespace are removed before
/// the title is emitted. If sanitization removes all visible content, the
/// function returns [`SetTerminalTitleResult::NoVisibleContent`] instead of
/// clearing the title because clearing and restoring are policy decisions for
/// higher-level callers. Mechanically, sanitization collapses whitespace runs
/// to single spaces, drops disallowed codepoints, and bounds the result to
/// [`MAX_TERMINAL_TITLE_CHARS`] visible characters before writing OSC 0.
pub(crate) fn set_terminal_title(title: &str) -> io::Result<SetTerminalTitleResult> {
    if !stdout().is_terminal() {
        return Ok(SetTerminalTitleResult::Applied);
    }

    let title = sanitize_terminal_title(title);
    if title.is_empty() {
        return Ok(SetTerminalTitleResult::NoVisibleContent);
    }

    execute!(stdout(), SetWindowTitle(title))?;
    Ok(SetTerminalTitleResult::Applied)
}

/// 写入空 OSC 标题载荷以清除当前可见标题，但不恢复 Codex 启动前的 shell 或程序标题。
/// Clears the current terminal title by writing an empty OSC title payload.
///
/// This clears the visible title; it does not restore whatever title the shell
/// or a previous program may have set before Codex started managing the title.
pub(crate) fn clear_terminal_title() -> io::Result<()> {
    if !stdout().is_terminal() {
        return Ok(());
    }

    execute!(stdout(), SetWindowTitle(String::new()))
}

#[derive(Debug, Clone)]
struct SetWindowTitle(String);

impl Command for SetWindowTitle {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        // 匹配 Crossterm 的 `SetTitle`，用 BEL 结束 OSC 0；部分集成虽接受 ST，
        // 却会把 ST 暴露在进程装饰中。
        // Match crossterm's SetTitle command and terminate OSC 0 with BEL.
        // Some terminal title integrations expose the ST terminator in process
        // decorations even though they otherwise accept the title update.
        write!(f, "\x1b]0;{}\x07", self.0)
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> io::Result<()> {
        Err(std::io::Error::other(
            "tried to execute SetWindowTitle using WinAPI; use ANSI instead",
        ))
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        true
    }
}

/// 将不可信标题规范化为有界单行：去除控制及双向格式字符、折叠空白并按字符数截断。
/// Normalizes untrusted title text into a single bounded display line.
///
/// This removes terminal control characters, strips invisible/bidi formatting
/// characters, collapses any whitespace run into a single ASCII space, and
/// truncates after [`MAX_TERMINAL_TITLE_CHARS`] emitted characters.
fn sanitize_terminal_title(title: &str) -> String {
    let mut sanitized = String::new();
    let mut chars_written = 0;
    let mut pending_space = false;

    for ch in title.chars() {
        if ch.is_whitespace() {
            // 只有已写入内容时才标记待补空格，从而无需额外 trim 即可去除前导空白。
            // Only set pending if we've already written content; this
            // strips leading whitespace without an extra trim pass.
            pending_space = !sanitized.is_empty();
            continue;
        }

        if is_disallowed_terminal_title_char(ch) {
            continue;
        }

        if pending_space {
            let remaining = MAX_TERMINAL_TITLE_CHARS.saturating_sub(chars_written);
            if remaining > 1 {
                sanitized.push(' ');
                chars_written += 1;
                pending_space = false;
            }
        }

        if chars_written >= MAX_TERMINAL_TITLE_CHARS {
            break;
        }

        sanitized.push(ch);
        chars_written += 1;
    }

    sanitized
}

/// 判断字符是否应从终端标题丢弃，包括普通控制字符和可能重排/隐藏文字的精选不可见格式字符。
/// Returns whether `ch` should be dropped from terminal-title output.
///
/// This includes both plain control characters and a curated set of invisible
/// formatting codepoints. The bidi entries here cover the Trojan-Source-style
/// text-reordering controls that can make a title render misleadingly relative
/// to its underlying byte sequence.
fn is_disallowed_terminal_title_char(ch: char) -> bool {
    if ch.is_control() {
        return true;
    }

    // 移除 Trojan Source 相关双向控制及常见非渲染格式字符，防止夹带终端控制语义或视觉误导。
    // Strip Trojan-Source-related bidi controls plus common non-rendering
    // formatting characters so title text cannot smuggle terminal control
    // semantics or visually misleading content.
    matches!(
        ch,
        '\u{00AD}'
            | '\u{034F}'
            | '\u{061C}'
            | '\u{180E}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{E0100}'..='\u{E01EF}'
    )
}

#[cfg(test)]
mod tests {
    use super::MAX_TERMINAL_TITLE_CHARS;
    use super::SetWindowTitle;
    use super::sanitize_terminal_title;
    use crossterm::Command;
    use pretty_assertions::assert_eq;

    #[test]
    fn sanitizes_terminal_title() {
        let sanitized =
            sanitize_terminal_title("  Project\t|\nWorking\x1b\x07\u{009D}\u{009C} |  Thread  ");
        assert_eq!(sanitized, "Project | Working | Thread");
    }

    #[test]
    fn strips_invisible_format_chars_from_terminal_title() {
        let sanitized = sanitize_terminal_title(
            "Pro\u{202E}j\u{2066}e\u{200F}c\u{061C}t\u{200B} \u{FEFF}T\u{2060}itle",
        );
        assert_eq!(sanitized, "Project Title");
    }

    #[test]
    fn truncates_terminal_title() {
        let input = "a".repeat(MAX_TERMINAL_TITLE_CHARS + 10);
        let sanitized = sanitize_terminal_title(&input);
        assert_eq!(sanitized.len(), MAX_TERMINAL_TITLE_CHARS);
    }

    #[test]
    fn truncation_prefers_visible_char_over_pending_space() {
        let input = format!("{} b", "a".repeat(MAX_TERMINAL_TITLE_CHARS - 1));
        let sanitized = sanitize_terminal_title(&input);
        assert_eq!(sanitized.len(), MAX_TERMINAL_TITLE_CHARS);
        assert_eq!(sanitized.chars().last(), Some('b'));
    }

    #[test]
    fn writes_osc_title_with_bel_terminator() {
        let mut out = String::new();
        SetWindowTitle("hello".to_string())
            .write_ansi(&mut out)
            .expect("encode terminal title");
        assert_eq!(out, "\x1b]0;hello\x07");
    }
}
