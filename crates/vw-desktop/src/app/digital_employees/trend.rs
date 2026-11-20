use iced::advanced::text::Alignment;
use iced::alignment::Vertical;
use iced::widget::canvas;
use iced::{Color, Pixels, Point, Rectangle, Renderer, Size, Theme, mouse};

use crate::app::Message;

use super::api::Trend;
use super::view::Palette;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Metric {
    Tasks,
    SuccessRate,
}

impl Metric {
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Tasks => "每天新建的任务量",
            Self::SuccessRate => "任务成功率",
        }
    }

    pub(super) fn value(self, trend: &Trend, index: usize) -> Option<f64> {
        match self {
            Self::Tasks => trend.tasks.get(index).copied().flatten().map(|n| n as f64),
            Self::SuccessRate => trend.success_rate.get(index).copied().flatten(),
        }
    }

    pub(super) fn format(self, value: Option<f64>) -> String {
        match (self, value) {
            (_, None) => "—".into(),
            (Self::Tasks, Some(n)) => format!("{n:.0} 个"),
            (Self::SuccessRate, Some(n)) if n.fract() == 0.0 => format!("{n:.0}%"),
            (Self::SuccessRate, Some(n)) => format!("{n:.1}%"),
        }
    }
}

pub(super) struct Chart<'a> {
    pub trend: &'a Trend,
    pub metric: Metric,
}

impl canvas::Program<Message> for Chart<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let left = 55.0;
        let right = bounds.width - 20.0;
        let top = 20.0;
        let bottom = bounds.height - 32.0;
        let days = &self.trend.days;
        if right <= left || bottom <= top || days.is_empty() {
            return Vec::new();
        }

        let colors = Palette::new(theme);
        let color = match self.metric {
            Metric::Tasks => colors.accent,
            Metric::SuccessRate => colors.blue,
        };
        let (ticks, maximum) = match self.metric {
            Metric::Tasks => {
                let peak = self.trend.tasks.iter().flatten().copied().max().unwrap_or(0) as f64;
                let raw_step = (peak / 3.0).max(1.0);
                let magnitude = 10_f64.powf(raw_step.log10().floor());
                let step = (raw_step / magnitude).ceil() * magnitude;
                (3, step * 3.0)
            }
            Metric::SuccessRate => (4, 100.0),
        };
        let x_at = |index: usize| {
            left + (right - left) * index as f32 / days.len().saturating_sub(1).max(1) as f32
        };
        let y_at = |value: f64| bottom - (bottom - top) * (value / maximum).clamp(0.0, 1.0) as f32;
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        for tick in 0..=ticks {
            let value = maximum * f64::from(tick) / f64::from(ticks);
            let y = y_at(value);
            frame.fill_rectangle(Point::new(left, y), Size::new(right - left, 1.0), colors.border);
            frame.fill_text(label(
                self.metric.format(Some(value)),
                Point::new(left - 10.0, y),
                Alignment::Right,
                colors.muted,
            ));
        }

        let max_labels = ((right - left) / 62.0).floor().max(1.0) as usize;
        let stride = days.len().saturating_sub(1).div_ceil(max_labels).max(1);
        for (index, date) in days.iter().enumerate() {
            let last = days.len() - 1;
            if index != 0 && index != last && (index % stride != 0 || last - index < stride / 2) {
                continue;
            }
            let date = match date.get(..5) {
                Some(prefix) if prefix.ends_with('-') => &date[5..],
                _ => date,
            };
            frame.fill_text(label(
                date.to_owned(),
                Point::new(x_at(index), bottom + 18.0),
                Alignment::Center,
                colors.muted,
            ));
        }

        let points: Vec<_> = (0..days.len())
            .map(|i| self.metric.value(self.trend, i).map(|v| Point::new(x_at(i), y_at(v))))
            .collect();
        for segment in points.split(Option::is_none) {
            let points: Vec<_> = segment.iter().flatten().copied().collect();
            let Some(first) = points.first().copied() else { continue };
            let last = points.last().copied().unwrap_or(first);
            if points.len() == 1 {
                frame.fill(&canvas::Path::circle(first, 2.5), color);
                continue;
            }
            let line = canvas::Path::new(|path| {
                path.move_to(first);
                for &point in &points[1..] {
                    path.line_to(point);
                }
            });
            let area = canvas::Path::new(|path| {
                path.move_to(Point::new(first.x, bottom));
                for &point in &points {
                    path.line_to(point);
                }
                path.line_to(Point::new(last.x, bottom));
                path.close();
            });
            frame.fill(&area, color.scale_alpha(0.10));
            frame.stroke(&line, canvas::Stroke::default().with_color(color).with_width(2.4));
        }

        let selected = cursor
            .position_in(bounds)
            .filter(|point| (left..=right).contains(&point.x) && (top..=bottom).contains(&point.y))
            .map(|point| {
                (((point.x - left) / (right - left)) * days.len().saturating_sub(1) as f32).round()
                    as usize
            })
            .unwrap_or(days.len() - 1);
        if let Some(point) = points[selected] {
            frame.fill(&canvas::Path::circle(point, 5.5), colors.surface);
            frame.fill(&canvas::Path::circle(point, 3.8), color);
            let (x, align) = if point.x - left < 50.0 {
                (point.x + 6.0, Alignment::Left)
            } else {
                (point.x - 3.0, Alignment::Right)
            };
            frame.fill_text(label(
                self.metric.format(self.metric.value(self.trend, selected)),
                Point::new(x, point.y - 12.0),
                align,
                colors.ink,
            ));
        }

        vec![frame.into_geometry()]
    }
}

fn label(content: String, position: Point, align_x: Alignment, color: Color) -> canvas::Text {
    canvas::Text {
        content,
        position,
        align_x,
        align_y: Vertical::Center,
        color,
        size: Pixels(12.0),
        font: iced::Font::with_name("Noto Sans CJK SC"),
        ..Default::default()
    }
}
