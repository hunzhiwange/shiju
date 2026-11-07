//! Single-line labels are measured in the same font used to draw them.

use super::SETTINGS_CONTROL_TEXT_SIZE;
use iced::advanced::text::{self, Paragraph as _, Renderer as _};
use iced::advanced::{Layout, Widget, layout, mouse, renderer, widget};
use iced::{Element, Font, Length, Pixels, Rectangle, Renderer, Size, Theme, alignment};

type Paragraph = <Renderer as text::Renderer>::Paragraph;

pub(super) struct Label(String);

impl Label {
    pub(super) fn new(value: String) -> Self {
        Self(if value.contains(['\n', '\r']) { value.replace(['\n', '\r'], " ") } else { value })
    }
}

struct State {
    label: String,
    width: f32,
    font: Font,
    paragraph: Paragraph,
}

impl<Message> Widget<Message, Theme, Renderer> for Label {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State {
            label: String::new(),
            width: f32::NAN,
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
        let size = Pixels(SETTINGS_CONTROL_TEXT_SIZE);
        let line_height = text::LineHeight::default();
        let height = f32::from(line_height.to_absolute(size));
        let state = tree.state.downcast_mut::<State>();
        let font = renderer.default_font();
        if state.width != width || state.font != font || state.label != self.0 {
            let format = text::Text {
                content: self.0.as_str(),
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
                let cuts: Vec<_> = self.0.char_indices().map(|(index, _)| index).collect();
                let mut shortest = Paragraph::with_text(format.with_content("…"));
                if shortest.min_width() > width {
                    shortest = Paragraph::default();
                } else {
                    let mut low = 1;
                    let mut high = cuts.len();
                    while low < high {
                        let mid = low + (high - low) / 2;
                        let candidate = format!("{}…", &self.0[..cuts[mid]]);
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
            state.label.clone_from(&self.0);
            state.width = width;
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
        if let Some(clip) = layout.bounds().intersection(viewport) {
            renderer.fill_paragraph(
                &tree.state.downcast_ref::<State>().paragraph,
                layout.position(),
                style.text_color,
                clip,
            );
        }
    }
}

impl<Message> From<Label> for Element<'_, Message> {
    fn from(label: Label) -> Self {
        Self::new(label)
    }
}
