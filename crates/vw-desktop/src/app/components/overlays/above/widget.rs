use super::{AboveOverlay, AboveOverlayElement, PointAboveOverlay, PointAboveOverlayElement};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::{Element, Length};
use iced::{Event, Point, Rectangle, Size, Vector};

/// 为 `AboveOverlay` 实现 `Widget` trait
///
/// 该实现将 `AboveOverlay` 集成到 Iced 的 Widget 体系中，
/// 允许它作为普通 widget 在界面中使用。
impl<Message, ThemeT, RendererT> Widget<Message, ThemeT, RendererT>
    for AboveOverlay<'_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    Message: Clone,
{
    /// 返回 widget 的子节点树
    ///
    /// 创建两个子节点：一个用于内容元素，一个用于覆盖层元素。
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content), widget::Tree::new(&self.overlay)]
    }

    /// 对比并更新 widget 树
    ///
    /// 当 widget 状态改变时，更新子节点树以反映这些变化。
    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.content.as_widget(), self.overlay.as_widget()]);
    }

    /// 返回 widget 的尺寸
    ///
    /// 返回内容元素的尺寸。
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    /// 返回 widget 的尺寸提示
    ///
    /// 返回内容元素的尺寸提示。
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    /// 执行布局计算
    ///
    /// 根据给定的限制条件计算内容元素的布局。
    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &RendererT,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    /// 绘制 widget
    ///
    /// 渲染内容元素到界面上。
    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut RendererT,
        theme: &ThemeT,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    /// 处理事件更新
    ///
    /// 将事件传递给内容元素进行处理。
    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &RendererT,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    /// 返回鼠标交互状态
    ///
    /// 根据内容元素的状态返回相应的鼠标交互类型。
    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &RendererT,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    /// 创建覆盖层元素
    ///
    /// 这是 `AboveOverlay` 的核心方法。当 `show` 为 `true` 时，
    /// 创建一个 `AboveOverlayElement` 覆盖层并返回。
    ///
    /// # 覆盖层创建逻辑
    ///
    /// 1. 首先获取内容元素可能产生的覆盖层
    /// 2. 如果 `show` 为 `true`，创建一个 `AboveOverlayElement`
    /// 3. 将两个覆盖层（如果都存在）组合成一个 `overlay::Group`
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &RendererT,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, ThemeT, RendererT>> {
        let mut children = tree.children.iter_mut();

        // 获取内容元素自身的覆盖层（如果有）
        let content = self.content.as_widget_mut().overlay(
            children.next().unwrap(),
            layout,
            renderer,
            viewport,
            translation,
        );

        // 如果需要显示覆盖层，创建 AboveOverlayElement
        let above = if self.show {
            let mut target_bounds = layout.bounds();
            target_bounds.x += translation.x;
            target_bounds.y += translation.y;
            Some(overlay::Element::new(Box::new(AboveOverlayElement {
                position: layout.position() + translation,
                target_bounds,
                viewport: *viewport,
                gap: self.gap,
                snap_within_viewport: self.snap_within_viewport,
                overlay: &mut self.overlay,
                tree: children.next().unwrap(),
                on_close: self.on_close.clone(),
                block_background_events: self.block_background_events,
            })))
        } else {
            None
        };

        // 组合所有覆盖层元素
        if content.is_some() || above.is_some() {
            Some(
                overlay::Group::with_children(content.into_iter().chain(above).collect()).overlay(),
            )
        } else {
            None
        }
    }
}

/// 为 `PointAboveOverlay` 实现 `Widget` trait
///
/// 该实现与 `AboveOverlay` 类似，但支持基于锚点的定位。
impl<Message, ThemeT, RendererT> Widget<Message, ThemeT, RendererT>
    for PointAboveOverlay<'_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    Message: Clone,
{
    /// 返回 widget 的子节点树
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content), widget::Tree::new(&self.overlay)]
    }

    /// 对比并更新 widget 树
    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.content.as_widget(), self.overlay.as_widget()]);
    }

    /// 返回 widget 的尺寸
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    /// 返回 widget 的尺寸提示
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    /// 执行布局计算
    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &RendererT,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    /// 绘制 widget
    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut RendererT,
        theme: &ThemeT,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    /// 处理事件更新
    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &RendererT,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    /// 返回鼠标交互状态
    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &RendererT,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    /// 创建覆盖层元素
    ///
    /// 与 `AboveOverlay` 类似，但覆盖层基于锚点定位。
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &RendererT,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, ThemeT, RendererT>> {
        let mut children = tree.children.iter_mut();

        // 获取内容元素自身的覆盖层
        let content = self.content.as_widget_mut().overlay(
            children.next().unwrap(),
            layout,
            renderer,
            viewport,
            translation,
        );

        // 如果需要显示覆盖层，创建 PointAboveOverlayElement
        let above = if self.show {
            // 计算锚点的绝对位置：基础位置 + 相对锚点偏移
            let base = layout.position() + translation;
            let anchor = Point::new(base.x + self.anchor.x, base.y + self.anchor.y);
            Some(overlay::Element::new(Box::new(PointAboveOverlayElement {
                anchor,
                target_bounds: layout.bounds(),
                viewport: *viewport,
                gap: self.gap,
                snap_within_viewport: self.snap_within_viewport,
                overlay: &mut self.overlay,
                tree: children.next().unwrap(),
                on_close: self.on_close.clone(),
            })))
        } else {
            None
        };

        // 组合所有覆盖层元素
        if content.is_some() || above.is_some() {
            Some(
                overlay::Group::with_children(content.into_iter().chain(above).collect()).overlay(),
            )
        } else {
            None
        }
    }
}

/// 实现 `From` trait 以支持将 `AboveOverlay` 转换为 `Element`
///
/// 这允许 `AboveOverlay` 直接在需要 `Element` 的地方使用。
impl<'a, Message, ThemeT, RendererT> From<AboveOverlay<'a, Message, ThemeT, RendererT>>
    for Element<'a, Message, ThemeT, RendererT>
where
    Message: 'a + Clone,
    RendererT: iced::advanced::Renderer + 'a,
    ThemeT: 'a,
{
    fn from(widget: AboveOverlay<'a, Message, ThemeT, RendererT>) -> Self {
        Element::new(widget)
    }
}

/// 实现 `From` trait 以支持将 `PointAboveOverlay` 转换为 `Element`
///
/// 这允许 `PointAboveOverlay` 直接在需要 `Element` 的地方使用。
impl<'a, Message, ThemeT, RendererT> From<PointAboveOverlay<'a, Message, ThemeT, RendererT>>
    for Element<'a, Message, ThemeT, RendererT>
where
    Message: 'a + Clone,
    RendererT: iced::advanced::Renderer + 'a,
    ThemeT: 'a,
{
    fn from(widget: PointAboveOverlay<'a, Message, ThemeT, RendererT>) -> Self {
        Element::new(widget)
    }
}
