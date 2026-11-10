use iced::advanced::widget::{Operation, Tree, operation, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::widget::pane_grid::{self, Direction, Pane, PaneGrid};
use iced::widget::{Id, Space, button, column, container, responsive, row, text};
use iced::{Border, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

use super::{TAB_HEADER_HEIGHT, fullscreen_button, hide_button, pane_view, toolbar_icon_button};
use crate::app::assets::Icon;
use crate::app::components::overlays::point_below::PointBelowOverlay;
use crate::app::i18n::I18n;
use crate::app::message::TerminalMessage;
use crate::app::terminal::panes::PANE_SPACING;
use crate::app::{App, Message};

const GRIP_WIDTH: f32 = 18.0;

pub(super) fn view(app: &App) -> Element<'_, Message> {
    responsive(move |size| {
        let panes = &app.terminal.panes;
        let grid = &panes.grid;
        let can_drag = grid.len() > 1;
        let regions = grid.layout().pane_regions(PANE_SPACING, 0.0, size);
        let top_right = regions.iter().find_map(|(pane, region)| {
            (region.y == 0.0 && region.x + region.width >= size.width - 0.5).then_some(*pane)
        });
        let mut grid = PaneGrid::new(grid, move |pane, state, _| {
            let tabs = state
                .tabs
                .iter()
                .filter_map(|id| app.terminal.tabs.iter().find(|tab| tab.id == *id))
                .collect();
            let mut controls = row![toolbar_icon_button(
                Icon::Plus,
                Message::Terminal(TerminalMessage::AddToPane(pane)),
            )];
            if state.active_id.is_some() {
                controls = controls.push(split_button(app, pane, size));
            }
            if Some(pane) == top_right {
                controls =
                    controls.push(hide_button()).push(fullscreen_button(app, TAB_HEADER_HEIGHT));
            }

            let (header, body) = pane_view(app, tabs, state.active_id, controls);
            // Only the left padding is draggable. Iced excludes the toolbar,
            // including the scrollable tabs, from the title bar's pick area.
            let title = pane_grid::TitleBar::new(Space::new())
                .controls(header)
                .always_show_controls()
                .padding(iced::Padding::ZERO.left(if can_drag { GRIP_WIDTH } else { 0.0 }))
                .style(grip_style);
            pane_grid::Content::new(body).title_bar(title)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(PANE_SPACING)
        .min_size(0);
        if can_drag {
            grid = grid
                .on_resize(0, move |event| {
                    Message::Terminal(TerminalMessage::ResizePane(
                        panes.instance.clone(),
                        event,
                        size,
                    ))
                })
                .on_drag(move |event| {
                    Message::Terminal(TerminalMessage::DragPane(
                        panes.instance.clone(),
                        event,
                        size,
                    ))
                });
        }
        Element::new(TerminalGrid(grid, &panes.instance))
    })
    .into()
}

fn grip_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette().background;
    let background = palette.base.color;
    let grip = palette.strong.color;
    // The opaque toolbar covers the gradient everywhere except the grip.
    let gradient = iced::gradient::Linear::new(0.0)
        .add_stop(0.32, background)
        .add_stop(0.38, grip)
        .add_stop(0.44, background)
        .add_stop(0.50, grip)
        .add_stop(0.56, background)
        .add_stop(0.62, grip)
        .add_stop(0.68, background);
    container::Style { background: Some(gradient.into()), ..Default::default() }
}

fn split_button(app: &App, pane: Pane, size: Size) -> Element<'static, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let mut items = column![].spacing(0);
    for (direction, label) in [
        (Direction::Left, i18n.__("向左拆分")),
        (Direction::Right, i18n.__("向右拆分")),
        (Direction::Up, i18n.__("向上拆分")),
        (Direction::Down, i18n.__("向下拆分")),
    ] {
        items = items.push(
            button(text(label).size(13))
                .width(Length::Fill)
                .height(30)
                .padding([4, 8])
                .style(button::text)
                .on_press(Message::Terminal(TerminalMessage::Split(pane, direction, size))),
        );
    }
    let menu = container(items).width(140).padding(4).style(|theme: &Theme| {
        let palette = theme.extended_palette().background;
        container::Style {
            background: Some(palette.base.color.into()),
            border: Border::default().rounded(8).width(1).color(palette.strong.color),
            ..Default::default()
        }
    });
    PointBelowOverlay::new(
        toolbar_icon_button(
            Icon::Columns,
            Message::Terminal(TerminalMessage::ToggleSplitMenu(pane)),
        ),
        menu,
    )
    .show(app.terminal.panes.split_menu == Some(pane))
    .anchor(Point::new(0.0, TAB_HEADER_HEIGHT))
    .gap(2.0)
    .capture_keyboard()
    .on_close(Message::Terminal(TerminalMessage::CloseSplitMenu))
    .into()
}

// Iced fills PaneGrid's pane order in diff, not state. Seed it before the first
// layout so a focus operation on a newly shown grid survives the next diff.
struct TerminalGrid<'a>(PaneGrid<'a, Message>, &'a Id);

struct GridMemory {
    instance: Id,
    grid: Tree,
}

impl TerminalGrid<'_> {
    fn memory(&self) -> GridMemory {
        let mut grid = Tree { tag: self.0.tag(), state: self.0.state(), children: Vec::new() };
        self.0.diff(&mut grid);
        GridMemory { instance: self.1.clone(), grid }
    }
}

#[derive(Default)]
struct FocusedWidget(Option<Id>);

impl Operation for FocusedWidget {
    fn focusable(&mut self, id: Option<&Id>, _: Rectangle, state: &mut dyn operation::Focusable) {
        if state.is_focused() && id.is_some() {
            self.0 = id.cloned();
        }
    }

    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }
}

impl Widget<Message, Theme, Renderer> for TerminalGrid<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<TerminalGrid<'static>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(self.memory())
    }

    fn diff(&self, tree: &mut Tree) {
        let memory = tree.state.downcast_mut::<GridMemory>();
        if memory.instance == *self.1 {
            self.0.diff(&mut memory.grid);
        } else {
            *memory = self.memory();
        }
    }

    fn size(&self) -> Size<Length> {
        self.0.size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.0.layout(&mut tree.state.downcast_mut::<GridMemory>().grid, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.0.operate(
            &mut tree.state.downcast_mut::<GridMemory>().grid,
            layout,
            renderer,
            operation,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        use iced::{keyboard, touch, window};
        use mouse::{Button, Event as Mouse, Interaction};

        let grid = &mut tree.state.downcast_mut::<GridMemory>().grid;
        let mut focused = FocusedWidget::default();
        if matches!(event, Event::Mouse(Mouse::ButtonPressed(Button::Left)))
            && matches!(
                self.0.mouse_interaction(grid, layout, cursor, viewport, renderer),
                Interaction::Grab
                    | Interaction::ResizingHorizontally
                    | Interaction::ResizingVertically
            )
        {
            // Terminal views normally lose focus on a press outside their body.
            // Keep the actual focused widget when the press starts a grid gesture.
            self.0.operate(grid, layout, renderer, &mut focused);
        }

        let escape = matches!(
            event,
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            })
        );
        if (escape
            || matches!(
                event,
                Event::Window(window::Event::Unfocused)
                    | Event::Touch(touch::Event::FingerLost { .. })
            ))
            && matches!(
                self.0.mouse_interaction(
                    grid,
                    layout,
                    mouse::Cursor::Unavailable,
                    viewport,
                    renderer
                ),
                Interaction::Grabbing
                    | Interaction::ResizingHorizontally
                    | Interaction::ResizingVertically
            )
        {
            self.0.update(
                grid,
                &Event::Mouse(Mouse::ButtonReleased(Button::Left)),
                layout,
                mouse::Cursor::Unavailable,
                renderer,
                clipboard,
                shell,
                viewport,
            );
            shell.request_redraw();
            if escape {
                shell.capture_event();
                return;
            }
        }

        // Iced's outer-edge hit test accepts a cursor outside the grid on
        // the perpendicular axis. Only an in-panel release can dock a pane.
        let cursor = if matches!(
            event,
            Event::Mouse(Mouse::ButtonReleased(Button::Left))
                | Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. })
        ) && !cursor.is_over(layout.bounds())
        {
            mouse::Cursor::Unavailable
        } else {
            cursor
        };
        self.0.update(grid, event, layout, cursor, renderer, clipboard, shell, viewport);
        if let Some(id) = focused.0 {
            self.0.operate(grid, layout, renderer, &mut operation::focusable::focus(id));
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.0.draw(
            &tree.state.downcast_ref::<GridMemory>().grid,
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.0.mouse_interaction(
            &tree.state.downcast_ref::<GridMemory>().grid,
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.0.overlay(
            &mut tree.state.downcast_mut::<GridMemory>().grid,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}
