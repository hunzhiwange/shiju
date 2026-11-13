//! Bounded, sequence-aware PCM history. Repeated summaries decay to silence.
use std::collections::VecDeque;

#[derive(Default)]
pub(crate) struct Waveform {
    samples: VecDeque<f32>,
    sequence: u64,
}
impl Waveform {
    pub(crate) fn push(&mut self, sequence: u64, peak: f32) {
        let amplitude = if sequence > self.sequence {
            self.sequence = sequence;
            peak.clamp(0.0, 1.0)
        } else {
            self.samples.back().copied().unwrap_or_default() * 0.5
        };
        if self.samples.len() == 120 {
            self.samples.pop_front();
        }
        self.samples.push_back(amplitude);
    }
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }
    pub(super) fn view(&self) -> iced::Element<'static, crate::app::Message> {
        use iced::widget::{responsive, svg};
        let samples = self.samples.clone();
        responsive(move |size| {
            let width = size.width.max(1.0);
            let count = (width / 5.0).floor() as usize;
            let mut data = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} 36"><g stroke="currentColor" stroke-width="2" stroke-linecap="round">"#
            );
            for (index, amplitude) in samples.iter().rev().take(count).enumerate() {
                let x = width - 3.0 - index as f32 * 5.0;
                let half = 1.0 + amplitude * 15.0;
                data.push_str(&format!(
                    r#"<path d="M{x} {}V{}"/>"#, 18.0 - half, 18.0 + half
                ));
            }
            data.push_str("</g></svg>");
            svg(svg::Handle::from_memory(data.into_bytes()))
                .width(iced::Length::Fill)
                .height(36)
                .style(|theme: &iced::Theme, _| svg::Style { color: Some(theme.palette().text) })
                .into()
        })
        .width(iced::Length::Fill)
        .height(36)
        .into()
    }
}
#[cfg(test)]
#[path = "voice_waveform_tests.rs"]
mod tests;
