use super::*;

/// 为 `InlineRightOverlay` 实现 `Widget` trait
///
/// 该实现将组件集成到 Iced 的组件系统中，处理布局、绘制、
/// 事件处理等核心功能。
impl<Message, ThemeT, RendererT> Widget<Message, ThemeT, RendererT>
    for InlineRightOverlay<'_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    Message: Clone,
{
    /// 获取组件的子组件树
    ///
    /// 返回主内容和覆盖层两个子组件的组件树。
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content), widget::Tree::new(&self.overlay)]
    }

    /// 比较并更新组件树
    ///
    /// 当组件状态变化时，更新子组件树以保持同步。
    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.content.as_widget(), self.overlay.as_widget()]);
    }

    /// 获取组件的尺寸约束
    ///
    /// 返回主内容区域的尺寸约束。
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    /// 获取组件的尺寸提示
    ///
    /// 返回主内容区域的尺寸提示信息。
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    /// 计算组件的布局
    ///
    /// 基于给定的限制条件，计算主内容区域的布局节点。
    /// 覆盖层的布局在 `overlay` 方法中单独处理。
    ///
    /// # 参数
    ///
    /// - `tree`: 组件树，用于存储布局状态
    /// - `renderer`: 渲染器引用
    /// - `limits`: 布局限制条件
    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &RendererT,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    /// 绘制组件
    ///
    /// 绘制主内容区域。覆盖层的绘制在 `overlay` 方法中处理。
    ///
    /// # 参数
    ///
    /// - `tree`: 组件树
    /// - `renderer`: 渲染器实例
    /// - `theme`: 当前主题
    /// - `style`: 渲染样式
    /// - `layout`: 布局信息
    /// - `cursor`: 鼠标光标状态
    /// - `viewport`: 视口矩形区域
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

    /// 处理组件的事件
    ///
    /// 将事件转发给主内容区域进行处理。
    ///
    /// # 参数
    ///
    /// - `tree`: 组件树
    /// - `event`: 待处理的事件
    /// - `layout`: 布局信息
    /// - `cursor`: 鼠标光标状态
    /// - `renderer`: 渲染器引用
    /// - `clipboard`: 剪贴板接口
    /// - `shell`: 消息 shell，用于发布消息
    /// - `viewport`: 视口矩形区域
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

    /// 获取鼠标交互状态
    ///
    /// 返回主内容区域的鼠标交互类型（如指针、手型等）。
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
    /// 这是该组件的核心方法，负责创建右侧覆盖层的显示逻辑。
    /// 当 `show` 为 `true` 时，创建并返回覆盖层元素。
    ///
    /// # 工作流程
    ///
    /// 1. 获取主内容的覆盖层元素（如果有）
    /// 2. 如果 `show` 为 `true`，创建 `InlineRightOverlayElement`
    /// 3. 将所有覆盖层元素组合成一个 `Group` 返回
    ///
    /// # 参数
    ///
    /// - `tree`: 组件树
    /// - `layout`: 主内容的布局信息
    /// - `renderer`: 渲染器引用
    /// - `viewport`: 视口矩形区域
    /// - `translation`: 位置偏移向量
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &RendererT,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, ThemeT, RendererT>> {
        // 获取子组件树的迭代器
        let mut children = tree.children.iter_mut();

        // 获取主内容区域自己的覆盖层元素（如果有）
        let content = self.content.as_widget_mut().overlay(
            children.next().unwrap(),
            layout,
            renderer,
            viewport,
            translation,
        );

        // 如果覆盖层需要显示，创建 InlineRightOverlayElement
        let inside_right = if self.show {
            Some(overlay::Element::new(Box::new(InlineRightOverlayElement {
                // 计算覆盖层的基准位置（主内容位置 + 偏移）
                position: layout.position() + translation,
                // 主内容的边界矩形
                target_bounds: layout.bounds(),
                // 当前视口
                viewport: *viewport,
                // 覆盖层与主内容的间距
                gap: self.gap,
                // 是否限制在视口内
                snap_within_viewport: self.snap_within_viewport,
                // 覆盖层内容
                overlay: &mut self.overlay,
                // 覆盖层的组件树
                tree: children.next().unwrap(),
                // 关闭消息
                on_close: self.on_close.clone(),
            })))
        } else {
            None
        };

        // 如果有任何覆盖层元素，将它们组合成 Group 返回
        if content.is_some() || inside_right.is_some() {
            Some(
                overlay::Group::with_children(content.into_iter().chain(inside_right).collect())
                    .overlay(),
            )
        } else {
            None
        }
    }
}

/// 实现从 `InlineRightOverlay` 到 `Element` 的转换
///
/// 该实现允许将 `InlineRightOverlay` 直接用作 `Element`，
/// 使其能够在任何接受 `Element` 的地方使用。
impl<'a, Message, ThemeT, RendererT> From<InlineRightOverlay<'a, Message, ThemeT, RendererT>>
    for Element<'a, Message, ThemeT, RendererT>
where
    Message: 'a + Clone,
    RendererT: iced::advanced::Renderer + 'a,
    ThemeT: 'a,
{
    fn from(widget: InlineRightOverlay<'a, Message, ThemeT, RendererT>) -> Self {
        Element::new(widget)
    }
}
