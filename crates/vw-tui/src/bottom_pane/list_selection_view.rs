//! 本模块集中实现 `list_selection_view` 在底部面板中的职责。
//! 关键声明 `MAX_POPUP_ROWS`、`MENU_HORIZONTAL_INSET`、`SideContentWidth`、`SelectionRowDisplay` 共同界定本文件的对外交互面。
//! 状态读取与更新均沿用既有类型，不在渲染路径中引入额外业务副作用。
//! 交互分支通过明确枚举或返回值向上层传递选择结果。
//! 空数据、越界索引与不可用选项按现有分支保持可预期的回退。
//! 布局计算以当前可用区域为边界，避免向小终端尺寸外绘制。
//! 键盘输入只在当前界面状态内解释，未匹配输入继续遵循上层约定。
//! 取消与关闭路径保持已有返回语义，不暗中确认待处理操作。
//! 选择、滚动与焦点值在使用前由原有逻辑约束到有效范围。


//! 从 Codex 复制的通用选择表面，由 VibeWindow 拥有。
//! Generic selection surface copied from Codex and owned by VibeWindow.
//!
//! This module contains no command lookup, session I/O, file search, theme persistence, or
//! configuration writes. The bridge supplies rows and consumes V3Action.
use crate::bottom_pane::scroll_state::ScrollState;
use crate::bottom_pane::selection_popup_common::{
    ColumnWidthConfig, ColumnWidthMode, GenericDisplayRow, measure_rows_height_with_col_width_mode,
    render_menu_surface, render_rows_single_line_with_col_width_mode,
    render_rows_with_col_width_mode,
};
use crate::bottom_pane::selection_row_layout::SelectionDescriptionLayout;
use crate::bottom_pane::selection_tabs::{SelectionTab, render_tab_bar, tab_bar_height};
use crate::key_hint::is_plain_text_key_event;
use crate::render::renderable::{ColumnRenderable, Renderable};
use crate::vibe::action::{V3Action, V3SelectionItem};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;
const MAX_POPUP_ROWS: usize = 8;
const MENU_HORIZONTAL_INSET: u16 = 2;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SideContentWidth {
    #[default]
    None,
    Fixed(u16),
    Half,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SelectionRowDisplay {
    #[default]
    Wrapped,
    SingleLine,
}
#[derive(Default)]
pub(crate) struct SelectionItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub search_value: Option<String>,
    pub is_current: bool,
    pub is_default: bool,
    pub is_disabled: bool,
    pub disabled_reason: Option<String>,
    pub name_prefix_spans: Vec<Span<'static>>,
    pub display_shortcut: Option<crate::key_hint::ShortcutHint>,
    pub toggle: Option<SelectionToggle>,
    pub disabled_gutter_marker: Option<&'static str>,
}
impl SelectionItem {
    pub(crate) fn from_v3(item: V3SelectionItem) -> Self {
        Self {
            id: item.id,
            name: item.label,
            description: item.description,
            search_value: item.search_value,
            is_current: item.is_current,
            is_default: item.is_default,
            is_disabled: item.disabled,
            disabled_reason: item.disabled_reason,
            ..Self::default()
        }
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SelectionToggle {
    pub is_on: bool,
}
pub(crate) struct SelectionViewParams {
    pub surface: String,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub footer_note: Option<Line<'static>>,
    pub footer_hint: Option<Line<'static>>,
    pub items: Vec<SelectionItem>,
    pub tabs: Vec<SelectionTab>,
    pub initial_tab_id: Option<String>,
    pub is_searchable: bool,
    pub search_placeholder: Option<String>,
    pub col_width_mode: ColumnWidthMode,
    pub row_display: SelectionRowDisplay,
    pub description_layout: SelectionDescriptionLayout,
    pub name_column_width: Option<usize>,
    pub header: Box<dyn Renderable>,
    pub initial_selected_idx: Option<usize>,
    pub side_content: Box<dyn Renderable>,
    pub side_content_width: SideContentWidth,
    pub side_content_min_width: u16,
    pub stacked_side_content: Option<Box<dyn Renderable>>,
}
impl Default for SelectionViewParams {
    fn default() -> Self {
        Self {
            surface: "selection".into(),
            title: None,
            subtitle: None,
            footer_note: None,
            footer_hint: None,
            items: Vec::new(),
            tabs: Vec::new(),
            initial_tab_id: None,
            is_searchable: false,
            search_placeholder: None,
            col_width_mode: ColumnWidthMode::AutoVisible,
            row_display: SelectionRowDisplay::Wrapped,
            description_layout: SelectionDescriptionLayout::Columns,
            name_column_width: None,
            header: Box::new(()),
            initial_selected_idx: None,
            side_content: Box::new(()),
            side_content_width: SideContentWidth::None,
            side_content_min_width: 0,
            stacked_side_content: None,
        }
    }
}
pub(crate) struct ListSelectionView {
    surface: String,
    footer_note: Option<Line<'static>>,
    footer_hint: Option<Line<'static>>,
    items: Vec<SelectionItem>,
    tabs: Vec<SelectionTab>,
    active_tab_idx: Option<usize>,
    state: ScrollState,
    search_query: String,
    search_placeholder: Option<String>,
    is_searchable: bool,
    col_width: ColumnWidthConfig,
    row_display: SelectionRowDisplay,
    header: Box<dyn Renderable>,
    initial_selected_idx: Option<usize>,
    side_content: Box<dyn Renderable>,
    side_content_width: SideContentWidth,
    side_content_min_width: u16,
    stacked_side_content: Option<Box<dyn Renderable>>,
    last_selected_id: Option<String>,
}
impl ListSelectionView {
    pub(crate) fn new(params: SelectionViewParams) -> Self {
        let active_tab_idx = if params.tabs.is_empty() {
            None
        } else {
            Some(
                params
                    .initial_tab_id
                    .as_ref()
                    .and_then(|id| params.tabs.iter().position(|tab| &tab.id == id))
                    .unwrap_or(0),
            )
        };
        let mut header = params.header;
        if params.title.is_some() || params.subtitle.is_some() {
            header = Box::new(ColumnRenderable::with([
                header,
                Box::new(params.title.clone().map(|value| Line::from(value.bold()))),
                Box::new(params.subtitle.clone().map(|value| Line::from(value.dim()))),
            ]));
        }
        let mut view = Self {
            surface: params.surface,
            footer_note: params.footer_note,
            footer_hint: params.footer_hint,
            items: params.items,
            tabs: params.tabs,
            active_tab_idx,
            state: ScrollState::new(),
            search_query: String::new(),
            search_placeholder: params.search_placeholder,
            is_searchable: params.is_searchable,
            col_width: ColumnWidthConfig::new(params.col_width_mode, params.name_column_width)
                .with_description_layout(params.description_layout),
            row_display: params.row_display,
            header,
            initial_selected_idx: params.initial_selected_idx,
            side_content: params.side_content,
            side_content_width: params.side_content_width,
            side_content_min_width: params.side_content_min_width,
            stacked_side_content: params.stacked_side_content,
            last_selected_id: None,
        };
        view.apply_filter();
        view
    }
    pub(crate) fn from_v3(
        surface: impl Into<String>,
        title: impl Into<String>,
        items: Vec<V3SelectionItem>,
    ) -> Self {
        Self::new(SelectionViewParams {
            surface: surface.into(),
            title: Some(title.into()),
            items: items.into_iter().map(SelectionItem::from_v3).collect(),
            ..SelectionViewParams::default()
        })
    }
    fn active_items(&self) -> &[SelectionItem] {
        self.active_tab_idx
            .and_then(|idx| self.tabs.get(idx))
            .map(|tab| tab.items.as_slice())
            .unwrap_or(&self.items)
    }
    fn active_items_mut(&mut self) -> &mut [SelectionItem] {
        if let Some(idx) = self.active_tab_idx
            && let Some(tab) = self.tabs.get_mut(idx)
        {
            return &mut tab.items;
        }
        &mut self.items
    }
    fn filtered_indices(&self) -> Vec<usize> {
        let query = self.search_query.to_lowercase();
        self.active_items()
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                query.is_empty()
                    || item
                        .search_value
                        .as_deref()
                        .unwrap_or(&item.name)
                        .to_lowercase()
                        .contains(&query)
            })
            .map(|(idx, _)| idx)
            .collect()
    }
    fn enabled(item: &SelectionItem) -> bool {
        !item.is_disabled && item.disabled_reason.is_none()
    }
    fn apply_filter(&mut self) {
        let indices = self.filtered_indices();
        let preferred = self
            .initial_selected_idx
            .take()
            .or_else(|| self.state.selected_idx.and_then(|idx| indices.get(idx).copied()))
            .or_else(|| {
                self.active_items().iter().position(|item| item.is_current && Self::enabled(item))
            });
        self.state.selected_idx =
            preferred.and_then(|actual| indices.iter().position(|idx| *idx == actual));
        if self.state.selected_idx.is_none() {
            self.state.selected_idx = indices
                .iter()
                .position(|idx| self.active_items().get(*idx).is_some_and(Self::enabled));
        }
        self.state.clamp_selection(indices.len());
        self.state.ensure_visible(indices.len(), MAX_POPUP_ROWS.min(indices.len().max(1)));
    }
    fn selected_actual_idx(&self) -> Option<usize> {
        self.state.selected_idx.and_then(|idx| self.filtered_indices().get(idx).copied())
    }
    fn selected_item(&self) -> Option<&SelectionItem> {
        self.selected_actual_idx().and_then(|idx| self.active_items().get(idx))
    }
    fn skip_disabled(&mut self, down: bool, indices: &[usize]) {
        for _ in 0..indices.len() {
            let disabled = self
                .state
                .selected_idx
                .and_then(|idx| indices.get(idx))
                .and_then(|idx| self.active_items().get(*idx))
                .is_some_and(|item| !Self::enabled(item));
            if !disabled {
                break;
            }
            if down {
                self.state.move_down_wrap(indices.len());
            } else {
                self.state.move_up_wrap(indices.len());
            }
        }
    }
    fn move_selection(&mut self, down: bool) {
        let indices = self.filtered_indices();
        if down {
            self.state.move_down_wrap(indices.len());
        } else {
            self.state.move_up_wrap(indices.len());
        }
        self.skip_disabled(down, &indices);
        self.state.ensure_visible(indices.len(), MAX_POPUP_ROWS.min(indices.len().max(1)));
    }
    fn switch_tab(&mut self, delta: isize) {
        let Some(current) = self.active_tab_idx else { return };
        if self.tabs.is_empty() {
            return;
        }
        let len = self.tabs.len() as isize;
        self.active_tab_idx = Some((current as isize + delta).rem_euclid(len) as usize);
        self.search_query.clear();
        self.state.reset();
        self.apply_filter();
    }
    fn accept(&mut self) -> V3Action {
        let Some(item) = self.selected_item() else { return V3Action::Cancel };
        if !Self::enabled(item) {
            return V3Action::None;
        }
        let item_id = item.id.clone();
        self.last_selected_id = Some(item_id.clone());
        V3Action::Select { surface: self.surface.clone(), item_id }
    }
    pub(crate) fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        let plain = is_plain_text_key_event(event);
        match event.code {
            KeyCode::Up | KeyCode::Char('k') if !self.is_searchable || !plain => {
                self.move_selection(false);
                V3Action::None
            }
            KeyCode::Down | KeyCode::Char('j') if !self.is_searchable || !plain => {
                self.move_selection(true);
                V3Action::None
            }
            KeyCode::PageUp => {
                let len = self.filtered_indices().len();
                self.state.page_up_clamped(len, MAX_POPUP_ROWS);
                V3Action::None
            }
            KeyCode::PageDown => {
                let len = self.filtered_indices().len();
                self.state.page_down_clamped(len, MAX_POPUP_ROWS);
                V3Action::None
            }
            KeyCode::Home => {
                let len = self.filtered_indices().len();
                self.state.jump_top(len, MAX_POPUP_ROWS);
                V3Action::None
            }
            KeyCode::End => {
                let len = self.filtered_indices().len();
                self.state.jump_bottom(len, MAX_POPUP_ROWS);
                V3Action::None
            }
            KeyCode::Left if !self.is_searchable || !plain => {
                self.switch_tab(-1);
                V3Action::None
            }
            KeyCode::Right if !self.is_searchable || !plain => {
                self.switch_tab(1);
                V3Action::None
            }
            KeyCode::Backspace if self.is_searchable => {
                self.search_query.pop();
                self.apply_filter();
                V3Action::None
            }
            KeyCode::Esc if self.is_searchable && !self.search_query.is_empty() => {
                self.search_query.clear();
                self.apply_filter();
                V3Action::None
            }
            KeyCode::Esc => V3Action::Cancel,
            KeyCode::Enter => self.accept(),
            KeyCode::Char(' ') => {
                let Some(idx) = self.selected_actual_idx() else { return V3Action::None };
                let can_toggle =
                    self.active_items().get(idx).and_then(|item| item.toggle.as_ref()).is_some();
                if can_toggle
                    && let Some(toggle) =
                        self.active_items_mut().get_mut(idx).and_then(|item| item.toggle.as_mut())
                {
                    toggle.is_on = !toggle.is_on;
                }
                V3Action::None
            }
            KeyCode::Char(ch)
                if self.is_searchable
                    && !event.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.search_query.push(ch);
                self.apply_filter();
                V3Action::None
            }
            _ => V3Action::None,
        }
    }
    pub(crate) fn handle_paste(&mut self, pasted: &str) {
        if self.is_searchable {
            let query = pasted.trim();
            if !query.is_empty() {
                self.search_query.push_str(query);
                self.apply_filter();
            }
        }
    }
    pub(crate) fn selected_index(&self) -> Option<usize> {
        self.selected_actual_idx()
    }
    pub(crate) fn take_last_selected_id(&mut self) -> Option<String> {
        self.last_selected_id.take()
    }
    fn rows(&self) -> Vec<GenericDisplayRow> {
        let indices = self.filtered_indices();
        let selected = self.state.selected_idx;
        indices
            .iter()
            .enumerate()
            .filter_map(|(visible_idx, actual_idx)| {
                self.active_items().get(*actual_idx).map(|item| {
                    let disabled = !Self::enabled(item);
                    let prefix = if selected == Some(visible_idx) { "› " } else { "  " };
                    let prefix = if disabled {
                        format!("{prefix}   ")
                    } else {
                        format!("{prefix}{}. ", visible_idx + 1)
                    };
                    let marker = if item.is_current {
                        " (current)"
                    } else if item.is_default {
                        " (default)"
                    } else {
                        ""
                    };
                    GenericDisplayRow {
                        name: format!("{}{}", item.name, marker),
                        name_prefix_spans: vec![prefix.clone().into()],
                        display_shortcut: item.display_shortcut,
                        description: item.description.clone(),
                        disabled_reason: item.disabled_reason.clone(),
                        is_disabled: disabled,
                        wrap_indent: Some(UnicodeWidthStr::width(prefix.as_str())),
                        ..Default::default()
                    }
                })
            })
            .collect()
    }
    fn content_height(&self, width: u16) -> u16 {
        let rows = self.rows();
        let width = width.saturating_sub(MENU_HORIZONTAL_INSET * 2);
        let list_height = match self.row_display {
            SelectionRowDisplay::Wrapped => measure_rows_height_with_col_width_mode(
                &rows,
                &self.state,
                MAX_POPUP_ROWS,
                width,
                self.col_width,
            ),
            SelectionRowDisplay::SingleLine => rows.len().clamp(1, MAX_POPUP_ROWS) as u16,
        };
        let tabs =
            self.active_tab_idx.map(|idx| tab_bar_height(&self.tabs, idx, width)).unwrap_or(0);
        self.header.desired_height(width)
            + tabs
            + u16::from(self.is_searchable)
            + list_height
            + u16::from(self.footer_note.is_some())
            + u16::from(self.footer_hint.is_some())
            + 1
    }
}
impl Renderable for ListSelectionView {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let content = render_menu_surface(area, buf);
        if content.is_empty() {
            return;
        }
        let rows = self.rows();
        let tabs_height = self
            .active_tab_idx
            .map(|idx| tab_bar_height(&self.tabs, idx, content.width))
            .unwrap_or(0);
        let header_height = self.header.desired_height(content.width);
        let search_height = u16::from(self.is_searchable);
        let footer_height =
            u16::from(self.footer_note.is_some()) + u16::from(self.footer_hint.is_some()) + 1;
        let list_height = content
            .height
            .saturating_sub(header_height + tabs_height + search_height + footer_height);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(header_height),
                Constraint::Length(tabs_height),
                Constraint::Length(search_height),
                Constraint::Min(list_height),
                Constraint::Length(footer_height),
            ])
            .split(content);
        self.header.render(chunks[0], buf);
        if let Some(idx) = self.active_tab_idx {
            render_tab_bar(&self.tabs, idx, chunks[1], buf);
        }
        if self.is_searchable && chunks[2].height > 0 {
            let query = if self.search_query.is_empty() {
                self.search_placeholder.as_deref().unwrap_or("Search")
            } else {
                &self.search_query
            };
            Line::from(vec!["/ ".dim(), query.to_string().into()]).render(chunks[2], buf);
        }
        match self.row_display {
            SelectionRowDisplay::Wrapped => {
                render_rows_with_col_width_mode(
                    chunks[3],
                    buf,
                    &rows,
                    &self.state,
                    MAX_POPUP_ROWS,
                    "no matches",
                    self.col_width,
                );
            }
            SelectionRowDisplay::SingleLine => {
                render_rows_single_line_with_col_width_mode(
                    chunks[3],
                    buf,
                    &rows,
                    &self.state,
                    MAX_POPUP_ROWS,
                    "no matches",
                    self.col_width,
                );
            }
        }
        if chunks[4].height > 0 {
            let mut y = chunks[4].y;
            if let Some(note) = &self.footer_note {
                note.clone().dim().render(Rect::new(chunks[4].x, y, chunks[4].width, 1), buf);
                y = y.saturating_add(1);
            }
            if let Some(hint) = &self.footer_hint {
                hint.clone().dim().render(Rect::new(chunks[4].x, y, chunks[4].width, 1), buf);
            }
        }
    }
    fn desired_height(&self, width: u16) -> u16 {
        self.content_height(width).saturating_add(2)
    }
}
impl From<V3SelectionItem> for SelectionItem {
    fn from(value: V3SelectionItem) -> Self {
        Self::from_v3(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::buffer::Buffer;
    fn render(view: &ListSelectionView, width: u16) -> String {
        let area = Rect::new(0, 0, width, view.desired_height(width));
        let mut buf = Buffer::empty(area);
        view.render(area, &mut buf);
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn items() -> Vec<V3SelectionItem> {
        vec![
            V3SelectionItem {
                id: "read".into(),
                label: "Read only".into(),
                description: Some("Inspect files".into()),
                is_current: true,
                ..Default::default()
            },
            V3SelectionItem {
                id: "write".into(),
                label: "Full access".into(),
                description: Some("Modify files".into()),
                ..Default::default()
            },
            V3SelectionItem {
                id: "locked".into(),
                label: "Unavailable".into(),
                disabled: true,
                disabled_reason: Some("not configured".into()),
                ..Default::default()
            },
        ]
    }
    // 验证 `v3_selection_surfaces_reuse_copied_codex_list_view` 场景的触发条件与预期结果。
    #[test]
    fn v3_selection_surfaces_reuse_copied_codex_list_view() {
        let mut view = ListSelectionView::from_v3("permissions", "Permissions", items());
        assert_eq!(view.handle_key_event(KeyEvent::from(KeyCode::Down)), V3Action::None);
        assert_eq!(
            view.handle_key_event(KeyEvent::from(KeyCode::Enter)),
            V3Action::Select { surface: "permissions".into(), item_id: "write".into() }
        );
        assert_snapshot!(render(&view, 48), @r###"
Permissions
  1. Read only (current)     Inspect files
› 2. Full access             Modify files
     Unavailable (disabled)  disabled: not
     configured
"###);
    }
    // 验证 `filtering_skips_disabled_rows_and_empty_state_is_rendered` 场景的触发条件与预期结果。
    #[test]
    fn filtering_skips_disabled_rows_and_empty_state_is_rendered() {
        let mut view = ListSelectionView::new(SelectionViewParams {
            surface: "search".into(),
            title: Some("Search".into()),
            is_searchable: true,
            items: items().into_iter().map(SelectionItem::from_v3).collect(),
            ..Default::default()
        });
        view.handle_key_event(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        assert_eq!(view.selected_index(), None);
        assert!(render(&view, 40).contains("no matches"));
    }
}
