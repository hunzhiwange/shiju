//! 滑块触摸与滚轮预览、提交行为。
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn metrics_slider_previews_touch_and_wheel_and_commits_on_touch_release() {
    use super::{Message as Action, metrics_slider};
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, Shell, Widget, layout, widget::Tree};
    use iced::{Event, Font, Pixels, Point, Size, keyboard, mouse, touch};

    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut slider = metrics_slider::<()>(10.0..=30.0, 14.0, Action::FontSizePreview);
    let mut tree = Tree::new(&slider as &dyn Widget<Action, iced::Theme, iced::Renderer>);
    let node = slider.layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(100.0, 40.0)),
    );
    let bounds = node.bounds();
    let start = Point::new(40.0, bounds.center_y());
    let end = Point::new(65.0, bounds.center_y());
    let id = touch::Finger(0);
    let mut dispatch = |event, position| {
        let mut messages = Vec::new();
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
        messages
            .into_iter()
            .map(|message| match message {
                Action::FontSizePreview(size) => Action::FontSizePreview(size),
                Action::CommitMetrics => Action::CommitMetrics,
                other => panic!("unexpected slider message: {other:?}"),
            })
            .collect::<Vec<Action<()>>>()
    };

    let pressed =
        dispatch(Event::Touch(touch::Event::FingerPressed { id, position: start }), start);
    let moved = dispatch(Event::Touch(touch::Event::FingerMoved { id, position: end }), end);
    let released = dispatch(Event::Touch(touch::Event::FingerLifted { id, position: end }), end);
    assert!(matches!(pressed.as_slice(), [Action::FontSizePreview(_)]));
    assert!(matches!(moved.as_slice(), [Action::FontSizePreview(_)]));
    assert!(matches!(released.as_slice(), [Action::CommitMetrics]));

    assert!(
        dispatch(
            Event::Keyboard(keyboard::Event::ModifiersChanged(keyboard::Modifiers::CTRL)),
            end
        )
        .is_empty()
    );
    let wheel = dispatch(
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
        }),
        end,
    );
    assert!(matches!(wheel.as_slice(), [Action::FontSizePreview(_)]));
}
