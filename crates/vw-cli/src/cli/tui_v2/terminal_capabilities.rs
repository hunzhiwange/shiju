//! 终端颜色与背景能力。

use std::io::{self, Write};

use crossterm::ExecutableCommand;
use crossterm::cursor::MoveTo;
use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};

/// 将不可信文本中的终端控制字符替换为可见占位符。
///
/// 换行和制表符仍是正常的文本布局输入；其余 C0、DEL 与 C1 字符都不能进入
/// renderer/backend，避免被解释为 ANSI、OSC 或其他终端控制序列。
pub(crate) fn sanitize_terminal_text(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if (character.is_control() && !matches!(character, '\n' | '\t'))
                || ('\u{80}'..='\u{9f}').contains(&character)
            {
                '\u{fffd}'
            } else {
                character
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiColorLevel {
    None,
    Ansi,
    Ansi256,
    TrueColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiBackground {
    Dark,
    Light,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerminalCapabilities {
    pub(crate) color_level: TuiColorLevel,
    pub(crate) background: TuiBackground,
    pub(crate) apple_terminal: bool,
    pub(crate) osc8_hyperlinks: bool,
}

impl TerminalCapabilities {
    pub(crate) fn detect(background: TuiBackground) -> Self {
        Self::from_environment_values(
            std::env::var_os("NO_COLOR").is_some(),
            std::env::var("COLORTERM").ok().as_deref(),
            std::env::var("TERM").ok().as_deref(),
            std::env::var("TERM_PROGRAM").ok().as_deref(),
            background,
        )
    }

    pub(crate) fn from_environment_values(
        no_color: bool,
        color_term: Option<&str>,
        term: Option<&str>,
        terminal_program: Option<&str>,
        background: TuiBackground,
    ) -> Self {
        let apple_terminal =
            terminal_program.is_some_and(|value| value.eq_ignore_ascii_case("Apple_Terminal"));
        let color_term = color_term.unwrap_or_default().trim().to_ascii_lowercase();
        let term = term.unwrap_or_default().trim().to_ascii_lowercase();
        let color_level = if no_color || term == "dumb" {
            TuiColorLevel::None
        } else if apple_terminal {
            TuiColorLevel::Ansi256
        } else if matches!(color_term.as_str(), "truecolor" | "24bit")
            || term.contains("truecolor")
            || term.contains("direct")
        {
            TuiColorLevel::TrueColor
        } else if term.contains("256color") {
            TuiColorLevel::Ansi256
        } else {
            TuiColorLevel::Ansi
        };
        let osc8_hyperlinks = !no_color
            && term != "dumb"
            && !apple_terminal
            && terminal_program.is_some_and(|value| {
                ["iterm.app", "wezterm", "kitty", "ghostty", "vscode", "windows_terminal"]
                    .iter()
                    .any(|supported| value.eq_ignore_ascii_case(supported))
            });

        Self { color_level, background, apple_terminal, osc8_hyperlinks }
    }
}

impl Default for TerminalCapabilities {
    fn default() -> Self {
        Self {
            color_level: TuiColorLevel::TrueColor,
            background: TuiBackground::Dark,
            apple_terminal: false,
            osc8_hyperlinks: false,
        }
    }
}

/// fullscreen 模式能力计划。非 TTY 与 dumb terminal 使用空计划，保留 scrollback。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerminalModePlan {
    fullscreen: bool,
    extended_keyboard: bool,
}

impl TerminalModePlan {
    pub(crate) fn for_environment(
        is_tty: bool,
        term: Option<&str>,
        _terminal_program: Option<&str>,
    ) -> Self {
        let term = term.unwrap_or_default().trim();
        let fullscreen = is_tty && !term.eq_ignore_ascii_case("dumb");
        let extended_keyboard = fullscreen;
        Self { fullscreen, extended_keyboard }
    }

    pub(crate) fn is_fullscreen(self) -> bool {
        self.fullscreen
    }

    fn write_enter(self, writer: &mut impl Write) -> io::Result<()> {
        if !self.fullscreen {
            return Ok(());
        }

        writer.execute(EnterAlternateScreen)?;
        writer.execute(Clear(ClearType::All))?;
        writer.execute(MoveTo(0, 0))?;
        writer.execute(EnableMouseCapture)?;
        writer.execute(EnableBracketedPaste)?;
        writer.execute(EnableFocusChange)?;
        if self.extended_keyboard {
            writer.execute(PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
            ))?;
        }
        Ok(())
    }

    fn write_leave(self, writer: &mut impl Write) -> io::Result<()> {
        if !self.fullscreen {
            return Ok(());
        }

        if self.extended_keyboard {
            writer.execute(PopKeyboardEnhancementFlags)?;
        }
        writer.execute(DisableFocusChange)?;
        writer.execute(DisableBracketedPaste)?;
        writer.execute(DisableMouseCapture)?;
        writer.execute(LeaveAlternateScreen)?;
        Ok(())
    }
}

/// 记录当前模式是否已进入，保证正常、错误、panic 和挂起路径可以统一恢复。
#[derive(Debug)]
pub(crate) struct TerminalModeSession {
    plan: TerminalModePlan,
    active: bool,
}

impl TerminalModeSession {
    pub(crate) fn new(plan: TerminalModePlan) -> Self {
        Self { plan, active: false }
    }

    pub(crate) fn enter(&mut self, writer: &mut impl Write) -> io::Result<()> {
        if self.active || !self.plan.is_fullscreen() {
            return Ok(());
        }

        self.active = true;
        if let Err(error) = self.plan.write_enter(writer) {
            let _ = self.plan.write_leave(writer);
            self.active = false;
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn leave(&mut self, writer: &mut impl Write) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }

        self.plan.write_leave(writer)?;
        self.active = false;
        Ok(())
    }

    pub(crate) fn suspend(&mut self, writer: &mut impl Write) -> io::Result<()> {
        self.leave(writer)
    }

    pub(crate) fn resume(&mut self, writer: &mut impl Write) -> io::Result<()> {
        self.enter(writer)
    }
}
