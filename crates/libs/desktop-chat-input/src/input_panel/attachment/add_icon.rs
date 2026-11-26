use std::f32::consts::FRAC_PI_4;

use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, renderer, svg, widget::tree};
use iced::animation::{Animation, Easing};
use iced::time::Instant;
use iced::{Event, Length, Radians, Rectangle, Size, Theme, mouse, window};

use super::{BOTTOM_BAR_ICON_SIZE, ComposerIcon};

pub(super) struct AddIcon {
    expanded: bool,
    handle: svg::Handle,
}

impl AddIcon {
    pub(super) fn new(expanded: bool) -> Self {
        Self { expanded, handle: ComposerIcon::Add.handle() }
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for AddIcon {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Animation<bool>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Animation::new(self.expanded).quick().easing(Easing::EaseInOutCubic))
    }

    fn size(&self) -> Size<Length> {
        let size = Length::Fixed(BOTTOM_BAR_ICON_SIZE);
        Size::new(size, size)
    }

    fn layout(
        &mut self,
        _tree: &mut tree::Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, BOTTOM_BAR_ICON_SIZE, BOTTOM_BAR_ICON_SIZE)
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            let animation = tree.state.downcast_mut::<Animation<bool>>();
            animation.go_mut(self.expanded, *now);
            if animation.is_animating(*now) {
                shell.request_redraw();
            }
        }
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use svg::Renderer as _;

        let animation = tree.state.downcast_ref::<Animation<bool>>();
        let angle = animation.interpolate(0.0, FRAC_PI_4, Instant::now());
        let image =
            svg::Svg::new(self.handle.clone()).rotation(Radians(angle)).color(theme.palette().text);
        renderer.draw_svg(image, layout.bounds(), *viewport);
    }
}
