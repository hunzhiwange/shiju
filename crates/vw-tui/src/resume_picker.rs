//! VibeWindow session picker: generic selection UI over bridge-provided session rows.
use crate::bottom_pane::list_selection_view::{
    ListSelectionView, SelectionItem, SelectionViewParams,
};
use crate::render::renderable::Renderable;
use crate::vibe::action::{V3Action, V3SelectionItem};
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
pub(crate) struct ResumePicker {
    view: ListSelectionView,
}
impl ResumePicker {
    pub(crate) fn new(items: Vec<V3SelectionItem>) -> Self {
        Self {
            view: ListSelectionView::new(SelectionViewParams {
                surface: "resume".into(),
                title: Some("Resume session".into()),
                is_searchable: true,
                search_placeholder: Some("filter sessions".into()),
                items: items.into_iter().map(SelectionItem::from_v3).collect(),
                ..Default::default()
            }),
        }
    }
    pub(crate) fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        self.view.handle_key_event(event)
    }
    pub(crate) fn handle_paste(&mut self, text: &str) {
        self.view.handle_paste(text);
    }
    pub(crate) fn selected_index(&self) -> Option<usize> {
        self.view.selected_index()
    }
}
impl Renderable for ResumePicker {
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
    fn resume_picker_is_data_driven_and_cancelable() {
        let mut picker = ResumePicker::new(vec![V3SelectionItem::new("thread-1", "Fix parser")]);
        assert_eq!(picker.handle_key_event(KeyEvent::from(KeyCode::Esc)), V3Action::Cancel);
    }
}
