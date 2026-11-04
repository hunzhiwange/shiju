use super::*;

#[test]
fn snap_eases_to_nearest_level_without_jumping_on_release() {
    let now = Instant::now();
    for (position, target) in [(0.2, 0), (1.49, 1), (1.51, 2), (3.8, 4)] {
        let mut state = State::new(0, "model".into());
        state.position = position;
        state.dragging = true;
        assert_eq!(state.release(now), target);
        assert_eq!(state.position, position);
        state.animate(now + SNAP_DURATION / 2);
        let halfway = state.position;
        assert!((halfway - target as f32).abs() < (position - target as f32).abs());
        assert_ne!(halfway, target as f32);
        state.animate(now + SNAP_DURATION);
        assert_eq!(state.position, target as f32);
        assert!(state.snap.is_none());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn drag_is_continuous_commits_once_and_survives_view_rebuild() {
    use iced::advanced::renderer::Headless;
    use iced::{Font, Pixels};
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let options = vec![None, Some(ReasoningEffortDto::Low), Some(ReasoningEffortDto::High)];
    let mut slider = ReasoningSlider { selected: 0, options, identity: "model".into() };
    let mut tree = tree::Tree::new(&slider as &dyn Widget<Message, Theme, iced::Renderer>);
    let node = slider.layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(320.0, 40.0)),
    );
    let bounds = node.bounds();
    let point = |value: f32| {
        Point::new(
            HANDLE_RADIUS + value / 2.0 * (bounds.width - 2.0 * HANDLE_RADIUS),
            bounds.center_y(),
        )
    };
    let mut messages = Vec::new();
    for (event, position) in [
        (Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)), point(0.0)),
        (Event::Mouse(mouse::Event::CursorMoved { position: point(0.37) }), point(0.37)),
        (Event::Mouse(mouse::Event::CursorMoved { position: point(1.63) }), point(1.63)),
    ] {
        slider.update(
            &mut tree,
            &event,
            Layout::new(&node),
            mouse::Cursor::Available(position),
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut Shell::new(&mut messages),
            &bounds,
        );
    }
    assert!(messages.is_empty(), "dragging must not persist intermediate levels");
    assert!((tree.state.downcast_ref::<State>().position - 1.63).abs() < 0.001);
    slider.update(
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        Layout::new(&node),
        mouse::Cursor::Available(point(1.63)),
        &renderer,
        &mut iced::advanced::clipboard::Null,
        &mut Shell::new(&mut messages),
        &bounds,
    );
    assert_eq!(messages.len(), 1);
    assert!(matches!(
        messages[0],
        Message::Chat(crate::app::message::ChatMessage::ReasoningEffortSelected(Some(
            ReasoningEffortDto::High
        )))
    ));
    slider.selected = 2;
    slider.diff(&mut tree);
    assert!(tree.state.downcast_ref::<State>().snap.is_some());
    let position = tree.state.downcast_ref::<State>().position;
    assert!((position - 1.63).abs() < 0.001);
    // 在吸附过程中重新抓取滑块，位置保持连续。
    slider.update(
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&node),
        mouse::Cursor::Available(point(position)),
        &renderer,
        &mut iced::advanced::clipboard::Null,
        &mut Shell::new(&mut messages),
        &bounds,
    );
    assert!(tree.state.downcast_ref::<State>().snap.is_none());
    assert!(tree.state.downcast_ref::<State>().dragging);
    slider.selected = 0;
    slider.diff(&mut tree);
    assert_eq!(tree.state.downcast_ref::<State>().position, 0.0);
    assert!(!tree.state.downcast_ref::<State>().dragging);
}

#[tokio::test(flavor = "multi_thread")]
async fn animated_thumb_renders_distinct_frames_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::{Font, Pixels};
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut slider = ReasoningSlider {
        selected: 0,
        options: vec![None, Some(ReasoningEffortDto::Low), Some(ReasoningEffortDto::High)],
        identity: "model".into(),
    };
    for theme in [Theme::Light, Theme::Dark] {
        let mut tree = tree::Tree::new(&slider as &dyn Widget<Message, Theme, iced::Renderer>);
        let node = slider.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(320.0, 40.0)),
        );
        let now = Instant::now();
        let state = tree.state.downcast_mut::<State>();
        state.position = 1.55;
        state.release(now);
        let mut frames = Vec::new();
        for elapsed in [Duration::ZERO, SNAP_DURATION / 2, SNAP_DURATION] {
            slider.update(
                &mut tree,
                &Event::Window(iced::window::Event::RedrawRequested(now + elapsed)),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &renderer,
                &mut iced::advanced::clipboard::Null,
                &mut Shell::new(&mut Vec::new()),
                &node.bounds(),
            );
            slider.draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &node.bounds(),
            );
            frames.push(renderer.screenshot(Size::new(288, 36), 1.0, Color::TRANSPARENT));
        }
        assert_ne!(frames[0], frames[1]);
        assert_ne!(frames[1], frames[2]);
        assert_eq!(tree.state.downcast_ref::<State>().position, 2.0);
        assert!(tree.state.downcast_ref::<State>().snap.is_none());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn left_endpoint_hides_primary_fill_under_thumb_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::{Font, Pixels};
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut slider = ReasoningSlider {
        selected: 0,
        options: vec![None, Some(ReasoningEffortDto::Low), Some(ReasoningEffortDto::High)],
        identity: "model".into(),
    };
    for theme in [Theme::Light, Theme::Dark] {
        let mut tree = tree::Tree::new(&slider as &dyn Widget<Message, Theme, iced::Renderer>);
        let node = slider.layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(320.0, 40.0)),
        );
        let mut frames = Vec::new();
        for position in [0.0, 2.0] {
            tree.state.downcast_mut::<State>().position = position;
            slider.draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &node.bounds(),
            );
            frames.push(renderer.screenshot(Size::new(288, 36), 1.0, Color::TRANSPARENT));
        }
        let offset = (18 * 288 + 80) * 4;
        let primary = &frames[1][offset..offset + 4];
        assert_ne!(&frames[0][offset..offset + 4], primary);
        assert!(
            !frames[0].chunks_exact(4).any(|pixel| pixel == primary),
            "minimum level must not expose blue fill around the thumb"
        );
    }
}
