//! 探索摘要数字的逐位翻转绘制。
use iced::widget::canvas::{Frame, Geometry, Program, Text as CanvasText};
use iced::widget::{canvas, row};
use iced::{Alignment, Color, Element, Length, Theme};
use crate::card_input::Message;
use crate::card_style::chat_secondary_subtle_text_color;
use crate::tool_text_support::chat_text_font;

#[derive(Debug, Clone)]
struct FlipNumberText {
    previous: String,
    current: String,
    progress: f32,
    font_size: f32,
}

impl Program<Message> for FlipNumberText {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let color = chat_secondary_subtle_text_color(theme);
        let progress = self.progress.clamp(0.0, 1.0);
        let center_x = bounds.width * 0.5;
        let center_y = bounds.height * 0.5;

        if self.previous == self.current || progress <= 0.001 {
            draw_flip_text(
                &mut frame,
                &self.current,
                center_x,
                center_y,
                self.font_size,
                1.0,
                0.0,
                color,
            );
            return vec![frame.into_geometry()];
        }

        if progress < 0.5 {
            let phase = (progress / 0.5).clamp(0.0, 1.0);
            let eased = phase * phase * (3.0 - 2.0 * phase);
            draw_flip_text(
                &mut frame,
                &self.previous,
                center_x,
                center_y,
                (self.font_size * (1.0 - eased * 0.62)).max(1.0),
                1.0 - eased * 0.96,
                -eased * self.font_size * 0.62,
                color,
            );
            draw_flip_text(
                &mut frame,
                &self.current,
                center_x,
                center_y,
                (self.font_size * (0.34 + eased * 0.22)).max(1.0),
                eased * 0.28,
                (1.0 - eased) * self.font_size * 0.78,
                color,
            );
        } else {
            let phase = ((progress - 0.5) / 0.5).clamp(0.0, 1.0);
            let eased = phase * phase * (3.0 - 2.0 * phase);
            draw_flip_text(
                &mut frame,
                &self.current,
                center_x,
                center_y,
                (self.font_size * (0.56 + eased * 0.44)).max(1.0),
                0.28 + eased * 0.72,
                (1.0 - eased) * self.font_size * 0.42,
                color,
            );
        }

        vec![frame.into_geometry()]
    }
}

fn draw_flip_text(
    frame: &mut Frame,
    content: &str,
    center_x: f32,
    center_y: f32,
    font_size: f32,
    alpha: f32,
    shift_y: f32,
    color: Color,
) {
    frame.fill_text(CanvasText {
        content: content.to_string(),
        position: iced::Point::new(center_x, center_y + shift_y),
        color: color.scale_alpha(alpha.clamp(0.0, 1.0)),
        size: font_size.into(),
        font: chat_text_font(),
        align_x: iced::widget::text::Alignment::Center,
        align_y: iced::alignment::Vertical::Center,
        ..Default::default()
    });
}

pub(super) fn right_aligned_slot_char(
    chars: &[char],
    slot_idx: usize,
    total_slots: usize,
) -> String {
    let left_padding = total_slots.saturating_sub(chars.len());
    if slot_idx < left_padding { String::new() } else { chars[slot_idx - left_padding].to_string() }
}

fn flip_number_slot_view<'a>(
    previous: &str,
    current: &str,
    progress: f32,
    font_size: f32,
) -> Element<'a, Message> {
    let width = (font_size * 0.64).ceil().max(font_size * 0.60);
    canvas(FlipNumberText {
        previous: previous.to_string(),
        current: current.to_string(),
        progress,
        font_size,
    })
    .width(Length::Fixed(width))
    .height(Length::Fixed((font_size * 1.62).ceil()))
    .into()
}

pub(super) fn flip_number_view<'a>(
    previous: &str,
    current: &str,
    progress: f32,
    font_size: f32,
) -> Element<'a, Message> {
    let previous_chars = previous.chars().collect::<Vec<_>>();
    let current_chars = current.chars().collect::<Vec<_>>();
    let slot_count = previous_chars.len().max(current_chars.len()).max(1);
    let mut content = row![].spacing(0).align_y(Alignment::Center);

    for slot_idx in 0..slot_count {
        let previous_char = right_aligned_slot_char(&previous_chars, slot_idx, slot_count);
        let current_char = right_aligned_slot_char(&current_chars, slot_idx, slot_count);
        content =
            content.push(flip_number_slot_view(&previous_char, &current_char, progress, font_size));
    }

    content.into()
}
