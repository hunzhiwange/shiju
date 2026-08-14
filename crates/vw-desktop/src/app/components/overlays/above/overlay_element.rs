use iced::advanced::{Clipboard, Layout, Shell, layout, mouse, overlay, renderer, widget};
use iced::{Element, Event, Point, Rectangle, Size, Vector};

pub(super) fn compute_above_position(
    position: Point,
    target_bounds: Rectangle,
    viewport: Rectangle,
    overlay_size: Size,
    gap: f32,
    snap_within_viewport: bool,
) -> Point {
    let mut x = position.x;
    let mut y = target_bounds.y - gap - overlay_size.height;

    if snap_within_viewport {
        let min_x = viewport.x;
        let max_x = (viewport.x + viewport.width - overlay_size.width).max(viewport.x);
        let min_y = viewport.y;
        let max_y = (viewport.y + viewport.height - overlay_size.height).max(viewport.y);

        x = x.clamp(min_x, max_x);
        y = y.clamp(min_y, max_y);
    }

    Point::new(x, y)
}

pub(super) fn compute_point_above_position(
    anchor: Point,
    viewport: Rectangle,
    overlay_size: Size,
    gap: f32,
    snap_within_viewport: bool,
) -> Point {
    let mut x = anchor.x - overlay_size.width / 2.0;
    let mut y = anchor.y - gap - overlay_size.height;

    if snap_within_viewport {
        x = x.clamp(0.0, (viewport.width - overlay_size.width).max(0.0));
        y = y.clamp(0.0, (viewport.height - overlay_size.height).max(0.0));
    }

    Point::new(x, y)
}

/// 上方覆盖层元素（内部实现）
///
/// 这是实际渲染覆盖层的内部结构，实现了 `overlay::Overlay` trait。
/// 它负责计算覆盖层的布局、处理事件和绘制覆盖层。
///
/// # 生命周期参数
///
/// - `'a`：覆盖层元素的生命周期
/// - `'b`：对 widget 树的可变借用生命周期
pub(super) struct AboveOverlayElement<'a, 'b, Message, ThemeT, RendererT> {
    /// 覆盖层的目标位置（内容元素的左上角位置）
    pub(super) position: Point,
    /// 目标元素的边界矩形
    pub(super) target_bounds: Rectangle,
    /// 视口矩形
    pub(super) viewport: Rectangle,
    /// 覆盖层与目标元素之间的间距
    pub(super) gap: f32,
    /// 是否将覆盖层限制在视口范围内
    pub(super) snap_within_viewport: bool,
    /// 覆盖层元素的引用
    pub(super) overlay: &'b mut Element<'a, Message, ThemeT, RendererT>,
    /// widget 树的引用
    pub(super) tree: &'b mut widget::Tree,
    /// 关闭覆盖层时触发的消息
    pub(super) on_close: Option<Message>,
    /// 是否阻止用户输入事件继续传递到底层界面
    pub(super) block_background_events: bool,
}

/// 为 `AboveOverlayElement` 实现 `Overlay` trait
///
/// 该实现定义了覆盖层的布局、事件处理、鼠标交互和绘制行为。
impl<Message, ThemeT, RendererT> overlay::Overlay<Message, ThemeT, RendererT>
    for AboveOverlayElement<'_, '_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    Message: Clone,
{
    /// 执行覆盖层的布局计算
    ///
    /// # 布局算法
    ///
    /// 1. 计算覆盖层可用的最大高度（目标位置上方的空间）
    /// 2. 根据限制条件布局覆盖层元素
    /// 3. 计算覆盖层的显示位置（目标位置上方，考虑间距）
    /// 4. 如果启用 `snap_within_viewport`，将位置限制在视口范围内
    fn layout(&mut self, renderer: &RendererT, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);

        // 计算目标位置上方的可用空间
        let space_above = self.target_bounds.y - self.viewport.y - self.gap;
        // 根据 snap_within_viewport 决定最大高度限制
        let max_h = if self.snap_within_viewport { space_above.max(0.0) } else { bounds.height };

        // 使用计算出的高度限制布局覆盖层
        let node = self.overlay.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(viewport.width, max_h)),
        );

        let size = node.size();

        // 计算覆盖层的初始位置（目标位置上方，考虑间距）
        let position = compute_above_position(
            self.position,
            self.target_bounds,
            self.viewport,
            size,
            self.gap,
            self.snap_within_viewport,
        );

        node.move_to(position)
    }

    /// 将 widget 操作转发给覆盖层内容，使 `scroll_to` / `snap_to` / `focus` 能命中内部控件。
    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &RendererT,
        operation: &mut dyn widget::Operation,
    ) {
        self.overlay.as_widget_mut().operate(self.tree, layout, renderer, operation);
    }

    fn overlay<'a>(
        &'a mut self,
        layout: Layout<'a>,
        renderer: &RendererT,
    ) -> Option<overlay::Element<'a, Message, ThemeT, RendererT>> {
        self.overlay.as_widget_mut().overlay(
            self.tree,
            layout,
            renderer,
            &self.viewport,
            Vector::ZERO,
        )
    }

    /// 处理覆盖层的事件
    ///
    /// # 事件处理逻辑
    ///
    /// 1. 检测鼠标左键点击事件
    /// 2. 如果点击发生在覆盖层外部且设置了 `on_close`，触发关闭消息
    /// 3. 将事件传递给覆盖层元素处理
    /// 4. 如果鼠标事件发生在覆盖层内部，捕获事件以防止传递到底层
    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &RendererT,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let bounds = layout.bounds();

        // 模态弹层等待鼠标释放后关闭，确保同一次点击不会落到底层控件。
        let close_event = if self.block_background_events {
            matches!(event, Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)))
        } else {
            matches!(event, Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)))
        };
        if close_event
            && let Some(on_close) = &self.on_close
            && let Some(cursor_position) = cursor.position()
            && !bounds.contains(cursor_position)
        {
            shell.publish(on_close.clone());
            shell.capture_event();
        }

        // 将事件传递给覆盖层元素
        self.overlay
            .as_widget_mut()
            .update(self.tree, event, layout, cursor, renderer, clipboard, shell, &bounds);

        let user_input_event =
            matches!(event, Event::Mouse(_) | Event::Touch(_) | Event::Keyboard(_));
        // 模态弹层捕获全部用户输入；普通弹层仅捕获内部鼠标事件。
        if self.block_background_events && user_input_event {
            shell.capture_event();
        } else if matches!(event, Event::Mouse(_))
            && let Some(cursor_position) = cursor.position()
            && bounds.contains(cursor_position)
        {
            shell.capture_event();
        }
    }

    /// 返回覆盖层的鼠标交互状态
    ///
    /// 委托给覆盖层元素的 `mouse_interaction` 方法。
    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &RendererT,
    ) -> mouse::Interaction {
        self.overlay.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            &self.viewport,
            renderer,
        )
    }

    /// 绘制覆盖层
    ///
    /// 渲染覆盖层元素到界面上。
    fn draw(
        &self,
        renderer: &mut RendererT,
        theme: &ThemeT,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let bounds = layout.bounds();

        self.overlay
            .as_widget()
            .draw(self.tree, renderer, theme, defaults, layout, cursor, &bounds);
    }
}

/// 基于锚点的上方覆盖层元素（内部实现）
///
/// 与 `AboveOverlayElement` 类似，但基于指定的锚点进行定位。
/// 覆盖层会水平居中于锚点位置。
///
/// # 生命周期参数
///
/// - `'a`：覆盖层元素的生命周期
/// - `'b`：对 widget 树的可变借用生命周期
#[allow(dead_code)]
pub(super) struct PointAboveOverlayElement<'a, 'b, Message, ThemeT, RendererT> {
    /// 覆盖层的锚点位置
    pub(super) anchor: Point,
    /// 目标元素的边界矩形
    pub(super) target_bounds: Rectangle,
    /// 视口矩形
    pub(super) viewport: Rectangle,
    /// 覆盖层与锚点之间的间距
    pub(super) gap: f32,
    /// 是否将覆盖层限制在视口范围内
    pub(super) snap_within_viewport: bool,
    /// 覆盖层元素的引用
    pub(super) overlay: &'b mut Element<'a, Message, ThemeT, RendererT>,
    /// widget 树的引用
    pub(super) tree: &'b mut widget::Tree,
    /// 关闭覆盖层时触发的消息
    pub(super) on_close: Option<Message>,
}

/// 为 `PointAboveOverlayElement` 实现 `Overlay` trait
///
/// 该实现与 `AboveOverlayElement` 类似，但覆盖层水平居中于锚点。
impl<Message, ThemeT, RendererT> overlay::Overlay<Message, ThemeT, RendererT>
    for PointAboveOverlayElement<'_, '_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    Message: Clone,
{
    /// 执行覆盖层的布局计算
    ///
    /// # 布局算法
    ///
    /// 与 `AboveOverlayElement` 类似，但覆盖层水平居中于锚点位置：
    /// - x 坐标 = anchor.x - size.width / 2.0
    /// - y 坐标 = anchor.y - gap - size.height
    fn layout(&mut self, renderer: &RendererT, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);

        // 计算锚点上方的可用空间
        let space_above = self.anchor.y - self.gap;
        let max_h = if self.snap_within_viewport { space_above.max(0.0) } else { bounds.height };

        // 布局覆盖层
        let node = self.overlay.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(viewport.width, max_h)),
        );

        let size = node.size();

        // 计算覆盖层位置：水平居中于锚点，垂直方向在锚点上方
        let position = compute_point_above_position(
            self.anchor,
            self.viewport,
            size,
            self.gap,
            self.snap_within_viewport,
        );

        node.move_to(position)
    }

    /// 将 widget 操作转发给覆盖层内容，使 `scroll_to` / `snap_to` / `focus` 能命中内部控件。
    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &RendererT,
        operation: &mut dyn widget::Operation,
    ) {
        self.overlay.as_widget_mut().operate(self.tree, layout, renderer, operation);
    }

    /// 处理覆盖层的事件
    ///
    /// 与 `AboveOverlayElement` 的事件处理逻辑相同。
    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &RendererT,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        // 处理点击覆盖层外部区域关闭覆盖层
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && let Some(on_close) = &self.on_close
            && let Some(cursor_position) = cursor.position()
        {
            let bounds = layout.bounds();
            if !bounds.contains(cursor_position) {
                shell.publish(on_close.clone());
                shell.capture_event();
            }
        }

        let bounds = layout.bounds();

        // 将事件传递给覆盖层元素
        self.overlay
            .as_widget_mut()
            .update(self.tree, event, layout, cursor, renderer, clipboard, shell, &bounds);

        // 如果鼠标事件发生在覆盖层内部，捕获事件
        if matches!(event, Event::Mouse(_))
            && let Some(cursor_position) = cursor.position()
            && bounds.contains(cursor_position)
        {
            shell.capture_event();
        }
    }

    /// 返回覆盖层的鼠标交互状态
    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &RendererT,
    ) -> mouse::Interaction {
        self.overlay.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            &self.viewport,
            renderer,
        )
    }

    /// 绘制覆盖层
    fn draw(
        &self,
        renderer: &mut RendererT,
        theme: &ThemeT,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let bounds = layout.bounds();

        self.overlay
            .as_widget()
            .draw(self.tree, renderer, theme, defaults, layout, cursor, &bounds);
    }
}
