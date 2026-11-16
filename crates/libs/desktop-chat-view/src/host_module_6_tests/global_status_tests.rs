use super::status_fixture::{StatusFixture as App, chat_global_status_view};
#[cfg(not(target_arch = "wasm32"))]
use iced::advanced::{Layout, layout, renderer::Headless, widget::Tree};
#[cfg(not(target_arch = "wasm32"))]
use iced::{Font, Pixels, Size};
use session_ui_types::{ChatMessage, ChatRole, TokenUsage};
fn assistant(content: &str) -> ChatMessage {
    ChatMessage {
        role: ChatRole::Assistant,
        content: content.to_string(),
        model: None,
        think_timing: vec![],
        processing_timing: None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn thinking_pulse_shapes_all_animation_frames_with_fixed_layout() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.requesting = true;
    app.chat.push(assistant("<think>正在分析界面渲染</think>"));
    let limits = layout::Limits::new(Size::ZERO, Size::new(800.0, 100.0));
    let mut expected = None;

    for frame in 0..48 {
        app.status_animation_frame = frame;
        let mut element = chat_global_status_view(&app);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
        let row = Layout::new(&node).children().next().expect("status row");
        let indicator = row.children().next().expect("thinking indicator");
        let geometry = (indicator.bounds().size(), row.bounds().height);

        if let Some(expected) = expected {
            assert_eq!(geometry, expected, "frame {frame}");
        } else {
            expected = Some(geometry);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn global_status_spinner_rotates_at_text_height() {
    use crate::global_status::COMPACT_ACTIVITY_SLOT_SIZE;
    use desktop_ui_controls::status_animation::STATUS_SPINNER_FRAMES;
    use iced::advanced::{mouse, renderer::Renderer as _, renderer::Style};
    use iced::{Rectangle, Theme};

    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
        .await
        .expect("tiny-skia headless renderer should be available");
    let mut app = App::new().0;
    app.requesting = true;
    app.chat.push(assistant(
        "tool read\n{\"status\":\"running\",\"input\":\"{\\\"filePath\\\":\\\"含中文/很长的工具摘要.rs\\\"}\"}",
    ));
    app.usage = TokenUsage { input_tokens: 1200, output_tokens: 34, ..Default::default() };

    let viewport = Rectangle::with_size(Size::new(400.0, 60.0));
    let limits = layout::Limits::new(Size::ZERO, viewport.size());
    let image_size = Size::new(800, 120);
    for theme in [Theme::Light, Theme::Dark] {
        let mut spinner_top = f32::INFINITY;
        let mut spinner_bottom = f32::NEG_INFINITY;
        for frame in 0..STATUS_SPINNER_FRAMES.len() {
            app.status_animation_frame = frame;
            let mut element = chat_global_status_view(&app);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
            let layout = Layout::new(&node);
            let row = layout.children().next().expect("status row");
            let mut children = row.children();
            let indicator = children.next().expect("spinner slot");
            let detail = children.next().expect("status text").bounds();
            let slot_size = COMPACT_ACTIVITY_SLOT_SIZE;
            assert_eq!(indicator.bounds().size(), Size::new(slot_size, slot_size));

            renderer.reset(viewport);
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &Style::default(),
                layout,
                mouse::Cursor::Unavailable,
                &viewport,
            );
            let pixels = renderer.screenshot(image_size, 2.0, theme.palette().background);
            let background = &pixels[pixels.len() - 4..];
            let slot_width = slot_size as usize * 2 * 4;
            let mut spinner_rows = pixels
                .chunks_exact(image_size.width as usize * 4)
                .enumerate()
                .filter(|(_, row)| row[..slot_width].chunks_exact(4).any(|px| px != background))
                .map(|(y, _)| y);
            let first = spinner_rows.next().expect("spinner frame should be visible");
            let last = spinner_rows.next_back().unwrap_or(first);
            let text_center = detail.y + detail.height / 2.0;
            spinner_top = spinner_top.min((first as f32 + 0.5) / 2.0 - text_center);
            spinner_bottom = spinner_bottom.max((last as f32 + 0.5) / 2.0 - text_center);
        }
        // The full animation rotates around the text center.
        let offset = (spinner_top + spinner_bottom) / 2.0;
        assert!(offset.abs() <= 0.5, "theme {theme:?}, spinner offset {offset}");
    }
}

#[test]
fn global_status_view_builds_for_hidden_and_visible_states() {
    let mut app = App::new().0;

    let _hidden = chat_global_status_view(&app);
    drop(_hidden);

    app.requesting = true;
    app.started = 1;
    app.chat.push(assistant(
        "tool read\n{\"status\":\"running\",\"input\":\"{\\\"filePath\\\":\\\"src/app.rs\\\"}\"}",
    ));
    app.usage = TokenUsage { input_tokens: 1200, output_tokens: 34, ..Default::default() };
    app.status_animation_frame = 7;

    let _visible = chat_global_status_view(&app);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn global_status_detail_fills_remaining_width() {
    async fn detail_width(viewport_width: f32) -> f32 {
        let renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("tiny-skia headless renderer should be available");
        let mut app = App::new().0;
        app.requesting = true;
        app.chat.push(assistant(&format!("<think>{}</think>", "很长的思考内容".repeat(40))));

        let mut element = chat_global_status_view(&app);
        let limits = layout::Limits::new(Size::ZERO, Size::new(viewport_width, 100.0));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
        let row = Layout::new(&node).children().next().expect("status row");

        row.children().nth(1).expect("status detail").bounds().width
    }

    let narrow = detail_width(360.0).await;
    let wide = detail_width(800.0).await;

    assert!(wide > narrow + 400.0, "narrow={narrow}, wide={wide}");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn global_status_keeps_height_during_request_state_transitions() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut app = App::new().0;
    app.requesting = true;
    let limits = layout::Limits::new(Size::ZERO, Size::new(480.0, 100.0));
    let mut expected_height = None;
    for content in [
        "",
        "<think>正在分析",
        "tool read\n{\"status\":\"running\",\"input\":\"src/main.rs\"}",
        "正常回复",
        "<think>继续分析",
    ] {
        app.chat = if content.is_empty() { Vec::new() } else { vec![assistant(content)] };
        for frame in [0, 12, 24, 36] {
            app.status_animation_frame = frame;
            let mut element = chat_global_status_view(&app);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
            let height = node.size().height;
            assert!(height > 0.0);
            if let Some(expected) = expected_height {
                assert_eq!(height, expected, "content={content}, frame={frame}");
            } else {
                expected_height = Some(height);
            }
        }
    }
    app.requesting = false;
    let mut element = chat_global_status_view(&app);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    assert_eq!(node.size().height, 0.0);
}
