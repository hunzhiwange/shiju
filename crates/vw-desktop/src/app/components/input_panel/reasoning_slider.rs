//! 连续跟手、松手后缓动吸附的推理强度滑杆；动画状态仅保存在控件树中。
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer, widget::tree};
use iced::mouse;
use iced::theme::palette;
use iced::time::{Duration, Instant};
use iced::{
    Border, Color, Degrees, Element, Event, Length, Point, Rectangle, Size, Theme, gradient,
};
use vw_gateway_client::vw_api_types::ReasoningEffortDto;

use super::{HANDLE_RADIUS, SLIDER_HEIGHT, SLIDER_WIDTH, reasoning_effort_message};
use crate::app::Message;

const SNAP_DURATION: Duration = Duration::from_millis(180);

#[cfg(test)]
#[path = "reasoning_slider_tests.rs"]
mod tests;

struct Snap {
    from: f32,
    to: f32,
    started: Instant,
}

struct State {
    position: f32,
    committed: usize,
    dragging: bool,
    grab_offset: f32,
    modifiers: iced::keyboard::Modifiers,
    touch: Option<iced::touch::Finger>,
    snap: Option<Snap>,
    identity: String,
}

impl State {
    fn new(selected: usize, identity: String) -> Self {
        Self {
            position: selected as f32,
            committed: selected,
            dragging: false,
            grab_offset: 0.0,
            modifiers: iced::keyboard::Modifiers::default(),
            touch: None,
            snap: None,
            identity,
        }
    }

    fn animate(&mut self, now: Instant) {
        if let Some(snap) = &self.snap {
            let progress = (now.saturating_duration_since(snap.started).as_secs_f32()
                / SNAP_DURATION.as_secs_f32())
            .clamp(0.0, 1.0);
            // Cubic ease-out：松手后减速进入档位，终点速度为零。
            self.position = snap.from + (snap.to - snap.from) * (1.0 - (1.0 - progress).powi(3));
            if progress >= 1.0 {
                self.position = snap.to;
                self.snap = None;
            }
        }
    }

    fn release(&mut self, now: Instant) -> usize {
        let target = self.position.round() as usize;
        self.dragging = false;
        self.touch = None;
        self.committed = target;
        self.snap = Some(Snap { from: self.position, to: target as f32, started: now });
        target
    }
}

pub(super) struct ReasoningSlider {
    pub(super) selected: usize,
    pub(super) options: Vec<Option<ReasoningEffortDto>>,
    pub(super) identity: String,
}

impl ReasoningSlider {
    fn maximum(&self) -> f32 {
        self.options.len().saturating_sub(1).max(1) as f32
    }

    fn locate(&self, x: f32, bounds: Rectangle) -> f32 {
        ((x - bounds.x - HANDLE_RADIUS) / (bounds.width - 2.0 * HANDLE_RADIUS).max(1.0))
            .clamp(0.0, 1.0)
            * self.maximum()
    }
}

impl Widget<Message, Theme, iced::Renderer> for ReasoningSlider {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new(self.selected, self.identity.clone()))
    }

    fn diff(&self, tree: &mut tree::Tree) {
        let state = tree.state.downcast_mut::<State>();
        // 模型/会话切换和外部重置不能继承旧拖动或吸附状态。
        if state.identity != self.identity || state.committed != self.selected {
            *state = State::new(self.selected, self.identity.clone());
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(SLIDER_WIDTH), Length::Fixed(SLIDER_HEIGHT))
    }

    fn layout(
        &mut self,
        _tree: &mut tree::Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, SLIDER_WIDTH, SLIDER_HEIGHT)
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        if let Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            state.animate(*now);
            if state.snap.is_some() {
                shell.request_redraw();
            }
            return;
        }
        if matches!(event, Event::Window(iced::window::Event::Unfocused)) && state.dragging {
            *state = State::new(state.committed, self.identity.clone());
            shell.request_redraw();
            return;
        }
        if let Event::Keyboard(iced::keyboard::Event::ModifiersChanged(modifiers)) = event {
            state.modifiers = *modifiers;
        }
        if !state.dragging && cursor.is_over(bounds) {
            use iced::keyboard::{Event as KeyEvent, Key, key::Named};
            let direction = match event {
                Event::Keyboard(KeyEvent::KeyPressed {
                    key: Key::Named(Named::ArrowUp), ..
                }) => Some(1.0),
                Event::Keyboard(KeyEvent::KeyPressed {
                    key: Key::Named(Named::ArrowDown), ..
                }) => Some(-1.0),
                Event::Mouse(mouse::Event::WheelScrolled { delta })
                    if state.modifiers.control() =>
                {
                    let (mouse::ScrollDelta::Lines { y, .. }
                    | mouse::ScrollDelta::Pixels { y, .. }) = delta;
                    Some(if *y < 0.0 { -1.0 } else { 1.0 })
                }
                _ => None,
            };
            if let Some(direction) = direction {
                let now = Instant::now();
                state.animate(now);
                let target =
                    (state.committed as f32 + direction).clamp(0.0, self.maximum()) as usize;
                state.committed = target;
                state.snap = Some(Snap { from: state.position, to: target as f32, started: now });
                shell.publish(reasoning_effort_message(self.options[target]));
                shell.capture_event();
                shell.request_redraw();
                return;
            }
        }
        let press = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => cursor.position(),
            Event::Touch(iced::touch::Event::FingerPressed { position, .. }) => Some(*position),
            _ => None,
        };
        if let Some(point) = press.filter(|point| bounds.contains(*point)) {
            state.animate(Instant::now());
            let center = bounds.x
                + HANDLE_RADIUS
                + state.position / self.maximum() * (bounds.width - HANDLE_RADIUS * 2.0);
            state.grab_offset =
                if (point.x - center).abs() <= HANDLE_RADIUS { point.x - center } else { 0.0 };
            state.position = self.locate(point.x - state.grab_offset, bounds);
            state.dragging = true;
            state.snap = None;
            state.touch = match event {
                Event::Touch(iced::touch::Event::FingerPressed { id, .. }) => Some(*id),
                _ => None,
            };
            shell.capture_event();
            shell.request_redraw();
        } else if state.dragging {
            let moved = match event {
                Event::Mouse(mouse::Event::CursorMoved { position }) if state.touch.is_none() => {
                    Some(*position)
                }
                Event::Touch(iced::touch::Event::FingerMoved { id, position })
                    if state.touch == Some(*id) =>
                {
                    Some(*position)
                }
                _ => None,
            };
            if let Some(point) = moved {
                state.position = self.locate(point.x - state.grab_offset, bounds);
                shell.capture_event();
                shell.request_redraw();
            }
            let released = match event {
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    state.touch.is_none()
                }
                Event::Touch(
                    iced::touch::Event::FingerLifted { id, .. }
                    | iced::touch::Event::FingerLost { id, .. },
                ) => state.touch == Some(*id),
                _ => false,
            };
            if released {
                let point = match event {
                    Event::Touch(iced::touch::Event::FingerLifted { position, .. }) => {
                        Some(*position)
                    }
                    Event::Mouse(_) => cursor.position(),
                    _ => None,
                };
                if let Some(point) = point {
                    state.position = self.locate(point.x - state.grab_offset, bounds);
                }
                let target = state.release(Instant::now());
                // 只在松手后提交一次；随后重建视图也不会打断控件树中的吸附动画。
                shell.publish(reasoning_effort_message(self.options[target]));
                shell.capture_event();
                shell.request_redraw();
            }
        }
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let center = Point::new(
            bounds.x
                + HANDLE_RADIUS
                + state.position / self.maximum() * (bounds.width - HANDLE_RADIUS * 2.0),
            bounds.center_y(),
        );
        let quad = |bounds, radius: f32| renderer::Quad {
            bounds,
            border: Border { radius: radius.into(), ..Border::default() },
            ..renderer::Quad::default()
        };
        let track = Rectangle { y: center.y - 10.0, height: 20.0, ..bounds };
        let track_palette = theme.extended_palette().background.strong;
        renderer.fill_quad(quad(track, 10.0), track_palette.color);
        if state.position > 0.0 {
            let fill_bounds = Rectangle { width: (center.x - track.x).max(track.height), ..track };
            let start = Color::from_rgb8(32, 103, 216);
            let end = Color::from_rgb8(132, 47, 185);
            // 按填充宽度截取渐变，避免拖动时整段颜色随滑块拉伸。
            let fill = gradient::Linear::new(Degrees(90.0))
                .add_stop(0.0, start)
                .add_stop(1.0, palette::mix(start, end, fill_bounds.width / track.width));
            renderer.fill_quad(quad(fill_bounds, 10.0), fill);
        }
        for index in 0..self.options.len() {
            let x = bounds.x
                + HANDLE_RADIUS
                + index as f32 / self.maximum() * (bounds.width - HANDLE_RADIUS * 2.0);
            renderer.fill_quad(
                quad(Rectangle::new(Point::new(x - 3.0, center.y - 3.0), Size::new(6.0, 6.0)), 3.0),
                if x < center.x { Color::WHITE } else { track_palette.text },
            );
        }
        renderer.fill_quad(
            renderer::Quad {
                border: Border {
                    width: 1.0,
                    color: theme.palette().text.scale_alpha(0.24),
                    radius: HANDLE_RADIUS.into(),
                },
                bounds: Rectangle::new(
                    Point::new(center.x - HANDLE_RADIUS, center.y - HANDLE_RADIUS),
                    Size::new(HANDLE_RADIUS * 2.0, HANDLE_RADIUS * 2.0),
                ),
                ..renderer::Quad::default()
            },
            Color::WHITE,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if tree.state.downcast_ref::<State>().dragging {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::None
        }
    }
}

impl From<ReasoningSlider> for Element<'_, Message> {
    fn from(slider: ReasoningSlider) -> Self {
        Self::new(slider)
    }
}
