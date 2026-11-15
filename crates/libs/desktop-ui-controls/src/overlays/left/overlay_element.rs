use super::*;

/// LeftOverlayElement 保存 left 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
pub(super) struct LeftOverlayElement<'a, 'b, Message, ThemeT, RendererT> {
    // position 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) position: Point,
    // target_bounds 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) target_bounds: Rectangle,
    // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) viewport: Rectangle,
    // gap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) gap: f32,
    // snap 开关让浮层在靠近窗口边缘时仍保持可见。
    // snap_within_viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) snap_within_viewport: bool,
    // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) overlay: &'b mut Element<'a, Message, ThemeT, RendererT>,
    // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) tree: &'b mut widget::Tree,
    // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) on_close: Option<Message>,
}

impl<Message, ThemeT, RendererT> overlay::Overlay<Message, ThemeT, RendererT>
    for LeftOverlayElement<'_, '_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    Message: Clone,
{
    /// 处理 layout 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn layout(&mut self, renderer: &RendererT, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);

        // snap 开关让浮层在靠近窗口边缘时仍保持可见。
        let max_w = if self.snap_within_viewport { viewport.width } else { bounds.width };
        let node = self.overlay.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(max_w, viewport.height)),
        );

        let size = node.size();

        let position = compute_left_position(
            self.position,
            self.target_bounds,
            self.viewport,
            size,
            self.gap,
            self.snap_within_viewport,
        );

        node.move_to(position)
    }

    /// 处理 update 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn update(
        &mut self,
        // event 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        event: &Event,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // clipboard 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        clipboard: &mut dyn Clipboard,
        // shell 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shell: &mut Shell<'_, Message>,
    ) {
        let bounds = layout.bounds();

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && let Some(on_close) = &self.on_close
            && let Some(cursor_position) = cursor.position()
            && !self.target_bounds.contains(cursor_position)
            && !bounds.contains(cursor_position)
        {
            shell.publish(on_close.clone());
            shell.capture_event();
        }

        self.overlay
            .as_widget_mut()
            .update(self.tree, event, layout, cursor, renderer, clipboard, shell, &bounds);

        if matches!(event, Event::Mouse(_)) {
            shell.capture_event();
        }
    }

    /// 处理 mouse interaction 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn mouse_interaction(
        &self,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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

    /// 处理 draw 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn draw(
        &self,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &mut RendererT,
        // theme 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        theme: &ThemeT,
        // defaults 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        defaults: &renderer::Style,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
    ) {
        let bounds = layout.bounds();

        self.overlay
            .as_widget()
            .draw(self.tree, renderer, theme, defaults, layout, cursor, &bounds);
    }
}

/// PointLeftOverlayElement 保存 left 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
pub(super) struct PointLeftOverlayElement<'a, 'b, Message, ThemeT, RendererT> {
    // anchor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) anchor: Point,
    // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) viewport: Rectangle,
    // gap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) gap: f32,
    // snap 开关让浮层在靠近窗口边缘时仍保持可见。
    // snap_within_viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) snap_within_viewport: bool,
    // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) overlay: &'b mut Element<'a, Message, ThemeT, RendererT>,
    // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) tree: &'b mut widget::Tree,
    // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) on_close: Option<Message>,
    // capture_events 控制被动浮层是否阻止事件继续传递到底层内容。
    pub(super) capture_events: bool,
}

impl<Message, ThemeT, RendererT> overlay::Overlay<Message, ThemeT, RendererT>
    for PointLeftOverlayElement<'_, '_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    Message: Clone,
{
    /// 处理 layout 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn layout(&mut self, renderer: &RendererT, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);

        // snap 开关让浮层在靠近窗口边缘时仍保持可见。
        let max_w = if self.snap_within_viewport { viewport.width } else { bounds.width };
        let node = self.overlay.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(max_w, viewport.height)),
        );

        let size = node.size();

        let position = compute_point_left_position(
            self.anchor,
            self.viewport,
            size,
            self.gap,
            self.snap_within_viewport,
        );

        node.move_to(position)
    }

    /// 处理 update 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn update(
        &mut self,
        // event 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        event: &Event,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // clipboard 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        clipboard: &mut dyn Clipboard,
        // shell 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shell: &mut Shell<'_, Message>,
    ) {
        let bounds = layout.bounds();
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && let Some(on_close) = &self.on_close
            && let Some(cursor_position) = cursor.position()
        {
            if !bounds.contains(cursor_position) {
                #[cfg(debug_assertions)]
                println!(
                    "PointLeftOverlay: Closing overlay (click outside). Bounds: {:?}, Cursor: {:?}",
                    bounds, cursor_position
                );
                shell.publish(on_close.clone());
                shell.capture_event();
            } else {
                #[cfg(debug_assertions)]
                println!(
                    "PointLeftOverlay: Click inside. Bounds: {:?}, Cursor: {:?}",
                    bounds, cursor_position
                );
            }
        }

        self.overlay
            .as_widget_mut()
            .update(self.tree, event, layout, cursor, renderer, clipboard, shell, &bounds);

        if self.capture_events && matches!(event, Event::Mouse(_)) {
            shell.capture_event();
        }
    }

    /// 处理 mouse interaction 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn mouse_interaction(
        &self,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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

    /// 处理 draw 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn draw(
        &self,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &mut RendererT,
        // theme 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        theme: &ThemeT,
        // defaults 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        defaults: &renderer::Style,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
    ) {
        self.overlay.as_widget().draw(
            self.tree,
            renderer,
            theme,
            defaults,
            layout,
            cursor,
            &layout.bounds(),
        );
    }
}
