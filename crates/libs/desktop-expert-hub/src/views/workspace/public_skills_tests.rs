//! 桌面组合测试复用的渲染交互工具。领域断言位于 desktop-expert-hub。
use config_ui::DesktopLocale;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::operation::{Focusable, Operation, Scrollable};
use iced::advanced::{Layout, clipboard, layout, mouse, renderer, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector};
use iced_runtime::user_interface::{Cache, UserInterface};

use super::Message;
pub(super) const CONTROL_LOCALES: [(DesktopLocale, &str); 3] = [
    (DesktopLocale::SimplifiedChinese, "虚构技能目录"),
    (DesktopLocale::TraditionalChinese, "虛構技能目錄"),
    (DesktopLocale::English, "Sample skill catalog"),
];

pub(super) async fn control_renderer() -> Renderer {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    Renderer::new(Font::with_name("Noto Sans CJK SC"), Pixels(14.0), Some("tiny-skia"))
        .await
        .expect("headless renderer")
}

struct FocusBounds(Rectangle);

impl Operation for FocusBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        if bounds == self.0 {
            state.focus();
        } else {
            state.unfocus();
        }
    }
}

#[derive(Default)]
struct MenuBounds(Option<Rectangle>);

impl Operation for MenuBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        _translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if id == Some(&iced::advanced::widget::Id::from("settings-select-menu")) {
            self.0 = Some(bounds);
        }
    }
}

pub(super) struct SkillControls<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    layout: layout::Node,
    size: Size<u32>,
}

impl<'a> SkillControls<'a> {
    pub(super) fn new(mut view: Element<'a, Message>, renderer: &mut Renderer, width: u32) -> Self {
        let size = Size::new(width, 680);
        let bounds = Size::new(width as f32, size.height as f32);
        let mut tree = Tree::new(view.as_widget());
        let layout = view.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        let ui = UserInterface::build(view, bounds, Cache::new(), renderer);
        Self { ui, layout, size }
    }

    pub(super) fn bounds(&self, path: &[usize]) -> Rectangle {
        let bounds = path
            .iter()
            .fold(Layout::new(&self.layout), |layout, &index| {
                layout.children().nth(index).unwrap_or_else(|| panic!("missing layout {path:?}"))
            })
            .bounds();
        assert!(bounds.width > 0.0 && bounds.x + bounds.width <= self.size.width as f32);
        bounds
    }

    fn send(
        &mut self,
        renderer: &mut Renderer,
        event: Event,
        cursor: mouse::Cursor,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(&[event], cursor, renderer, &mut clipboard::Null, &mut messages);
        messages
    }

    pub(super) fn enter(&mut self, renderer: &mut Renderer, bounds: Rectangle) -> Vec<Message> {
        self.ui.operate(renderer, &mut FocusBounds(bounds));
        self.key(renderer, Named::Enter)
    }

    pub(super) fn key(&mut self, renderer: &mut Renderer, key: Named) -> Vec<Message> {
        self.send(
            renderer,
            Event::Keyboard(keyboard::Event::KeyPressed {
                modified_key: Key::Named(key),
                key: Key::Named(key),
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::empty(),
                text: None,
                repeat: false,
            }),
            mouse::Cursor::Unavailable,
        )
    }

    pub(super) fn click(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
        let cursor = mouse::Cursor::Available(point);
        let mut messages = self.send(
            renderer,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            cursor,
        );
        messages.extend(self.send(
            renderer,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            cursor,
        ));
        messages
    }

    fn draw(&mut self, renderer: &mut Renderer, theme: &Theme, cursor: mouse::Cursor) -> Vec<u8> {
        self.send(
            renderer,
            Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now())),
            cursor,
        );
        self.ui.draw(
            renderer,
            theme,
            &renderer::Style { text_color: theme.palette().text },
            cursor,
        );
        renderer.screenshot(self.size, 1.0, theme.palette().background)
    }

    fn menu_bounds(&mut self, renderer: &Renderer, name: &str) -> Rectangle {
        let mut menu = MenuBounds::default();
        self.ui.operate(renderer, &mut menu);
        menu.0.unwrap_or_else(|| panic!("{name} should open the settings menu on Enter"))
    }

    pub(super) fn choose_second(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        trigger: Rectangle,
        name: &str,
    ) -> Vec<Message> {
        let closed = self.draw(renderer, theme, mouse::Cursor::Unavailable);
        assert!(self.enter(renderer, trigger).is_empty());
        let opened = self.draw(renderer, theme, mouse::Cursor::Unavailable);
        let menu = self.menu_bounds(renderer, name);

        // Menu rows are 36px high with 2px between them.
        let first_y = (menu.y + 18.0) as usize;
        let second = Point::new(menu.center().x, menu.y + 56.0);
        let width = self.size.width as usize;
        let pixel = |pixels: &[u8], x: usize, y: usize| {
            let offset = (y * width + x) * 4;
            [pixels[offset], pixels[offset + 1], pixels[offset + 2], pixels[offset + 3]]
        };
        let right = (menu.x + menu.width) as usize;
        let mark_x = right - 25;
        let region = |center_y: usize| {
            (center_y - 7..center_y + 7)
                .flat_map(move |y| (mark_x - 9..mark_x + 9).map(move |x| (x, y)))
        };
        let changed =
            region(first_y).filter(|&(x, y)| pixel(&closed, x, y) != pixel(&opened, x, y)).count();
        assert!(changed > 10, "{name} should draw an overlay on Enter");

        let cursor = mouse::Cursor::Available(second);
        self.send(renderer, Event::Mouse(mouse::Event::CursorMoved { position: second }), cursor);
        let hovered = self.draw(renderer, theme, cursor);
        let changed = region(second.y as usize)
            .filter(|&(x, y)| pixel(&opened, x, y) != pixel(&hovered, x, y))
            .count();
        assert!(changed > 10, "{name} should highlight the hovered option");

        let background = pixel(&hovered, mark_x, first_y - 9);
        let marks = region(first_y).filter(|&(x, y)| pixel(&hovered, x, y) != background).count();
        assert!(marks > 10, "{name} should draw a checkmark for the current option");

        self.click(renderer, second)
    }
}
