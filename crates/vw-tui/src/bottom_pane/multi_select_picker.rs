//! 多选择选件控件，用于从列表中多选多个项。
//! Multi-select picker widget for selecting multiple items from a list.
//!
//! This module provides a fuzzy-searchable, scrollable picker that allows users
//! to toggle multiple items on/off. It supports:
//!
//! - **Fuzzy search**: Type to filter items by name
//! - **Toggle selection**: Space to toggle items on/off
//! - **Reordering**: Optional left/right arrow support to reorder items
//! - **Live preview**: Optional callback to show a preview of current selections
//! - **Callbacks**: Hooks for change, confirm, and cancel events
//!
//! # Example
//!
//! ```ignore
//! let picker = MultiSelectPicker::new(
//!     "Select Items".to_string(),
//!     Some("Choose which items to enable".to_string()),
//!     app_event_tx,
//! )
//! .items(vec![
//!     MultiSelectItem {
//!         id: "a".into(),
//!         name: "Item A".into(),
//!         description: None,
//!         enabled: true,
//!         orderable: true,
//!         section_break_after: false,
//!     },
//!     MultiSelectItem {
//!         id: "b".into(),
//!         name: "Item B".into(),
//!         description: None,
//!         enabled: false,
//!         orderable: true,
//!         section_break_after: false,
//!     },
//! ])
//! .on_confirm(|selected_ids, tx| { /* handle confirmation */ })
//! .build();
//! ```

use codex_utils_fuzzy_match::fuzzy_match;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Widget;

use super::selection_popup_common::GenericDisplayRow;
use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::CancellationEvent;
use crate::bottom_pane::bottom_pane_view::BottomPaneView;
use crate::bottom_pane::popup_consts::MAX_POPUP_ROWS;
use crate::bottom_pane::scroll_state::ScrollState;
use crate::bottom_pane::selection_popup_common::render_rows_single_line;
use crate::key_hint;
use crate::key_hint::KeyBindingListExt;
use crate::key_hint::is_plain_text_key_event;
use crate::keymap::ListAction;
use crate::keymap::ListKeymap;
use crate::keymap::RuntimeKeymap;
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::render::Insets;
use crate::render::RectExt;
use crate::render::renderable::ColumnRenderable;
use crate::render::renderable::Renderable;
use crate::style::user_message_style;
use crate::text_formatting::truncate_text;

/// 最大显示长度，用于在截断前保留项目名称。
/// Maximum display length for item names before truncation.
const ITEM_NAME_TRUNCATE_LEN: usize = 21;

/// 当搜索框为空时显示的占位文本
/// Placeholder text shown in the search input when empty.
const SEARCH_PLACEHOLDER: &str = "Type to search";

/// 搜索前显示的预缀文本（模拟命令行界面）
/// Prefix displayed before the search query (mimics a command prompt).
const SEARCH_PROMPT_PREFIX: &str = "> ";

const SECTION_BREAK_ROW: &str = "  ───────────────────────";

/// 列表项重排的方向
/// Direction for reordering items in the list.
enum Direction {
    Up,
    Down,
}

/// 回调函数：当任何项的状态发生变动（切换或重排）时触发。
/// Callback invoked when any item's state changes (toggled or reordered).
/// Receives the full list of items and the event sender.
pub type ChangeCallBack = Box<dyn Fn(&[MultiSelectItem], &AppEventSender) + Send + Sync>;

/// 用户确认其选择时触发回调（按回车键）。 接收所有启用项的 ID 列表。
/// Callback invoked when the user confirms their selection (presses Enter).
/// Receives a list of IDs for all enabled items.
pub type ConfirmCallback = Box<dyn Fn(&[String], &AppEventSender) + Send + Sync>;

/// 用户取消选择器时触发回调（按 Esc 键）
/// Callback invoked when the user cancels the picker (presses Escape).
pub type CancelCallback = Box<dyn Fn(&AppEventSender) + Send + Sync>;

/// 生成当前项目状态的可选预览行的回调函数。
/// Callback to generate an optional preview line based on current item states.
/// Returns `None` to hide the preview area.
pub type PreviewCallback = Box<dyn Fn(&[MultiSelectItem]) -> Option<Line<'static>> + Send + Sync>;

/// 多选择器中的单个可选项
/// A single selectable item in the multi-select picker.
///
/// Each item has a unique identifier, display name, optional description,
/// and an enabled/disabled state that can be toggled by the user.
pub(crate) struct MultiSelectItem {
    /// 启用此项时返回的唯一标识符
    /// Unique identifier returned in the confirm callback when this item is enabled.
    pub id: String,

    /// 显示名称在选择器列表中的内容。如果过长则会被截断。
    /// Display name shown in the picker list. Will be truncated if too long.
    pub name: String,

    /// 名称旁显示的可选描述（暗色）。
    /// Optional description shown alongside the name (dimmed).
    pub description: Option<String>,

    /// 是否当前选中或启用此项。
    /// Whether this item is currently selected/enabled.
    pub enabled: bool,

    /// 当启用订单时，该物品是否可以被移动。
    /// Whether this item can be moved when ordering is enabled.
    pub orderable: bool,

    /// 当可见的另一个项目紧随此项时，是否绘制分隔线。
    /// Whether to draw a divider after this item when another visible item follows.
    pub section_break_after: bool,
}

impl Default for MultiSelectItem {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: None,
            enabled: false,
            orderable: true,
            section_break_after: false,
        }
    }
}

struct BuiltRows {
    rows: Vec<GenericDisplayRow>,
    state: ScrollState,
}

/// 带有模糊搜索和可选重排的多选择选框控件。
/// A multi-select picker widget with fuzzy search and optional reordering.
///
/// The picker displays a scrollable list of items with checkboxes. Users can:
/// - Type to fuzzy-search and filter the list
/// - Use Up/Down (or Ctrl+P/Ctrl+N) to navigate
/// - Press Space to toggle the selected item
/// - Press Enter to confirm and close
/// - Press Escape to cancel and close
/// - Use Left/Right arrows to reorder items (if ordering is enabled)
///
/// Create instances using the builder pattern via [`MultiSelectPicker::new`].
pub(crate) struct MultiSelectPicker {
    /// 所有选中的项目（未过滤）。
    /// All items in the picker (unfiltered).
    items: Vec<MultiSelectItem>,

    /// 可见列表的滚动与选择状态
    /// Scroll and selection state for the visible list.
    state: ScrollState,

    /// 已关闭的选区（确认或取消）
    /// Whether the picker has been closed (confirmed or cancelled).
    pub(crate) complete: bool,

    /// 应用事件发送通道
    /// Channel for sending application events.
    app_event_tx: AppEventSender,

    /// 标题和副标题的头部控件
    /// Header widget displaying title and subtitle.
    header: Box<dyn Renderable>,

    /// 显示键盘提示的脚本报表行
    /// Footer line showing keyboard hints.
    footer_hint: Line<'static>,

    /// 用户当前搜索或过滤查询
    /// Current search/filter query entered by the user.
    search_query: String,

    /// 当前过滤条件匹配到的 `items` 的索引，按显示顺序排列。
    /// Indices into `items` that match the current filter, in display order.
    filtered_indices: Vec<usize>,

    /// 是否启用左右箭头重排功能。
    /// Whether left/right arrow reordering is enabled.
    ordering_enabled: bool,

    /// 导航与完成功能共享的键绑定列表
    /// Shared list keybindings for navigation and completion.
    keymap: ListKeymap,

    /// 生成当前项目状态预览行的可选回调函数
    /// Optional callback to generate a preview line from current item states.
    preview_builder: Option<PreviewCallback>,

    /// 缓存预览行（随项变更而更新）
    /// Cached preview line (updated on item changes).
    preview_line: Option<Line<'static>>,

    /// 回调函数，用于处理物品变更（切换或重排）
    /// Callback invoked when items change (toggle or reorder).
    on_change: Option<ChangeCallBack>,

    /// 用户确认其选择时回调被调用
    /// Callback invoked when the user confirms their selection.
    on_confirm: Option<ConfirmCallback>,

    /// 用户取消选择器时调用的回调函数
    /// Callback invoked when the user cancels the picker.
    on_cancel: Option<CancelCallback>,
}

impl MultiSelectPicker {
    /// 创建一个新的构建器，用于构建 `MultiSelectPicker`。
    /// Creates a new builder for constructing a `MultiSelectPicker`.
    ///
    /// # Arguments
    ///
    /// * `title` - The main title displayed at the top of the picker
    /// * `subtitle` - Optional subtitle displayed below the title (dimmed)
    /// * `app_event_tx` - Event sender for dispatching application events
    pub fn builder(
        title: String,
        subtitle: Option<String>,
        app_event_tx: AppEventSender,
    ) -> MultiSelectPickerBuilder {
        MultiSelectPickerBuilder::new(title, subtitle, app_event_tx)
    }

    /// 应用当前搜索查询以过滤和排序项目。 更新 `filtered_indices` 字段，仅包含匹配项，并按模糊匹配分数排序。尝试在仍匹配过滤条件时保留当前选择。
    /// Applies the current search query to filter and sort items.
    ///
    /// Updates `filtered_indices` to contain only matching items, sorted by
    /// fuzzy match score. Attempts to preserve the current selection if it
    /// still matches the filter.
    fn apply_filter(&mut self) {
        // 在可能时保留当前选择，过滤并排序。
        // Filter + sort while preserving the current selection when possible.
        let previously_selected = self
            .state
            .selected_idx
            .and_then(|visible_idx| self.filtered_indices.get(visible_idx).copied());

        let filter = self.search_query.trim();
        if filter.is_empty() {
            self.filtered_indices = (0..self.items.len()).collect();
        } else {
            let mut matches: Vec<(usize, i32)> = Vec::new();
            for (idx, item) in self.items.iter().enumerate() {
                let display_name = item.name.as_str();
                if let Some((_indices, score)) = match_item(filter, display_name, &item.name) {
                    matches.push((idx, score));
                }
            }

            matches.sort_by(|a, b| {
                a.1.cmp(&b.1).then_with(|| {
                    let an = self.items[a.0].name.as_str();
                    let bn = self.items[b.0].name.as_str();
                    an.cmp(bn)
                })
            });

            self.filtered_indices = matches.into_iter().map(|(idx, _score)| idx).collect();
        }

        let len = self.filtered_indices.len();
        self.state.selected_idx = previously_selected
            .and_then(|actual_idx| {
                self.filtered_indices
                    .iter()
                    .position(|idx| *idx == actual_idx)
            })
            .or_else(|| (len > 0).then_some(0));

        let visible = Self::max_visible_rows(len);
        self.state.clamp_selection(len);
        self.state.ensure_visible(len, visible);
    }

    /// 返回过滤后可见项的数量
    /// Returns the number of items visible after filtering.
    fn visible_len(&self) -> usize {
        self.filtered_indices.len()
    }

    /// 一次最多显示多少行数据
    /// Returns the maximum number of rows that can be displayed at once.
    fn max_visible_rows(len: usize) -> usize {
        MAX_POPUP_ROWS.min(len.max(1))
    }

    /// 计算行内容可用宽度（包含边框）。
    /// Calculates the width available for row content (accounts for borders).
    fn rows_width(total_width: u16) -> u16 {
        total_width.saturating_sub(2)
    }

    /// 计算行列表区域所需的总高度
    /// Calculates the height needed for the row list area.
    fn rows_height(&self, rows: &BuiltRows) -> u16 {
        rows.rows
            .len()
            .clamp(1, MAX_POPUP_ROWS)
            .try_into()
            .unwrap_or(1)
    }

    /// 构建所有当前可见（过滤后）的显示行。 每条行显示：› [x] 项目名称，其中 › 表示光标位置。 [x] 或 [ ] 表示启用/禁用状态。
    /// Builds the display rows for all currently visible (filtered) items.
    ///
    /// Each row shows: `› [x] Item Name` where `›` indicates cursor position
    /// and `[x]` or `[ ]` indicates enabled/disabled state.
    fn build_rows(&self) -> BuiltRows {
        let mut rows = Vec::new();
        let mut visible_to_row = Vec::with_capacity(self.filtered_indices.len());
        for (visible_idx, actual_idx) in self.filtered_indices.iter().enumerate() {
            let Some(item) = self.items.get(*actual_idx) else {
                continue;
            };
            visible_to_row.push(rows.len());
            let is_selected = self.state.selected_idx == Some(visible_idx);
            let prefix = if is_selected { '›' } else { ' ' };
            let marker = if item.enabled { 'x' } else { ' ' };
            let item_name = truncate_text(&item.name, ITEM_NAME_TRUNCATE_LEN);
            let name = format!("{prefix} [{marker}] {item_name}");
            rows.push(GenericDisplayRow {
                name,
                description: item.description.clone(),
                ..Default::default()
            });

            if item.section_break_after && visible_idx + 1 < self.filtered_indices.len() {
                rows.push(GenericDisplayRow {
                    name: SECTION_BREAK_ROW.to_string(),
                    is_disabled: true,
                    ..Default::default()
                });
            }
        }

        let selected_idx = self
            .state
            .selected_idx
            .and_then(|visible_idx| visible_to_row.get(visible_idx).copied());
        let scroll_top = visible_to_row
            .get(self.state.scroll_top)
            .copied()
            .unwrap_or(0);
        BuiltRows {
            rows,
            state: ScrollState {
                selected_idx,
                scroll_top,
            },
        }
    }

    /// 向上移动选择光标，若位于顶部则自动回滚至底部。
    /// Moves the selection cursor up, wrapping to the bottom if at the top.
    fn move_up(&mut self) {
        let len = self.visible_len();
        self.state.move_up_wrap(len);
        let visible = Self::max_visible_rows(len);
        self.state.ensure_visible(len, visible);
    }

    /// 向下移动选择光标，若已位于底部则向上循环。
    /// Moves the selection cursor down, wrapping to the top if at the bottom.
    fn move_down(&mut self) {
        let len = self.visible_len();
        self.state.move_down_wrap(len);
        let visible = Self::max_visible_rows(len);
        self.state.ensure_visible(len, visible);
    }

    fn page_up(&mut self) {
        let len = self.visible_len();
        let visible = Self::max_visible_rows(len);
        self.state.page_up_clamped(len, visible);
    }

    fn page_down(&mut self) {
        let len = self.visible_len();
        let visible = Self::max_visible_rows(len);
        self.state.page_down_clamped(len, visible);
    }

    fn jump_top(&mut self) {
        let len = self.visible_len();
        let visible = Self::max_visible_rows(len);
        self.state.jump_top(len, visible);
    }

    fn jump_bottom(&mut self) {
        let len = self.visible_len();
        let visible = Self::max_visible_rows(len);
        self.state.jump_bottom(len, visible);
    }

    /// 切换当前选中的项目状态。
    /// Toggles the enabled state of the currently selected item.
    ///
    /// Updates the preview line and invokes the `on_change` callback if set.
    fn toggle_selected(&mut self) {
        let Some(idx) = self.state.selected_idx else {
            return;
        };
        let Some(actual_idx) = self.filtered_indices.get(idx).copied() else {
            return;
        };
        let Some(item) = self.items.get_mut(actual_idx) else {
            return;
        };

        item.enabled = !item.enabled;
        self.update_preview_line();
        if let Some(on_change) = &self.on_change {
            on_change(&self.items, &self.app_event_tx);
        }
    }

    /// 确认当前选择并关闭选择器。
    /// Confirms the current selection and closes the picker.
    ///
    /// Collects the IDs of all enabled items and passes them to the
    /// `on_confirm` callback. Does nothing if already complete.
    fn confirm_selection(&mut self) {
        if self.complete {
            return;
        }
        self.complete = true;

        if let Some(on_confirm) = &self.on_confirm {
            let selected_ids: Vec<String> = self
                .items
                .iter()
                .filter(|item| item.enabled)
                .map(|item| item.id.clone())
                .collect();
            on_confirm(&selected_ids, &self.app_event_tx);
        }
    }

    /// 在列表中向上或向下移动当前选中的项目。
    /// Moves the currently selected item up or down in the list.
    ///
    /// Only works when:
    /// - The search query is empty (reordering is disabled during filtering)
    /// - Ordering is enabled via [`MultiSelectPickerBuilder::enable_ordering`]
    ///
    /// Updates the preview line and invokes the `on_change` callback.
    fn move_selected_item(&mut self, direction: Direction) {
        if !self.search_query.is_empty() {
            return;
        }

        let Some(visible_idx) = self.state.selected_idx else {
            return;
        };
        let Some(actual_idx) = self.filtered_indices.get(visible_idx).copied() else {
            return;
        };

        let len = self.items.len();
        if len == 0 {
            return;
        }

        if !self
            .items
            .get(actual_idx)
            .is_some_and(|item| item.orderable)
        {
            return;
        }

        let new_idx = match direction {
            Direction::Up if actual_idx > 0 => actual_idx - 1,
            Direction::Down if actual_idx + 1 < len => actual_idx + 1,
            _ => return,
        };

        if !self.items.get(new_idx).is_some_and(|item| item.orderable) {
            return;
        }

        // 将底层列表中的物品移动
        // move item in underlying list
        self.items.swap(actual_idx, new_idx);

        self.update_preview_line();
        if let Some(on_change) = &self.on_change {
            on_change(&self.items, &self.app_event_tx);
        }

        // 重建过滤索引，以保持搜索与过滤的一致性
        // rebuild filtered indices to keep search/filter consistent
        self.apply_filter();

        // 将选区恢复到已移动的对象
        // restore selection to moved item
        let moved_idx = new_idx;
        if let Some(new_visible_idx) = self
            .filtered_indices
            .iter()
            .position(|idx| *idx == moved_idx)
        {
            self.state.selected_idx = Some(new_visible_idx);
        }
    }

    /// 使用预览回调重新生成预览行。
    /// Regenerates the preview line using the preview callback.
    ///
    /// Called after any item state change (toggle or reorder).
    fn update_preview_line(&mut self) {
        self.preview_line = self
            .preview_builder
            .as_ref()
            .and_then(|builder| builder(&self.items));
    }

    /// 关闭选择器而不确认，并调用 `on_cancel` 回调。
    /// Closes the picker without confirming, invoking the `on_cancel` callback.
    ///
    /// Does nothing if already complete.
    pub fn close(&mut self) {
        if self.complete {
            return;
        }
        self.complete = true;
        if let Some(on_cancel) = &self.on_cancel {
            on_cancel(&self.app_event_tx);
        }
    }
}

impl BottomPaneView for MultiSelectPicker {
    fn keymap_contexts(&self) -> crate::keymap::KeymapContextSet {
        crate::keymap::KeymapContextSet::new(crate::keymap::KeymapContext::List)
    }

    fn is_complete(&self) -> bool {
        self.complete
    }

    fn on_ctrl_c(&mut self) -> CancellationEvent {
        self.close();
        CancellationEvent::Handled
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        // 可打印字符总是用于搜索。移动别名仅通过非文本事件或修改后的绑定生效。
        // Printable characters always feed search. Movement aliases such as
        // plain j/k only apply through non-text events or modified bindings.
        let allow_plain_char_navigation = !is_plain_text_key_event(key_event);

        match key_event {
            _ if allow_plain_char_navigation
                && self.ordering_enabled
                && self.keymap.move_left.is_pressed(key_event) =>
            {
                self.move_selected_item(Direction::Up);
            }
            _ if allow_plain_char_navigation
                && self.ordering_enabled
                && self.keymap.move_right.is_pressed(key_event) =>
            {
                self.move_selected_item(Direction::Down);
            }
            _ if allow_plain_char_navigation && self.keymap.move_up.is_pressed(key_event) => {
                self.move_up()
            }
            _ if allow_plain_char_navigation && self.keymap.move_down.is_pressed(key_event) => {
                self.move_down()
            }
            _ if allow_plain_char_navigation && self.keymap.page_up.is_pressed(key_event) => {
                self.page_up()
            }
            _ if allow_plain_char_navigation && self.keymap.page_down.is_pressed(key_event) => {
                self.page_down()
            }
            _ if allow_plain_char_navigation && self.keymap.jump_top.is_pressed(key_event) => {
                self.jump_top()
            }
            _ if allow_plain_char_navigation && self.keymap.jump_bottom.is_pressed(key_event) => {
                self.jump_bottom()
            }
            KeyEvent {
                code: KeyCode::Backspace,
                ..
            } => {
                self.search_query.pop();
                self.apply_filter();
            }
            KeyEvent {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.toggle_selected(),
            _ if self.keymap.accept.is_pressed(key_event) => self.confirm_selection(),
            _ if self.keymap.cancel.is_pressed(key_event) => self.close(),
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers,
                ..
            } if !modifiers.contains(KeyModifiers::CONTROL)
                && !modifiers.contains(KeyModifiers::ALT) =>
            {
                self.search_query.push(c);
                self.apply_filter();
            }
            _ => {}
        }
    }
}

impl Renderable for MultiSelectPicker {
    fn desired_height(&self, width: u16) -> u16 {
        let rows = self.build_rows();
        let rows_height = self.rows_height(&rows);
        let preview_height = if self.preview_line.is_some() { 1 } else { 0 };

        let mut height = self.header.desired_height(width.saturating_sub(4));
        height = height.saturating_add(rows_height + 3);
        height = height.saturating_add(2);
        height.saturating_add(1 + preview_height)
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        // 为键提示行预留底部一行。
        // Reserve the footer line for the key-hint row.
        let preview_height = if self.preview_line.is_some() { 1 } else { 0 };
        let footer_height = 1 + preview_height;
        let [content_area, footer_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(footer_height)]).areas(area);

        Block::default()
            .style(user_message_style())
            .render(content_area, buf);

        let header_height = self
            .header
            .desired_height(content_area.width.saturating_sub(4));
        let rows = self.build_rows();
        let rows_width = Self::rows_width(content_area.width);
        let rows_height = self.rows_height(&rows);
        let [header_area, _, search_area, list_area] = Layout::vertical([
            Constraint::Max(header_height),
            Constraint::Max(1),
            Constraint::Length(2),
            Constraint::Length(rows_height),
        ])
        .areas(content_area.inset(Insets::vh(/*v*/ 1, /*h*/ 2)));

        self.header.render(header_area, buf);

        // 将搜索提示词渲染为两行，以模拟输入编辑器的风格。
        // Render the search prompt as two lines to mimic the composer.
        if search_area.height >= 2 {
            let [placeholder_area, input_area] =
                Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(search_area);
            Line::from(SEARCH_PLACEHOLDER.dim()).render(placeholder_area, buf);
            let line = if self.search_query.is_empty() {
                Line::from(vec![SEARCH_PROMPT_PREFIX.dim()])
            } else {
                Line::from(vec![
                    SEARCH_PROMPT_PREFIX.dim(),
                    self.search_query.clone().into(),
                ])
            };
            line.render(input_area, buf);
        } else if search_area.height > 0 {
            let query_span = if self.search_query.is_empty() {
                SEARCH_PLACEHOLDER.dim()
            } else {
                self.search_query.clone().into()
            };
            Line::from(query_span).render(search_area, buf);
        }

        if list_area.height > 0 {
            let render_area = Rect {
                x: list_area.x.saturating_sub(2),
                y: list_area.y,
                width: rows_width.max(1),
                height: list_area.height,
            };
            render_rows_single_line(
                render_area,
                buf,
                &rows.rows,
                &rows.state,
                render_area.height as usize,
                "no matches",
            );
        }

        let hint_area = if let Some(preview_line) = &self.preview_line {
            let [preview_area, hint_area] =
                Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(footer_area);
            let preview_area = Rect {
                x: preview_area.x + 2,
                y: preview_area.y,
                width: preview_area.width.saturating_sub(2),
                height: preview_area.height,
            };
            let max_preview_width = preview_area.width.saturating_sub(2) as usize;
            let preview_line =
                truncate_line_with_ellipsis_if_overflow(preview_line.clone(), max_preview_width);
            preview_line.render(preview_area, buf);
            hint_area
        } else {
            footer_area
        };
        let hint_area = Rect {
            x: hint_area.x + 2,
            y: hint_area.y,
            width: hint_area.width.saturating_sub(2),
            height: hint_area.height,
        };
        self.footer_hint.clone().dim().render(hint_area, buf);
    }
}

/// 构建一个具有流畅 API 的 [`MultiSelectPicker`] 构建器。
/// Builder for constructing a [`MultiSelectPicker`] with a fluent API.
///
/// # Example
///
/// ```ignore
/// let picker = MultiSelectPicker::new("Title".into(), None, tx)
///     .items(items)
///     .enable_ordering()
///     .on_preview(|items| Some(Line::from("Preview")))
///     .on_confirm(|ids, tx| { /* handle */ })
///     .on_cancel(|tx| { /* handle */ })
///     .build();
/// ```
pub(crate) struct MultiSelectPickerBuilder {
    title: String,
    subtitle: Option<String>,
    instructions: Vec<Span<'static>>,
    items: Vec<MultiSelectItem>,
    ordering_enabled: bool,
    app_event_tx: AppEventSender,
    keymap: ListKeymap,
    preview_builder: Option<PreviewCallback>,
    on_change: Option<ChangeCallBack>,
    on_confirm: Option<ConfirmCallback>,
    on_cancel: Option<CancelCallback>,
}

impl MultiSelectPickerBuilder {
    /// 创建一个新的构建器，使用指定的标题、可选副标题和事件发送者。
    /// Creates a new builder with the given title, optional subtitle, and event sender.
    pub fn new(title: String, subtitle: Option<String>, app_event_tx: AppEventSender) -> Self {
        Self {
            title,
            subtitle,
            instructions: Vec::new(),
            items: Vec::new(),
            ordering_enabled: false,
            app_event_tx,
            keymap: RuntimeKeymap::defaults().list,
            preview_builder: None,
            on_change: None,
            on_confirm: None,
            on_cancel: None,
        }
    }

    /// 设置可选择的列表项
    /// Sets the list of selectable items.
    pub fn items(mut self, items: Vec<MultiSelectItem>) -> Self {
        self.items = items;
        self
    }

    /// 支持按左右方向键重排项目。
    /// Enables left/right arrow keys for reordering items.
    ///
    /// Reordering is only active when the search query is empty.
    pub fn enable_ordering(mut self) -> Self {
        self.ordering_enabled = true;
        self
    }

    /// 设置用于导航和补全的共享列表键映射
    /// Sets the shared list keymap used for navigation and completion.
    pub fn list_keymap(mut self, keymap: ListKeymap) -> Self {
        self.keymap = keymap;
        self
    }

    /// 设置回调，用于从当前项目状态生成预览行。
    /// Sets a callback to generate a preview line from the current item states.
    ///
    /// The callback receives all items and should return a [`Line`] to display,
    /// or `None` to hide the preview area.
    pub fn on_preview<F>(mut self, callback: F) -> Self
    where
        F: Fn(&[MultiSelectItem]) -> Option<Line<'static>> + Send + Sync + 'static,
    {
        self.preview_builder = Some(Box::new(callback));
        self
    }

    /// 当项的状态发生变化时，设置回调。
    /// Sets a callback invoked whenever an item's state changes.
    ///
    /// This includes both toggles and reordering operations.
    #[allow(dead_code)]
    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&[MultiSelectItem], &AppEventSender) + Send + Sync + 'static,
    {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// 当用户确认选择时触发回调（按 Enter 键）
    /// Sets a callback invoked when the user confirms their selection (Enter).
    ///
    /// The callback receives a list of IDs for all enabled items.
    pub fn on_confirm<F>(mut self, callback: F) -> Self
    where
        F: Fn(&[String], &AppEventSender) + Send + Sync + 'static,
    {
        self.on_confirm = Some(Box::new(callback));
        self
    }

    /// 当用户取消选择器时（按 ESC 键）触发回调。
    /// Sets a callback invoked when the user cancels the picker (Escape).
    pub fn on_cancel<F>(mut self, callback: F) -> Self
    where
        F: Fn(&AppEventSender) + Send + Sync + 'static,
    {
        self.on_cancel = Some(Box::new(callback));
        self
    }

    /// 构建所有配置选项的 [`MultiSelectPicker`]。
    /// Builds the [`MultiSelectPicker`] with all configured options.
    ///
    /// Initializes the filter to show all items and generates the initial
    /// preview line if a preview callback was set.
    pub fn build(self) -> MultiSelectPicker {
        let mut header = ColumnRenderable::new();
        header.push(Line::from(self.title.bold()));

        if let Some(subtitle) = self.subtitle {
            header.push(Line::from(subtitle.dim()));
        }

        let instructions = if self.instructions.is_empty() {
            let mut spans = vec![
                "Press ".into(),
                key_hint::plain(KeyCode::Char(' ')).into(),
                " to toggle".into(),
            ];
            if self.ordering_enabled
                && let (Some(move_left), Some(move_right)) = (
                    self.keymap.primary_hint(ListAction::MoveLeft),
                    self.keymap.primary_hint(ListAction::MoveRight),
                )
            {
                spans.push("; ".into());
                spans.push(move_left.into());
                spans.push("/".into());
                spans.push(move_right.into());
                spans.push(" to move".into());
            }
            if let Some(accept) = self.keymap.primary_hint(ListAction::Accept) {
                spans.push("; ".into());
                spans.push(accept.into());
                spans.push(" to confirm and close".into());
            }
            if let Some(cancel) = self.keymap.primary_hint(ListAction::Cancel) {
                spans.push("; ".into());
                spans.push(cancel.into());
                spans.push(" to close".into());
            }
            spans
        } else {
            self.instructions
        };

        let mut view = MultiSelectPicker {
            items: self.items,
            state: ScrollState::new(),
            complete: false,
            app_event_tx: self.app_event_tx,
            header: Box::new(header),
            footer_hint: Line::from(instructions),
            ordering_enabled: self.ordering_enabled,
            keymap: self.keymap,
            search_query: String::new(),
            filtered_indices: Vec::new(),
            preview_builder: self.preview_builder,
            preview_line: None,
            on_change: self.on_change,
            on_confirm: self.on_confirm,
            on_cancel: self.on_cancel,
        };
        view.apply_filter();
        view.update_preview_line();
        view
    }
}

/// 根据过滤字符串对物品进行模糊匹配。
/// Performs fuzzy matching on an item against a filter string.
///
/// Tries to match against the display name first, then falls back to name if different. Returns
/// the matching character indices (if matched on display name) and a score for sorting.
///
/// # Arguments
///
/// * `filter` - The search query to match against
/// * `display_name` - The primary name to match (shown to user)
/// * `name` - A secondary/canonical name to try if display name doesn't match
///
/// # Returns
///
/// * `Some((Some(indices), score))` - Matched on display name with highlight indices
/// * `Some((None, score))` - Matched on skill name only (no highlights for display)
/// * `None` - No match
pub(crate) fn match_item(
    filter: &str,
    display_name: &str,
    name: &str,
) -> Option<(Option<Vec<usize>>, i32)> {
    if let Some((indices, score)) = fuzzy_match(display_name, filter) {
        return Some((Some(indices), score));
    }
    if display_name != name
        && let Some((_indices, score)) = fuzzy_match(name, filter)
    {
        return Some((None, score));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_event::AppEvent;
    use pretty_assertions::assert_eq;
    use tokio::sync::mpsc::unbounded_channel;

    fn test_picker(items: Vec<MultiSelectItem>) -> MultiSelectPicker {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        MultiSelectPicker::builder(
            "Test".to_string(),
            /*subtitle*/ None,
            AppEventSender::new(tx),
        )
        .items(items)
        .enable_ordering()
        .build()
    }

    fn item(id: &str, orderable: bool, section_break_after: bool) -> MultiSelectItem {
        MultiSelectItem {
            id: id.to_string(),
            name: id.to_string(),
            orderable,
            section_break_after,
            ..Default::default()
        }
    }

    // 验证 `non_orderable_items_cannot_move_or_be_crossed` 场景的触发条件与预期结果。
    #[test]
    fn non_orderable_items_cannot_move_or_be_crossed() {
        let mut picker = test_picker(vec![
            item(
                "theme-colors",
                /*orderable*/ false,
                /*section_break_after*/ true,
            ),
            item(
                "model", /*orderable*/ true, /*section_break_after*/ false,
            ),
            item(
                "branch", /*orderable*/ true, /*section_break_after*/ false,
            ),
        ]);

        picker.move_selected_item(Direction::Down);
        assert_eq!(
            picker
                .items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["theme-colors", "model", "branch"]
        );

        picker.move_down();
        picker.move_selected_item(Direction::Up);
        assert_eq!(
            picker
                .items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["theme-colors", "model", "branch"]
        );
    }

    // 验证 `horizontal_list_keys_reorder_orderable_items` 场景的触发条件与预期结果。
    #[test]
    fn horizontal_list_keys_reorder_orderable_items() {
        let mut picker = test_picker(vec![
            item(
                "model", /*orderable*/ true, /*section_break_after*/ false,
            ),
            item(
                "branch", /*orderable*/ true, /*section_break_after*/ false,
            ),
        ]);

        picker.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL));
        assert_eq!(
            picker
                .items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["branch", "model"]
        );

        picker.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
        assert_eq!(
            picker
                .items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["model", "branch"]
        );
    }

    // 验证 `section_break_after_item_renders_separator_row` 场景的触发条件与预期结果。
    #[test]
    fn section_break_after_item_renders_separator_row() {
        let picker = test_picker(vec![
            item(
                "theme-colors",
                /*orderable*/ false,
                /*section_break_after*/ true,
            ),
            item(
                "model", /*orderable*/ true, /*section_break_after*/ false,
            ),
        ]);

        let rows = picker.build_rows();

        assert_eq!(
            rows.rows
                .iter()
                .map(|row| row.name.as_str())
                .collect::<Vec<_>>(),
            vec!["› [ ] theme-colors", SECTION_BREAK_ROW, "  [ ] model"]
        );
        assert_eq!(rows.state.selected_idx, Some(0));
    }

    // 验证 `searchable_plain_j_updates_query_instead_of_navigating` 场景的触发条件与预期结果。
    #[test]
    fn searchable_plain_j_updates_query_instead_of_navigating() {
        let mut picker = test_picker(vec![
            item(
                "alpha", /*orderable*/ true, /*section_break_after*/ false,
            ),
            item(
                "jupiter", /*orderable*/ true, /*section_break_after*/ false,
            ),
        ]);

        picker.handle_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));

        assert_eq!(picker.search_query, "j");
        assert_eq!(picker.filtered_indices, vec![1]);
        assert_eq!(picker.state.selected_idx, Some(0));
    }

    // 验证 `page_and_jump_navigation_use_list_keymap` 场景的触发条件与预期结果。
    #[test]
    fn page_and_jump_navigation_use_list_keymap() {
        let (tx, _rx) = unbounded_channel::<AppEvent>();
        let mut keymap = RuntimeKeymap::defaults().list;
        keymap.page_down = vec![key_hint::ctrl(KeyCode::Char('d'))];
        keymap.page_up = vec![key_hint::ctrl(KeyCode::Char('u'))];
        keymap.jump_bottom = vec![key_hint::ctrl(KeyCode::Char('e'))];
        keymap.jump_top = vec![key_hint::ctrl(KeyCode::Char('a'))];
        let mut picker = MultiSelectPicker::builder(
            "Test".to_string(),
            /*subtitle*/ None,
            AppEventSender::new(tx),
        )
        .items(
            (0..12)
                .map(|idx| {
                    item(
                        &format!("item-{idx}"),
                        /*orderable*/ true,
                        /*section_break_after*/ false,
                    )
                })
                .collect(),
        )
        .list_keymap(keymap)
        .build();

        picker.handle_key_event(KeyEvent::from(KeyCode::PageDown));
        assert_eq!(picker.state.selected_idx, Some(0));

        picker.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(picker.state.selected_idx, Some(8));

        picker.handle_key_event(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(picker.state.selected_idx, Some(0));

        picker.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
        assert_eq!(picker.state.selected_idx, Some(11));

        picker.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert_eq!(picker.state.selected_idx, Some(0));
    }
}
