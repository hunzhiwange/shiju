//! 保留依赖上层历史插入与超链接标记的既有断言。

use super::implementation::tests::CaptureBackend;
use super::*;
use pretty_assertions::assert_eq;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget, Wrap};

#[test]
fn ordinary_redraws_with_known_size_do_not_query_backend_size() {
    let mut terminal =
        Terminal::with_options(CaptureBackend::new(/*width*/ 80, /*height*/ 24))
            .expect("terminal");
    let screen_size = terminal.last_known_screen_size;

    for _ in 0..3 {
        terminal.draw_with_size(screen_size, |_| {}).expect("draw");
    }

    terminal.set_viewport_area(Rect::new(
        /*x*/ 0, /*y*/ 23, /*width*/ 80, /*height*/ 1,
    ));
    crate::insert_history::insert_history_lines(&mut terminal, vec![Line::from("history")])
        .expect("insert history");

    assert_eq!(terminal.backend().size_call_count.get(), 1);
}

#[test]
fn terminal_draw_coalesces_wrapped_hyperlink_output() {
    let auth_url = format!(
        "https://auth.openai.com/oauth/authorize?response_type=code&state={}",
        "x".repeat(/*n*/ 400)
    );
    let width = 44;
    let height = 20;
    let area = Rect::new(0, 0, width, height);
    let mut terminal =
        Terminal::with_options(CaptureBackend::new(width, height)).expect("terminal");
    terminal.set_viewport_area(area);

    terminal
        .draw(|frame| {
            Paragraph::new(vec![
                Line::from(vec!["  ".into(), auth_url.as_str().cyan().underlined()]),
                "".into(),
                "  Press Esc to cancel".into(),
            ])
            .wrap(Wrap { trim: false })
            .render(area, frame.buffer_mut());
            crate::terminal_hyperlinks::mark_url_hyperlink(frame.buffer_mut(), area, &auth_url);
        })
        .expect("draw");

    let output = terminal.backend().output();
    let open = format!("\x1b]8;;{auth_url}\x07");
    let close = "\x1b]8;;\x07";
    assert_eq!(output.matches(&open).count(), 1);
    assert_eq!(output.matches(close).count(), 1);
    let footer = output.find("Press").expect("footer");
    assert!(output.find(close).expect("hyperlink close") < footer);
}
