use std::f32::consts::TAU;
use std::time::Duration;

use iced::time::Instant;
use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke};
use iced::{Color, Event, Point, Rectangle, Renderer, Theme, mouse, window};

pub(super) struct Orb {
    pub(super) expanded: bool,
}

impl canvas::Program<super::Message> for Orb {
    type State = Option<Instant>;

    fn update(
        &self,
        start: &mut Self::State,
        event: &Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<canvas::Action<super::Message>> {
        let Event::Window(window::Event::RedrawRequested(now)) = event else {
            return None;
        };
        start.get_or_insert(*now);
        Some(canvas::Action::request_redraw_at(*now + Duration::from_millis(33)))
    }

    fn draw(
        &self,
        start: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let phase = start.map_or(0.0, |start| start.elapsed().as_secs_f32() * 1.8);
        let mut frame = Frame::new(renderer, bounds.size());
        let center = frame.center();
        let accent = Color::from_rgb(0.20, 0.80, 1.0);
        let violet = Color::from_rgb(0.63, 0.38, 1.0);
        let pink = Color::from_rgb(1.0, 0.38, 0.74);
        let energy = if self.expanded { 1.0 } else { 0.74 };
        let pulse = 0.88 + 0.12 * phase.sin();

        for (radius, alpha) in [(13.8, 0.10), (12.7, 0.17), (11.8, 0.26)] {
            frame.fill(&Path::circle(center, radius), violet.scale_alpha(alpha * energy * pulse));
        }
        let core = Path::circle(center, 11.2);
        let dark = theme.extended_palette().is_dark;
        frame.fill(
            &core,
            if dark {
                Color::from_rgb(0.05, 0.06, 0.18)
            } else {
                Color::from_rgb(0.08, 0.10, 0.24)
            },
        );
        frame
            .stroke(&core, Stroke::default().with_width(0.65).with_color(accent.scale_alpha(0.62)));

        let turn = 0.72 * (phase * 0.78).sin();
        let roll = 0.31 * (phase * 1.16).sin() + 0.09 * (phase * 1.9).sin();
        let (sin_roll, cos_roll) = roll.sin_cos();
        let width = turn.cos();
        let shear = 0.22 * turn.sin();
        let point = |x: f32, y: f32| {
            let x = x * width + y * shear;
            Point::new(
                center.x + x * cos_roll - y * sin_roll,
                center.y + x * sin_roll + y * cos_roll,
            )
        };

        let bend = 1.7 * (phase * 1.47).sin();
        let cross = 0.65;
        let cross_y = -6.1 + 11.6 * cross;
        let cross_bend = 2.0 * cross * (1.0 - cross) * bend;
        let letter = Path::new(|path| {
            path.move_to(point(0.0, -6.1));
            path.quadratic_curve_to(point(-2.85 + bend, -0.3), point(-5.7, 5.5));
            path.move_to(point(0.0, -6.1));
            path.quadratic_curve_to(point(2.85 + bend, -0.3), point(5.7, 5.5));
            // Keep the crossbar attached to both legs while the curves bend.
            path.move_to(point(-5.7 * cross + cross_bend, cross_y));
            path.quadratic_curve_to(
                point(cross_bend, cross_y + 0.85 * (phase * 1.47 + 0.7).sin()),
                point(5.7 * cross + cross_bend, cross_y),
            );
        });
        let stroke =
            Stroke::default().with_line_cap(LineCap::Round).with_line_join(LineJoin::Round);
        frame.stroke(
            &letter,
            stroke.with_width(4.0).with_color(violet.scale_alpha(0.44 * energy * pulse)),
        );
        frame.stroke(&letter, stroke.with_width(2.2).with_color(accent.scale_alpha(0.72)));
        frame.stroke(&letter, stroke.with_width(1.15).with_color(Color::from_rgb(0.86, 0.96, 1.0)));

        for (index, color) in [accent, violet, pink].into_iter().enumerate() {
            let angle = index as f32 * TAU / 3.0 + phase * 0.85;
            let radius = 11.5 + 1.1 * (angle * 2.0 + phase).sin();
            let particle = polar(center, radius, angle);
            frame.fill(&Path::circle(particle, 1.25), color.scale_alpha(0.26 * energy));
            frame.fill(&Path::circle(particle, 0.65), color.scale_alpha(energy));
        }

        vec![frame.into_geometry()]
    }
}

fn polar(center: Point, radius: f32, angle: f32) -> Point {
    Point::new(center.x + radius * angle.cos(), center.y + radius * angle.sin())
}
