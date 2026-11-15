use crate::app::{App, Message};
use iced::{Element, Color};
use super::{DiffSplitPaneTone, markers};
pub(in crate::app::components::git_panel) fn merge_diff_row<'a>(
    content: Element<'a, Message>,
    // tone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tone: DiffSplitPaneTone,
    // emphasized 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    emphasized: bool,
) -> Element<'a, Message> { desktop_git_ui::git_panel::diff_view::styles::merge_diff_row(content, tone, emphasized) }

pub(in crate::app::components::git_panel) fn diff_highlight_enabled(_app: &App) -> bool { desktop_git_ui::git_panel::diff_view::styles::diff_highlight_enabled(&crate::app::components::git_panel::diff_view::diff_view_state(_app)) }

pub(in crate::app::components::git_panel) fn diff_split_pane<'a>(
    content: Element<'a, Message>,
    // tone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tone: DiffSplitPaneTone,
    // emphasized 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    emphasized: bool,
) -> Element<'a, Message> { desktop_git_ui::git_panel::diff_view::styles::diff_split_pane(content, tone, emphasized) }

pub(in crate::app::components::git_panel) fn diff_split_pane_with_background<'a>(
    content: Element<'a, Message>,
    // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    background: Color,
    // emphasized 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    emphasized: bool,
) -> Element<'a, Message> { desktop_git_ui::git_panel::diff_view::styles::diff_split_pane_with_background(content, background, emphasized) }

pub(in crate::app::components::git_panel) fn diff_split_divider<'a>() -> Element<'a, Message> { desktop_git_ui::git_panel::diff_view::styles::diff_split_divider() }

pub(in crate::app::components::git_panel) fn split_line_number_area(
    file: &str,
    // line_info 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    line_info: Option<(usize, bool)>,
    // content 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    content: &str,
    // tone 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    tone: markers::LineNumberTone,
) -> Element<'static, Message> { desktop_git_ui::git_panel::diff_view::styles::split_line_number_area(file, line_info, content, tone).map(crate::app::components::git_panel::diff_view::map_diff_message) }

pub(in crate::app::components::git_panel) fn diff_split_row<'a>(
    children: impl IntoIterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> { desktop_git_ui::git_panel::diff_view::styles::diff_split_row(children) }
