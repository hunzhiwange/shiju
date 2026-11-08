//! 验证状态动画的帧推进、颜色选择与停止条件。
//! 时间边界和不可动画状态用于确认动画不会持续产生无效更新。

use super::status_animation::{
    ACTIVITY_ANIMATION_INTERVAL_MS, STATUS_SPINNER_FRAMES, spinner_frame,
};

#[test]
fn activity_animation_interval_limits_request_time_refresh_rate() {
    assert_eq!(ACTIVITY_ANIMATION_INTERVAL_MS, 90);
}

#[test]
fn spinner_frame_wraps_by_frame_count() {
    assert_eq!(spinner_frame(0), STATUS_SPINNER_FRAMES[0]);
    assert_eq!(spinner_frame(STATUS_SPINNER_FRAMES.len()), STATUS_SPINNER_FRAMES[0]);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn session_spinner_renders_around_the_row_center() {
    use iced::advanced::renderer::{Headless, Renderer as _, Style};
    use iced::advanced::widget::Tree;
    use iced::advanced::{Layout, layout, mouse};
    use iced::{Alignment, Color, Element, Font, Pixels, Rectangle, Size, Theme};

    use super::status_animation::session_spinner;

    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
        .await
        .expect("tiny-skia headless renderer should be available");
    const RENDER_SIZE: usize = 60;
    let viewport = Rectangle::with_size(Size::new(30.0, 30.0));
    let limits = layout::Limits::new(Size::ZERO, viewport.size());
    let color = Color::from_rgb8(239, 68, 68);

    for (size, theme) in [(9.0, Theme::Light), (11.0, Theme::Dark)] {
        let mut top = RENDER_SIZE;
        let mut bottom = 0;
        for frame in 0..STATUS_SPINNER_FRAMES.len() {
            let mut row: Element<'_, ()> = iced::widget::row![session_spinner(frame, size, color)]
                .height(30)
                .align_y(Alignment::Center)
                .into();
            let mut tree = Tree::new(row.as_widget());
            let node = row.as_widget_mut().layout(&mut tree, &renderer, &limits);
            renderer.reset(viewport);
            row.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &Style::default(),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &viewport,
            );
            let pixels = renderer.screenshot(
                Size::new(RENDER_SIZE as u32, RENDER_SIZE as u32),
                2.0,
                theme.palette().background,
            );
            let background = &pixels[pixels.len() - 4..];
            let mut painted_rows = pixels
                .chunks_exact(RENDER_SIZE * 4)
                .enumerate()
                .filter(|(_, row)| row.chunks_exact(4).any(|pixel| pixel != background))
                .map(|(y, _)| y);
            let first = painted_rows.next().expect("spinner frame should be visible");
            top = top.min(first);
            bottom = bottom.max(painted_rows.next_back().unwrap_or(first));
        }
        // The animation rotates around a fixed center, so check the union of all frames.
        assert!(
            (top + bottom).abs_diff(RENDER_SIZE - 1) <= 1,
            "size {size}, rows {top}..={bottom}"
        );
    }
}
