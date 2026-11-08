//! Single-line labels are measured in the same font used to draw them.

use iced::advanced::text::{self, Paragraph as _, Renderer as _};
use iced::advanced::{Layout, Widget, layout, mouse, renderer, widget};
use iced::{Element, Font, Length, Pixels, Rectangle, Renderer, Size, Theme, alignment};

type Paragraph = <Renderer as text::Renderer>::Paragraph;

pub(crate) struct SingleLineLabel {
    value: String,
    size: Pixels,
    font: Option<Font>,
    centered: bool,
}

impl SingleLineLabel {
    pub(crate) fn new(value: String, size: f32) -> Self {
        const LINE_BREAKS: [char; 7] =
            ['\n', '\r', '\x0b', '\x0c', '\u{85}', '\u{2028}', '\u{2029}'];
        let value = if value.contains(LINE_BREAKS) {
            value.replace("\r\n", " ").replace(LINE_BREAKS, " ")
        } else {
            value
        };
        Self { value, size: Pixels(size), font: None, centered: false }
    }

    pub(crate) fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    pub(crate) fn centered(mut self) -> Self {
        self.centered = true;
        self
    }
}

struct State {
    label: String,
    width: f32,
    size: Pixels,
    font: Font,
    paragraph: Paragraph,
}

impl<Message> Widget<Message, Theme, Renderer> for SingleLineLabel {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State {
            label: String::new(),
            width: f32::NAN,
            size: Pixels(0.0),
            font: Font::DEFAULT,
            paragraph: Paragraph::default(),
        })
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let width = limits.max().width;
        let size = self.size;
        let line_height = text::LineHeight::default();
        let height = f32::from(line_height.to_absolute(size));
        let state = tree.state.downcast_mut::<State>();
        let font = self.font.unwrap_or_else(|| renderer.default_font());
        if state.width != width
            || state.size != size
            || state.font != font
            || state.label != self.value
        {
            let format = text::Text {
                content: self.value.as_str(),
                bounds: Size::new(f32::INFINITY, height),
                size,
                line_height,
                font,
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::None,
            };
            let full = Paragraph::with_text(format);
            state.paragraph = if full.min_width() <= width {
                full
            } else {
                let cuts: Vec<_> = self.value.char_indices().map(|(index, _)| index).collect();
                let mut shortest = Paragraph::with_text(format.with_content("…"));
                if shortest.min_width() > width {
                    shortest = Paragraph::default();
                } else {
                    let mut low = 1;
                    let mut high = cuts.len();
                    while low < high {
                        let mid = low + (high - low) / 2;
                        let candidate = format!("{}…", &self.value[..cuts[mid]]);
                        let paragraph =
                            Paragraph::with_text(format.with_content(candidate.as_str()));
                        if paragraph.min_width() <= width {
                            shortest = paragraph;
                            low = mid + 1;
                        } else {
                            high = mid;
                        }
                    }
                }
                shortest
            };
            state.label.clone_from(&self.value);
            state.width = width;
            state.size = size;
            state.font = font;
        }
        layout::Node::new(limits.resolve(Length::Fill, Length::Shrink, Size::new(width, height)))
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        if let Some(clip) = bounds.intersection(viewport) {
            let paragraph = &tree.state.downcast_ref::<State>().paragraph;
            let mut position = layout.position();
            if self.centered {
                position.x += (bounds.width - paragraph.min_width()) / 2.0;
            }
            renderer.fill_paragraph(paragraph, position, style.text_color, clip);
        }
    }
}

impl<Message> From<SingleLineLabel> for Element<'_, Message> {
    fn from(label: SingleLineLabel) -> Self {
        Self::new(label)
    }
}

#[cfg(test)]
#[path = "single_line_label_tests.rs"]
mod tests;
