//! 将最终历史行插入终端回滚区。
//!
//! VibeWindow v3 直接使用终端回滚区保存最终聊天历史，因此插入历史单元是转义序列操作，
//! 而不是普通 Ratatui 渲染。
//! Inserts finalized history rows into terminal scrollback.
//!
//! VibeWindow v3 uses terminal scrollback itself for finalized chat history, so inserting a history
//! cell is an escape-sequence operation rather than a normal ratatui render.

use std::fmt;
use std::io;
use std::io::Write;

use tui_render::line_utils::line_to_static;
use tui_render::terminal_hyperlinks::HyperlinkLine;
use tui_render::terminal_hyperlinks::decorate_spans;
use tui_render::terminal_hyperlinks::plain_hyperlink_lines;
use tui_render::terminal_hyperlinks::remap_wrapped_line;
use tui_text::wrapping::RtOptions;
use tui_text::wrapping::adaptive_wrap_line;
use tui_text::wrapping::line_contains_url_like;
use tui_text::wrapping::line_has_mixed_url_and_non_url_tokens;
use crossterm::Command;
use crossterm::cursor::MoveDown;
use crossterm::cursor::MoveTo;
use crossterm::cursor::MoveToColumn;
use crossterm::cursor::RestorePosition;
use crossterm::cursor::SavePosition;
use crossterm::queue;
use crossterm::style::Color as CColor;
use crossterm::style::Colors;
use crossterm::style::Print;
use crossterm::style::SetAttribute;
use crossterm::style::SetBackgroundColor;
use crossterm::style::SetColors;
use crossterm::style::SetForegroundColor;
use crossterm::terminal::Clear;
use crossterm::terminal::ClearType;
use ratatui::layout::Size;
use ratatui::prelude::Backend;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::text::Line;
use ratatui::text::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryLineWrapPolicy {
    PreWrap,
    Terminal,
}

/// 选择在视口上方写入历史时使用的终端转义策略；全屏插入可在局部滚动区域不可靠时
/// 保留终端原生回滚区，并为 Zellij 保持终端管理的软换行。
/// Selects the terminal escape strategy used when writing history above the viewport.
///
/// Full-screen insertion preserves terminal-native scrollback when partial scroll regions are
/// unreliable and keeps terminal-managed soft wrapping intact for Zellij.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertHistoryMode {
    Standard,
    FullScreen,
}

/// 使用终端后端 writer 在视口上方插入 `lines`，避免直接引用 stdout。
/// Insert `lines` above the viewport using the terminal's backend writer
/// (avoids direct stdout references).
pub fn insert_history_lines<B>(
    terminal: &mut crate::custom_terminal::Terminal<B>,
    lines: Vec<Line>,
) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
{
    insert_history_lines_with_wrap_policy(terminal, lines, HistoryLineWrapPolicy::PreWrap)
}

pub fn insert_history_lines_with_wrap_policy<B>(
    terminal: &mut crate::custom_terminal::Terminal<B>,
    lines: Vec<Line>,
    wrap_policy: HistoryLineWrapPolicy,
) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
{
    insert_history_lines_with_mode_and_wrap_policy(
        terminal,
        lines,
        InsertHistoryMode::Standard,
        wrap_policy,
    )
}

pub fn insert_history_lines_with_mode_and_wrap_policy<B>(
    terminal: &mut crate::custom_terminal::Terminal<B>,
    lines: Vec<Line>,
    mode: InsertHistoryMode,
    wrap_policy: HistoryLineWrapPolicy,
) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
{
    let screen_size = terminal.last_known_screen_size;
    insert_history_hyperlink_lines_with_mode_and_wrap_policy(
        terminal,
        &plain_hyperlink_lines(lines.iter().map(line_to_static).collect()),
        mode,
        wrap_policy,
        screen_size,
    )
}

pub fn insert_history_hyperlink_lines_with_mode_and_wrap_policy<B>(
    terminal: &mut crate::custom_terminal::Terminal<B>,
    lines: &[HyperlinkLine],
    mode: InsertHistoryMode,
    wrap_policy: HistoryLineWrapPolicy,
    screen_size: Size,
) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
{
    let mut area = terminal.viewport_area;
    let mut should_update_area = false;
    let last_cursor_pos = terminal.last_known_cursor_pos;

    // 为终端回滚区预换行：近似纯 URL 行保持完整以便终端识别可点击链接；
    // URL 与正文混合行自适应换行且不拆 URL；普通行也走同一路径并退化为标准换行。
    // Pre-wrap lines for terminal scrollback. Three paths:
    //
    // - URL-only-ish lines are kept intact (no hard newlines inserted) so that
    //   terminal emulators can match them as clickable links. The
    //   terminal will character-wrap these lines at the viewport
    //   boundary.
    // - Mixed lines (URL + non-URL prose) are adaptively wrapped so
    //   non-URL text still wraps naturally while URL tokens remain
    //   unsplit.
    // - Non-URL lines also flow through adaptive wrapping; behavior is
    //   equivalent to standard wrapping when no URL is present.
    let wrap_width = area.width.max(1) as usize;
    let (wrapped, wrapped_rows) = wrap_history_hyperlink_lines(lines, wrap_width, wrap_policy);
    let wrapped_lines = wrapped_rows as u16;
    match mode {
        InsertHistoryMode::FullScreen => {
            // 当前视口会在同一绘制帧立即替换；先清除，避免终端滚动把输入框内容卷入历史。
            // The existing viewport is immediately replaced in the same draw pass. Clear it
            // before terminal scrolling can move composer contents into scrollback.
            terminal.clear_after_position(area.as_position())?;
            let writer = terminal.backend_mut();
            queue!(writer, MoveTo(/*x*/ 0, area.top()))?;
            for (index, line) in wrapped.iter().enumerate() {
                if index > 0 {
                    queue!(writer, Print("\r\n"))?;
                }
                write_history_line(writer, line, wrap_width)?;
            }

            // 直接写原始文本可保留终端软换行元数据；即使重放批次高于可见历史区，
            // 也通过空视口行推进，使历史紧邻输入框上方结束。
            // Writing raw source text through the terminal preserves its soft-wrap metadata.
            // Advance through empty rows for the viewport so history ends immediately above the
            // composer even when a replay batch is taller than the visible history region.
            for _ in 0..area.height {
                queue!(writer, Print("\r\n"), Clear(ClearType::UntilNewLine))?;
            }
            queue!(writer, MoveTo(last_cursor_pos.x, last_cursor_pos.y))?;

            let viewport_top = area
                .top()
                .saturating_add(wrapped_lines)
                .min(screen_size.height.saturating_sub(area.height));
            if area.y != viewport_top {
                area.y = viewport_top;
                should_update_area = true;
            }
        }
        InsertHistoryMode::Standard => {
            let writer = terminal.backend_mut();
            let cursor_top = if area.bottom() < screen_size.height {
                // 视口不在屏幕底部时向下滚动腾出空间，但不得越过屏幕底部。
                // If the viewport is not at the bottom of the screen, scroll it down to make room.
                // Don't scroll it past the bottom of the screen.
                let scroll_amount = wrapped_lines.min(screen_size.height - area.bottom());

                let top_1based = area.top() + 1;
                queue!(writer, SetScrollRegion(top_1based..screen_size.height))?;
                queue!(writer, MoveTo(/*x*/ 0, area.top()))?;
                for _ in 0..scroll_amount {
                    queue!(writer, Print("\x1bM"))?;
                }
                queue!(writer, ResetScrollRegion)?;

                let cursor_top = area.top().saturating_sub(1);
                area.y += scroll_amount;
                should_update_area = true;
                cursor_top
            } else {
                area.top().saturating_sub(1)
            };

            // 将滚动区域限制在屏幕顶部到视口顶部，光标置于区域末端后插入行，
            // 从而只滚动历史区域而保持视口内容不动。
            // Limit the scroll region to the lines from the top of the screen to the
            // top of the viewport. With this in place, when we add lines inside this
            // area, only the lines in this area will be scrolled. We place the cursor
            // at the end of the scroll region, and add lines starting there.
            //
            // ┌─Screen───────────────────────┐
            // │┌╌Scroll region╌╌╌╌╌╌╌╌╌╌╌╌╌╌┐│
            // │┆                            ┆│
            // │┆                            ┆│
            // │┆                            ┆│
            // │█╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┘│
            // │╭─Viewport───────────────────╮│
            // ││                            ││
            // │╰────────────────────────────╯│
            // └──────────────────────────────┘
            queue!(writer, SetScrollRegion(1..area.top()))?;

            // 此处用 `MoveTo` 而非 `set_cursor_position`，避免修改终端缓存的最近光标位置；
            // 获取并恢复光标后，历史插入应保持光标位置中性。
            // NB: we are using MoveTo instead of set_cursor_position here to avoid messing with the
            // terminal's last_known_cursor_position, which hopefully will still be accurate after we
            // fetch/restore the cursor position. insert_history_lines should be cursor-position-neutral :)
            queue!(writer, MoveTo(/*x*/ 0, cursor_top))?;

            for line in &wrapped {
                queue!(writer, Print("\r\n"))?;
                write_history_line(writer, line, wrap_width)?;
            }

            queue!(writer, ResetScrollRegion)?;
            queue!(writer, MoveTo(last_cursor_pos.x, last_cursor_pos.y))?;
        }
    }

    if should_update_area {
        terminal.set_viewport_area(area);
    }
    if wrapped_lines > 0 {
        terminal.note_history_rows_inserted(wrapped_lines);
    }

    Ok(())
}

pub fn wrap_history_hyperlink_lines(
    lines: &[HyperlinkLine],
    wrap_width: usize,
    wrap_policy: HistoryLineWrapPolicy,
) -> (Vec<HyperlinkLine>, usize) {
    let mut wrapped = Vec::new();
    let mut wrapped_rows = 0usize;

    for line in lines {
        let line_wrapped = match wrap_policy {
            HistoryLineWrapPolicy::Terminal => vec![line.clone()],
            HistoryLineWrapPolicy::PreWrap
                if line_contains_url_like(&line.line)
                    && !line_has_mixed_url_and_non_url_tokens(&line.line) =>
            {
                vec![line.clone()]
            }
            HistoryLineWrapPolicy::PreWrap => remap_wrapped_line(
                line,
                adaptive_wrap_line(
                    &line.line,
                    RtOptions::new(wrap_width)
                        .subsequent_indent(leading_whitespace_prefix(&line.line)),
                )
                .into_iter()
                .map(|line| line_to_static(&line))
                .collect(),
            ),
        };
        wrapped_rows += line_wrapped
            .iter()
            .map(|wrapped_line| wrapped_line.width().max(/*other*/ 1).div_ceil(wrap_width))
            .sum::<usize>();
        wrapped.extend(line_wrapped);
    }

    (wrapped, wrapped_rows)
}

pub(crate) fn leading_whitespace_prefix(line: &Line<'_>) -> Line<'static> {
    let mut spans = Vec::new();
    for span in &line.spans {
        let prefix_end = span
            .content
            .char_indices()
            .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
            .unwrap_or(span.content.len());
        if prefix_end > 0 {
            spans.push(Span::styled(span.content[..prefix_end].to_string(), span.style));
        }
        if prefix_end < span.content.len() {
            break;
        }
    }
    Line::from(spans).style(line.style)
}

/// 渲染单条已换行历史：为宽行清除续行、设置前景/背景并写入带样式 span；
/// 调用方负责光标定位和前导 `\r\n`。
/// Render a single wrapped history line: clear continuation rows for wide lines,
/// set foreground/background colors, and write styled spans. Caller is responsible
/// for cursor positioning and any leading `\r\n`.
fn write_history_line<W: Write>(
    writer: &mut W,
    line: &HyperlinkLine,
    wrap_width: usize,
) -> io::Result<()> {
    let physical_rows = line.width().max(1).div_ceil(wrap_width) as u16;
    if physical_rows > 1 {
        queue!(writer, SavePosition)?;
        for _ in 1..physical_rows {
            queue!(writer, MoveDown(1), MoveToColumn(0))?;
            queue!(writer, Clear(ClearType::UntilNewLine))?;
        }
        queue!(writer, RestorePosition)?;
    }
    queue!(
        writer,
        SetColors(Colors::new(
            line.line
                .style
                .fg
                .map(crate::custom_terminal::crossterm_color)
                .unwrap_or(CColor::Reset),
            line.line
                .style
                .bg
                .map(crate::custom_terminal::crossterm_color)
                .unwrap_or(CColor::Reset)
        ))
    )?;
    queue!(writer, Clear(ClearType::UntilNewLine))?;
    // 将行级样式合并到每个 span，使 ANSI 颜色反映块引用等行样式。
    // Merge line-level style into each span so that ANSI colors reflect
    // line styles (e.g., blockquotes with green fg).
    let merged_spans: Vec<Span> = line
        .line
        .spans
        .iter()
        .map(|s| Span { style: s.style.patch(line.line.style), content: s.content.clone() })
        .collect();
    let merged_line =
        HyperlinkLine { line: Line::from(merged_spans), hyperlinks: line.hyperlinks.clone() };
    let decorated = decorate_spans(&merged_line);
    write_spans(writer, decorated.iter())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetScrollRegion(pub std::ops::Range<u16>);

impl Command for SetScrollRegion {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        write!(f, "\x1b[{};{}r", self.0.start, self.0.end)
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::io::Result<()> {
        panic!("tried to execute SetScrollRegion command using WinAPI, use ANSI instead");
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        // TODO(nornagon)：确认 Windows 是否支持此路径。
        // TODO(nornagon): is this supported on Windows?
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResetScrollRegion;

impl Command for ResetScrollRegion {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        write!(f, "\x1b[r")
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> std::io::Result<()> {
        panic!("tried to execute ResetScrollRegion command using WinAPI, use ANSI instead");
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        // TODO(nornagon)：确认 Windows 是否支持此路径。
        // TODO(nornagon): is this supported on Windows?
        true
    }
}

struct ModifierDiff {
    pub from: Modifier,
    pub to: Modifier,
}

impl ModifierDiff {
    fn queue<W>(self, mut w: W) -> io::Result<()>
    where
        W: io::Write,
    {
        use crossterm::style::Attribute as CAttribute;
        let removed = self.from - self.to;
        if removed.contains(Modifier::REVERSED) {
            queue!(w, SetAttribute(CAttribute::NoReverse))?;
        }
        if removed.contains(Modifier::BOLD) {
            queue!(w, SetAttribute(CAttribute::NormalIntensity))?;
            if self.to.contains(Modifier::DIM) {
                queue!(w, SetAttribute(CAttribute::Dim))?;
            }
        }
        if removed.contains(Modifier::ITALIC) {
            queue!(w, SetAttribute(CAttribute::NoItalic))?;
        }
        if removed.contains(Modifier::UNDERLINED) {
            queue!(w, SetAttribute(CAttribute::NoUnderline))?;
        }
        if removed.contains(Modifier::DIM) {
            queue!(w, SetAttribute(CAttribute::NormalIntensity))?;
        }
        if removed.contains(Modifier::CROSSED_OUT) {
            queue!(w, SetAttribute(CAttribute::NotCrossedOut))?;
        }
        if removed.contains(Modifier::SLOW_BLINK) || removed.contains(Modifier::RAPID_BLINK) {
            queue!(w, SetAttribute(CAttribute::NoBlink))?;
        }

        let added = self.to - self.from;
        if added.contains(Modifier::REVERSED) {
            queue!(w, SetAttribute(CAttribute::Reverse))?;
        }
        if added.contains(Modifier::BOLD) {
            queue!(w, SetAttribute(CAttribute::Bold))?;
        }
        if added.contains(Modifier::ITALIC) {
            queue!(w, SetAttribute(CAttribute::Italic))?;
        }
        if added.contains(Modifier::UNDERLINED) {
            queue!(w, SetAttribute(CAttribute::Underlined))?;
        }
        if added.contains(Modifier::DIM) {
            queue!(w, SetAttribute(CAttribute::Dim))?;
        }
        if added.contains(Modifier::CROSSED_OUT) {
            queue!(w, SetAttribute(CAttribute::CrossedOut))?;
        }
        if added.contains(Modifier::SLOW_BLINK) {
            queue!(w, SetAttribute(CAttribute::SlowBlink))?;
        }
        if added.contains(Modifier::RAPID_BLINK) {
            queue!(w, SetAttribute(CAttribute::RapidBlink))?;
        }

        Ok(())
    }
}

fn write_spans<'a, I>(mut writer: &mut impl Write, content: I) -> io::Result<()>
where
    I: IntoIterator<Item = &'a Span<'a>>,
{
    let mut fg = Color::Reset;
    let mut bg = Color::Reset;
    let mut last_modifier = Modifier::empty();
    for span in content {
        let mut modifier = Modifier::empty();
        modifier.insert(span.style.add_modifier);
        modifier.remove(span.style.sub_modifier);
        if modifier != last_modifier {
            let diff = ModifierDiff { from: last_modifier, to: modifier };
            diff.queue(&mut writer)?;
            last_modifier = modifier;
        }
        let next_fg = span.style.fg.unwrap_or(Color::Reset);
        let next_bg = span.style.bg.unwrap_or(Color::Reset);
        if next_fg != fg || next_bg != bg {
            queue!(
                writer,
                SetColors(Colors::new(
                    crate::custom_terminal::crossterm_color(next_fg),
                    crate::custom_terminal::crossterm_color(next_bg)
                ))
            )?;
            fg = next_fg;
            bg = next_bg;
        }

        queue!(writer, Print(&span.content))?;
    }

    queue!(
        writer,
        SetForegroundColor(CColor::Reset),
        SetBackgroundColor(CColor::Reset),
        SetAttribute(crossterm::style::Attribute::Reset),
    )
}

#[cfg(test)]
#[path = "insert_history_tests.rs"]
mod tests;
