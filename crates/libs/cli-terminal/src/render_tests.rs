//! CLI 基础绘制原语的缓冲区测试。
//! 通过 `TestBackend` 校验扫描线背景和执行指示器的字符、颜色、修饰与居中位置。
//! 还覆盖空区域、单单元格和动画偏移重合等极端布局。

use super::render::{render_execution_indicator, render_scanline_background};
use super::theme::{ACCENT_CYAN, EXECUTION_DOT, SCANLINE_DARK, SCANLINE_LIGHT, SURFACE_BASE};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

fn render_buffer(width: u16, height: u16, render: impl FnOnce(&mut ratatui::Frame<'_>)) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("create test terminal");

    terminal.draw(render).expect("draw test frame");

    terminal.backend().buffer().clone()
}

fn cell(buffer: &Buffer, x: u16, y: u16) -> &Cell {
    buffer.cell((x, y)).expect("cell inside test buffer")
}

fn row_symbols(buffer: &Buffer, x: u16, y: u16, width: u16) -> String {
    let mut symbols = String::new();
    for column in x..x + width {
        symbols.push_str(cell(buffer, column, y).symbol());
    }
    symbols
}

#[test]
fn scanline_background_alternates_dark_and_light_rows_inside_area() {
    let area = Rect::new(1, 1, 4, 3);
    let buffer = render_buffer(6, 5, |frame| render_scanline_background(frame, area));

    for x in area.x..area.x + area.width {
        assert_eq!(cell(&buffer, x, 1).symbol(), " ");
        assert_eq!(cell(&buffer, x, 1).bg, SCANLINE_DARK);
        assert_eq!(cell(&buffer, x, 2).bg, SCANLINE_LIGHT);
        assert_eq!(cell(&buffer, x, 3).bg, SCANLINE_DARK);
    }
    assert_eq!(cell(&buffer, 0, 0).bg, Color::Reset);
    assert_eq!(cell(&buffer, 5, 4).bg, Color::Reset);
}

#[test]
fn scanline_background_empty_areas_leave_buffer_unchanged() {
    let buffer = render_buffer(3, 2, |frame| {
        render_scanline_background(frame, Rect::new(0, 0, 0, 2));
        render_scanline_background(frame, Rect::new(0, 0, 3, 0));
    });

    for y in 0..2 {
        for x in 0..3 {
            assert_eq!(cell(&buffer, x, y).symbol(), " ");
            assert_eq!(cell(&buffer, x, y).bg, Color::Reset);
        }
    }
}

#[test]
fn execution_indicator_idle_centers_a_solid_surface_row() {
    let area = Rect::new(1, 1, 4, 4);
    let buffer = render_buffer(6, 6, |frame| {
        render_execution_indicator(frame, area, false, 99);
    });

    assert_eq!(row_symbols(&buffer, 1, 3, 4), "    ");
    for x in area.x..area.x + area.width {
        assert_eq!(cell(&buffer, x, 3).bg, SURFACE_BASE);
    }
    assert_eq!(cell(&buffer, 1, 2).bg, Color::Reset);
    assert_eq!(cell(&buffer, 1, 4).bg, Color::Reset);
}

#[test]
fn execution_indicator_busy_wraps_spinner_and_styles_blocks_and_dots() {
    let area = Rect::new(1, 1, 5, 3);
    let buffer = render_buffer(7, 5, |frame| {
        render_execution_indicator(frame, area, true, 5);
    });

    assert_eq!(row_symbols(&buffer, 1, 2, 5), ".■.■.");
    for x in [1, 3, 5] {
        let dot = cell(&buffer, x, 2);
        assert_eq!(dot.fg, EXECUTION_DOT);
        assert_eq!(dot.bg, SURFACE_BASE);
        assert_eq!(dot.modifier, Modifier::empty());
    }
    for x in [2, 4] {
        let block = cell(&buffer, x, 2);
        assert_eq!(block.fg, ACCENT_CYAN);
        assert_eq!(block.bg, SURFACE_BASE);
        assert!(block.modifier.contains(Modifier::BOLD));
    }
}

#[test]
fn execution_indicator_busy_separates_blocks_when_offsets_meet() {
    let area = Rect::new(0, 0, 5, 1);
    let buffer = render_buffer(5, 1, |frame| {
        render_execution_indicator(frame, area, true, 2);
    });

    assert_eq!(row_symbols(&buffer, 0, 0, 5), "..■■.");
    assert_eq!(cell(&buffer, 2, 0).fg, ACCENT_CYAN);
    assert_eq!(cell(&buffer, 3, 0).fg, ACCENT_CYAN);
}

#[test]
fn execution_indicator_busy_single_cell_renders_one_block() {
    let area = Rect::new(1, 0, 1, 3);
    let buffer = render_buffer(3, 3, |frame| {
        render_execution_indicator(frame, area, true, usize::MAX);
    });

    let block = cell(&buffer, 1, 1);
    assert_eq!(block.symbol(), "■");
    assert_eq!(block.fg, ACCENT_CYAN);
    assert_eq!(block.bg, SURFACE_BASE);
    assert!(block.modifier.contains(Modifier::BOLD));
}

#[test]
fn execution_indicator_empty_areas_leave_buffer_unchanged() {
    let buffer = render_buffer(3, 2, |frame| {
        render_execution_indicator(frame, Rect::new(0, 0, 0, 2), true, 0);
        render_execution_indicator(frame, Rect::new(0, 0, 3, 0), false, 0);
    });

    for y in 0..2 {
        for x in 0..3 {
            assert_eq!(cell(&buffer, x, y).symbol(), " ");
            assert_eq!(cell(&buffer, x, y).bg, Color::Reset);
        }
    }
}
