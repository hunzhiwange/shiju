//! 构建带序号、选中标记和禁用弱化样式的通用选择行。

use crate::render::renderable::Renderable;
use crate::render::renderable::RowRenderable;
use crate::vibe::action::V3SelectionItem;
use ratatui::style::Style;
use ratatui::style::Styled as _;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;
use unicode_width::UnicodeWidthStr;

pub(crate) fn selection_option_row_for_item(
    index: usize,
    item: &V3SelectionItem,
    is_selected: bool,
) -> Box<dyn Renderable> {
    // V3 项只把稳定显示数据传给纯渲染函数，不在此触发选择行为。
    selection_option_row_with_dim(index, item.label.clone(), is_selected, item.disabled)
}

pub(crate) fn selection_option_row(
    index: usize,
    label: String,
    is_selected: bool,
) -> Box<dyn Renderable> {
    selection_option_row_with_dim(index, label, is_selected, /*dim*/ false)
}

pub(crate) fn selection_option_row_with_dim(
    index: usize,
    label: String,
    is_selected: bool,
    dim: bool,
) -> Box<dyn Renderable> {
    // 选中行的箭头和一基序号共享样式，禁用行仅在未选中时弱化。
    let prefix =
        if is_selected { format!("› {}. ", index + 1) } else { format!("  {}. ", index + 1) };
    let style = if is_selected {
        Style::default().cyan()
    } else if dim {
        Style::default().dim()
    } else {
        Style::default()
    };
    // 使用 Unicode 显示宽度为标签分配剩余列，避免宽字符导致错位。
    let prefix_width = UnicodeWidthStr::width(prefix.as_str()) as u16;
    let mut row = RowRenderable::new();
    row.push(prefix_width, prefix.set_style(style));
    row.push(u16::MAX, Paragraph::new(label).style(style).wrap(Wrap { trim: false }));
    row.into()
}
