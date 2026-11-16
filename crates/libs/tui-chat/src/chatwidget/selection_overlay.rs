//! 产品中立的通用选择覆盖层。
//!
//! 调用方只提供 surface、稳定条目 ID 与展示文本；该层不解析业务对象或执行副作用。

use tui_widgets::list_selection_view::{
    ListSelectionView, SelectionItem, SelectionViewParams,
};
use tui_render::renderable::Renderable;
use tui_contracts::action::{V3Action, V3SelectionItem};
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

pub(super) struct SelectionOverlay {
    view: ListSelectionView,
    pub(super) surface: String,
}

impl SelectionOverlay {
    pub(super) fn new(
        surface: impl Into<String>,
        title: impl Into<String>,
        items: Vec<V3SelectionItem>,
    ) -> Self {
        let surface = surface.into();
        Self {
            surface: surface.clone(),
            view: ListSelectionView::new(SelectionViewParams {
                surface,
                title: Some(title.into()),
                items: items.into_iter().map(SelectionItem::from_v3).collect(),
                is_searchable: true,
                search_placeholder: Some("Search".to_string()),
                ..SelectionViewParams::default()
            }),
        }
    }

    pub(super) fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        self.view.handle_key_event(event)
    }

    pub(super) fn handle_paste(&mut self, text: &str) {
        self.view.handle_paste(text);
    }
}

impl Renderable for SelectionOverlay {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.view.render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.view.desired_height(width)
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.view.cursor_pos(area)
    }
}
