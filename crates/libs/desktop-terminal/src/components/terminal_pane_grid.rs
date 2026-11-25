use iced::advanced::Renderer as _;
use iced::advanced::widget::{Operation, Tree, operation, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::widget::pane_grid::{self, Pane, PaneGrid, Target};
use iced::widget::{Id, responsive, row};
use iced::{Border, Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector};

use super::pane_drag::{TabDrag, TabTitleHit, drop_target};
use super::{fullscreen_button, pane_view, split_menu, toolbar_icon_button, toolbar_tooltip};
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use crate::message::TerminalMessage;
use crate::panes::{PANE_SPACING, TerminalPanes};
use crate::{ViewInput, message::Message};

const DIVIDER_HIT_WIDTH: f32 = 9.0;
const DRAG_THRESHOLD: f32 = 5.0;

pub(super) fn view<'a>(app: ViewInput<'a>) -> Element<'a, Message> {
    let shortcut_focus = app.focused
            .then(|| {
                app
                    .tabs
                    .iter()
                    .find(|tab| Some(tab.id) == app.active_id && tab.edit_title.is_none())
            })
            .flatten()
            .map(|tab| tab.term.widget_id());
    shared(app.panes, shortcut_focus, Message::Terminal, move |pane, state, top_right| {
            let tabs = state
                .tabs
                .iter()
                .filter_map(|id| app.tabs.iter().find(|tab| tab.id == *id))
                .collect();
            let i18n = I18n::new(app.desktop_locale);
            let mut controls = row![toolbar_tooltip(
                toolbar_icon_button(Icon::Plus, Message::Terminal(TerminalMessage::AddToPane(pane))),
                i18n.__("新建终端"),
            )];
            if state.active_id.is_some() {
                controls = controls.push(split_menu::split_button(app, pane));
            }
            if top_right {
                controls = controls
                    .push(toolbar_tooltip(
                        toolbar_icon_button(Icon::EyeSlash, Message::Hide),
                        i18n.__("隐藏终端面板"),
                    ))
                    .push(fullscreen_button(app));
            }

            let (header, body) = pane_view(app, tabs, state.active_id, controls);
            (header, body)
    })
}

pub(super) fn shared<'a, M: Clone + 'a>(
    panes: &'a TerminalPanes,
    shortcut_focus: Option<&'a Id>,
    on_message: impl Fn(TerminalMessage) -> M + Clone + 'a,
    content: impl Fn(Pane, &crate::panes::TerminalPane, bool) -> (Element<'a, M>, Element<'a, M>) + 'a,
) -> Element<'a, M> {
    responsive(move |size| {
        let grid = &panes.grid;
        let mut spacing = PANE_SPACING;
        let mut regions = grid.layout().pane_regions(spacing, 0.0, size);
        // Iced can produce negative leaf bounds if there is less room than the gaps.
        if regions.values().any(|region| region.width < 0.0 || region.height < 0.0) {
            spacing = 0.0;
            regions = grid.layout().pane_regions(spacing, 0.0, size);
        }
        if let Some(pane) = grid.maximized() {
            regions = [(pane, Rectangle::with_size(size))].into();
        }
        let top_right = regions.iter().find_map(|(pane, region)| {
            (region.y == 0.0 && region.x + region.width >= size.width - 0.5).then_some(*pane)
        });
        let make_message = on_message.clone();
        let mut grid = PaneGrid::new(grid, |pane, state, _| {
            let (header, body) = content(pane, state, Some(pane) == top_right);
            let header = iced::widget::mouse_area(header).on_double_click(
                on_message(TerminalMessage::TogglePaneMaximized(panes.instance.clone(), pane)),
            );
            pane_grid::Content::new(body).title_bar(pane_grid::TitleBar::new(header))
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(spacing)
        .min_size(0)
        .style(super::pane_divider_style);
        if panes.grid.len() > 1 && panes.grid.maximized().is_none() {
            grid = grid.on_resize(DIVIDER_HIT_WIDTH - spacing, move |event| {
                make_message(TerminalMessage::ResizePane(panes.instance.clone(), event))
            });
        }

        Element::new(TerminalGrid {
            grid, panes, regions, shortcut_focus, on_message: Box::new(on_message.clone()),
        })
    })
    .into()
}

// Iced fills PaneGrid's pane order in diff, not state. Seed it before the first
// layout so a focus operation on a newly shown grid survives the next diff.
struct TerminalGrid<'a, M> {
    grid: PaneGrid<'a, M>,
    panes: &'a TerminalPanes,
    regions: std::collections::BTreeMap<Pane, Rectangle>,
    shortcut_focus: Option<&'a Id>,
    on_message: Box<dyn Fn(TerminalMessage) -> M + 'a>,
}

struct GridMemory {
    instance: Id,
    grid: Tree,
    drag: Option<TabDrag>,
    title_click: Option<(u64, mouse::Click)>,
}

impl<M: Clone> TerminalGrid<'_, M> {
    fn target(
        &self,
        drag: TabDrag,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) -> Option<(Target, Rectangle)> {
        let grid = &self.panes.grid;
        let source = grid.get(drag.pane)?;
        let can_split_source = source.tabs.len() > 1;
        if grid.maximized().is_some()
            || !source.tabs.contains(&drag.tab)
            || (grid.len() == 1 && !can_split_source)
        {
            return None;
        }
        let bounds = layout.bounds();
        let offset = Vector::new(bounds.x, bounds.y);
        drop_target(
            cursor.position()?,
            bounds,
            drag.pane,
            can_split_source,
            self.regions.iter().map(|(pane, region)| (*pane, *region + offset)),
        )
    }

    fn memory(&self) -> GridMemory {
        let mut grid =
            Tree { tag: self.grid.tag(), state: self.grid.state(), children: Vec::new() };
        self.grid.diff(&mut grid);
        GridMemory { instance: self.panes.instance.clone(), grid, drag: None, title_click: None }
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

impl<M: Clone> Widget<M, Theme, Renderer> for TerminalGrid<'_, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<GridMemory>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(self.memory())
    }

    fn diff(&self, tree: &mut Tree) {
        let memory = tree.state.downcast_mut::<GridMemory>();
        if memory.instance == self.panes.instance {
            self.grid.diff(&mut memory.grid);
        } else {
            *memory = self.memory();
        }
    }

    fn size(&self) -> Size<Length> {
        self.grid.size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.grid.layout(&mut tree.state.downcast_mut::<GridMemory>().grid, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.grid.operate(
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
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        use iced::{keyboard, touch, window};
        use mouse::{Button, Event as Mouse, Interaction};

        let GridMemory { grid, drag, title_click, .. } = tree.state.downcast_mut();
        if let Event::Keyboard(event @ keyboard::Event::KeyPressed { repeat, .. }) = event
            && let Some(id) = self.shortcut_focus
            && let Some(direction) = split_menu::shortcut(event)
        {
            let mut focused = FocusedWidget::default();
            self.grid.operate(grid, layout, renderer, &mut focused);
            if focused.0.as_ref() == Some(id) {
                if !repeat {
                    shell.publish((self.on_message)(TerminalMessage::Split(
                        self.panes.instance.clone(),
                        self.panes.focused,
                        direction,
                    )));
                }
                shell.capture_event();
                return;
            }
        }

        let escape = matches!(
            event,
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            })
        );
        let cancel = escape
            || matches!(
                event,
                Event::Window(window::Event::Unfocused)
                    | Event::Touch(touch::Event::FingerLost { .. })
            );
        let release = Event::Mouse(Mouse::ButtonReleased(Button::Left));
        let resizing = |interaction| {
            matches!(
                interaction,
                Interaction::ResizingHorizontally | Interaction::ResizingVertically
            )
        };
        let mut focused = FocusedWidget::default();
        let mut cursor = cursor;
        if cancel {
            *title_click = None;
        }
        if matches!(event, Event::Mouse(Mouse::ButtonPressed(button)) if *button != Button::Left) {
            *title_click = None;
        }
        if cancel
            && (drag.is_some()
                || resizing(self.grid.mouse_interaction(
                    grid,
                    layout,
                    mouse::Cursor::Unavailable,
                    viewport,
                    renderer,
                )))
        {
            *drag = None;
            self.grid.update(
                grid,
                &release,
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
        } else if matches!(event, Event::Mouse(Mouse::ButtonPressed(Button::Left))) {
            let interaction = self.grid.mouse_interaction(grid, layout, cursor, viewport, renderer);
            *drag = None;
            if !resizing(interaction)
                && let Some(origin) = cursor.position_over(layout.bounds())
            {
                let local = origin - Vector::new(layout.bounds().x, layout.bounds().y);
                if let Some((&pane, _)) = self.regions.iter().find(|(_, region)| region.contains(local))
                    && let Some(state) = self.panes.grid.get(pane)
                {
                    let mut hit = TabTitleHit::new(origin, &state.tabs);
                    self.grid.operate(grid, layout, renderer, &mut hit);
                    *drag = hit.hit.map(|tab| TabDrag { pane, tab, origin, moving: false });
                }
            }
            // Preserve the focused terminal when pressing a divider or a tab title.
            if drag.is_none() {
                *title_click = None;
            }
            if drag.is_some() || resizing(interaction) {
                self.grid.operate(grid, layout, renderer, &mut focused);
            }
        } else if let Event::Mouse(Mouse::CursorMoved { position }) = event
            && let Some(drag) = drag.as_mut()
        {
            drag.moving |= drag.origin.distance(*position) >= DRAG_THRESHOLD;
            if drag.moving {
                *title_click = None;
                shell.request_redraw();
                shell.capture_event();
                return;
            }
        } else if *event == release
            && let Some(pressed) = drag.take()
        {
            if pressed.moving {
                if let Some((target, _)) = self.target(pressed, layout, cursor) {
                    shell.publish((self.on_message)(TerminalMessage::DockTab(
                        self.panes.instance.clone(),
                        pressed.pane,
                        pressed.tab,
                        target,
                    )));
                }
                // Clear the tab button's pressed state without activating the tab.
                cursor = mouse::Cursor::Unavailable;
                shell.request_redraw();
            } else if let Some(position) = cursor.position_over(layout.bounds())
                && pressed.origin.distance(position) < DRAG_THRESHOLD
            {
                let previous =
                    title_click.filter(|(tab, _)| *tab == pressed.tab).map(|(_, click)| click);
                let click = mouse::Click::new(position, Button::Left, previous);
                if click.kind() == mouse::click::Kind::Double {
                    shell.publish((self.on_message)(TerminalMessage::TogglePaneMaximized(
                        self.panes.instance.clone(),
                        pressed.pane,
                    )));
                    *title_click = None;
                } else {
                    *title_click = Some((pressed.tab, click));
                }
            }
        }

        self.grid.update(grid, event, layout, cursor, renderer, clipboard, shell, viewport);
        if let Some(id) = focused.0 {
            self.grid.operate(grid, layout, renderer, &mut operation::focusable::focus(id));
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
        let memory = tree.state.downcast_ref::<GridMemory>();
        let drag = memory.drag.filter(|drag| drag.moving);
        self.grid.draw(
            &memory.grid,
            renderer,
            theme,
            style,
            layout,
            if drag.is_some() { mouse::Cursor::Unavailable } else { cursor },
            viewport,
        );
        if let Some(drag) = drag
            && let Some((_, bounds)) = self.target(drag, layout, cursor)
            && let Some(bounds) = bounds.intersection(viewport)
        {
            let accent = theme.extended_palette().primary.base.color;
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border::default().width(1).color(accent),
                    ..Default::default()
                },
                accent.scale_alpha(0.19),
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let memory = tree.state.downcast_ref::<GridMemory>();
        if let Some(drag) = memory.drag {
            return if drag.moving {
                mouse::Interaction::Grabbing
            } else {
                mouse::Interaction::Grab
            };
        }
        self.grid.mouse_interaction(&memory.grid, layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        self.grid.overlay(
            &mut tree.state.downcast_mut::<GridMemory>().grid,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}
