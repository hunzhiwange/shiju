use crate::backend::{
    Backend, Command, LinkAction, MouseButton, RenderableContent,
};
use crate::bindings::{BindingAction, InputKind};
use crate::terminal::{Event, Terminal};
use crate::theme::TerminalStyle;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::Point as TerminalGridPoint;
use alacritty_terminal::selection::SelectionType;
use alacritty_terminal::term::{cell, TermMode};
use alacritty_terminal::vte::ansi::{self as ansi, NamedColor};
use iced::advanced::renderer::{Quad, Renderer as _};
use iced::alignment::Vertical;
use iced::font::{Style as FontStyle, Weight as FontWeight};
use iced::mouse::{Cursor, ScrollDelta};
use iced::widget::canvas::{Path, Text};
use iced::widget::container;
use iced::{Border, Color, Element, Length, Point, Rectangle, Size, Theme, Vector};
use iced_core::clipboard::Kind as ClipboardKind;
use iced_core::input_method::{self, InputMethod, Preedit, Purpose};
use iced::time::{Duration, Instant};
use iced_core::keyboard::{key, Event as KeyEvent, Key, Modifiers};
use iced_core::mouse::{self, Click};
use iced_core::text::{Alignment, LineHeight, Shaping};
use iced_core::widget::operation::{self, Focusable};
use iced_graphics::core::widget::{tree, Tree};
use iced_graphics::core::Widget;
use iced_graphics::geometry::Stroke;

const REPEAT_DELAY: Duration = Duration::from_millis(250);
const REPEAT_INTERVAL: Duration = Duration::from_millis(20);
const SELECTION_SCROLL_INTERVAL: Duration = Duration::from_millis(40);
const LINK_CLICK_TOLERANCE: f32 = 4.0;
const SCROLLBAR_WIDTH: f32 = 4.0;
const SCROLLBAR_GUTTER: f32 = SCROLLBAR_WIDTH + 3.0;

fn text_bounds(mut bounds: Rectangle) -> Rectangle {
    bounds.width = (bounds.width - SCROLLBAR_GUTTER).max(0.0);
    bounds
}

struct Scrollbar {
    track: Rectangle,
    thumb: Rectangle,
    history: usize,
}

impl Scrollbar {
    fn new(bounds: Rectangle, content: &RenderableContent) -> Option<Self> {
        let grid = &content.grid;
        let history = grid.history_size();
        if history == 0 || bounds.width <= SCROLLBAR_GUTTER || bounds.height < 2.0 {
            return None;
        }
        let track = Rectangle {
            x: bounds.x + bounds.width - SCROLLBAR_WIDTH,
            width: SCROLLBAR_WIDTH,
            ..bounds
        };
        let thumb_height = (track.height * grid.screen_lines() as f32 / grid.total_lines() as f32)
            .max(20.0_f32.min(track.height / 2.0));
        let travel = track.height - thumb_height;
        let thumb = Rectangle {
            y: track.y + travel * (1.0 - grid.display_offset() as f32 / history as f32),
            height: thumb_height,
            ..track
        };
        Some(Self { track, thumb, history })
    }

    fn offset_at(&self, y: f32, grab: f32) -> usize {
        let progress = (y - grab - self.track.y) / (self.track.height - self.thumb.height);
        ((1.0 - progress.clamp(0.0, 1.0)) * self.history as f32).round() as usize
    }

    fn hit_bounds(&self) -> Rectangle {
        Rectangle {
            x: self.track.x - (SCROLLBAR_GUTTER - SCROLLBAR_WIDTH),
            width: SCROLLBAR_GUTTER,
            ..self.track
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SelectionScroll {
    position: Point,
    next_tick: Instant,
}

#[derive(Debug, Clone)]
struct HeldKey {
    physical_key: key::Physical,
    bytes: Vec<u8>,
    next_repeat: Instant,
}

pub struct TerminalView<'a> {
    term: &'a Terminal,
}

impl<'a> TerminalView<'a> {
    pub fn show(term: &'a Terminal) -> Element<'a, Event> {
        container(Self { term })
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| term.theme.container_style())
            .into()
    }

    pub fn focus<Message: 'static>(
        id: iced::widget::Id,
    ) -> iced::Task<Message> {
        iced::widget::operation::focus(id)
    }

    fn is_cursor_hovered_hyperlink(&self, state: &TerminalViewState) -> bool {
        let content = self.term.backend.renderable_content();
        if let Some(hyperlink_range) = &content.hovered_hyperlink {
            return hyperlink_range.contains(&state.mouse_position_on_grid);
        }

        false
    }

    fn handle_resize(
        &mut self,
        state: &mut TerminalViewState,
        layout_size: Size,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
    ) {
        if state.size != layout_size {
            state.size = layout_size;
            let cmd = Command::Resize(
                Some(layout_size),
                Some(self.term.font.measure),
            );
            shell.publish(Event::BackendCall(self.term.id, cmd));
        }
    }

    fn handle_scrollbar(
        &self,
        state: &mut TerminalViewState,
        bounds: Rectangle,
        cursor: Option<Point>,
        event: &mouse::Event,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
    ) -> bool {
        use mouse::{Button::Left, Event::*};

        let content = self.term.backend.renderable_content();
        let Some(bar) = Scrollbar::new(bounds, content) else {
            state.scrollbar_grab = None;
            return false;
        };
        let offset = match event {
            ButtonPressed(Left) => {
                let Some(position) = cursor.filter(|point| bar.hit_bounds().contains(*point)) else {
                    return false;
                };
                let grab = if position.y >= bar.thumb.y
                    && position.y < bar.thumb.y + bar.thumb.height
                {
                    position.y - bar.thumb.y
                } else {
                    bar.thumb.height / 2.0
                };
                state.cancel_drag();
                state.scrollbar_grab = Some(grab);
                Some(bar.offset_at(position.y, grab))
            },
            CursorMoved { position } if state.scrollbar_grab.is_some() => {
                state.scrollbar_grab.map(|grab| bar.offset_at(position.y, grab))
            },
            ButtonReleased(Left) if state.scrollbar_grab.take().is_some() => None,
            _ => return false,
        };
        if let Some(offset) = offset.filter(|&offset| offset != content.grid.display_offset()) {
            shell.publish(Event::BackendCall(self.term.id, Command::ScrollTo(offset)));
        }
        shell.capture_event();
        shell.request_redraw();
        true
    }

    fn handle_focus(
        &self,
        event: &iced_core::Event,
        state: &mut TerminalViewState,
        is_cursor_in_layout: bool,
    ) {
        use iced::Event::Mouse;
        use iced_core::mouse::{Button::Left, Event::ButtonPressed};

        if let Mouse(ButtonPressed(Left)) = event {
            if is_cursor_in_layout {
                state.focus();
            } else {
                state.unfocus();
            }
        }
    }

    fn input_method<'state>(
        &self,
        state: &'state TerminalViewState,
        bounds: Rectangle,
    ) -> InputMethod<&'state str> {
        let content = self.term.backend.renderable_content();
        let point = content.grid.cursor.point;
        let width =
            f32::from(content.terminal_size.cell_width).min(bounds.width);
        let height =
            f32::from(content.terminal_size.cell_height).min(bounds.height);
        let x = (point.column.0 as f32 * width).min(bounds.width - width);
        let row = point.line.0 as f32 + content.grid.display_offset() as f32;
        let y = (row * height).clamp(0.0, bounds.height - height);

        InputMethod::Enabled {
            cursor: Rectangle::new(
                Point::new(bounds.x + x, bounds.y + y),
                Size::new(width, height),
            ),
            purpose: Purpose::Terminal,
            preedit: state.preedit.as_ref().map(Preedit::as_ref),
        }
    }

    fn handle_mouse_event(
        &self,
        state: &mut TerminalViewState,
        bounds: Rectangle,
        cursor_in_layout: Option<Point>,
        event: &iced::mouse::Event,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
    ) -> Vec<Command> {
        use iced_core::mouse::{Button::Left, Event::*};

        let mut commands = Vec::new();
        let content = self.term.backend.renderable_content();
        let terminal_mode = content.terminal_mode;
        let selecting = state.drag == Some(Drag::Selection);

        match event {
            ButtonPressed(Left) if state.is_focused() => {
                if let Some(position) = cursor_in_layout {
                    state.link_press = (text_bounds(bounds).contains(position)
                        && self.link_binding(state, terminal_mode))
                    .then_some(position);
                    Self::handle_left_button_pressed(
                        state,
                        &terminal_mode,
                        position,
                        bounds.position(),
                        &mut commands,
                    );
                }
            },
            CursorMoved { position } if state.is_focused() => {
                if state.link_press.is_some_and(|press| {
                    press.distance(*position) > LINK_CLICK_TOLERANCE
                }) {
                    state.link_press = None;
                }
                if cursor_in_layout.is_some() && bounds.contains(*position) {
                    Self::handle_cursor_moved(
                        state,
                        content,
                        position,
                        bounds.position(),
                        &mut commands,
                    );
                } else if selecting {
                    commands.push(Command::SelectUpdate((
                        position.x - bounds.x,
                        position.y - bounds.y,
                    )));
                }

                if selecting
                    && self
                        .selection_scroll_command(bounds, *position)
                        .is_some()
                {
                    // Keep the deadline when further motion arrives at the edge.
                    let next_tick = state.selection_scroll.map_or_else(
                        || Instant::now() + SELECTION_SCROLL_INTERVAL,
                        |scroll| scroll.next_tick,
                    );
                    state.selection_scroll = Some(SelectionScroll {
                        position: *position,
                        next_tick,
                    });
                    shell.request_redraw_at(next_tick);
                } else {
                    state.selection_scroll = None;
                }
            },
            ButtonReleased(Left) => {
                if let Some(position) = cursor_in_layout.filter(|_| state.is_focused()) {
                    let open_link = text_bounds(bounds).contains(position)
                        && self.link_binding(state, terminal_mode)
                        && state.link_press.is_some_and(|press| {
                            press.distance(position) <= LINK_CLICK_TOLERANCE
                        });
                    if open_link {
                        state.mouse_position_on_grid = Backend::selection_point(
                            position.x - bounds.x,
                            position.y - bounds.y,
                            &content.terminal_size,
                            content.grid.display_offset(),
                        );
                    }
                    Self::handle_button_released(
                        state,
                        &terminal_mode,
                        open_link,
                        &mut commands,
                    );
                } else {
                    state.cancel_drag();
                    if selecting {
                        shell.capture_event();
                    }
                }
            },
            WheelScrolled { delta } => {
                if let Some(position) = cursor_in_layout {
                    state.link_press = None;
                    let height = f32::from(content.terminal_size.cell_height).max(1.0);
                    if let Some(lines) = state.wheel_lines(*delta, height) {
                        shell.capture_event();
                        if lines != 0 {
                            commands.push(Command::WheelScroll {
                                lines,
                                modifiers: state.keyboard_modifiers,
                                position: (position.x - bounds.x, position.y - bounds.y),
                            });
                        }
                    }
                }
            },
            _ => {},
        }

        commands
    }

    fn link_binding(
        &self,
        state: &TerminalViewState,
        terminal_mode: TermMode,
    ) -> bool {
        self.term.bindings.get_action(
            InputKind::Mouse(mouse::Button::Left),
            state.keyboard_modifiers,
            terminal_mode,
        ) == BindingAction::LinkOpen
    }

    fn selection_scroll_command(
        &self,
        bounds: Rectangle,
        position: Point,
    ) -> Option<Command> {
        let content = self.term.backend.renderable_content();
        if bounds.height <= 0.0 {
            return None;
        }

        let x = position.x - bounds.x;
        let y = position.y - bounds.y;
        let edge = (f32::from(content.terminal_size.cell_height) * 0.5)
            .min(bounds.height * 0.25);
        let offset = content.grid.display_offset();

        if y <= edge && offset < content.grid.history_size() {
            Some(Command::SelectScroll(1, (x, 0.0)))
        } else if y >= bounds.height - edge && offset > 0 {
            Some(Command::SelectScroll(-1, (x, bounds.height)))
        } else {
            None
        }
    }

    fn handle_left_button_pressed(
        state: &mut TerminalViewState,
        terminal_mode: &TermMode,
        cursor_position: Point,
        layout_position: Point,
        commands: &mut Vec<Command>,
    ) {
        // Shift-drag selects emulator text even when the TUI handles the mouse.
        let cmd = if terminal_mode.intersects(TermMode::MOUSE_MODE)
            && !state.keyboard_modifiers.shift()
        {
            state.drag = Some(Drag::Mouse);
            Command::MouseReport(
                MouseButton::LeftButton,
                state.keyboard_modifiers,
                state.mouse_position_on_grid,
                true,
            )
        } else {
            state.drag = Some(Drag::Selection);
            let current_click = Click::new(
                cursor_position,
                mouse::Button::Left,
                state.last_click,
            );
            let selection_type = match current_click.kind() {
                mouse::click::Kind::Single => SelectionType::Simple,
                mouse::click::Kind::Double => SelectionType::Semantic,
                mouse::click::Kind::Triple => SelectionType::Lines,
            };
            state.last_click = Some(current_click);
            Command::SelectStart(
                selection_type,
                (
                    cursor_position.x - layout_position.x,
                    cursor_position.y - layout_position.y,
                ),
            )
        };
        commands.push(cmd);
        state.selection_scroll = None;
    }

    fn handle_cursor_moved(
        state: &mut TerminalViewState,
        terminal_content: &RenderableContent,
        position: &Point,
        layout_position: Point,
        commands: &mut Vec<Command>,
    ) {
        let cursor_x = position.x - layout_position.x;
        let cursor_y = position.y - layout_position.y;
        state.mouse_position_on_grid = Backend::selection_point(
            cursor_x,
            cursor_y,
            &terminal_content.terminal_size,
            terminal_content.grid.display_offset(),
        );

        // Keep each gesture with the receiver of the button press.
        match state.drag {
            Some(Drag::Mouse) if terminal_content.terminal_mode
                .intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION) =>
            {
                commands.push(Command::MouseReport(
                    MouseButton::LeftMove,
                    state.keyboard_modifiers,
                    state.mouse_position_on_grid,
                    true,
                ));
            },
            Some(Drag::Selection) => commands.push(Command::SelectUpdate((cursor_x, cursor_y))),
            _ => {},
        }

        // Handle link hover if applicable
        if state.keyboard_modifiers == Modifiers::COMMAND {
            commands.push(Command::ProcessLink(
                LinkAction::Hover,
                state.mouse_position_on_grid,
            ));
        }
    }

    fn handle_button_released(
        state: &mut TerminalViewState,
        terminal_mode: &TermMode,
        open_link: bool,
        commands: &mut Vec<Command>,
    ) {
        if state.drag == Some(Drag::Mouse) && terminal_mode.intersects(TermMode::MOUSE_MODE) {
            commands.push(Command::MouseReport(
                MouseButton::LeftButton,
                state.keyboard_modifiers,
                state.mouse_position_on_grid,
                false,
            ));
        }
        state.cancel_drag();

        if open_link {
            commands.push(Command::ProcessLink(
                LinkAction::Open,
                state.mouse_position_on_grid,
            ));
        }
    }

    fn handle_keyboard_event(
        &self,
        state: &mut TerminalViewState,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
        event: &iced::keyboard::Event,
    ) -> Option<Command> {
        let mut binding_action = BindingAction::Ignore;
        let last_content = self.term.backend.renderable_content();
        match event {
            iced::keyboard::Event::ModifiersChanged(m) => {
                state.keyboard_modifiers = *m;
                let action = if state.keyboard_modifiers == Modifiers::COMMAND {
                    LinkAction::Hover
                } else {
                    LinkAction::Clear
                };
                return Some(Command::ProcessLink(
                    action,
                    state.mouse_position_on_grid,
                ));
            },
            iced::keyboard::Event::KeyPressed {
                key,
                modifiers,
                text,
                ..
            } => {
                binding_action = self.term.bindings.key_action(
                    key,
                    *modifiers,
                    text.as_deref(),
                    last_content.terminal_mode,
                );
            },
            _ => {},
        }

        match binding_action {
            BindingAction::Char(c) => {
                let mut buf = [0, 0, 0, 0];
                let str = c.encode_utf8(&mut buf);
                return Some(Command::Write(str.as_bytes().to_vec()));
            },
            BindingAction::Esc(seq) => {
                return Some(Command::Write(seq.as_bytes().to_vec()));
            },
            BindingAction::Paste => {
                shell.capture_event();
                return clipboard
                    .read(ClipboardKind::Standard)
                    .map(Command::Paste);
            },
            BindingAction::Copy => {
                shell.capture_event();
                let text = self.term.selected_text();
                if !text.is_empty() {
                    clipboard.write(ClipboardKind::Standard, text);
                }
            },
            BindingAction::SelectAll => return Some(Command::SelectAll),
            BindingAction::Clear => return Some(Command::Clear),
            _ => {},
        };

        None
    }
}

impl Widget<Event, Theme, iced::Renderer> for TerminalView<'_> {
    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<TerminalViewState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(TerminalViewState::new(self.term.widget_id().clone()))
    }

    fn diff(&self, tree: &mut Tree) {
        let state = tree.state.downcast_mut::<TerminalViewState>();
        if &state.widget_id != self.term.widget_id() {
            tree.state = self.state();
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &iced_core::layout::Limits,
    ) -> iced_core::layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        iced::advanced::layout::Node::new(size)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: iced_core::Layout<'_>,
        _renderer: &iced::Renderer,
        operation: &mut dyn operation::Operation,
    ) {
        let state = tree.state.downcast_mut::<TerminalViewState>();
        let wid = self.term.widget_id();
        operation.focusable(Some(wid), layout.bounds(), state);
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<TerminalViewState>();
        let content = self.term.backend.renderable_content();
        let bounds = layout.bounds();
        let text_area = text_bounds(bounds);
        let Some(clip) = text_area.intersection(viewport) else { return };
        let term_size = content.terminal_size;
        let cell_width = term_size.cell_width as f32;
        let cell_height = term_size.cell_height as f32;
        let font_size = self.term.font.size;
        let font_scale_factor = self.term.font.scale_factor;
        let geom = self.term.cache.draw(renderer, text_area.size(), |frame| {
            // Precompute constants used in the inner loop
            let display_offset = content.grid.display_offset() as f32;
            let cell_size = Size::new(cell_width, cell_height);
            let half_w = cell_width * 0.5;
            let half_h = cell_height * 0.5;
            // We use the background pallete color as a default
            // because the widget global background color must be the same
            let default_bg = self
                .term
                .theme
                .get_color(ansi::Color::Named(NamedColor::Background));

            let mut last_line: Option<i32> = None;
            let mut bg_batch_rect = BackgroundRect::default();

            for indexed in content.grid.display_iter() {
                // Compute per-cell geometry cheaply
                let line = indexed.point.line.0;
                let col = indexed.point.column.0 as f32;

                // Resolve position point for this cell
                let x = col * cell_width;
                let y = (line as f32 + display_offset) * cell_height;
                let cell_center_y = y + half_h;
                let cell_center_x = x + half_w;

                // Resolve colors for this cell
                let mut fg = self.term.theme.get_color(indexed.fg);
                let mut bg = self.term.theme.get_color(indexed.bg);
                // Pre-swap background: the block cursor is painted in the
                // cell's (pre-swap) fg, so this is the contrasting color
                // for the glyph under it regardless of INVERSE/selection.
                let cell_bg = bg;

                // If the new line was detected,
                // need to flush pending background rect and init the new one
                if last_line != Some(line) {
                    if bg_batch_rect.can_flush() {
                        let line = last_line.unwrap_or(line);
                        frame.fill(
                            &bg_batch_rect.build(line),
                            bg_batch_rect.color,
                        );
                    }

                    last_line = Some(line);
                    bg_batch_rect = BackgroundRect::default()
                        .with_cell_height(cell_height)
                        .with_display_offset(display_offset);
                }

                // Handle dim, inverse, and selected text
                if indexed
                    .cell
                    .flags
                    .intersects(cell::Flags::DIM | cell::Flags::DIM_BOLD)
                {
                    fg.a *= 0.7;
                }
                if indexed.cell.flags.contains(cell::Flags::INVERSE)
                    || content
                        .selectable_range
                        .is_some_and(|r| r.contains(indexed.point))
                {
                    std::mem::swap(&mut fg, &mut bg);
                }

                // Batch draw backgrounds: skip default background (container already paints it)
                if bg != default_bg {
                    if bg_batch_rect.can_extend(bg, x) {
                        // Same color and contiguous: extend current run
                        bg_batch_rect.extend(cell_width);
                    } else {
                        // New colored run (or non-contiguous): flush previous run if any
                        if bg_batch_rect.can_flush() {
                            frame.fill(
                                &bg_batch_rect.build(line),
                                bg_batch_rect.color,
                            );
                        }

                        // Start a new run but do not draw yet; wait for potential extensions
                        bg_batch_rect = BackgroundRect::default()
                            .with_cell_height(cell_height)
                            .with_display_offset(display_offset)
                            .activate()
                            .with_color(bg)
                            .with_start_x(x)
                            .with_width(cell_width);
                    }
                } else if bg_batch_rect.can_flush() {
                    // Background returns to default, flush current background rect and init the new one
                    frame.fill(&bg_batch_rect.build(line), bg_batch_rect.color);

                    bg_batch_rect = BackgroundRect::default()
                        .with_cell_height(cell_height)
                        .with_display_offset(display_offset);
                }

                // Draw hovered hyperlink underline (rare; keep per-cell for correctness)
                if content.hovered_hyperlink.as_ref().is_some_and(|range| {
                    range.contains(&indexed.point)
                        && range.contains(&state.mouse_position_on_grid)
                }) || indexed.cell.flags.contains(cell::Flags::UNDERLINE)
                {
                    let underline_height = y + cell_size.height;
                    let underline = Path::line(
                        Point::new(x, underline_height),
                        Point::new(x + cell_size.width, underline_height),
                    );
                    frame.stroke(
                        &underline,
                        Stroke::default()
                            .with_width(font_size * 0.15)
                            .with_color(fg),
                    );
                }

                // Handle cursor rendering
                if content.grid.cursor.point == indexed.point
                    && content.terminal_mode.contains(TermMode::SHOW_CURSOR)
                {
                    let cursor_color =
                        self.term.theme.get_color(content.cursor.fg);
                    let cursor_rect =
                        Path::rectangle(Point::new(x, y), cell_size);
                    frame.fill(&cursor_rect, cursor_color);
                }

                // Draw text
                if indexed.c != ' ' && indexed.c != '\t' {
                    // The glyph under the block cursor must contrast with
                    // the cursor rect (painted above in the cell's pre-swap
                    // fg). Using the post-swap bg — or gating this on
                    // APP_CURSOR, a keypad mode unrelated to rendering —
                    // made the glyph invisible whenever the cell was
                    // INVERSE or inside a selection.
                    if content.grid.cursor.point == indexed.point
                        && content.terminal_mode.contains(TermMode::SHOW_CURSOR)
                    {
                        fg = cell_bg;
                    }
                    // Resolve font style (bold/italic) from cell flags
                    let mut font = self.term.font.font_type;
                    if indexed
                        .cell
                        .flags
                        .intersects(cell::Flags::BOLD | cell::Flags::DIM_BOLD)
                    {
                        font.weight = FontWeight::Bold;
                    }
                    if indexed.cell.flags.contains(cell::Flags::ITALIC) {
                        font.style = FontStyle::Italic;
                    }
                    let text = Text {
                        content: indexed.cell.c.to_string(),
                        position: Point::new(cell_center_x, cell_center_y),
                        font,
                        size: iced_core::Pixels(font_size),
                        color: fg,
                        align_x: Alignment::Center,
                        align_y: Vertical::Center,
                        shaping: Shaping::Advanced,
                        line_height: LineHeight::Relative(font_scale_factor),
                        ..Default::default()
                    };
                    frame.fill_text(text);
                }
            }

            // Flush any remaining background run at the end
            if bg_batch_rect.can_flush() {
                frame.fill(
                    &bg_batch_rect.build(last_line.unwrap_or(0)),
                    bg_batch_rect.color,
                );
            }
        });

        use iced::advanced::graphics::geometry::Renderer as _;
        // Cache pane-local coordinates so a moved pane stays inside its clip.
        renderer.with_layer(clip, |renderer| {
            renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
                renderer.draw_geometry(geom);
            });
        });
        if let Some(bar) = Scrollbar::new(bounds, content) {
            let foreground = self.term.theme.get_color(ansi::Color::Named(NamedColor::Foreground));
            let highlighted = state.scrollbar_grab.is_some() || cursor.is_over(bar.hit_bounds());
            renderer.with_layer(*viewport, |renderer| {
                renderer.fill_quad(
                    Quad { bounds: bar.track, border: Border::default().rounded(2), ..Quad::default() },
                    foreground.scale_alpha(0.12),
                );
                renderer.fill_quad(
                    Quad { bounds: bar.thumb, border: Border::default().rounded(2), ..Quad::default() },
                    foreground.scale_alpha(if highlighted { 0.75 } else { 0.5 }),
                );
            });
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced_core::Event,
        layout: iced_graphics::core::Layout<'_>,
        cursor: Cursor,
        _renderer: &iced::Renderer,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        shell: &mut iced_graphics::core::Shell<'_, Event>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<TerminalViewState>();
        let bounds = layout.bounds();
        self.handle_resize(state, text_bounds(bounds).size(), shell);
        let cursor_in_layout = bounds.intersection(viewport).and_then(|area| cursor.position_over(area));
        if let iced::Event::Mouse(event) = event {
            if cursor.is_levitating() {
                state.scrollbar_grab = None;
            }
            if self.handle_scrollbar(state, bounds, cursor_in_layout, event, shell) {
                return;
            }
        }
        self.handle_focus(event, state, cursor_in_layout.is_some());

        let commands = match event {
            iced::Event::Mouse(mouse_event) => self.handle_mouse_event(
                state,
                bounds,
                cursor_in_layout,
                mouse_event,
                shell,
            ),
            iced::Event::InputMethod(event)
                if state.is_focused() && state.window_focused =>
            {
                state.held_key = None;
                match event {
                    input_method::Event::Opened
                    | input_method::Event::Closed => {
                        state.preedit = None;
                    },
                    input_method::Event::Preedit(content, selection) => {
                        state.preedit =
                            (!content.is_empty()).then(|| Preedit {
                                content: content.clone(),
                                selection: selection.clone().filter(|range| {
                                    content.get(range.clone()).is_some()
                                }),
                                text_size: Some(iced::Pixels(
                                    self.term.font.size,
                                )),
                            });
                    },
                    input_method::Event::Commit(text) => {
                        state.preedit = None;
                        if !text.is_empty() {
                            shell.publish(Event::BackendCall(
                                self.term.id,
                                Command::Write(text.as_bytes().to_vec()),
                            ));
                        }
                    },
                }
                shell.request_redraw();
                shell.capture_event();
                return;
            },
            iced::Event::Window(iced::window::Event::Unfocused) => {
                state.window_focused = false;
                state.cancel_drag();
                state.held_key = None;
                state.preedit = None;
                shell.request_redraw();
                return;
            },
            iced::Event::Window(iced::window::Event::Focused) => {
                state.window_focused = true;
                shell.request_redraw();
                return;
            },
            iced::Event::Window(iced::window::Event::RedrawRequested(now))
                if state.is_focused() && state.window_focused =>
            {
                if let Some(held) = &mut state.held_key {
                    if *now >= held.next_repeat {
                        shell.publish(Event::BackendCall(
                            self.term.id,
                            Command::Write(held.bytes.clone()),
                        ));
                        held.next_repeat = *now + REPEAT_INTERVAL;
                    }
                    shell.request_redraw_at(held.next_repeat);
                }
                if let Some(scroll) = &mut state.selection_scroll {
                    if let Some(command) =
                        self.selection_scroll_command(bounds, scroll.position)
                    {
                        if *now >= scroll.next_tick {
                            shell.publish(Event::BackendCall(
                                self.term.id,
                                command,
                            ));
                            scroll.next_tick = *now + SELECTION_SCROLL_INTERVAL;
                        }
                        shell.request_redraw_at(scroll.next_tick);
                    } else {
                        state.selection_scroll = None;
                    }
                }
                shell.request_input_method(
                    &self.input_method(state, layout.bounds()),
                );
                return;
            },
            iced::Event::Keyboard(keyboard_event) => {
                if !state.is_focused() {
                    if let KeyEvent::ModifiersChanged(modifiers) = keyboard_event {
                        state.keyboard_modifiers = *modifiers;
                    }
                    return;
                }

                match keyboard_event {
                    KeyEvent::KeyReleased { physical_key, .. }
                        if state.held_key.as_ref().is_some_and(|held| {
                            &held.physical_key == physical_key
                        }) =>
                    {
                        state.held_key = None;
                    },
                    KeyEvent::ModifiersChanged(_) | KeyEvent::KeyPressed { repeat: false, .. } => {
                        state.held_key = None;
                    },
                    KeyEvent::KeyPressed { physical_key, repeat: true, .. }
                        if state.held_key.as_ref().is_some_and(|held| {
                            &held.physical_key == physical_key
                        }) =>
                    {
                        shell.capture_event();
                        return;
                    },
                    _ => {},
                }

                let command =
                    self.handle_keyboard_event(state, clipboard, shell, keyboard_event);
                if let (
                    KeyEvent::KeyPressed {
                        key: Key::Named(
                            key::Named::Backspace | key::Named::Delete
                            | key::Named::ArrowLeft | key::Named::ArrowRight
                            | key::Named::ArrowUp | key::Named::ArrowDown,
                        ),
                        physical_key,
                        modifiers,
                        repeat: false,
                        ..
                    },
                    Some(Command::Write(bytes)),
                ) = (keyboard_event, &command)
                {
                    if modifiers.is_empty() && state.window_focused && state.preedit.is_none() {
                        let next_repeat = Instant::now() + REPEAT_DELAY;
                        state.held_key = Some(HeldKey {
                            physical_key: *physical_key,
                            bytes: bytes.clone(),
                            next_repeat,
                        });
                        shell.request_redraw_at(next_repeat);
                    }
                }
                command.into_iter().collect()
            },
            _ => Vec::new(),
        };

        if !commands.is_empty()
            && !matches!(event, iced::Event::Keyboard(KeyEvent::ModifiersChanged(_)))
        {
            shell.capture_event();
        }

        for cmd in commands {
            shell.publish(Event::BackendCall(self.term.id, cmd));
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: iced_core::Layout<'_>,
        cursor: iced_core::mouse::Cursor,
        viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> iced_core::mouse::Interaction {
        let state = tree.state.downcast_ref::<TerminalViewState>();
        let bounds = layout.bounds();
        let content = self.term.backend.renderable_content();
        if state.scrollbar_grab.is_some() {
            return mouse::Interaction::Grabbing;
        }
        if cursor.is_over(*viewport)
            && Scrollbar::new(bounds, content).is_some_and(|bar| cursor.is_over(bar.hit_bounds()))
        {
            return mouse::Interaction::Grab;
        }
        let mut cursor_mode = iced_core::mouse::Interaction::Idle;
        if cursor.is_over(text_bounds(bounds))
            && !content.terminal_mode.contains(TermMode::SGR_MOUSE)
        {
            cursor_mode = iced_core::mouse::Interaction::Text;
        }

        if self.is_cursor_hovered_hyperlink(state) {
            cursor_mode = iced_core::mouse::Interaction::Pointer;
        }

        cursor_mode
    }
}

impl<'a> From<TerminalView<'a>> for Element<'a, Event, Theme, iced::Renderer> {
    fn from(widget: TerminalView<'a>) -> Self {
        Self::new(widget)
    }
}

#[derive(Debug, Clone)]
struct TerminalViewState {
    focus: bool,
    drag: Option<Drag>,
    selection_scroll: Option<SelectionScroll>,
    scrollbar_grab: Option<f32>,
    link_press: Option<Point>,
    last_click: Option<mouse::Click>,
    scroll_lines: f32,
    keyboard_modifiers: Modifiers,
    size: Size<f32>,
    mouse_position_on_grid: TerminalGridPoint,
    widget_id: iced::widget::Id,
    window_focused: bool,
    preedit: Option<Preedit>,
    held_key: Option<HeldKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Drag {
    Selection,
    Mouse,
}

impl TerminalViewState {
    fn new(widget_id: iced::widget::Id) -> Self {
        Self {
            focus: false,
            drag: None,
            selection_scroll: None,
            scrollbar_grab: None,
            link_press: None,
            last_click: None,
            scroll_lines: 0.0,
            keyboard_modifiers: Modifiers::empty(),
            size: Size::from([0.0, 0.0]),
            mouse_position_on_grid: TerminalGridPoint::default(),
            widget_id,
            window_focused: true,
            preedit: None,
            held_key: None,
        }
    }

    fn cancel_drag(&mut self) {
        self.drag = None;
        self.selection_scroll = None;
        self.scrollbar_grab = None;
        self.link_press = None;
    }

    fn wheel_lines(&mut self, delta: ScrollDelta, cell_height: f32) -> Option<i32> {
        let delta = match delta {
            ScrollDelta::Lines { y, .. } => y,
            ScrollDelta::Pixels { y, .. } => y / cell_height,
        };
        if !delta.is_finite() || delta == 0.0 {
            return None;
        }
        self.scroll_lines += delta;
        let lines = self.scroll_lines.trunc();
        self.scroll_lines -= lines;
        Some(lines as i32)
    }
}

impl operation::Focusable for TerminalViewState {
    fn is_focused(&self) -> bool {
        self.focus
    }

    fn focus(&mut self) {
        self.focus = true;
    }

    fn unfocus(&mut self) {
        self.focus = false;
        self.cancel_drag();
        self.held_key = None;
        self.preedit = None;
    }
}

#[derive(Default)]
struct BackgroundRect {
    display_offset: f32,
    cell_height: f32,
    is_active: bool,
    color: Color,
    start_x: f32,
    width: f32,
}

impl BackgroundRect {
    fn with_display_offset(mut self, value: f32) -> Self {
        self.display_offset = value;
        self
    }

    fn with_cell_height(mut self, value: f32) -> Self {
        self.cell_height = value;
        self
    }

    fn with_width(mut self, value: f32) -> Self {
        self.width = value;
        self
    }

    fn with_start_x(mut self, value: f32) -> Self {
        self.start_x = value;
        self
    }

    fn with_color(mut self, value: Color) -> Self {
        self.color = value;
        self
    }

    fn activate(mut self) -> Self {
        self.is_active = true;
        self
    }

    fn build(&self, line: i32) -> Path {
        let flush_y = (line as f32 + self.display_offset) * self.cell_height;
        Path::rectangle(
            Point::new(self.start_x, flush_y),
            Size::new(self.width, self.cell_height),
        )
    }

    fn can_flush(&self) -> bool {
        self.is_active && self.width > 0.0
    }

    fn can_extend(&self, bg: Color, x: f32) -> bool {
        self.is_active
            && bg == self.color
            && (self.start_x + self.width - x).abs() < f32::EPSILON
    }

    fn extend(&mut self, value: f32) {
        self.width += value;
    }
}

#[cfg(test)]
#[path = "link_click_tests.rs"]
mod link_click_tests;

#[cfg(test)]
#[path = "selection_scroll_tests.rs"]
mod selection_scroll_tests;

#[cfg(test)]
#[path = "scrollback_tests.rs"]
mod scrollback_tests;

#[cfg(test)]
mod tests {
    use super::*;

    mod wrap_bracketed_paste_tests {
        use crate::backend::paste_bytes;

        #[test]
        fn wraps_only_when_active() {
            let input = b"line 1\nline 2".to_vec();
            assert_eq!(paste_bytes(input.clone(), false), input);
            let wrapped = paste_bytes(input.clone(), true);
            assert!(wrapped.starts_with(b"\x1b[200~"));
            assert!(wrapped.ends_with(b"\x1b[201~"));
            assert_eq!(&wrapped[6..wrapped.len() - 6], input.as_slice());
        }

        #[test]
        fn filters_escape_and_ctrl_c() {
            // A clipboard containing `\x1b[201~` must not be able to close the bracketed paste early.
            let wrapped = paste_bytes(b"a\x1b[201~b\x03c".to_vec(), true);
            let body = &wrapped[6..wrapped.len() - 6];
            assert_eq!(body, b"a[201~bc");
            assert!(!body.contains(&0x1b));
            assert!(!body.contains(&0x03));
        }
    }

    mod handle_left_button_pressed_tests {
        use super::*;
        use alacritty_terminal::index::{Column, Line};

        #[test]
        fn handles_mouse_mode_with_left_click() {
            let mut state = TerminalViewState::new(iced::widget::Id::unique());
            let terminal_mode = TermMode::MOUSE_MODE;
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();
            let _modifiers = Modifiers::empty();

            TerminalView::handle_left_button_pressed(
                &mut state,
                &terminal_mode,
                cursor_position,
                layout_position,
                &mut commands,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Command::MouseReport(
                    MouseButton::LeftButton,
                    _modifiers,
                    TerminalGridPoint {
                        line: Line(0),
                        column: Column(0),
                    },
                    true,
                )
            ));
            assert_eq!(state.drag, Some(Drag::Mouse));
        }

        #[test]
        fn starts_simple_selection_with_left_click() {
            let terminal_mode = TermMode::SGR_MOUSE;
            let cursor_position = Point { x: 200.0, y: 150.0 };
            let layout_position = Point { x: 50.0, y: 50.0 };

            let cases = vec![
                SelectionType::Simple,
                SelectionType::Semantic,
                SelectionType::Lines,
            ];

            for _selection_type in cases {
                let mut state =
                    TerminalViewState::new(iced::widget::Id::unique());
                state.keyboard_modifiers = Modifiers::SHIFT;
                let mut commands = Vec::new();

                TerminalView::handle_left_button_pressed(
                    &mut state,
                    &terminal_mode,
                    cursor_position,
                    layout_position,
                    &mut commands,
                );

                assert_eq!(commands.len(), 1);
                assert!(matches!(
                    commands[0],
                    Command::SelectStart(_selection_type, (150.0, 100.0))
                ),);
                assert_eq!(state.drag, Some(Drag::Selection));
            }
        }
    }

    mod handle_cursor_moved_tests {
        use alacritty_terminal::index::{Column, Line};

        use super::*;

        #[test]
        fn updates_mouse_position_on_grid() {
            let mut state = TerminalViewState::new(iced::widget::Id::unique());
            let terminal_content = RenderableContent::default();
            let mut commands = Vec::new();
            let cases = vec![
                (
                    Point { x: 0.0, y: 0.0 },
                    Point { x: 1.0, y: 1.0 },
                    TerminalGridPoint {
                        line: Line(1),
                        column: Column(1),
                    },
                ),
                (
                    Point { x: 0.0, y: 0.0 },
                    Point { x: 2.0, y: 2.0 },
                    TerminalGridPoint {
                        line: Line(2),
                        column: Column(2),
                    },
                ),
                (
                    Point { x: 0.0, y: 0.0 },
                    Point { x: 30.0, y: 2.0 },
                    TerminalGridPoint {
                        line: Line(2),
                        column: Column(30),
                    },
                ),
                (
                    Point { x: 10.0, y: 0.0 },
                    Point { x: 30.0, y: 2.0 },
                    TerminalGridPoint {
                        line: Line(2),
                        column: Column(20),
                    },
                ),
                (
                    Point { x: 10.0, y: 10.0 },
                    Point { x: 30.0, y: 2.0 },
                    TerminalGridPoint {
                        line: Line(0),
                        column: Column(20),
                    },
                ),
            ];

            for (layout_position, cursor_position, expected) in cases {
                TerminalView::handle_cursor_moved(
                    &mut state,
                    &terminal_content,
                    &cursor_position,
                    layout_position,
                    &mut commands,
                );

                assert_eq!(state.mouse_position_on_grid, expected);
            }
        }

        #[test]
        fn generates_drag_update_command_when_dragged() {
            let mut state = TerminalViewState::new(iced::widget::Id::unique());
            state.drag = Some(Drag::Selection);
            let terminal_content = RenderableContent::default();
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();

            TerminalView::handle_cursor_moved(
                &mut state,
                &terminal_content,
                &cursor_position,
                layout_position,
                &mut commands,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Command::SelectUpdate((95.0, 145.0))
            ));
        }

        #[test]
        fn generates_drag_update_command_when_dragged_in_mouse_motion_mode() {
            let mut state = TerminalViewState::new(iced::widget::Id::unique());
            state.drag = Some(Drag::Mouse);
            let mut terminal_content = RenderableContent::default();
            terminal_content.terminal_mode = TermMode::MOUSE_MOTION;
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();
            let _modifiers = Modifiers::empty();

            TerminalView::handle_cursor_moved(
                &mut state,
                &terminal_content,
                &cursor_position,
                layout_position,
                &mut commands,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Command::MouseReport(
                    MouseButton::LeftMove,
                    _modifiers,
                    TerminalGridPoint {
                        line: Line(49),
                        column: Column(79),
                    },
                    true,
                )
            ));
        }

        #[test]
        fn reports_drag_when_dragged_in_mouse_drag_mode() {
            // Button-motion tracking (1002) — what e.g. tmux with `mouse on`
            // requests — must also report drags, not fall back to the
            // widget's own selection.
            let mut state = TerminalViewState::new(iced::widget::Id::unique());
            state.drag = Some(Drag::Mouse);
            let mut terminal_content = RenderableContent::default();
            terminal_content.terminal_mode = TermMode::MOUSE_DRAG;
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();
            let _modifiers = Modifiers::empty();

            TerminalView::handle_cursor_moved(
                &mut state,
                &terminal_content,
                &cursor_position,
                layout_position,
                &mut commands,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Command::MouseReport(
                    MouseButton::LeftMove,
                    _modifiers,
                    TerminalGridPoint {
                        line: Line(49),
                        column: Column(79),
                    },
                    true,
                )
            ));
        }

        #[test]
        fn generates_drag_update_command_when_dragged_in_srg_mode_with_key_mods(
        ) {
            let mut state = TerminalViewState::new(iced::widget::Id::unique());
            state.keyboard_modifiers = Modifiers::SHIFT;
            state.drag = Some(Drag::Selection);
            let mut terminal_content = RenderableContent::default();
            terminal_content.terminal_mode = TermMode::SGR_MOUSE;
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();

            TerminalView::handle_cursor_moved(
                &mut state,
                &terminal_content,
                &cursor_position,
                layout_position,
                &mut commands,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Command::SelectUpdate((95.0, 145.0))
            ));
        }

        #[test]
        fn generates_drag_update_and_link_open() {
            let mut state = TerminalViewState::new(iced::widget::Id::unique());
            state.keyboard_modifiers = Modifiers::COMMAND;
            state.drag = Some(Drag::Selection);
            let mut terminal_content = RenderableContent::default();
            terminal_content.terminal_mode = TermMode::SGR_MOUSE;
            let layout_position = Point { x: 5.0, y: 5.0 };
            let cursor_position = Point { x: 100.0, y: 150.0 };
            let mut commands = Vec::new();

            TerminalView::handle_cursor_moved(
                &mut state,
                &terminal_content,
                &cursor_position,
                layout_position,
                &mut commands,
            );

            assert_eq!(commands.len(), 2);
            assert!(matches!(
                commands[0],
                Command::SelectUpdate((95.0, 145.0))
            ));
            assert!(matches!(
                commands[1],
                Command::ProcessLink(
                    LinkAction::Hover,
                    TerminalGridPoint {
                        line: Line(49),
                        column: Column(79),
                    },
                )
            ));
        }
    }

    mod handle_button_released_tests {
        use super::*;
        use alacritty_terminal::index::{Column, Line};

        #[test]
        fn mouse_mode_activated() {
            let mut state = TerminalViewState::new(iced::widget::Id::unique());
            state.drag = Some(Drag::Mouse);
            let terminal_mode = TermMode::MOUSE_MODE;
            let mut commands = Vec::new();
            let _modifiers = Modifiers::empty();

            TerminalView::handle_button_released(
                &mut state,
                &terminal_mode,
                false,
                &mut commands,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Command::MouseReport(
                    MouseButton::LeftButton,
                    _modifiers,
                    TerminalGridPoint {
                        line: Line(0),
                        column: Column(0)
                    },
                    false
                )
            ));
        }

        #[test]
        fn link_open_on_button_release() {
            let mut state = TerminalViewState::new(iced::widget::Id::unique());
            state.keyboard_modifiers = Modifiers::COMMAND;
            state.drag = Some(Drag::Mouse);
            let terminal_mode = TermMode::MOUSE_MODE;
            let mut commands = Vec::new();
            let _modifiers = Modifiers::empty();

            TerminalView::handle_button_released(
                &mut state,
                &terminal_mode,
                true,
                &mut commands,
            );

            assert_eq!(commands.len(), 2);
            assert!(matches!(
                commands[0],
                Command::MouseReport(
                    MouseButton::LeftButton,
                    _modifiers,
                    TerminalGridPoint {
                        line: Line(0),
                        column: Column(0)
                    },
                    false
                )
            ));
            assert!(matches!(
                commands[1],
                Command::ProcessLink(
                    LinkAction::Open,
                    TerminalGridPoint {
                        line: Line(0),
                        column: Column(0)
                    }
                ),
            ));
        }

        #[test]
        fn link_open_on_button_release_in_non_mouse_mode() {
            let mut state = TerminalViewState::new(iced::widget::Id::unique());
            state.keyboard_modifiers = Modifiers::COMMAND;
            state.mouse_position_on_grid = TerminalGridPoint {
                line: Line(4),
                column: Column(10),
            };
            let terminal_mode = TermMode::empty(); // Assume SGR_MOUSE mode doesn't affect link opening
            let mut commands = Vec::new();

            TerminalView::handle_button_released(
                &mut state,
                &terminal_mode,
                true,
                &mut commands,
            );

            assert_eq!(commands.len(), 1);
            assert!(matches!(
                commands[0],
                Command::ProcessLink(
                    LinkAction::Open,
                    TerminalGridPoint {
                        line: Line(4),
                        column: Column(10)
                    }
                ),
            ));
        }
    }
}
