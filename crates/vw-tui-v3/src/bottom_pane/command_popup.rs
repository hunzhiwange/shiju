//! Command completion surface. VibeWindow supplies command rows; this module only filters and renders.
use crate::bottom_pane::list_selection_view::SelectionItem;
use crate::bottom_pane::scroll_state::ScrollState;
use crate::bottom_pane::selection_popup_common::{
    ColumnWidthConfig, ColumnWidthMode, GenericDisplayRow, measure_rows_height_with_col_width_mode,
    render_rows_with_col_width_mode,
};
use crate::vibe::action::{V3Action, V3SelectionItem};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::WidgetRef;
const MAX_POPUP_ROWS: usize = 8;
const COMMAND_COLUMN_WIDTH: ColumnWidthConfig =
    ColumnWidthConfig::new(ColumnWidthMode::AutoAllRows, None);
pub(crate) type CommandItem = V3SelectionItem;
pub(crate) struct CommandPopup {
    command_filter: String,
    commands: Vec<V3SelectionItem>,
    state: ScrollState,
}
impl CommandPopup {
    pub(crate) fn new(commands: Vec<V3SelectionItem>) -> Self {
        let mut popup = Self { command_filter: String::new(), commands, state: ScrollState::new() };
        popup.reset_selection();
        popup
    }
    pub(crate) fn from_selection_items(commands: Vec<SelectionItem>) -> Self {
        Self::new(
            commands
                .into_iter()
                .map(|item| V3SelectionItem {
                    id: item.id,
                    label: item.name,
                    description: item.description,
                    search_value: item.search_value,
                    disabled: item.is_disabled,
                    disabled_reason: item.disabled_reason,
                    is_current: item.is_current,
                    is_default: item.is_default,
                })
                .collect(),
        )
    }
    pub(crate) fn on_composer_text_change(&mut self, text: String) {
        let first_line = text.lines().next().unwrap_or("");
        self.command_filter = first_line
            .strip_prefix('/')
            .unwrap_or(first_line)
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_lowercase();
        self.reset_selection();
    }
    fn filtered(&self) -> Vec<(usize, Option<Vec<usize>>)> {
        let filter = self.command_filter.as_str();
        self.commands
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                let name = item.label.to_lowercase();
                let id = item.id.to_lowercase();
                if filter.is_empty() {
                    return Some((idx, None));
                }
                name.find(filter)
                    .or_else(|| id.find(filter))
                    .map(|start| (idx, Some((start..start + filter.chars().count()).collect())))
            })
            .collect()
    }
    fn reset_selection(&mut self) {
        let len = self.filtered().len();
        self.state.reset();
        self.state.clamp_selection(len);
        self.state.ensure_visible(len, MAX_POPUP_ROWS.min(len.max(1)));
    }
    fn rows(&self) -> Vec<GenericDisplayRow> {
        self.filtered()
            .into_iter()
            .map(|(idx, matches)| {
                let item = &self.commands[idx];
                GenericDisplayRow {
                    name: item.label.clone(),
                    description: item.description.clone(),
                    match_indices: matches,
                    is_disabled: item.disabled || item.disabled_reason.is_some(),
                    disabled_reason: item.disabled_reason.clone(),
                    ..Default::default()
                }
            })
            .collect()
    }
    pub(crate) fn selected_item(&self) -> Option<&V3SelectionItem> {
        let index = self.filtered().get(self.state.selected_idx?)?.0;
        self.commands.get(index)
    }
    pub(crate) fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        let len = self.filtered().len();
        match event.code {
            KeyCode::Up | KeyCode::Char('k') => self.state.move_up_wrap(len),
            KeyCode::Down | KeyCode::Char('j') => self.state.move_down_wrap(len),
            KeyCode::PageUp => self.state.page_up_clamped(len, MAX_POPUP_ROWS),
            KeyCode::PageDown => self.state.page_down_clamped(len, MAX_POPUP_ROWS),
            KeyCode::Enter => {
                if let Some(item) = self
                    .selected_item()
                    .filter(|item| !item.disabled && item.disabled_reason.is_none())
                {
                    return V3Action::Select {
                        surface: "command_palette".into(),
                        item_id: item.id.clone(),
                    };
                }
            }
            KeyCode::Esc => return V3Action::Cancel,
            _ => {}
        }
        self.state.ensure_visible(len, MAX_POPUP_ROWS.min(len.max(1)));
        V3Action::None
    }
    pub(crate) fn calculate_required_height(&self, width: u16) -> u16 {
        measure_rows_height_with_col_width_mode(
            &self.rows(),
            &self.state,
            MAX_POPUP_ROWS,
            width,
            COMMAND_COLUMN_WIDTH,
        )
    }
}
impl WidgetRef for &CommandPopup {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        render_rows_with_col_width_mode(
            area,
            buf,
            &self.rows(),
            &self.state,
            MAX_POPUP_ROWS,
            "no commands",
            COMMAND_COLUMN_WIDTH,
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_popup_filters_vibe_window_rows() {
        let mut popup = CommandPopup::new(vec![
            V3SelectionItem {
                id: "search".into(),
                label: "Search".into(),
                description: Some("Find text".into()),
                ..Default::default()
            },
            V3SelectionItem {
                id: "settings".into(),
                label: "Settings".into(),
                ..Default::default()
            },
        ]);
        popup.on_composer_text_change("/sea".into());
        assert_eq!(popup.selected_item().map(|item| item.id.as_str()), Some("search"));
    }
    #[test]
    fn command_popup_emits_selection_action_without_dispatching() {
        let mut popup = CommandPopup::new(vec![V3SelectionItem::new("todo", "Todo")]);
        assert_eq!(
            popup.handle_key_event(KeyEvent::from(KeyCode::Enter)),
            V3Action::Select { surface: "command_palette".into(), item_id: "todo".into() }
        );
    }
}
