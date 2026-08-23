//! 定义可测量、可绘制并可组合的轻量 TUI 渲染协议。
//! 行、列、弹性布局与内边距包装器都通过同一 `Renderable` 契约协作。

use std::cell::Cell;
use std::sync::Arc;

use crossterm::cursor::SetCursorStyle;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::WidgetRef;

use crate::render::Insets;
use crate::render::RectExt as _;

/// 所有可组合 TUI 组件共同实现的测量、绘制与光标契约。
pub trait Renderable {
    /// 把当前对象绘制到给定区域，并尊重该区域边界。
    fn render(&self, area: Rect, buf: &mut Buffer);
    /// 返回给定宽度下期望占用的行数。
    fn desired_height(&self, width: u16) -> u16;
    /// Renders visible rows after `scroll_offset` when direct scrolling is supported.
    ///
    /// Implementations returning `false` must leave `buf` unchanged so callers can use their
    /// existing full-height rendering fallback. Supporting wrappers must forward this method.
    /// 支持直接滚动时只绘制 `scroll_offset` 后的可见行；返回 `false` 必须保持缓冲区不变，包装器需转发此能力。
    fn render_scrolled(&self, _area: Rect, _buf: &mut Buffer, _scroll_offset: u16) -> bool {
        false
    }
    /// 如有文本输入光标，返回其在绝对渲染区域中的位置。
    fn cursor_pos(&self, _area: Rect) -> Option<(u16, u16)> {
        None
    }
    /// 返回当前区域应使用的终端光标形状。
    fn cursor_style(&self, _area: Rect) -> SetCursorStyle {
        SetCursorStyle::DefaultUserShape
    }
}

/// 同时容纳拥有型与借用型渲染子项，便于组合布局统一存储。
pub enum RenderableItem<'a> {
    Owned(Box<dyn Renderable + 'a>),
    Borrowed(&'a dyn Renderable),
}

impl<'a> Renderable for RenderableItem<'a> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        match self {
            RenderableItem::Owned(child) => child.render(area, buf),
            RenderableItem::Borrowed(child) => child.render(area, buf),
        }
    }

    fn desired_height(&self, width: u16) -> u16 {
        match self {
            RenderableItem::Owned(child) => child.desired_height(width),
            RenderableItem::Borrowed(child) => child.desired_height(width),
        }
    }

    fn render_scrolled(&self, area: Rect, buf: &mut Buffer, scroll_offset: u16) -> bool {
        match self {
            RenderableItem::Owned(child) => child.render_scrolled(area, buf, scroll_offset),
            RenderableItem::Borrowed(child) => child.render_scrolled(area, buf, scroll_offset),
        }
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        match self {
            RenderableItem::Owned(child) => child.cursor_pos(area),
            RenderableItem::Borrowed(child) => child.cursor_pos(area),
        }
    }

    fn cursor_style(&self, area: Rect) -> SetCursorStyle {
        match self {
            RenderableItem::Owned(child) => child.cursor_style(area),
            RenderableItem::Borrowed(child) => child.cursor_style(area),
        }
    }
}

impl<'a> From<Box<dyn Renderable + 'a>> for RenderableItem<'a> {
    fn from(value: Box<dyn Renderable + 'a>) -> Self {
        RenderableItem::Owned(value)
    }
}

impl<'a, R> From<R> for Box<dyn Renderable + 'a>
where
    R: Renderable + 'a,
{
    fn from(value: R) -> Self {
        Box::new(value)
    }
}

impl Renderable for () {
    fn render(&self, _area: Rect, _buf: &mut Buffer) {}
    fn desired_height(&self, _width: u16) -> u16 {
        0
    }
}

impl Renderable for &str {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.render_ref(area, buf);
    }
    fn desired_height(&self, _width: u16) -> u16 {
        1
    }
}

impl Renderable for String {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.render_ref(area, buf);
    }
    fn desired_height(&self, _width: u16) -> u16 {
        1
    }
}

impl<'a> Renderable for Span<'a> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.render_ref(area, buf);
    }
    fn desired_height(&self, _width: u16) -> u16 {
        1
    }
}

impl<'a> Renderable for Line<'a> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        Widget::render(self, area, buf);
    }
    fn desired_height(&self, _width: u16) -> u16 {
        1
    }
}

impl<'a> Renderable for Paragraph<'a> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.render_ref(area, buf);
    }
    fn desired_height(&self, width: u16) -> u16 {
        self.line_count(width) as u16
    }
}

impl<R: Renderable> Renderable for Option<R> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if let Some(renderable) = self {
            renderable.render(area, buf);
        }
    }

    fn desired_height(&self, width: u16) -> u16 {
        if let Some(renderable) = self { renderable.desired_height(width) } else { 0 }
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.as_ref().and_then(|renderable| renderable.cursor_pos(area))
    }

    fn cursor_style(&self, area: Rect) -> SetCursorStyle {
        self.as_ref()
            .map_or(SetCursorStyle::DefaultUserShape, |renderable| renderable.cursor_style(area))
    }
}

impl<R: Renderable> Renderable for Arc<R> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.as_ref().render(area, buf);
    }
    fn desired_height(&self, width: u16) -> u16 {
        self.as_ref().desired_height(width)
    }
    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.as_ref().cursor_pos(area)
    }
    fn cursor_style(&self, area: Rect) -> SetCursorStyle {
        self.as_ref().cursor_style(area)
    }
}

/// 按垂直方向依次排列子项的组合渲染器。
pub struct ColumnRenderable<'a> {
    children: Vec<RenderableItem<'a>>,
}

impl Renderable for ColumnRenderable<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let mut y = area.y;
        for child in &self.children {
            if y >= area.bottom() {
                break;
            }
            let child_area = Rect::new(area.x, y, area.width, child.desired_height(area.width))
                .intersection(area);
            if !child_area.is_empty() {
                child.render(child_area, buf);
            }
            y += child_area.height;
        }
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.children.iter().map(|child| child.desired_height(width)).sum()
    }

    /// Returns the cursor position of the first child that has a cursor position, offset by the
    /// child's position in the column.
    ///
    /// It is generally assumed that either zero or one child will have a cursor position.
    /// 返回首个拥有光标的子项位置并保留其列内偏移；通常最多一个子项报告光标。
    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        let mut y = area.y;
        for child in &self.children {
            let child_area = Rect::new(area.x, y, area.width, child.desired_height(area.width))
                .intersection(area);
            if !child_area.is_empty()
                && let Some((px, py)) = child.cursor_pos(child_area)
            {
                return Some((px, py));
            }
            y += child_area.height;
        }
        None
    }

    fn cursor_style(&self, area: Rect) -> SetCursorStyle {
        let mut y = area.y;
        for child in &self.children {
            let child_area = Rect::new(area.x, y, area.width, child.desired_height(area.width))
                .intersection(area);
            if !child_area.is_empty() && child.cursor_pos(child_area).is_some() {
                return child.cursor_style(child_area);
            }
            y += child_area.height;
        }
        SetCursorStyle::DefaultUserShape
    }
}

impl<'a> ColumnRenderable<'a> {
    /// 创建空列布局。
    pub fn new() -> Self {
        Self { children: vec![] }
    }

    /// 从可迭代子项批量构造列布局。
    pub fn with<I, T>(children: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<RenderableItem<'a>>,
    {
        Self { children: children.into_iter().map(Into::into).collect() }
    }

    /// 向列尾追加一个拥有型渲染子项。
    pub fn push(&mut self, child: impl Into<Box<dyn Renderable + 'a>>) {
        self.children.push(RenderableItem::Owned(child.into()));
    }
}

/// 弹性列中的一个子项，缓存指定宽度下的期望高度。
pub struct FlexChild<'a> {
    flex: i32,
    child: RenderableItem<'a>,
    cached_height: Cell<Option<(u16, u16)>>,
}

/// 在固定高度子项之外按权重分配剩余高度的列布局。
pub struct FlexRenderable<'a> {
    children: Vec<FlexChild<'a>>,
}

/// Lays out children in a column, with the ability to specify a flex factor for each child.
///
/// Children with flex factor > 0 will be allocated the remaining space after the non-flex children,
/// proportional to the flex factor.
/// 纵向排列子项，并把非弹性子项占用后的剩余空间按正权重分配。
impl<'a> FlexRenderable<'a> {
    pub fn new() -> Self {
        Self { children: vec![] }
    }

    pub fn push(&mut self, flex: i32, child: impl Into<RenderableItem<'a>>) {
        self.children.push(FlexChild { flex, child: child.into(), cached_height: Cell::new(None) });
    }

    /// Loosely inspired by Flutter's Flex widget.
    ///
    /// Ref https://github.com/flutter/flutter/blob/3fd81edbf1e015221e143c92b2664f4371bdc04a/packages/flutter/lib/src/rendering/flex.dart#L1205-L1209
    /// 分配算法宽松参考 Flutter 的 Flex 布局。
    fn allocate(&self, area: Rect) -> Vec<Rect> {
        let mut allocated_rects = Vec::with_capacity(self.children.len());
        let mut child_sizes = vec![0; self.children.len()];
        let mut allocated_size = 0;
        let mut flex_children = Vec::new();

        // 1. Allocate space to non-flex children.
        // 第一步先满足所有非弹性子项的期望高度。
        let max_size = area.height;
        for (i, child) in self.children.iter().enumerate() {
            let desired_height = if let Some((width, height)) = child.cached_height.get()
                && width == area.width
            {
                height
            } else {
                let height = child.child.desired_height(area.width);
                child.cached_height.set(Some((area.width, height)));
                height
            };
            if child.flex > 0 {
                flex_children.push((i, child.flex as u16, desired_height));
            } else {
                child_sizes[i] = desired_height.min(max_size.saturating_sub(allocated_size));
                allocated_size += child_sizes[i];
            }
        }
        let free_space = max_size.saturating_sub(allocated_size);
        // 2. Satisfy flex children that need less than their proportional share so their unused
        // space can be redistributed instead of leaving blank rows.
        // 第二步先满足低于比例份额的弹性子项，把其未用空间重新分配，避免空行。
        let mut remaining_space = free_space;
        while !flex_children.is_empty() {
            let total_flex = flex_children.iter().map(|(_, flex, _)| *flex).sum::<u16>();
            let mut satisfied_any = false;
            flex_children.retain(|(i, flex, desired_height)| {
                let proportional_share =
                    (u32::from(remaining_space) * u32::from(*flex) / u32::from(total_flex)) as u16;
                if *desired_height <= proportional_share {
                    child_sizes[*i] = *desired_height;
                    remaining_space = remaining_space.saturating_sub(*desired_height);
                    satisfied_any = true;
                    false
                } else {
                    true
                }
            });
            if !satisfied_any {
                break;
            }
        }
        // 3. Divide the remaining space proportionally. The final child absorbs rounding slack.
        // 第三步按权重分配剩余空间，最后一个子项吸收整数除法余量。
        let total_flex = flex_children.iter().map(|(_, flex, _)| *flex).sum::<u16>();
        let mut allocated_flex_space = 0;
        let last_flex_child_idx = flex_children.last().map(|(i, _, _)| *i);
        for (i, flex, desired_height) in flex_children {
            let max_child_extent = if Some(i) == last_flex_child_idx {
                remaining_space.saturating_sub(allocated_flex_space)
            } else {
                (u32::from(remaining_space) * u32::from(flex) / u32::from(total_flex)) as u16
            };
            let child_size = desired_height.min(max_child_extent);
            child_sizes[i] = child_size;
            allocated_flex_space += child_size;
        }

        let mut y = area.y;
        for size in child_sizes {
            let child_area = Rect::new(area.x, y, area.width, size);
            allocated_rects.push(child_area);
            y += child_area.height;
        }
        allocated_rects
    }
}

impl<'a> Renderable for FlexRenderable<'a> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.allocate(area).into_iter().zip(self.children.iter()).for_each(|(rect, child)| {
            child.child.render(rect, buf);
        });
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.allocate(Rect::new(0, 0, width, u16::MAX))
            .last()
            .map(|rect| rect.bottom())
            .unwrap_or(0)
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.allocate(area)
            .into_iter()
            .zip(self.children.iter())
            .find_map(|(rect, child)| child.child.cursor_pos(rect))
    }

    fn cursor_style(&self, area: Rect) -> SetCursorStyle {
        self.allocate(area)
            .into_iter()
            .zip(self.children.iter())
            .find_map(|(rect, child)| {
                child.child.cursor_pos(rect).map(|_| child.child.cursor_style(rect))
            })
            .unwrap_or(SetCursorStyle::DefaultUserShape)
    }
}

/// 按水平方向以显式宽度排列子项的组合渲染器。
pub struct RowRenderable<'a> {
    children: Vec<(u16, RenderableItem<'a>)>,
}

impl Renderable for RowRenderable<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let mut x = area.x;
        for (width, child) in &self.children {
            let available_width = area.width.saturating_sub(x - area.x);
            let child_area = Rect::new(x, area.y, (*width).min(available_width), area.height);
            if child_area.is_empty() {
                break;
            }
            child.render(child_area, buf);
            x = x.saturating_add(*width);
        }
    }
    fn desired_height(&self, width: u16) -> u16 {
        let mut max_height = 0;
        let mut width_remaining = width;
        for (child_width, child) in &self.children {
            let w = (*child_width).min(width_remaining);
            if w == 0 {
                break;
            }
            let height = child.desired_height(w);
            if height > max_height {
                max_height = height;
            }
            width_remaining = width_remaining.saturating_sub(w);
        }
        max_height
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        let mut x = area.x;
        for (width, child) in &self.children {
            let available_width = area.width.saturating_sub(x - area.x);
            let child_area = Rect::new(x, area.y, (*width).min(available_width), area.height);
            if !child_area.is_empty()
                && let Some(pos) = child.cursor_pos(child_area)
            {
                return Some(pos);
            }
            x = x.saturating_add(*width);
        }
        None
    }

    fn cursor_style(&self, area: Rect) -> SetCursorStyle {
        let mut x = area.x;
        for (width, child) in &self.children {
            let available_width = area.width.saturating_sub(x - area.x);
            let child_area = Rect::new(x, area.y, (*width).min(available_width), area.height);
            if !child_area.is_empty() && child.cursor_pos(child_area).is_some() {
                return child.cursor_style(child_area);
            }
            x = x.saturating_add(*width);
        }
        SetCursorStyle::DefaultUserShape
    }
}

impl<'a> RowRenderable<'a> {
    pub fn new() -> Self {
        Self { children: vec![] }
    }

    pub fn push(&mut self, width: u16, child: impl Into<Box<dyn Renderable>>) {
        self.children.push((width, RenderableItem::Owned(child.into())));
    }
}

/// 为子项四周保留内边距的包装渲染器。
pub struct InsetRenderable<'a> {
    child: RenderableItem<'a>,
    insets: Insets,
}

impl<'a> Renderable for InsetRenderable<'a> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.child.render(area.inset(self.insets), buf);
    }
    fn desired_height(&self, width: u16) -> u16 {
        self.child.desired_height(width - self.insets.left - self.insets.right)
            + self.insets.top
            + self.insets.bottom
    }

    /// Preserve clipped inset padding while forwarding only visible child rows.
    /// 转发子项可见行时仍保留被裁剪后的内边距。
    fn render_scrolled(&self, area: Rect, buf: &mut Buffer, scroll_offset: u16) -> bool {
        let top_padding = self.insets.top.saturating_sub(scroll_offset);
        let child_width =
            area.width.saturating_sub(self.insets.left.saturating_add(self.insets.right));
        if child_width == 0 || top_padding >= area.height {
            return true;
        }

        let child_offset = scroll_offset.saturating_sub(self.insets.top);
        // The fallback applies bottom padding to its clipped scratch buffer, even mid-scroll.
        // 回退路径即使从中间滚动，也会在裁剪后的临时缓冲区应用底部内边距。
        let child_height = self
            .child
            .desired_height(child_width)
            .saturating_sub(child_offset)
            .min(area.height.saturating_sub(top_padding).saturating_sub(self.insets.bottom));
        if child_height == 0 {
            return true;
        }

        let child_area = Rect::new(
            area.x.saturating_add(self.insets.left),
            area.y.saturating_add(top_padding),
            child_width,
            child_height,
        );
        if child_offset == 0 {
            self.child.render(child_area, buf);
            true
        } else {
            self.child.render_scrolled(child_area, buf, child_offset)
        }
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.child.cursor_pos(area.inset(self.insets))
    }

    fn cursor_style(&self, area: Rect) -> SetCursorStyle {
        self.child.cursor_style(area.inset(self.insets))
    }
}

impl<'a> InsetRenderable<'a> {
    /// 用指定内边距包装一个渲染子项。
    pub fn new(child: impl Into<RenderableItem<'a>>, insets: Insets) -> Self {
        Self { child: child.into(), insets }
    }
}

/// 为任意 `Renderable` 提供链式添加内边距的扩展方法。
pub trait RenderableExt<'a> {
    fn inset(self, insets: Insets) -> RenderableItem<'a>;
}

impl<'a, R> RenderableExt<'a> for R
where
    R: Renderable + 'a,
{
    fn inset(self, insets: Insets) -> RenderableItem<'a> {
        let child: RenderableItem<'a> =
            RenderableItem::Owned(Box::new(self) as Box<dyn Renderable + 'a>);
        RenderableItem::Owned(Box::new(InsetRenderable { child, insets }))
    }
}

#[cfg(test)]
#[path = "renderable_tests.rs"]
mod tests;
