use super::*;
use std::num::NonZeroU16;

use crate::test_backend::VT100Backend;
use insta::assert_snapshot;
use pretty_assertions::assert_eq;
use ratatui::backend::WindowSize;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;

pub(crate) struct CaptureBackend {
    output: Vec<u8>,
    size: Size,
    cursor: Position,
    pub(crate) size_call_count: std::cell::Cell<usize>,
}

impl CaptureBackend {
    pub(crate) fn new(width: u16, height: u16) -> Self {
        Self {
            output: Vec::new(),
            size: Size { width, height },
            cursor: Position { x: 0, y: 0 },
            size_call_count: std::cell::Cell::new(/*value*/ 0),
        }
    }

    pub(crate) fn output(&self) -> String {
        String::from_utf8_lossy(&self.output).into_owned()
    }
}

impl Write for CaptureBackend {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Backend for CaptureBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, _content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        Ok(self.cursor)
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.cursor = position.into();
        Ok(())
    }

    fn clear(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn clear_region(&mut self, _clear_type: ClearType) -> io::Result<()> {
        Ok(())
    }

    fn append_lines(&mut self, _line_count: u16) -> io::Result<()> {
        Ok(())
    }

    fn size(&self) -> io::Result<Size> {
        self.size_call_count.set(self.size_call_count.get().saturating_add(/*rhs*/ 1));
        Ok(self.size)
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        Ok(WindowSize { columns_rows: self.size, pixels: self.size })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn invalidate_viewport_repaints_default_style_spaces_over_stale_terminal_cells() {
    let width = 32;
    let height = 2;
    let area = Rect::new(/*x*/ 0, /*y*/ 0, width, height);
    let mut terminal =
        Terminal::with_options(VT100Backend::new(width, height)).expect("terminal");
    terminal.set_viewport_area(area);

    // 历史插入直接写终端，会留下差异缓冲中不存在的单元；内部默认样式空格仍必须覆盖这些旧内容。
    // History insertion writes directly to the terminal, leaving cells that are absent from
    // the diff buffers. Interior default-style spaces must still overwrite those stale cells.
    write!(terminal.backend_mut(), "probe-08tcleantwords stale\r\nprobe-09xcleanxwords stale")
        .expect("prefill terminal");
    assert!(terminal.backend().vt100().screen().contents().contains("probe-08tcleantwords"));

    terminal.invalidate_viewport();
    terminal
        .draw(|frame| {
            Paragraph::new(vec![
                Line::from("probe-08 clean words"),
                Line::from("probe-09 clean words"),
            ])
            .render(area, frame.buffer_mut());
        })
        .expect("redraw invalidated viewport");

    assert_snapshot!(terminal.backend().vt100().screen().contents(), @r"
    probe-08 clean words
    probe-09 clean words
    ");
}


#[test]
fn resize_draw_applies_event_dimensions_without_querying_backend_size() {
    let mut terminal =
        Terminal::with_options(CaptureBackend::new(/*width*/ 12, /*height*/ 4))
            .expect("terminal");
    let mut snapshots = Vec::new();

    for width in [12, 8] {
        let size = Size::new(width, /*height*/ 4);
        let area = Rect::new(/*x*/ 0, /*y*/ 0, size.width, size.height);
        terminal.set_viewport_area(area);
        terminal
            .draw_with_size(size, |frame| {
                Paragraph::new("alpha beta")
                    .wrap(Wrap { trim: false })
                    .render(area, frame.buffer_mut());
            })
            .expect("draw resized frame");

        let rendered = (0..size.height)
            .map(|y| {
                (0..size.width)
                    .map(|x| terminal.previous_buffer()[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        snapshots.push(rendered.trim_end().to_string());
    }

    assert_eq!(terminal.backend().size_call_count.get(), 1);
    assert_eq!(terminal.last_known_screen_size, Size::new(/*width*/ 8, /*height*/ 4));
    assert_snapshot!(snapshots.join("\n\n"), @r"
    alpha beta

    alpha
    beta
    ");
}

#[test]
fn diff_buffers_only_updates_changed_cells_when_row_tails_are_unchanged() {
    let area = Rect::new(/*x*/ 0, /*y*/ 0, /*width*/ 10, /*height*/ 2);
    let mut previous = Buffer::empty(area);
    previous.set_string(0, 0, "-", Style::default());
    previous.set_string(0, 1, "中", Style::default());

    assert_eq!(diff_buffers(&previous, &previous).len(), 0);

    let mut next = previous.clone();
    next.set_string(0, 0, "\\", Style::default());

    let commands = diff_buffers(&previous, &next);
    assert_eq!(commands.len(), 1, "unexpected draw commands: {commands:?}");
    assert!(matches!(
        commands.as_slice(),
        [DrawCommand::Put { x: 0, y: 0, cell }] if cell.symbol() == "\\"
    ));
}

#[test]
fn diff_buffers_does_not_emit_clear_to_end_for_full_width_row() {
    let area = Rect::new(0, 0, 3, 2);
    let previous = Buffer::empty(area);
    let mut next = Buffer::empty(area);

    next.cell_mut((2, 0)).expect("cell should exist").set_symbol("X");

    let commands = diff_buffers(&previous, &next);

    let clear_count = commands
        .iter()
        .filter(|command| matches!(command, DrawCommand::ClearToEnd { y, .. } if *y == 0))
        .count();
    assert_eq!(
        0, clear_count,
        "expected diff_buffers not to emit ClearToEnd; commands: {commands:?}",
    );
    assert!(
        commands.iter().any(|command| matches!(command, DrawCommand::Put { x: 2, y: 0, .. })),
        "expected diff_buffers to update the final cell; commands: {commands:?}",
    );
}

#[test]
fn diff_buffers_clear_to_end_starts_after_wide_char() {
    let area = Rect::new(0, 0, 10, 1);
    for (before, after) in [("中文", "中"), ("ｶﾞﾞ", "ｶﾞ")] {
        let mut previous = Buffer::empty(area);
        let mut next = Buffer::empty(area);

        previous.set_string(0, 0, before, Style::default());
        next.set_string(0, 0, after, Style::default());

        let commands = diff_buffers(&previous, &next);
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, DrawCommand::ClearToEnd { x: 2, y: 0, .. })),
            "expected clear-to-end after {before:?} became {after:?}; commands: {commands:?}"
        );
    }

    let mut terminal =
        Terminal::with_options(VT100Backend::new(area.width, area.height)).expect("terminal");
    terminal.set_viewport_area(area);
    for text in ["ｶﾞﾞ", "ｶﾞ"] {
        terminal
            .draw(|frame| Paragraph::new(text).render(area, frame.buffer_mut()))
            .expect("draw");
    }
    assert_snapshot!(terminal.backend().vt100().screen().contents(), @"ｶﾞ");
}


#[test]
fn diff_buffers_emits_always_update_cells() {
    use ratatui::buffer::CellDiffOption;

    for text in ["abc", "a  "] {
        let mut previous = Buffer::with_lines([text]);
        let mut next = Buffer::with_lines([text]);
        previous[(1, 0)].set_diff_option(CellDiffOption::AlwaysUpdate);
        next[(1, 0)].set_diff_option(CellDiffOption::AlwaysUpdate);

        let commands = diff_buffers(&previous, &next);
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, DrawCommand::Put { x: 1, y: 0, .. })),
            "expected the always-update cell in {text:?} to be emitted; commands: {commands:?}"
        );
    }
}

#[test]
fn diff_buffers_clears_styled_trailing_cell_replaced_by_forced_width_cell() {
    use ratatui::buffer::CellDiffOption;

    let area = Rect::new(0, 0, 7, 1);
    let mut previous = Buffer::empty(area);
    let mut next = Buffer::empty(area);
    previous.set_string(
        0,
        0,
        "漢 tail",
        Style::default().bg(Color::Blue).add_modifier(Modifier::UNDERLINED),
    );
    next.set_string(0, 0, "a tail", Style::default());
    next[(0, 0)]
        .set_symbol("\x1b]8;;https://example.com\x07a\x1b]8;;\x07")
        .set_diff_option(CellDiffOption::ForcedWidth(NonZeroU16::MIN));

    let commands = diff_buffers(&previous, &next);

    assert!(
        commands.iter().any(|command| matches!(command, DrawCommand::Put { x: 1, y: 0, .. })),
        "expected the styled trailing cell to be cleared; commands: {commands:?}"
    );
}

#[test]
fn terminal_draw_applies_requested_cursor_style() {
    let mut output = Vec::new();
    let mut terminal =
        Terminal::with_options(CaptureBackend::new(/*width*/ 2, /*height*/ 1))
            .expect("terminal");
    terminal.set_viewport_area(Rect::new(0, 0, 2, 1));

    terminal
        .try_draw(|frame| {
            frame.set_cursor_style(SetCursorStyle::SteadyBar);
            frame.set_cursor_position((0, 0));
            io::Result::Ok(())
        })
        .expect("draw");

    queue!(output, SetCursorStyle::SteadyBar).expect("queue style");
    let expected = String::from_utf8(output).expect("utf8");
    let actual = terminal.backend().output();
    assert!(
        actual.contains(&expected),
        "expected terminal output to contain cursor style {expected:?}, got {actual:?}"
    );
}

#[test]
fn reset_cursor_style_emits_default_user_shape() {
    let mut output = Vec::new();
    let mut terminal =
        Terminal::with_options(CaptureBackend::new(/*width*/ 2, /*height*/ 1))
            .expect("terminal");

    terminal.reset_cursor_style().expect("reset cursor style");
    ratatui::backend::Backend::flush(terminal.backend_mut()).expect("flush backend");

    queue!(output, SetCursorStyle::DefaultUserShape).expect("queue style");
    let expected = String::from_utf8(output).expect("utf8");
    let actual = terminal.backend().output();
    assert!(
        actual.contains(&expected),
        "expected terminal output to contain cursor style reset {expected:?}, got {actual:?}"
    );
}
