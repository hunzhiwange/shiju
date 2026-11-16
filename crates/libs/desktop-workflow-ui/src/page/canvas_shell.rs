//! 工作流画布外壳视图模块，负责组合画布区域、悬浮工具栏、上下文菜单和快速插入面板。

use super::*;

pub(super) fn build_canvas_area<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a WorkflowState,
    canvas_base: Element<'a, WorkflowMessage>,
) -> Element<'a, Message> {
    crate::view::canvas_shell::build_canvas_area(i18n, state, canvas_base)
        .map(Message::WorkflowTool)
}

pub(super) use crate::view::canvas_shell::available_node_types;
