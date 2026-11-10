//! 月额度环。只在余额变化时请求动画帧。

use std::f32::consts::TAU;

use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer, svg, widget::tree};
use iced::animation::{Animation, Easing};
use iced::time::Instant;
use iced::widget::{container, stack, text};
use iced::{
    Background, Border, Color, Element, Event, Length, Rectangle, Size, Theme, mouse, window,
};
use renderer::Renderer as _;
use svg::Renderer as _;

use crate::app::Message;

const SIZE: f32 = 26.0;
const CRITICAL_BALANCE: f32 = 0.01;

pub(super) enum QuotaRing {
    Limited(f64),
    Unlimited,
    Unknown,
}

impl QuotaRing {
    fn label(&self) -> String {
        match self {
            Self::Limited(remaining) if *remaining > 0.0 && *remaining < 0.01 => "<1%".into(),
            Self::Limited(remaining) if *remaining < 1.0 => {
                format!("{}%", (remaining * 100.0).round().min(99.0) as u32)
            }
            Self::Limited(_) => "100%".into(),
            Self::Unlimited => "∞".into(),
            Self::Unknown => "—".into(),
        }
    }

    pub(super) fn view(self) -> Element<'static, Message> {
        let label = text(self.label()).size(8);
        stack![Element::new(self), container(label).center(Length::Fill),]
            .width(SIZE)
            .height(SIZE)
            .into()
    }

    fn fraction(&self) -> Option<f32> {
        match self {
            Self::Limited(remaining) => Some(*remaining as f32),
            Self::Unlimited => Some(1.0),
            Self::Unknown => None,
        }
    }
}

#[derive(Default)]
struct RingState {
    progress: f32,
    animation: Option<Animation<f32>>,
}

impl RingState {
    fn update(&mut self, target: Option<f32>, now: Instant) -> bool {
        let Some(target) = target else {
            *self = Self::default();
            return false;
        };
        let animation = self
            .animation
            .get_or_insert_with(|| Animation::new(0.0).slow().easing(Easing::EaseOutCubic));
        if animation.value() != target {
            animation.go_mut(target, now);
        }
        self.progress = animation.interpolate_with(|value| value, now);
        animation.is_animating(now)
    }
}

impl Widget<Message, Theme, iced::Renderer> for QuotaRing {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<RingState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(RingState::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(SIZE), Length::Fixed(SIZE))
    }

    fn layout(
        &mut self,
        _tree: &mut tree::Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, SIZE, SIZE)
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if let Event::Window(window::Event::RedrawRequested(now)) = event
            && tree.state.downcast_mut::<RingState>().update(self.fraction(), *now)
        {
            shell.request_redraw();
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
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let remaining = self.fraction();
        let color = remaining.map(remaining_color);
        let track_color = match (remaining, color) {
            (Some(remaining), Some(color)) if remaining <= CRITICAL_BALANCE => color,
            _ => theme.extended_palette().background.strong.color,
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds: bounds.shrink(iced::Padding::new(1.0)),
                border: Border { color: track_color, width: 2.0, radius: (SIZE / 2.0).into() },
                ..renderer::Quad::default()
            },
            Background::Color(Color::TRANSPARENT),
        );

        let progress = tree.state.downcast_ref::<RingState>().progress;
        if let Some(color) = color
            && progress > 0.0
        {
            // Use the vector icon renderer so the arc also appears with the software backend.
            let circumference = TAU * 11.0;
            let length = circumference * progress;
            let arc = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 26 26">
                <circle cx="13" cy="13" r="11" fill="none" stroke="white" stroke-width="2"
                    stroke-linecap="round" stroke-dasharray="{length:.4} {circumference:.4}"
                    transform="rotate(-90 13 13)"/>
                </svg>"#
            );
            let svg = svg::Svg::new(svg::Handle::from_memory(arc.into_bytes())).color(color);
            renderer.draw_svg(svg, bounds, *viewport);
        }
    }
}

fn remaining_color(remaining: f32) -> Color {
    // 余额在 1% 及以下保持纯红，中点为黄，满额为纯绿。
    let red = ((1.0 - remaining) * 2.0).min(1.0);
    let green = ((remaining - CRITICAL_BALANCE) / (0.5 - CRITICAL_BALANCE)).clamp(0.0, 1.0);
    Color::from_rgb(red, green, 0.0)
}

#[cfg(test)]
#[path = "account_quota_ring_tests.rs"]
mod tests;
