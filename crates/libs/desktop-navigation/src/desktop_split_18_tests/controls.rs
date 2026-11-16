//! 验证顶部栏窗口控件在不同平台与窗口状态下的行为。
//!
//! 平台与窗口状态分开构造，确保最小化、最大化和关闭动作保持稳定映射。

use crate::components::top_bar::controls;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, mouse, overlay, renderer, widget::Tree};
use iced::{Color, Event, Font, Rectangle, Renderer, Size, Theme, Vector};

#[tokio::test]
async fn settings_tooltip_draws_readable_chinese_text_on_dark_background() {
    let theme = Theme::custom(
        "Low contrast",
        iced::theme::Palette {
            background: Color::from_rgb8(16, 16, 16),
            text: Color::from_rgb8(32, 32, 32),
            ..Theme::Dark.palette()
        },
    );
    iced::advanced::graphics::text::font_system()
        .write()
        .unwrap()
        .load_font(std::borrow::Cow::Borrowed(desktop_assets::fonts::NOTO_SANS_CJK_SC_REGULAR));
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 16.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    let viewport = Rectangle::with_size(Size::new(180.0, 90.0));
    let mut button = controls::settings_button(desktop_i18n::I18n::new(
        config_ui::DesktopLocale::SimplifiedChinese,
    ));
    let mut tree = Tree::new(button.as_widget());
    let node = button.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, viewport.size()),
    );
    let root = Layout::new(&node);
    let position = root.bounds().center();
    let cursor = mouse::Cursor::Available(position);
    button.as_widget_mut().update(
        &mut tree,
        &Event::Mouse(mouse::Event::CursorMoved { position }),
        root,
        cursor,
        &renderer,
        &mut clipboard::Null,
        &mut Shell::new(&mut Vec::new()),
        &viewport,
    );
    let overlay = button
        .as_widget_mut()
        .overlay(&mut tree, root, &renderer, &viewport, Vector::ZERO)
        .expect("hovered settings tooltip");
    let mut overlay = overlay::Nested::new(overlay);
    let node = overlay.layout(&renderer, viewport.size());
    overlay.draw(
        &mut renderer,
        &theme,
        &renderer::Style { text_color: Color::BLACK },
        Layout::new(&node),
        cursor,
    );

    let pixels = renderer.screenshot(Size::new(180, 90), 1.0, theme.palette().background);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[..3].iter().all(|&value| value > 160)));
}
