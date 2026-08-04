use std::convert::Infallible;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use ratatui::backend::{Backend, TestBackend};
use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::{Frame, Terminal};

use super::{TuiRenderer, TuiThemeName};
use crate::cli::tui_v2::state::TuiState;
use crate::cli::tui_v2::terminal_capabilities::TerminalCapabilities;

const UPDATE_FIXTURES_ENV: &str = "VW_UPDATE_TUI_FIXTURES";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrameCellSnapshot {
    pub(crate) symbol: String,
    pub(crate) foreground: String,
    pub(crate) background: String,
    pub(crate) underline: String,
    pub(crate) modifiers: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrameSnapshot {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) cursor: Option<Position>,
    pub(crate) cells: Vec<FrameCellSnapshot>,
}

impl FrameSnapshot {
    fn serialize(&self) -> String {
        let mut output = format!("frame width={} height={}\n", self.width, self.height);
        match self.cursor {
            Some(cursor) => {
                writeln!(output, "cursor x={} y={}", cursor.x, cursor.y)
                    .expect("writing to String is infallible");
            }
            None => output.push_str("cursor hidden\n"),
        }

        for (index, cell) in self.cells.iter().enumerate() {
            let x = u16::try_from(index % usize::from(self.width)).unwrap_or(u16::MAX);
            let y = u16::try_from(index / usize::from(self.width)).unwrap_or(u16::MAX);
            writeln!(
                output,
                "cell x={x} y={y} symbol={:?} style fg={} bg={} underline={} modifiers={}",
                cell.symbol, cell.foreground, cell.background, cell.underline, cell.modifiers
            )
            .expect("writing to String is infallible");
        }

        output
    }
}

pub(crate) fn snapshot_buffer(buffer: &Buffer, cursor: Option<Position>) -> FrameSnapshot {
    let area = buffer.area;
    let mut cells = Vec::with_capacity(usize::from(area.width) * usize::from(area.height));
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = buffer.cell(Position::new(x, y)).expect("position is inside buffer area");
            cells.push(FrameCellSnapshot {
                symbol: cell.symbol().to_string(),
                foreground: format!("{:?}", cell.fg),
                background: format!("{:?}", cell.bg),
                underline: format!("{:?}", cell.underline_color),
                modifiers: format!("{:?}", cell.modifier),
            });
        }
    }

    FrameSnapshot { width: area.width, height: area.height, cursor, cells }
}

pub(crate) fn render_snapshot(
    width: u16,
    height: u16,
    render: impl FnOnce(&mut Frame<'_>),
) -> Result<FrameSnapshot, Infallible> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(render)?;
    let cursor = terminal.backend_mut().get_cursor_position().ok();
    Ok(snapshot_buffer(terminal.backend().buffer(), cursor))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_tui_snapshot_with_theme(
    width: u16,
    height: u16,
    state: &TuiState,
    badge_label: &str,
    endpoint_label: &str,
    spinner_frame: usize,
    theme_name: TuiThemeName,
    capabilities: TerminalCapabilities,
) -> Result<FrameSnapshot, Infallible> {
    let renderer = TuiRenderer::for_theme(theme_name, capabilities);
    render_snapshot(width, height, |frame| {
        renderer.render_frame(frame, state, badge_label, endpoint_label, spinner_frame);
    })
}

pub(crate) fn first_frame_difference(
    expected: &FrameSnapshot,
    actual: &FrameSnapshot,
) -> Option<String> {
    if (expected.width, expected.height) != (actual.width, actual.height) {
        return Some(format!(
            "frame size differs: expected={}x{} actual={}x{}",
            expected.width, expected.height, actual.width, actual.height
        ));
    }
    if expected.cursor != actual.cursor {
        return Some(format!(
            "cursor differs: expected={:?} actual={:?}",
            expected.cursor, actual.cursor
        ));
    }

    for (index, (expected_cell, actual_cell)) in
        expected.cells.iter().zip(actual.cells.iter()).enumerate()
    {
        if expected_cell != actual_cell {
            let x = index % usize::from(expected.width);
            let y = index / usize::from(expected.width);
            return Some(format!(
                "frame differs at x={x} y={y}: symbol/style expected={expected_cell:?} actual={actual_cell:?}"
            ));
        }
    }

    if expected.cells.len() != actual.cells.len() {
        return Some(format!(
            "cell count differs: expected={} actual={}",
            expected.cells.len(),
            actual.cells.len()
        ));
    }
    None
}

pub(crate) fn assert_frame_fixture(
    fixture_path: &Path,
    actual: &FrameSnapshot,
) -> Result<(), String> {
    let actual_text = actual.serialize();
    if std::env::var_os(UPDATE_FIXTURES_ENV).is_some() {
        let parent = fixture_path
            .parent()
            .ok_or_else(|| format!("fixture has no parent: {}", fixture_path.display()))?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("create fixture directory failed: {error}"))?;
        fs::write(fixture_path, &actual_text)
            .map_err(|error| format!("write fixture failed: {error}"))?;
        return Ok(());
    }

    let expected_text = fs::read_to_string(fixture_path)
        .map_err(|error| format!("read fixture {} failed: {error}", fixture_path.display()))?;
    first_text_difference(&expected_text, &actual_text)
}

fn first_text_difference(expected: &str, actual: &str) -> Result<(), String> {
    let mut expected_lines = expected.lines();
    let mut actual_lines = actual.lines();
    let mut line_number = 1usize;
    loop {
        match (expected_lines.next(), actual_lines.next()) {
            (None, None) => return Ok(()),
            (expected_line, actual_line) if expected_line == actual_line => {
                line_number = line_number.saturating_add(1);
            }
            (expected_line, actual_line) => {
                let expected_line = expected_line.unwrap_or("<missing>");
                let actual_line = actual_line.unwrap_or("<missing>");
                if let Some((x, y)) =
                    cell_coordinates(expected_line).or_else(|| cell_coordinates(actual_line))
                {
                    return Err(format!(
                        "fixture differs at line {line_number}, x={x} y={y}: expected `{expected_line}`, actual `{actual_line}`"
                    ));
                }
                return Err(format!(
                    "fixture differs at line {line_number}: expected `{expected_line}`, actual `{actual_line}`"
                ));
            }
        }
    }
}

fn cell_coordinates(line: &str) -> Option<(&str, &str)> {
    let line = line.strip_prefix("cell x=")?;
    let (x, line) = line.split_once(" y=")?;
    let (y, _) = line.split_once(' ')?;
    Some((x, y))
}
