//! Product-only adaptation of the copied Codex welcome widget.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Widget;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::WidgetRef;
use ratatui::widgets::Wrap;

const MIN_ANIMATION_HEIGHT: u16 = 37;
const MIN_ANIMATION_WIDTH: u16 = 60;
const GENERIC_WELCOME_FRAME: &str = include_str!("../../frames/default/frame_1.txt");

/// Welcome copy shown before a VibeWindow session is rendered.
#[derive(Clone, Copy, Debug)]
pub struct WelcomeWidget {
    animations_enabled: bool,
}

impl Default for WelcomeWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl WelcomeWidget {
    #[must_use]
    pub const fn new() -> Self {
        Self { animations_enabled: true }
    }

    #[must_use]
    pub const fn with_animations_enabled(mut self, enabled: bool) -> Self {
        self.animations_enabled = enabled;
        self
    }
}

impl WidgetRef for &WelcomeWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let mut lines = Vec::new();
        if self.animations_enabled
            && area.height >= MIN_ANIMATION_HEIGHT
            && area.width >= MIN_ANIMATION_WIDTH
        {
            lines.extend(GENERIC_WELCOME_FRAME.lines().map(Line::from));
            lines.push(Line::default());
        }
        lines.push(Line::from(vec![
            "  ".into(),
            "Welcome to ".into(),
            "VibeWindow".bold(),
            ", your autonomous agent runtime".into(),
        ]));
        Paragraph::new(lines).wrap(Wrap { trim: false }).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_renders_vibewindow_copy_with_upstream_spacing() {
        let area = Rect::new(0, 0, 72, 2);
        let mut buf = Buffer::empty(area);
        let widget = WelcomeWidget::new();
        (&widget).render_ref(area, &mut buf);

        insta::assert_snapshot!(format!("{buf:?}"));
    }

    #[test]
    fn welcome_renders_copied_generic_frame_at_upstream_breakpoint() {
        let area = Rect::new(0, 0, MIN_ANIMATION_WIDTH, MIN_ANIMATION_HEIGHT);
        let mut buf = Buffer::empty(area);
        let widget = WelcomeWidget::new();
        (&widget).render_ref(area, &mut buf);

        let rendered = format!("{buf:?}");
        assert!(rendered.contains("Welcome to VibeWindow"));
        insta::assert_snapshot!(rendered);
    }
}
