//! 渲染门面：基础协议来自 tui-render，高亮保留在上层。

pub(crate) mod highlight;
pub(crate) mod line_utils;
pub(crate) use tui_render::{Insets, RectExt, renderable};
