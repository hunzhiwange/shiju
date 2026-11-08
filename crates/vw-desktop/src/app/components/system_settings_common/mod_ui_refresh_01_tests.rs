use super::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, settings_select, settings_text_input_style,
};
use crate::app::Message;
use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
use iced::advanced::{
    Clipboard, Layout, Shell, Widget, clipboard, layout, mouse,
    renderer::{self, Headless},
    widget,
};
use iced::keyboard::{self, Key, key::Named};
use iced::{Event, Font, Length, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector};

fn selected(value: String) -> Message {
    Message::Settings(SettingsMessage::General(GeneralSettingsMessage::TimezoneSelected(value)))
}

fn key(named: Named) -> Event {
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

async fn renderer() -> Renderer {
    <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")).await.unwrap()
}

type SelectWidget<'a> = dyn Widget<Message, Theme, Renderer> + 'a;

struct Ui {
    renderer: Renderer,
    viewport: Rectangle,
    tree: widget::Tree,
    node: layout::Node,
    messages: Vec<Message>,
}

impl Ui {
    fn new(control: &mut SelectWidget<'_>, renderer: Renderer) -> Self {
        Self::at_size(control, renderer, Size::new(640.0, 480.0))
    }

    fn at_size(control: &mut SelectWidget<'_>, renderer: Renderer, size: Size) -> Self {
        let viewport = Rectangle::with_size(size);
        let mut tree = widget::Tree::new(&*control);
        let node =
            control.layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, viewport.size()));
        Self { renderer, viewport, tree, node, messages: Vec::new() }
    }

    fn update(
        &mut self,
        control: &mut SelectWidget<'_>,
        event: Event,
        cursor: mouse::Cursor,
    ) -> bool {
        let mut shell = Shell::new(&mut self.messages);
        let clipboard: &mut dyn Clipboard = &mut clipboard::Null;
        if let Some(mut overlay) = control.overlay(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        ) {
            let overlay = overlay.as_overlay_mut();
            let menu = overlay.layout(&self.renderer, self.viewport.size());
            overlay.update(
                &event,
                Layout::new(&menu),
                cursor,
                &self.renderer,
                clipboard,
                &mut shell,
            );
        }
        if !shell.is_event_captured() {
            control.update(
                &mut self.tree,
                &event,
                Layout::new(&self.node),
                cursor,
                &self.renderer,
                clipboard,
                &mut shell,
                &self.viewport,
            );
        }
        shell.is_event_captured()
    }

    fn click_trigger(&mut self, control: &mut SelectWidget<'_>) {
        let cursor = mouse::Cursor::Available(Layout::new(&self.node).bounds().center());
        self.click(control, cursor);
    }

    fn click(&mut self, control: &mut SelectWidget<'_>, cursor: mouse::Cursor) {
        for event in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            self.update(control, Event::Mouse(event), cursor);
        }
    }

    fn press(&mut self, control: &mut SelectWidget<'_>, named: Named) -> bool {
        self.update(control, key(named), mouse::Cursor::Unavailable)
    }

    fn menu_bounds(&mut self, control: &mut SelectWidget<'_>) -> Option<Rectangle> {
        self.menu_layout(control).map(|node| Layout::new(&node).bounds())
    }

    fn menu_layout(&mut self, control: &mut SelectWidget<'_>) -> Option<layout::Node> {
        let mut overlay = control.overlay(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        )?;
        Some(overlay.as_overlay_mut().layout(&self.renderer, self.viewport.size()))
    }

    fn operate(&mut self, control: &mut SelectWidget<'_>, operation: &mut dyn widget::Operation) {
        control.operate(&mut self.tree, Layout::new(&self.node), &self.renderer, operation);
    }

    fn next_focus(&mut self, control: &mut SelectWidget<'_>) {
        self.run_operation(control, widget::operation::focusable::focus_next());
    }

    fn run_operation(
        &mut self,
        control: &mut SelectWidget<'_>,
        operation: impl widget::Operation + 'static,
    ) {
        use widget::operation::Outcome;
        let mut operation: Box<dyn widget::Operation> = Box::new(operation);
        loop {
            self.operate(control, operation.as_mut());
            match operation.finish() {
                Outcome::Chain(next) => operation = next,
                _ => break,
            }
        }
    }

    fn draw(&mut self, control: &mut SelectWidget<'_>, theme: &Theme, cursor: mouse::Cursor) {
        use iced::advanced::Renderer as _;
        self.renderer.reset(self.viewport);
        let defaults = renderer::Style { text_color: theme.palette().text };
        control.draw(
            &self.tree,
            &mut self.renderer,
            theme,
            &defaults,
            Layout::new(&self.node),
            cursor,
            &self.viewport,
        );
        if let Some(overlay) = control.overlay(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        ) {
            // Match Iced's draw path so text in the base layer stays below the menu.
            let mut overlay = iced::advanced::overlay::Nested::new(overlay);
            let menu = overlay.layout(&self.renderer, self.viewport.size());
            overlay.draw(&mut self.renderer, theme, &defaults, Layout::new(&menu), cursor);
        }
    }

    fn assert_selection(&mut self, expected: &str) {
        assert!(
            matches!(
                self.messages.as_slice(),
                [Message::Settings(SettingsMessage::General(GeneralSettingsMessage::TimezoneSelected(value)))]
                    if value == expected
            ),
            "expected exactly one selection: {expected}"
        );
        self.messages.clear();
    }
}

#[tokio::test]
async fn settings_text_input_matches_select_height() {
    let mut input = iced::widget::text_input("搜索", "示例")
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style);
    let mut select = settings_select(["示例".to_owned()], None, selected);

    let input = Ui::new(&mut input, renderer().await);
    let select = Ui::new(&mut select, renderer().await);

    assert!((input.node.size().height - select.node.size().height).abs() < 0.01);
}

#[tokio::test]
async fn ui_refresh_01_layout_and_interaction_keyboard_focus_opens_trigger() {
    let mut control = settings_select(["English".to_owned()], None, selected);
    let mut ui = Ui::new(&mut control, renderer().await);
    ui.press(&mut control, Named::Enter);
    assert!(ui.menu_bounds(&mut control).is_none(), "unfocused Enter must pass through");

    ui.next_focus(&mut control);
    ui.press(&mut control, Named::Enter);
    assert!(ui.menu_bounds(&mut control).is_some(), "Iced focus traversal must enable Enter");
    ui.press(&mut control, Named::Escape);
    assert!(ui.menu_bounds(&mut control).is_none());
    assert!(ui.messages.is_empty());
}

#[tokio::test]
async fn ui_refresh_01_preserves_values_when_options_are_replaced() {
    let mut control =
        settings_select(["A", "B"].map(str::to_owned), Some("A".to_owned()), selected);
    let mut ui = Ui::new(&mut control, renderer().await);
    ui.click_trigger(&mut control);
    ui.press(&mut control, Named::End);
    assert!(ui.menu_bounds(&mut control).is_some());

    let mut replacement =
        settings_select(["A", "C"].map(str::to_owned), Some("A".to_owned()), selected);
    ui.tree.diff(&replacement as &SelectWidget<'_>);
    ui.press(&mut replacement, Named::Enter);
    assert!(ui.messages.is_empty(), "Enter must not commit a replacement at a stale index");
}

#[tokio::test]
async fn ui_refresh_01_tab_leaves_menu_and_skips_disabled_trigger() {
    use iced::widget::column;
    let mut form = column![
        settings_select(["first".to_owned()], None, selected),
        settings_select(["disabled".to_owned()], None, selected).disabled(true),
        settings_select(["last".to_owned()], None, selected),
    ];
    let mut ui = Ui::new(&mut form, renderer().await);

    ui.next_focus(&mut form);
    ui.press(&mut form, Named::Space);
    assert!(ui.menu_bounds(&mut form).is_some());
    assert!(!ui.press(&mut form, Named::Tab), "Tab must reach the page focus handler");
    assert!(ui.menu_bounds(&mut form).is_none());
    assert!(ui.messages.is_empty());

    ui.next_focus(&mut form);
    ui.press(&mut form, Named::Enter);
    ui.press(&mut form, Named::Enter);
    ui.assert_selection("last");

    ui.run_operation(&mut form, widget::operation::focusable::focus_previous());
    ui.press(&mut form, Named::Enter);
    ui.press(&mut form, Named::Enter);
    ui.assert_selection("first");
}

#[tokio::test]
async fn ui_refresh_01_preserves_values_and_disabled_states() {
    let mut control =
        settings_select(["available".to_owned()], Some("saved, unavailable".to_owned()), selected)
            .placeholder("Choose a value");
    let mut ui = Ui::new(&mut control, renderer().await);
    ui.click_trigger(&mut control);
    assert!(ui.menu_bounds(&mut control).is_some());
    assert!(ui.press(&mut control, Named::Escape));
    assert!(ui.messages.is_empty(), "Esc cannot replace an unavailable saved value");
    ui.click_trigger(&mut control);

    for options in [vec!["available".to_owned()], vec![]] {
        let explicit_disable = !options.is_empty();
        let mut disabled =
            settings_select(options, Some("saved, unavailable".to_owned()), selected)
                .disabled(explicit_disable);
        ui.tree.diff(&disabled as &SelectWidget<'_>);
        assert!(ui.menu_bounds(&mut disabled).is_none(), "disablement must dismiss the menu");
        ui.click_trigger(&mut disabled);
        ui.next_focus(&mut disabled);
        assert!(!ui.press(&mut disabled, Named::Enter));
        assert!(ui.menu_bounds(&mut disabled).is_none());
        assert!(ui.messages.is_empty());
    }
}

#[tokio::test]
async fn ui_refresh_01_same_labels_cannot_redirect_an_in_flight_press() {
    #[derive(Clone, PartialEq)]
    struct Choice(u32);

    impl std::fmt::Display for Choice {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("Same label")
        }
    }

    let make = |id| settings_select([Choice(id)], None, |value| selected(value.0.to_string()));
    let mut control = make(1);
    let mut ui = Ui::new(&mut control, renderer().await);
    ui.click_trigger(&mut control);
    let cursor = mouse::Cursor::Available(ui.menu_bounds(&mut control).unwrap().center());
    ui.update(&mut control, Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)), cursor);

    let mut replacement = make(2);
    ui.tree.diff(&replacement as &SelectWidget<'_>);
    ui.update(
        &mut replacement,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        cursor,
    );
    assert!(ui.messages.is_empty());
    ui.click_trigger(&mut replacement);
    ui.click(&mut replacement, cursor);
    ui.assert_selection("2");
}

#[tokio::test]
async fn ui_refresh_01_wheel_and_keyboard_scroll_select_the_visible_value() {
    let mut control =
        settings_select((0..40).map(|n| n.to_string()), Some("0".to_owned()), selected);
    let mut ui = Ui::new(&mut control, renderer().await);
    ui.click_trigger(&mut control);
    let bounds = ui.menu_bounds(&mut control).unwrap();
    let cursor = mouse::Cursor::Available(Point::new(bounds.center_x(), bounds.y + 24.0));
    ui.update(
        &mut control,
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -114.0 },
        }),
        cursor,
    );
    ui.click(&mut control, cursor);
    ui.assert_selection("3");

    ui.press(&mut control, Named::Space);
    ui.press(&mut control, Named::End);
    let bounds = ui.menu_bounds(&mut control).unwrap();
    // End must scroll the last row under the pointer, even in a newly created overlay.
    let cursor =
        mouse::Cursor::Available(Point::new(bounds.center_x(), bounds.y + bounds.height - 24.0));
    ui.click(&mut control, cursor);
    ui.assert_selection("39");

    ui.click_trigger(&mut control);
    let outside = mouse::Cursor::Available(Point::new(630.0, 470.0));
    assert!(ui.update(
        &mut control,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        outside,
    ));
    assert!(!ui.press(&mut control, Named::Enter), "outside press must clear focus");
    assert!(ui.menu_bounds(&mut control).is_none());
    assert!(ui.messages.is_empty());
}

#[tokio::test]
async fn ui_refresh_01_layout_and_interaction_renders_long_labels_and_states() {
    use iced::widget::{column, container};

    let long_name = "用于测试的长标签與繁體文字 /fixtures/models/very-long-catalog-entry".repeat(4);
    let options = [long_name.clone(), "繁體中文模型".to_owned(), "English (fixture)".to_owned()]
        .into_iter()
        .chain((0..20).map(|n| format!("模型 {n}")));
    for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        for width in [420, 640, 960] {
            let make_options = || options.clone();
            let disabled = |label: &str| {
                settings_select(Vec::<String>::new(), None, selected)
                    .placeholder(label)
                    .width(Length::Fill)
            };
            let mut form = container(
                column![
                    settings_select(make_options(), Some(long_name.clone()), selected)
                        .width(Length::Fill),
                    settings_select(make_options(), None, selected)
                        .placeholder("请选择 / Choose")
                        .width(Length::Fill),
                    disabled("載入中…"),
                    disabled("加载失败 / Could not load"),
                    settings_select(make_options(), Some("English (fixture)".to_owned()), selected)
                        .disabled(true)
                        .width(Length::Fill),
                ]
                .spacing(10),
            )
            .padding(16)
            .width(Length::Fill);
            let mut ui = Ui::at_size(&mut form, renderer().await, Size::new(width as f32, 480.0));

            let field = Layout::new(&ui.node).children().next().unwrap().children().next().unwrap();
            for child in field.children().next().unwrap().children() {
                assert!((field.bounds().center_y() - child.bounds().center_y()).abs() < 0.5);
                assert!(field.bounds().contains(child.bounds().center()));
            }

            ui.draw(&mut form, &theme, mouse::Cursor::Unavailable);
            save_preview(&mut ui, &theme, &format!("select-fields-{theme_name}-{width}.png"));

            ui.next_focus(&mut form);
            ui.press(&mut form, Named::Enter);
            let group = ui.menu_layout(&mut form).unwrap();
            let menu = Layout::new(&group).children().next().unwrap();
            let bounds = menu.bounds();
            assert!(bounds.height <= 320.0);
            assert!(bounds.x >= 0.0 && bounds.x + bounds.width <= ui.viewport.width);
            assert!(bounds.y >= 0.0 && bounds.y + bounds.height <= ui.viewport.height);
            let position = Point::new(bounds.center_x(), bounds.y + 62.0);
            let cursor = mouse::Cursor::Available(position);
            ui.update(&mut form, Event::Mouse(mouse::Event::CursorMoved { position }), cursor);
            ui.draw(&mut form, &theme, cursor);
            save_preview(&mut ui, &theme, &format!("select-menu-{theme_name}-{width}.png"));
            ui.press(&mut form, Named::Escape);
            assert!(ui.messages.is_empty());
        }
    }
}

fn save_preview(ui: &mut Ui, theme: &Theme, file: &str) {
    let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR") else { return };
    let size = Size::new(ui.viewport.width as u32, ui.viewport.height as u32);
    let pixels = ui.renderer.screenshot(size, 1.0, theme.palette().background);
    std::fs::create_dir_all(&directory).unwrap();
    image::save_buffer(
        std::path::Path::new(&directory).join(file),
        &pixels,
        size.width,
        size.height,
        image::ColorType::Rgba8,
    )
    .unwrap();
}
