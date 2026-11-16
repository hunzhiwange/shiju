//! 渲染门面：基础协议来自 tui-render，高亮来自 tui-highlight。

pub(crate) use tui_highlight as highlight;
pub(crate) mod line_utils;
pub(crate) use tui_render::{Insets, RectExt, renderable};
