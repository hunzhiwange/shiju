use super::*;

impl<Message, ThemeT, RendererT> Widget<Message, ThemeT, RendererT>
    for LeftOverlay<'_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    Message: Clone,
{
    /// 处理 children 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回集合保持输入顺序或界面展示顺序，空集合表示没有可展示项。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content), widget::Tree::new(&self.overlay)]
    }

    /// 处理 diff 对应的局部职责。
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
    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.content.as_widget(), self.overlay.as_widget()]);
    }

    /// 处理 size 对应的局部职责。
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
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    /// 处理 size hint 对应的局部职责。
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
    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    /// 处理 layout 对应的局部职责。
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
    fn layout(
        &mut self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &mut widget::Tree,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // limits 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
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
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &widget::Tree,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &mut RendererT,
        // theme 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        theme: &ThemeT,
        // style 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        style: &renderer::Style,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &mut widget::Tree,
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
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &widget::Tree,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        viewport: &Rectangle,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
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

    /// 构建或定位 overlay，用于把浮层稳定附着到目标控件。
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
    fn overlay<'b>(
        &'b mut self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &'b mut widget::Tree,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'b>,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        viewport: &Rectangle,
        // translation 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, ThemeT, RendererT>> {
        let mut children = tree.children.iter_mut();

        let content = self.content.as_widget_mut().overlay(
            children.next().unwrap(),
            layout,
            renderer,
            viewport,
            translation,
        );

        let side = if self.show {
            Some(overlay::Element::new(Box::new(LeftOverlayElement {
                // position 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                position: layout.position() + translation,
                // target_bounds 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                target_bounds: layout.bounds(),
                // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                viewport: *viewport,
                // gap 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                gap: self.gap,
                // snap 开关让浮层在靠近窗口边缘时仍保持可见。
                // snap_within_viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                snap_within_viewport: self.snap_within_viewport,
                // overlay 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                overlay: &mut self.overlay,
                // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                tree: children.next().unwrap(),
                // on_close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                on_close: self.on_close.clone(),
            })))
        } else {
            None
        };

        if content.is_some() || side.is_some() {
            Some(overlay::Group::with_children(content.into_iter().chain(side).collect()).overlay())
        } else {
            None
        }
    }
}
