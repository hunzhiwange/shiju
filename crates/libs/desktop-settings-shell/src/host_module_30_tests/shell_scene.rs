#![allow(dead_code)] // Individual consumers exercise different shared assertion methods.
//! Shared shell scene assertions; callers supply real domain or cross-domain views.
use desktop_settings_appearance::general_fields;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::{Id, Operation};
use iced::advanced::{Layout, Shell, clipboard, layout, overlay, renderer, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
const SETTINGS_CONTENT_SCROLL: &str = "system-settings-content-scroll";
pub trait SceneMessage: Clone {
    fn copied_text(&self) -> Option<&str>;
}
pub struct Scene<'a, Message> {
    pub element: Element<'a, Message>,
    pub tree: Tree,
    pub node: layout::Node,
    pub renderer: Renderer,
    pub viewport: Rectangle,
}

impl<'a, Message: SceneMessage + 'static> Scene<'a, Message> {
    pub async fn new(mut element: Element<'a, Message>, width: u32, height: u32) -> Self {
        static FONTS: std::sync::Once = std::sync::Once::new();
        FONTS.call_once(|| {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in desktop_assets::fonts::load_all() {
                fonts.load_font(font);
            }
        });
        let font = Font::with_name(if cfg!(target_os = "macos") {
            "Noto Sans CJK SC"
        } else {
            "JetBrains Mono"
        });
        let renderer = Renderer::new(font, 16.0.into(), Some("tiny-skia")).await.unwrap();
        let viewport = Rectangle::with_size(Size::new(width as f32, height as f32));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        Self { element, tree, node, renderer, viewport }
    }

    pub fn bounds(&self, parent: &[usize], children: &[usize]) -> Rectangle {
        parent
            .iter()
            .chain(children)
            .fold(Layout::new(&self.node), |node, &index| node.children().nth(index).unwrap())
            .bounds()
    }

    pub fn dispatch(&mut self, event: Event, mut cursor: mouse::Cursor) -> Vec<Message> {
        let root = Layout::new(&self.node);
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        if let Some(layer) = self.element.as_widget_mut().overlay(
            &mut self.tree,
            root,
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        ) {
            let mut layer = overlay::Nested::new(layer);
            let node = layer.layout(&self.renderer, self.viewport.size());
            let layout = Layout::new(&node);
            let interaction = layer.mouse_interaction(layout, cursor, &self.renderer);
            layer.update(&event, layout, cursor, &self.renderer, &mut clipboard::Null, &mut shell);
            if interaction != mouse::Interaction::None {
                cursor = mouse::Cursor::Unavailable;
            }
        }
        if !shell.is_event_captured() {
            self.element.as_widget_mut().update(
                &mut self.tree,
                &event,
                root,
                cursor,
                &self.renderer,
                &mut clipboard::Null,
                &mut shell,
                &self.viewport,
            );
        }
        messages
    }

    pub fn overlay_bounds(&mut self) -> Option<Rectangle> {
        let layer = self.element.as_widget_mut().overlay(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        )?;
        let node = overlay::Nested::new(layer).layout(&self.renderer, self.viewport.size());
        let mut layout = Layout::new(&node).children().next()?;
        while layout.bounds() == self.viewport {
            layout = layout.children().next()?;
        }
        Some(layout.bounds())
    }

    pub fn click(&mut self, point: Point) -> Vec<Message> {
        [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ]
        .into_iter()
        .flat_map(|event| self.dispatch(Event::Mouse(event), mouse::Cursor::Available(point)))
        .collect()
    }

    pub fn key(&mut self, key: Key, text: Option<&str>) -> Vec<Message> {
        self.dispatch(
            Event::Keyboard(keyboard::Event::KeyPressed {
                modified_key: key.clone(),
                key,
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::empty(),
                text: text.map(Into::into),
                repeat: false,
            }),
            mouse::Cursor::Unavailable,
        )
    }

    pub fn snapshot(&mut self, theme: &Theme, cursor: mouse::Cursor, name: &str) -> Vec<u8> {
        use iced::advanced::Renderer as _;

        assert!(
            self.dispatch(
                Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now())),
                cursor,
            )
            .is_empty()
        );
        self.renderer.reset(self.viewport);
        let defaults = renderer::Style { text_color: theme.palette().text };
        let root = Layout::new(&self.node);
        self.element.as_widget().draw(
            &self.tree,
            &mut self.renderer,
            theme,
            &defaults,
            root,
            cursor,
            &self.viewport,
        );
        // Rebuild the overlay after event dispatch, as the native UI does.
        if let Some(layer) = self.element.as_widget_mut().overlay(
            &mut self.tree,
            root,
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        ) {
            let mut layer = overlay::Nested::new(layer);
            let node = layer.layout(&self.renderer, self.viewport.size());
            layer.draw(&mut self.renderer, theme, &defaults, Layout::new(&node), cursor);
        }
        let size = Size::new(self.viewport.width as u32, self.viewport.height as u32);
        let pixels = self.renderer.screenshot(size, 1.0, theme.palette().background);
        if !name.is_empty()
            && let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR")
        {
            let file = std::path::PathBuf::from(directory).join(format!("{name}.png"));
            image::save_buffer(file, &pixels, size.width, size.height, image::ColorType::Rgba8)
                .unwrap();
        }
        pixels
    }
}

#[derive(Default)]
pub struct FieldContents {
    pub target: Option<Id>,
    pub bounds: Option<Rectangle>,
    pub scroll_translation: Vector,
    pub scroll_bounds: Option<Rectangle>,
    pub text: Vec<String>,
}

impl Operation for FieldContents {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if self.target.as_ref() == id && id.is_some() {
            self.bounds = Some(bounds);
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        translation: Vector,
        _: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if id == Some(&Id::from(SETTINGS_CONTENT_SCROLL)) {
            self.scroll_translation = translation;
            self.scroll_bounds = Some(bounds);
        }
    }

    fn text(&mut self, _: Option<&Id>, _: Rectangle, value: &str) {
        self.text.push(value.to_owned());
    }
}

fn layout_at(layout: Layout<'_>, bounds: Rectangle) -> Option<Layout<'_>> {
    if layout.bounds() == bounds {
        Some(layout)
    } else {
        layout.children().find_map(|child| layout_at(child, bounds))
    }
}

impl<Message: SceneMessage + 'static> Scene<'_, Message> {
    pub async fn apply_scroll_task(&mut self, task: iced::Task<Message>) {
        use iced::advanced::widget::operation::Outcome;
        use iced::futures::StreamExt;

        let mut actions = iced_runtime::task::into_stream(task).expect("scroll operation");
        let Some(iced_runtime::Action::Widget(mut operation)) = actions.next().await else {
            panic!("expected a widget operation");
        };
        loop {
            self.element.as_widget_mut().operate(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                operation.as_mut(),
            );
            let Outcome::Chain(next) = operation.finish() else { break };
            operation = next;
        }
    }

    pub fn field_contents(&mut self, field: general_fields::GeneralSettingField) -> FieldContents {
        let mut contents = FieldContents { target: Some(Id::from(field.id)), ..Default::default() };
        self.element.as_widget_mut().operate(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            &mut contents,
        );
        assert!(contents.bounds.is_some(), "missing search field {}", field.id);
        contents
    }

    pub fn field_row(&mut self, field: general_fields::GeneralSettingField) -> Layout<'_> {
        let bounds = self.field_contents(field).bounds.unwrap();
        layout_at(Layout::new(&self.node), bounds).unwrap().children().next().unwrap()
    }

    pub fn help_trigger(&mut self, field: general_fields::GeneralSettingField) -> Point {
        let contents = self.field_contents(field);
        let translation = contents.scroll_translation;
        let label = self.field_row(field).children().next().unwrap();
        let icon = label.children().nth(1).expect("field question mark").bounds();
        assert_eq!(icon.size(), Size::new(22.0, 22.0));
        let point = icon.center() - translation;
        assert!(
            contents.scroll_bounds.expect("settings scroll viewport").contains(point),
            "question mark for {} lies outside the scroll viewport: {point:?}",
            field.id
        );
        point
    }

    pub fn control(&mut self, field: general_fields::GeneralSettingField) -> Layout<'_> {
        self.field_row(field)
            .children()
            .nth(1)
            .unwrap()
            .children()
            .next()
            .unwrap()
            .children()
            .next()
            .unwrap()
    }

    pub fn help_content(&mut self, theme: &Theme, point: Point, expected: &str) {
        let cursor = mouse::Cursor::Available(point);
        let idle = self.snapshot(theme, cursor, "");
        assert!(
            self.dispatch(Event::Mouse(mouse::Event::CursorMoved { position: point }), cursor)
                .is_empty()
        );
        let card = self.overlay_bounds().expect("hover opens the help card");
        assert!(card.x >= 0.0 && card.x + card.width <= self.viewport.width);
        assert!(card.y >= 0.0 && card.y + card.height <= self.viewport.height);
        assert!(self.snapshot(theme, cursor, "") != idle, "the card is drawn");
        assert!(
            self.dispatch(Event::Mouse(mouse::Event::CursorLeft), mouse::Cursor::Unavailable)
                .is_empty()
        );
        assert!(self.overlay_bounds().is_none());

        assert!(self.click(point).is_empty());
        assert!(
            self.dispatch(Event::Mouse(mouse::Event::CursorLeft), mouse::Cursor::Unavailable)
                .is_empty()
        );
        assert!(self.overlay_bounds().is_some(), "click pins the card");

        let start = Point::new(card.x + 12.5, card.y + 15.0);
        let end = Point::new(card.x + card.width - 1.0, card.y + card.height - 1.0);
        for (position, event) in [
            (start, mouse::Event::ButtonPressed(mouse::Button::Left)),
            (end, mouse::Event::CursorMoved { position: end }),
            (end, mouse::Event::ButtonReleased(mouse::Button::Left)),
        ] {
            assert!(
                self.dispatch(Event::Mouse(event), mouse::Cursor::Available(position)).is_empty()
            );
        }
        let copy = Key::Character("c".into());
        assert!(
            matches!(
                self.dispatch(
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        key: copy.clone(),
                        modified_key: copy,
                        physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyC),
                        location: keyboard::Location::Standard,
                        modifiers: if cfg!(target_os = "macos") {
                            keyboard::Modifiers::LOGO
                        } else {
                            keyboard::Modifiers::CTRL
                        },
                        text: None,
                        repeat: false,
                    }),
                    mouse::Cursor::Unavailable,
                ).as_slice(),
                [value] if value.copied_text() == Some(expected)
            ),
            "the card contains selectable localized help"
        );
        assert!(self.key(Key::Named(Named::Escape), None).is_empty());
        assert!(self.overlay_bounds().is_none());
    }
}
