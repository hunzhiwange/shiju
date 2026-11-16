//! Static transcript snapshot. Native selection belongs to the terminal emulator;
//! page navigation intentionally replaces the selection. No dynamic redraws.

use std::io::{self, Write};
use std::time::Duration;

use crossterm::{cursor::MoveTo, event::{self, Event, KeyCode, KeyEventKind, KeyModifiers}, execute, terminal::{self, Clear, ClearType}};
use ratatui::text::Line;

use super::{ActiveRunner, V3GatewayApp};

pub(super) fn run(app: &mut V3GatewayApp, runner: &mut ActiveRunner) -> io::Result<()> {
    // Freeze source lines once; background stream drains can mutate the live view
    // without changing either this snapshot or its reading position.
    let snapshot = app.transcript_lines(u16::MAX, true);
    vw_tui::with_native_selection(app.terminal_mode(), || {
        let mut size = terminal::size()?;
        let mut rows = wrap(&snapshot, size.0);
        let mut offset = 0;
        paint(&rows, offset, size)?;
        loop {
            let busy = runner.drain(app);
            if !event::poll(if busy { Duration::ZERO } else { Duration::from_millis(50) })? {
                continue;
            }
            // Consume every event here, including paste and mouse reports that
            // were already queued before mouse capture was disabled.
            let mut repaint = false;
            match event::read()? {
                Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                    if key.code == KeyCode::Esc
                        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
                    {
                        return Ok(());
                    }
                    let page = usize::from(size.1.saturating_sub(1).max(1));
                    let last = rows.len().saturating_sub(page);
                    let next = match key.code {
                        KeyCode::PageUp => offset.saturating_sub(page),
                        KeyCode::PageDown => offset.saturating_add(page).min(last),
                        KeyCode::Home => 0,
                        KeyCode::End => last,
                        _ => offset,
                    };
                    repaint = next != offset;
                    offset = next;
                }
                Event::Resize(width, height) => {
                    size = (width, height);
                    rows = wrap(&snapshot, width);
                    offset = offset.min(rows.len().saturating_sub(usize::from(height.saturating_sub(1).max(1))));
                    repaint = true;
                }
                _ => {}
            }
            if repaint {
                paint(&rows, offset, size)?;
            }
        }
    })
}

pub(crate) fn wrap(lines: &[Line<'_>], width: u16) -> Vec<String> {
    let width = usize::from(width.max(1));
    let mut rows = Vec::new();
    for line in lines {
        let mut row = String::new();
        let mut used = 0;
        for grapheme in line.styled_graphemes(ratatui::style::Style::default()) {
            if grapheme.symbol == "\n" {
                rows.push(std::mem::take(&mut row));
                used = 0;
                continue;
            }
            // Preserve grapheme clusters, including emoji and combining marks;
            // never execute transcript control sequences in the terminal.
            let symbol: String = grapheme.symbol.chars()
                .map(|ch| if ch.is_control() { ' ' } else { ch }).collect();
            let cells = ratatui::text::Span::raw(symbol.as_str()).width();
            if used > 0 && used + cells > width {
                rows.push(std::mem::take(&mut row));
                used = 0;
            }
            row.push_str(&symbol);
            used += cells;
        }
        rows.push(row);
    }
    rows
}

fn paint(rows: &[String], offset: usize, (width, height): (u16, u16)) -> io::Result<()> {
    if width == 0 || height == 0 {
        return Ok(());
    }
    let mut stdout = io::stdout().lock();
    execute!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;
    for (y, row) in rows.iter().skip(offset).take(usize::from(height.saturating_sub(1))).enumerate() {
        execute!(stdout, MoveTo(0, y as u16))?;
        write!(stdout, "{row}")?;
    }
    execute!(stdout, MoveTo(0, height - 1))?;
    let help = format!("只读快照 {}/{} | 原生选择复制 | PgUp/PgDn Home/End | Esc 返回", offset + 1, rows.len());
    let help = wrap(&[Line::from(help)], width);
    if let Some(line) = help.first() {
        write!(stdout, "{line}")?;
    }
    stdout.flush()
}
