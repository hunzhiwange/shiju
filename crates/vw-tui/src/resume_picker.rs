//! VibeWindow 会话选择器：在桥接层提供的会话行上复用通用选择界面。
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
    /// 通用列表负责搜索、选中状态、键盘输入和布局计算。
    view: ListSelectionView,
}
impl ResumePicker {
    pub(crate) fn new(items: Vec<V3SelectionItem>) -> Self {
        // 会话 ID 保持为稳定选择值，标题只承担展示和搜索用途。
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
    // 数据驱动的会话项必须保留通用列表的 Esc 取消语义。
    fn resume_picker_is_data_driven_and_cancelable() {
        let mut picker = ResumePicker::new(vec![V3SelectionItem::new("thread-1", "Fix parser")]);
        assert_eq!(picker.handle_key_event(KeyEvent::from(KeyCode::Esc)), V3Action::Cancel);
    }
}
