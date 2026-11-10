use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::widget::pane_grid::{self, Direction, Pane, PaneGrid};
use iced::widget::{button, column, container, responsive, row, text};
use iced::{Border, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

use super::{TAB_HEADER_HEIGHT, fullscreen_button, hide_button, pane_view, toolbar_icon_button};
use crate::app::assets::Icon;
use crate::app::components::overlays::point_below::PointBelowOverlay;
use crate::app::i18n::I18n;
use crate::app::message::TerminalMessage;
use crate::app::terminal::panes::PANE_SPACING;
use crate::app::{App, Message};

pub(super) fn view(app: &App) -> Element<'_, Message> {
    responsive(move |size| {
        let grid = &app.terminal.panes.grid;
        let regions = grid.layout().pane_regions(PANE_SPACING, 0.0, size);
        let top_right = regions.iter().find_map(|(pane, region)| {
            (region.y == 0.0 && region.x + region.width >= size.width - 0.5).then_some(*pane)
        });
        let grid = PaneGrid::new(grid, move |pane, state, _| {
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

            pane_grid::Content::new(pane_view(app, tabs, state.active_id, controls))
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(PANE_SPACING)
        .min_size(0);
        Element::new(TerminalGrid(grid))
    })
    .into()
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
struct TerminalGrid<'a>(PaneGrid<'a, Message>);

impl Widget<Message, Theme, Renderer> for TerminalGrid<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<TerminalGrid<'static>>()
    }

    fn state(&self) -> tree::State {
        let mut grid = Tree { tag: self.0.tag(), state: self.0.state(), children: Vec::new() };
        self.0.diff(&mut grid);
        tree::State::new(grid)
    }

    fn diff(&self, tree: &mut Tree) {
        self.0.diff(tree.state.downcast_mut());
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
        self.0.layout(tree.state.downcast_mut(), renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.0.operate(tree.state.downcast_mut(), layout, renderer, operation);
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
        self.0.update(
            tree.state.downcast_mut(),
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
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
        self.0.draw(tree.state.downcast_ref(), renderer, theme, style, layout, cursor, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.0.mouse_interaction(tree.state.downcast_ref(), layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.0.overlay(tree.state.downcast_mut(), layout, renderer, viewport, translation)
    }
}
