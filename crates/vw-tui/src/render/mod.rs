//! TUI 渲染基础模块，集中导出高亮、行工具和可组合渲染协议。
//! 本文件还定义饱和缩进矩形的通用几何操作。

use ratatui::layout::Rect;

pub(crate) mod highlight;
pub(crate) mod line_utils;
pub(crate) mod renderable;

/// 矩形四边需要扣除的终端单元格数量。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Insets {
    left: u16,
    top: u16,
    right: u16,
    bottom: u16,
}

impl Insets {
    /// 按上、左、下、右顺序构造不对称内边距。
    pub fn tlbr(top: u16, left: u16, bottom: u16, right: u16) -> Self {
        Self { top, left, bottom, right }
    }

    /// 用垂直值与水平值构造对称内边距。
    pub fn vh(v: u16, h: u16) -> Self {
        Self { top: v, left: h, bottom: v, right: h }
    }
}

/// 为 ratatui 矩形补充安全的内缩运算。
pub trait RectExt {
    /// 使用饱和算术内缩，过大的边距不会造成整数下溢。
    fn inset(&self, insets: Insets) -> Rect;
}

impl RectExt for Rect {
    fn inset(&self, insets: Insets) -> Rect {
        let horizontal = insets.left.saturating_add(insets.right);
        let vertical = insets.top.saturating_add(insets.bottom);
        Rect {
            x: self.x.saturating_add(insets.left),
            y: self.y.saturating_add(insets.top),
            width: self.width.saturating_sub(horizontal),
            height: self.height.saturating_sub(vertical),
        }
    }
}
