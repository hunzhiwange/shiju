//! VibeWindow theme picker over bridge-provided theme rows.
//! Theme application and persistence stay outside this crate; confirmation is a V3Action.
use crate::bottom_pane::list_selection_view::{
    ListSelectionView, SelectionItem, SelectionViewParams,
};
use crate::render::renderable::Renderable;
use crate::vibe::action::{V3Action, V3SelectionItem};
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Widget;
struct ThemePreview;
impl Renderable for ThemePreview {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        Line::from("Preview is supplied by VibeWindow").render(area, buf);
    }
    fn desired_height(&self, _width: u16) -> u16 {
        1
    }
}
pub(crate) struct ThemePicker {
    view: ListSelectionView,
}
impl ThemePicker {
    pub(crate) fn new(items: Vec<V3SelectionItem>) -> Self {
        Self {
            view: ListSelectionView::new(SelectionViewParams {
                surface: "theme".into(),
                title: Some("Theme".into()),
                items: items.into_iter().map(SelectionItem::from_v3).collect(),
                side_content: Box::new(ThemePreview),
                side_content_min_width: 20,
                ..Default::default()
            }),
        }
    }
    pub(crate) fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        self.view.handle_key_event(event)
    }
}
impl Renderable for ThemePicker {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.view.render(area, buf);
    }
    fn desired_height(&self, width: u16) -> u16 {
        self.view.desired_height(width)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;
    #[test]
    fn theme_picker_confirms_without_persisting_configuration() {
        let mut picker = ThemePicker::new(vec![V3SelectionItem::new("dark", "Dark")]);
        assert_eq!(
            picker.handle_key_event(KeyEvent::from(KeyCode::Enter)),
            V3Action::Select { surface: "theme".into(), item_id: "dark".into() }
        );
    }
}
