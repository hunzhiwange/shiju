#![cfg(unix)]
use crate::{Shell, TerminalState, TerminalTheme};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, widget::Tree};
use iced::{Font, Pixels, Size, Theme};
fn blank_terminal(is_visible: bool) -> TerminalState {
    TerminalState::blank_with_settings(
        is_visible,
        Shell::Bash,
        TerminalTheme::System,
        "JetBrains Mono".into(),
        13.0,
        200.0,
    )
}
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn terminal_typography_draws_mixed_text_and_cursor_on_the_same_grid() {
    use iced::{Color, Rectangle};
    use std::time::{Duration, Instant};

    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia renderer");
    for font in desktop_assets::fonts::load_all() {
        iced::advanced::graphics::text::font_system().write().unwrap().load_font(font);
    }
    let tmp = tempfile::tempdir().unwrap();
    let mut state = blank_terminal(true);
    let id = state.add_terminal_with_command(
        "Typography".into(),
        Some(tmp.path().into()),
        "read -r start; printf '\\033[2J\\033[H\\033[41m  \\033[0m中文 A\r\n\\033[41m  \\033[0mREADY English\\033[3;1H'; read -r finish".into(),
    ).expect("PTY terminal");
    let bounds = Size::new(480.0, 180.0);
    let resize = || {
        iced_term::Command::ProxyToBackend(iced_term::BackendCommand::Resize(Some(bounds), None))
    };
    state.tabs[0].term.handle(resize());
    state.tabs[0].term.handle(iced_term::Command::ProxyToBackend(
        iced_term::BackendCommand::Write(b"start\n".to_vec()),
    ));
    let started = Instant::now();
    loop {
        state.tabs[0].term.handle(resize());
        if state.tabs[0].term.text().iter().any(|line| line.contains("READY")) {
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(5), "PTY output was not received");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    for size in [12.0_f32, 15.0, 16.0, 20.0] {
        state.set_font_size(size);
        state.tabs[0].term.handle(resize());
        for theme in [Theme::Dark, Theme::Light] {
            let mut palette = crate::build_palette(&theme);
            palette.red = "#ff0000".into();
            state.apply_palette(palette);
            let mut element = iced_term::TerminalView::show(&state.tabs[0].term);
            let viewport = Rectangle::with_size(bounds);
            iced::advanced::Renderer::reset(&mut renderer, viewport);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &iced::advanced::renderer::Style::default(),
                Layout::new(&node),
                iced::mouse::Cursor::Unavailable,
                &viewport,
            );
            let pixels = renderer.screenshot(Size::new(480, 180), 1.0, Color::TRANSPARENT);
            let pixel = |x: usize, y: usize| &pixels[(y * 480 + x) * 4..(y * 480 + x) * 4 + 4];
            // Two ANSI background rows and the following block cursor must
            // share the measured (integer PTY) cell height.
            let row_height = (size * 1.4) as usize;
            // Growing the grid can bring the echoed handshake back from
            // scrollback, so locate the ANSI rows instead of assuming y=0.
            let first_row = (0..180)
                .find(|&y| pixel(1, y) == [255, 0, 0, 255])
                .expect("ANSI background rows must be visible");
            assert_eq!(first_row % row_height, 0);
            let red_rows =
                (first_row..180).take_while(|&y| pixel(1, y) == [255, 0, 0, 255]).count();
            assert_eq!(red_rows, row_height * 2, "size={size}, theme={theme:?}");
            let cursor_row = first_row + row_height * 2;
            let cursor = pixel(1, cursor_row).to_vec();
            assert_ne!(cursor, pixel(30, cursor_row));
            assert!((cursor_row..cursor_row + row_height).all(|y| pixel(1, y) == cursor));
            assert_ne!(pixel(1, cursor_row + row_height), cursor);
            // Mixed Chinese/ASCII glyphs must actually paint beyond the bars.
            let background = pixel(479, 179).to_vec();
            assert!((first_row..cursor_row).any(|y| (24..150).any(|x| pixel(x, y) != background)));
        }
    }
    state.close_terminal(id);
    state.reap_retired();
}
