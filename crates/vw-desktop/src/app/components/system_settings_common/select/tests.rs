use super::*;
use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
use iced::advanced::{clipboard, renderer::Headless};
use iced::{Font, Pixels, Point};

fn selected(value: String) -> Message {
    Message::Settings(SettingsMessage::General(GeneralSettingsMessage::TimezoneSelected(value)))
}

fn key_press(named: iced::keyboard::key::Named) -> Event {
    use iced::keyboard::{self, Key};
    Event::Keyboard(keyboard::Event::KeyPressed {
        key: Key::Named(named),
        modified_key: Key::Named(named),
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    })
}

#[tokio::test]
async fn select_menu_keyboard_can_choose_last_option_or_cancel() {
    use iced::keyboard::key::Named;
    let renderer =
        <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")).await.unwrap();
    for cancel in [false, true] {
        let mut select =
            settings_select((0..40).map(|n| n.to_string()), Some("0".to_owned()), selected);
        let mut tree = widget::Tree::new(&select as &dyn Widget<Message, Theme, Renderer>);
        let viewport = Rectangle::with_size(Size::new(640.0, 480.0));
        let node =
            select.layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, viewport.size()));
        open(&mut select, &mut tree, &renderer, &node, viewport);
        let mut messages = Vec::new();
        {
            let mut overlay = select
                .overlay(&mut tree, Layout::new(&node), &renderer, &viewport, Vector::ZERO)
                .unwrap();
            let mut menu = overlay.as_overlay_mut().layout(&renderer, viewport.size());
            for key in [Named::End, if cancel { Named::Escape } else { Named::Enter }] {
                let mut shell = Shell::new(&mut messages);
                overlay.as_overlay_mut().update(
                    &key_press(key),
                    Layout::new(&menu),
                    mouse::Cursor::Unavailable,
                    &renderer,
                    &mut clipboard::Null,
                    &mut shell,
                );
                assert!(shell.is_event_captured());
                menu = overlay.as_overlay_mut().layout(&renderer, viewport.size());
            }
        }
        assert!(!tree.state.downcast_ref::<State<String>>().menu.open);
        if cancel {
            assert!(messages.is_empty());
        } else {
            assert_eq!(messages.len(), 1);
            assert!(
                matches!(&messages[0], Message::Settings(SettingsMessage::General(GeneralSettingsMessage::TimezoneSelected(value))) if value == "39")
            );
        }
    }
}

fn dispatch(
    select: &mut SettingsSelect<'_, String>,
    tree: &mut widget::Tree,
    renderer: &Renderer,
    node: &layout::Node,
    viewport: Rectangle,
    event: Event,
    cursor: mouse::Cursor,
) -> Vec<Message> {
    let mut messages = Vec::new();
    select.update(
        tree,
        &event,
        Layout::new(node),
        cursor,
        renderer,
        &mut clipboard::Null,
        &mut Shell::new(&mut messages),
        &viewport,
    );
    messages
}

fn open(
    select: &mut SettingsSelect<'_, String>,
    tree: &mut widget::Tree,
    renderer: &Renderer,
    node: &layout::Node,
    viewport: Rectangle,
) {
    let cursor = mouse::Cursor::Available(Layout::new(node).bounds().center());
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        assert!(
            dispatch(select, tree, renderer, node, viewport, Event::Mouse(event), cursor)
                .is_empty()
        );
    }
    assert!(tree.state.downcast_ref::<State<String>>().menu.open);
}

#[tokio::test]
async fn select_menu_hover_survives_overlay_recreation_and_clears_on_exit() {
    let viewport = Rectangle::with_size(Size::new(480.0, 420.0));
    for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut select = settings_select(
            ["简体中文", "繁體中文", "English"].map(str::to_owned),
            Some("简体中文".to_owned()),
            selected,
        );
        let renderer = <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .unwrap();
        let mut tree = widget::Tree::new(&select as &dyn Widget<Message, Theme, Renderer>);
        let node = select
            .layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, viewport.size()))
            .move_to(Point::new(40.0, 40.0));
        open(&mut select, &mut tree, &renderer, &node, viewport);
        let bounds = {
            let mut overlay = select
                .overlay(&mut tree, Layout::new(&node), &renderer, &viewport, Vector::ZERO)
                .unwrap();
            Layout::new(&overlay.as_overlay_mut().layout(&renderer, viewport.size())).bounds()
        };
        for hovered in [Some(1), Some(2), None] {
            // Iced 会在事件与绘制之间重新取得 overlay，不能依赖旧 Button 的临时状态。
            let position = hovered
                .map(|index| {
                    Point::new(bounds.x + 150.0, bounds.y + 6.0 + index as f32 * 38.0 + 18.0)
                })
                .unwrap_or(Point::new(10.0, 400.0));
            let cursor = mouse::Cursor::Available(position);
            for event in [
                Event::Mouse(mouse::Event::CursorMoved { position }),
                Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now())),
            ] {
                let mut messages = Vec::new();
                let mut overlay = select
                    .overlay(&mut tree, Layout::new(&node), &renderer, &viewport, Vector::ZERO)
                    .unwrap();
                let menu = overlay.as_overlay_mut().layout(&renderer, viewport.size());
                overlay.as_overlay_mut().update(
                    &event,
                    Layout::new(&menu),
                    cursor,
                    &renderer,
                    &mut clipboard::Null,
                    &mut Shell::new(&mut messages),
                );
                assert!(messages.is_empty(), "hover must not change the selected value");
            }
            let mut renderer =
                <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                    .await
                    .unwrap();
            let defaults = renderer::Style { text_color: theme.palette().text };
            select.draw(
                &tree,
                &mut renderer,
                &theme,
                &defaults,
                Layout::new(&node),
                cursor,
                &viewport,
            );
            {
                let mut overlay = select
                    .overlay(&mut tree, Layout::new(&node), &renderer, &viewport, Vector::ZERO)
                    .unwrap();
                let menu = overlay.as_overlay_mut().layout(&renderer, viewport.size());
                overlay.as_overlay().draw(
                    &mut renderer,
                    &theme,
                    &defaults,
                    Layout::new(&menu),
                    cursor,
                );
            }
            let pixels = renderer.screenshot(Size::new(480, 420), 1.0, theme.palette().background);
            let sample = |index: usize| {
                let x = (bounds.x + 150.0) as usize;
                let y = (bounds.y + 6.0 + index as f32 * 38.0 + 18.0) as usize;
                &pixels[(y * 480 + x) * 4..(y * 480 + x) * 4 + 4]
            };
            for index in 1..3 {
                if hovered == Some(index) {
                    assert_ne!(
                        sample(index),
                        sample(0),
                        "hovered row must be highlighted after overlay recreation ({theme_name})"
                    );
                } else {
                    assert_eq!(
                        sample(index),
                        sample(0),
                        "previous hover must clear ({theme_name})"
                    );
                }
            }
            if let Ok(directory) = std::env::var("SHIJU_SETTINGS_PREVIEW_DIR") {
                std::fs::create_dir_all(&directory).unwrap();
                image::save_buffer(
                    std::path::Path::new(&directory).join(format!(
                        "select-hover-{theme_name}-{}.png",
                        hovered.map_or("outside".to_owned(), |n| n.to_string())
                    )),
                    &pixels,
                    480,
                    420,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
}

#[tokio::test]
async fn select_menu_click_selects_original_value_and_closes() {
    let renderer =
        <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")).await.unwrap();
    let mut select = settings_select(
        ["简体中文".to_owned(), "English".to_owned()],
        Some("简体中文".to_owned()),
        selected,
    );
    let mut tree = widget::Tree::new(&select as &dyn Widget<Message, Theme, Renderer>);
    let viewport = Rectangle::with_size(Size::new(640.0, 480.0));
    let node = select
        .layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, viewport.size()))
        .move_to(Point::new(40.0, 40.0));
    open(&mut select, &mut tree, &renderer, &node, viewport);
    let mut messages = Vec::new();
    {
        let mut overlay = select
            .overlay(&mut tree, Layout::new(&node), &renderer, &viewport, Vector::ZERO)
            .unwrap();
        let menu = overlay.as_overlay_mut().layout(&renderer, viewport.size());
        let bounds = Layout::new(&menu).bounds();
        let cursor =
            mouse::Cursor::Available(Point::new(bounds.x + 40.0, bounds.y + 6.0 + 38.0 + 18.0));
        for event in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            overlay.as_overlay_mut().update(
                &Event::Mouse(event),
                Layout::new(&menu),
                cursor,
                &renderer,
                &mut clipboard::Null,
                &mut Shell::new(&mut messages),
            );
        }
    }
    assert!(!tree.state.downcast_ref::<State<String>>().menu.open);
    assert_eq!(messages.len(), 1);
    assert!(
        matches!(&messages[0], Message::Settings(SettingsMessage::General(GeneralSettingsMessage::TimezoneSelected(value))) if value == "English")
    );
}

#[tokio::test]
async fn select_menu_outside_click_closes_without_changing_value() {
    let renderer =
        <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")).await.unwrap();
    let mut select = settings_select(["当前值".to_owned()], Some("当前值".to_owned()), selected);
    let mut tree = widget::Tree::new(&select as &dyn Widget<Message, Theme, Renderer>);
    let viewport = Rectangle::with_size(Size::new(640.0, 480.0));
    let node =
        select.layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, viewport.size()));
    open(&mut select, &mut tree, &renderer, &node, viewport);
    let mut messages = Vec::new();
    {
        let mut overlay = select
            .overlay(&mut tree, Layout::new(&node), &renderer, &viewport, Vector::ZERO)
            .unwrap();
        let menu = overlay.as_overlay_mut().layout(&renderer, viewport.size());
        let mut shell = Shell::new(&mut messages);
        overlay.as_overlay_mut().update(
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&menu),
            mouse::Cursor::Available(Point::new(600.0, 400.0)),
            &renderer,
            &mut clipboard::Null,
            &mut shell,
        );
        assert!(shell.is_event_captured());
    }
    assert!(messages.is_empty());
    assert!(!tree.state.downcast_ref::<State<String>>().menu.open);
}

#[tokio::test]
async fn select_menu_renders_checked_and_hovered_options_and_fits_viewport() {
    let viewport = Rectangle::with_size(Size::new(480.0, 420.0));
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        for (long, y) in [(false, 40.0), (true, 360.0)] {
            let mut renderer =
                <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                    .await
                    .unwrap();
            let options = if long {
                (0..40).map(|n| format!("模型选项 {n}")).collect()
            } else {
                vec![
                    "永不".to_owned(),
                    "1 天".to_owned(),
                    "7 天".to_owned(),
                    "14 天".to_owned(),
                    "30 天".to_owned(),
                    "90 天".to_owned(),
                ]
            };
            let mut select = settings_select(
                options,
                Some(if long { "模型选项 35" } else { "永不" }.to_owned()),
                selected,
            )
            .width(180.0);
            let mut tree = widget::Tree::new(&select as &dyn Widget<Message, Theme, Renderer>);
            let node = select
                .layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, viewport.size()))
                .move_to(Point::new(280.0, y));
            open(&mut select, &mut tree, &renderer, &node, viewport);
            tree.state.downcast_mut::<State<String>>().menu.keyboard_option =
                if long { Some(35) } else { None };
            let defaults = renderer::Style { text_color: theme.palette().text };
            select.draw(
                &tree,
                &mut renderer,
                &theme,
                &defaults,
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &viewport,
            );
            {
                let mut overlay = select
                    .overlay(&mut tree, Layout::new(&node), &renderer, &viewport, Vector::ZERO)
                    .unwrap();
                let menu = overlay.as_overlay_mut().layout(&renderer, viewport.size());
                let bounds = Layout::new(&menu).bounds();
                assert!(bounds.x >= 0.0 && bounds.x + bounds.width <= viewport.width);
                assert!(bounds.y >= 0.0 && bounds.y + bounds.height <= viewport.height);
                if long {
                    assert!(bounds.y + bounds.height < y);
                }
                let cursor = mouse::Cursor::Available(Point::new(
                    bounds.x + 40.0,
                    bounds.y + 6.0 + 38.0 + 18.0,
                ));
                let mut messages = Vec::new();
                overlay.as_overlay_mut().update(
                    &Event::Window(
                        iced::window::Event::RedrawRequested(iced::time::Instant::now()),
                    ),
                    Layout::new(&menu),
                    cursor,
                    &renderer,
                    &mut clipboard::Null,
                    &mut Shell::new(&mut messages),
                );
                overlay.as_overlay().draw(
                    &mut renderer,
                    &theme,
                    &defaults,
                    Layout::new(&menu),
                    cursor,
                );
                assert!(messages.is_empty());
            }
            let pixels = renderer.screenshot(Size::new(480, 420), 1.0, theme.palette().background);
            assert!(pixels.chunks_exact(4).any(|p| p[3] != 0));
            if let Ok(directory) = std::env::var("SHIJU_SETTINGS_PREVIEW_DIR") {
                std::fs::create_dir_all(&directory).unwrap();
                image::save_buffer(
                    std::path::Path::new(&directory).join(format!(
                        "select-{name}-{}.png",
                        if long { "scroll" } else { "open" }
                    )),
                    &pixels,
                    480,
                    420,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
}
