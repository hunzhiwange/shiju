use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::{Element, Length};
use iced::{Event, Point, Rectangle, Size, Theme, Vector};

fn available_space_below(target_bounds: Rectangle, viewport: Rectangle, gap: f32) -> f32 {
    (viewport.y + viewport.height) - (target_bounds.y + target_bounds.height + gap)
}

fn available_space_above(target_bounds: Rectangle, viewport: Rectangle, gap: f32) -> f32 {
    target_bounds.y - viewport.y - gap
}

fn should_place_overlay_above(
    target_bounds: Rectangle,
    viewport: Rectangle,
    overlay_height: f32,
    gap: f32,
) -> bool {
    let space_below = available_space_below(target_bounds, viewport, gap);
    let space_above = available_space_above(target_bounds, viewport, gap);

    overlay_height > space_below && space_above > space_below
}

fn compute_overlay_position(
    target_bounds: Rectangle,
    viewport: Rectangle,
    overlay_size: Size,
    gap: f32,
    snap_within_viewport: bool,
    place_above: bool,
) -> Point {
    let mut x = target_bounds.x;
    let below_y = target_bounds.y + target_bounds.height + gap;
    let above_y = target_bounds.y - overlay_size.height - gap;
    let mut y = if place_above { above_y } else { below_y };

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

fn layout_with_backdrop(viewport: Rectangle, overlay_node: layout::Node) -> layout::Node {
    layout::Node::with_children(viewport.size(), vec![overlay_node]).move_to(viewport.position())
}

pub struct BelowOverlay<'a, Message, ThemeT = Theme, RendererT = iced::Renderer> {
    content: Element<'a, Message, ThemeT, RendererT>,
    overlay: Element<'a, Message, ThemeT, RendererT>,
    show: bool,
    gap: f32,
    snap_within_viewport: bool,
    on_close: Option<Message>,
    capture_outside_click: bool,
}

impl<'a, Message, ThemeT, RendererT> BelowOverlay<'a, Message, ThemeT, RendererT> {
    pub fn new(
        content: impl Into<Element<'a, Message, ThemeT, RendererT>>,
        overlay: impl Into<Element<'a, Message, ThemeT, RendererT>>,
    ) -> Self {
        Self {
            content: content.into(),
            overlay: overlay.into(),
            show: false,
            gap: 0.0,
            snap_within_viewport: true,
            on_close: None,
            capture_outside_click: true,
        }
    }

    pub fn show(mut self, show: bool) -> Self {
        self.show = show;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn on_close(mut self, msg: Message) -> Self {
        self.on_close = Some(msg);
        self
    }

    pub fn snap_within_viewport(mut self, snap: bool) -> Self {
        self.snap_within_viewport = snap;
        self
    }

    pub fn capture_outside_click(mut self, capture: bool) -> Self {
        self.capture_outside_click = capture;
        self
    }
}

impl<Message, ThemeT, RendererT> Widget<Message, ThemeT, RendererT>
    for BelowOverlay<'_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    Message: Clone,
{
    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content), widget::Tree::new(&self.overlay)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[self.content.as_widget(), self.overlay.as_widget()]);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &RendererT,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

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

        if self.show
            && self.capture_outside_click
            && matches!(event, Event::Mouse(_))
            && cursor.position().is_some_and(|position| !layout.bounds().contains(position))
        {
            shell.capture_event();
        }
    }

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

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &RendererT,
        viewport: &Rectangle,
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

        let below = if self.show {
            let mut target_bounds = layout.bounds();
            target_bounds.x += translation.x;
            target_bounds.y += translation.y;
            Some(overlay::Element::new(Box::new(BelowOverlayElement {
                position: layout.position() + translation,
                target_bounds,
                viewport: *viewport,
                gap: self.gap,
                snap_within_viewport: self.snap_within_viewport,
                overlay: &mut self.overlay,
                tree: children.next().unwrap(),
                on_close: self.on_close.clone(),
                capture_outside_click: self.capture_outside_click,
            })))
        } else {
            None
        };

        if content.is_some() || below.is_some() {
            Some(
                overlay::Group::with_children(content.into_iter().chain(below).collect()).overlay(),
            )
        } else {
            None
        }
    }
}

impl<'a, Message, ThemeT, RendererT> From<BelowOverlay<'a, Message, ThemeT, RendererT>>
    for Element<'a, Message, ThemeT, RendererT>
where
    Message: 'a + Clone,
    RendererT: iced::advanced::Renderer + 'a,
    ThemeT: 'a,
{
    fn from(widget: BelowOverlay<'a, Message, ThemeT, RendererT>) -> Self {
        Element::new(widget)
    }
}

struct BelowOverlayElement<'a, 'b, Message, ThemeT, RendererT> {
    position: Point,
    target_bounds: Rectangle,
    viewport: Rectangle,
    gap: f32,
    snap_within_viewport: bool,
    overlay: &'b mut Element<'a, Message, ThemeT, RendererT>,
    tree: &'b mut widget::Tree,
    on_close: Option<Message>,
    capture_outside_click: bool,
}

impl<Message, ThemeT, RendererT> overlay::Overlay<Message, ThemeT, RendererT>
    for BelowOverlayElement<'_, '_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
    Message: Clone,
{
    fn layout(&mut self, renderer: &RendererT, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);

        let natural_max_h = if self.snap_within_viewport { viewport.height } else { bounds.height };
        let mut node = self.overlay.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(viewport.width, natural_max_h)),
        );

        let place_above = should_place_overlay_above(
            self.target_bounds,
            self.viewport,
            node.size().height,
            self.gap,
        );

        if self.snap_within_viewport {
            let available_height = if place_above {
                available_space_above(self.target_bounds, self.viewport, self.gap)
            } else {
                available_space_below(self.target_bounds, self.viewport, self.gap)
            }
            .max(0.0);

            if node.size().height > available_height {
                // 先确定展开方向，再按该方向的剩余空间收缩，避免覆盖触发控件。
                node = self.overlay.as_widget_mut().layout(
                    self.tree,
                    renderer,
                    &layout::Limits::new(Size::ZERO, Size::new(viewport.width, available_height)),
                );
            }
        }

        let overlay_position = compute_overlay_position(
            Rectangle {
                x: self.position.x,
                y: self.target_bounds.y,
                width: self.target_bounds.width,
                height: self.target_bounds.height,
            },
            self.viewport,
            node.size(),
            self.gap,
            self.snap_within_viewport,
            place_above,
        );

        layout_with_backdrop(viewport, node.move_to(overlay_position))
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &RendererT,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(overlay_layout) = layout.children().next() else {
            return;
        };
        let overlay_bounds = overlay_layout.bounds();
        let cursor_position = cursor.position();

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
            && let Some(on_close) = &self.on_close
            && let Some(cursor_position) = cursor_position
            && !self.target_bounds.contains(cursor_position)
            && !overlay_bounds.contains(cursor_position)
        {
            shell.publish(on_close.clone());
            if self.capture_outside_click {
                shell.capture_event();
            }
            return;
        }

        let is_over_overlay = cursor_position.is_some_and(|p| overlay_bounds.contains(p));
        let is_over_target = cursor_position.is_some_and(|p| self.target_bounds.contains(p));

        if !matches!(event, Event::Mouse(_)) || is_over_overlay {
            self.overlay.as_widget_mut().update(
                self.tree,
                event,
                overlay_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                &overlay_bounds,
            );
        }

        if matches!(event, Event::Mouse(_))
            && (is_over_overlay || (self.capture_outside_click && !is_over_target))
        {
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &RendererT,
    ) -> mouse::Interaction {
        let Some(overlay_layout) = layout.children().next() else {
            return mouse::Interaction::None;
        };

        self.overlay.as_widget().mouse_interaction(
            self.tree,
            overlay_layout,
            cursor,
            &self.viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        renderer: &mut RendererT,
        theme: &ThemeT,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let Some(overlay_layout) = layout.children().next() else {
            return;
        };
        let bounds = overlay_layout.bounds();

        self.overlay.as_widget().draw(
            self.tree,
            renderer,
            theme,
            defaults,
            overlay_layout,
            cursor,
            &bounds,
        );
    }
}

#[cfg(test)]
#[path = "below_tests.rs"]
mod tests;
