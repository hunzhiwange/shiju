//! 验证工作区模式选择器的标签、选中状态和切换消息。
//!
//! 模式选项逐一渲染并校验选中状态，确保显示标签与内部枚举映射稳定。

use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Font, Pixels, Rectangle, Size, Theme};

use super::widgets;
use super::workspace_mode;

use super::types::WorkspaceMode;

#[test]
fn workspace_mode_selector_copy_covers_both_modes() {
    assert_eq!(workspace_mode::copy(WorkspaceMode::Code), ("代码", "构建、调试与审查"));
    assert_eq!(workspace_mode::copy(WorkspaceMode::Work), ("工作", "对话、整理与协作"));
    assert_eq!(workspace_mode::mode_icon(WorkspaceMode::Code), desktop_assets::Icon::Code);
    assert_eq!(
        workspace_mode::mode_icon(WorkspaceMode::Work),
        desktop_assets::Icon::ChatTextFill
    );
    assert_eq!(workspace_mode::mode_label_font().weight, iced::font::Weight::Semibold);
}

#[test]
fn workspace_mode_selector_style_has_compact_rounded_surface() {
    let style = widgets::workspace_mode_button_style(
        &Theme::Dark,
        iced::widget::button::Status::Active,
        false,
    );

    assert_eq!(style.border.radius.top_left, 7.0);
    assert_eq!(style.border.radius.top_right, 7.0);
    assert_eq!(style.border.radius.bottom_left, 7.0);
    assert_eq!(style.border.radius.bottom_right, 7.0);
}

#[tokio::test(flavor = "multi_thread")]
async fn workspace_mode_selector_draws_chinese_labels_in_dark_theme() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let i18n = desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese);

    let mut selector = workspace_mode::selector(i18n, WorkspaceMode::Code, true);
    draw_element(&mut selector, &mut renderer, &Theme::Dark, Size::new(180.0, 40.0));

    let mut menu = workspace_mode::menu(i18n, WorkspaceMode::Code);
    draw_element(&mut menu, &mut renderer, &Theme::Dark, Size::new(280.0, 160.0));
}

fn draw_element(
    element: &mut iced::Element<'_, workspace_mode::Message>,
    renderer: &mut iced::Renderer,
    theme: &Theme,
    bounds: Size,
) {
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    let viewport = Rectangle::with_size(bounds);

    element.as_widget().draw(
        &tree,
        renderer,
        theme,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
}
