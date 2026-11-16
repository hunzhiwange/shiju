//! 验证终端面板的标签管理、尺寸计算、显示状态和交互控件。
//!
//! 终端夹具构造不同可见性与标签数量，用于校验尺寸、选中项和关闭边界。

use super::support::{Routing, dispatch, view};
use crate::{Shell, TerminalTheme};
type TerminalState = crate::TerminalState<Shell>;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, widget::Tree};
use iced::{Font, Pixels, Size, Theme};

fn test_app() -> Routing {
    Routing::new()
}

fn blank_terminal(is_visible: bool) -> TerminalState {
    TerminalState::blank_with_settings(
        is_visible,
        Shell::Bash,
        TerminalTheme::System,
        "JetBrains Mono".to_string(),
        13.0,
        200.0,
    )
}

// Keep real host updates for pane drag, docking and terminal selection contracts.

#[cfg(not(target_arch = "wasm32"))]
struct DragPanel {
    app: Routing,
    renderer: iced::Renderer,
    cache: iced_runtime::user_interface::Cache,
}

#[cfg(not(target_arch = "wasm32"))]
impl DragPanel {
    const SIZE: Size = Size::new(800.0, 400.0);

    async fn new() -> Self {
        let mut app = test_app();
        app.terminal = blank_terminal(true);
        assert!(app.terminal.add_terminal(None));
        Self {
            app,
            renderer: <iced::Renderer as Headless>::new(
                Font::DEFAULT,
                Pixels(16.0),
                Some("tiny-skia"),
            )
            .await
            .expect("tiny-skia renderer"),
            cache: Default::default(),
        }
    }

    fn split(
        &mut self,
        pane: iced::widget::pane_grid::Pane,
        direction: iced::widget::pane_grid::Direction,
    ) -> iced::widget::pane_grid::Pane {
        self.app.terminal.split_terminal(pane, direction, None).unwrap();
        self.app.terminal.panes.focused
    }

    fn bounds(&mut self, pane: iced::widget::pane_grid::Pane) -> iced::Rectangle {
        let index = self.app.terminal.panes.grid.iter().position(|(id, _)| *id == pane).unwrap();
        let mut view = view(&self.app);
        let mut tree = Tree::new(view.as_widget());
        let node = view.as_widget_mut().layout(
            &mut tree,
            &self.renderer,
            &layout::Limits::new(Size::ZERO, Self::SIZE),
        );
        Layout::new(&node)
            .children()
            .next()
            .unwrap()
            .children()
            .next()
            .unwrap()
            .children()
            .nth(index)
            .unwrap()
            .bounds()
    }

    fn event(&mut self, event: iced::mouse::Event, point: iced::Point) {
        use iced_runtime::user_interface::UserInterface;

        let mut ui = UserInterface::build(
            view(&self.app),
            Self::SIZE,
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );
        let mut messages = Vec::new();
        let _ = ui.update(
            &[iced::Event::Mouse(event)],
            iced::mouse::Cursor::Available(point),
            &mut self.renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        for theme in [Theme::Light, Theme::Dark] {
            ui.draw(
                &mut self.renderer,
                &theme,
                &iced::advanced::renderer::Style { text_color: theme.palette().text },
                iced::mouse::Cursor::Available(point),
            );
        }
        self.cache = ui.into_cache();
        for message in messages {
            dispatch(&mut self.app, message);
        }
    }

    fn drag(&mut self, from: iced::Point, to: iced::Point) {
        use iced::mouse::{Button, Event};

        self.event(Event::ButtonPressed(Button::Left), from);
        self.event(Event::CursorMoved { position: to }, to);
        self.event(Event::ButtonReleased(Button::Left), to);
    }

    fn tab_title(&mut self, pane: iced::widget::pane_grid::Pane) -> iced::Point {
        let bounds = self.bounds(pane);
        iced::Point::new(bounds.x + 40.0, bounds.y + 14.0)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn terminal_drag_divider_resizes_nested_panes() {
    use iced::Point;
    use iced::widget::pane_grid::Direction;

    let mut panel = DragPanel::new().await;
    let first = panel.app.terminal.panes.focused;
    let right = panel.split(first, Direction::Right);
    let bottom = panel.split(first, Direction::Down);

    let before = panel.bounds(first);
    let vertical = (before.width + panel.bounds(right).x) / 2.0;
    panel.drag(Point::new(vertical + 3.0, 100.0), Point::new(500.0, 100.0));
    let wider = panel.bounds(first);
    eprintln!("vertical divider {vertical}, first pane {before:?} -> {wider:?}");
    assert!(wider.width > before.width + 90.0);
    assert_eq!(panel.bounds(bottom).width, wider.width);

    let horizontal = (wider.height + panel.bounds(bottom).y) / 2.0;
    panel.drag(Point::new(100.0, horizontal - 3.0), Point::new(100.0, 390.0));
    let taller = panel.bounds(first);
    let lower = panel.bounds(bottom);
    eprintln!("horizontal divider {horizontal}, panes {taller:?} and {lower:?}");
    assert!(taller.height > wider.height + 100.0);
    assert!(lower.height <= 10.0);
    assert_eq!(panel.bounds(right).height, DragPanel::SIZE.height);

    let point = Point::new(100.0, 120.0);
    panel.event(iced::mouse::Event::CursorMoved { position: point }, point);
    assert_eq!(panel.bounds(first), taller);
    assert!(!panel.app.terminal.is_dragging);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread")]
async fn terminal_drag_header_docks_panes_without_capturing_terminal_selection() {
    use iced::Point;
    use iced::widget::pane_grid::Direction;

    let mut panel = DragPanel::new().await;
    let first = panel.app.terminal.panes.focused;
    let right = panel.split(first, Direction::Right);
    let bottom = panel.split(first, Direction::Down);
    panel.app.terminal.tabs[0].default_title = false;
    panel.app.terminal.tabs[0].title = "开发环境的长终端标签".repeat(15);
    let before = panel.bounds(first);
    let target = panel.bounds(right);
    let title = panel.tab_title(first);
    panel.drag(title, target.center());
    assert_eq!(panel.bounds(first), target);
    assert_eq!(panel.bounds(right), before);

    let title = panel.tab_title(first);
    panel.drag(title, Point::new(100.0, 300.0));
    let docked = panel.bounds(first);
    assert!(docked.x < panel.bounds(bottom).x);
    assert_eq!(docked.y, panel.bounds(bottom).y);

    let title = panel.tab_title(first);
    panel.drag(title, Point::new(798.0, 250.0));
    let edge = panel.bounds(first);
    assert_eq!(edge.height, DragPanel::SIZE.height);
    assert_eq!(edge.x + edge.width, DragPanel::SIZE.width);

    let title = panel.tab_title(first);
    panel.drag(title, Point::new(900.0, 350.0));
    assert_eq!(panel.bounds(first), edge);
    panel.drag(title, Point::new(799.0, 250.0));
    assert_eq!(panel.bounds(first), edge);

    // Clicking the title still activates the tab. Dragging inside the terminal selects text.
    panel.drag(title, title);
    assert_eq!(panel.app.terminal.panes.focused, first);
    assert_eq!(panel.bounds(first), edge);
    use iced::mouse::{Button, Event};
    panel.event(Event::ButtonPressed(Button::Left), Point::new(edge.x + 45.0, 54.0));
    let inside = Point::new(edge.x + 120.0, 74.0);
    panel.event(Event::CursorMoved { position: inside }, inside);
    let other_pane = Point::new(100.0, 80.0);
    panel.event(Event::CursorMoved { position: other_pane }, other_pane);
    panel.event(Event::ButtonReleased(Button::Left), inside);
    assert_eq!(panel.bounds(first), edge);
    let tab = &panel.app.terminal.tabs[0];
    assert!(tab.term.has_selection(), "terminal body should still process mouse selection");
}
