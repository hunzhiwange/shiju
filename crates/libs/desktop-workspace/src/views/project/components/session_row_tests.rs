use super::*;
use iced::advanced::{Layout, layout, renderer::Headless, widget::Tree};
use iced::{Font, Pixels, Size, Theme, mouse};
#[tokio::test]
async fn running_indicator_rotates_around_a_centered_breathing_dot() {
    use iced::Rectangle;
    use iced::advanced::renderer::{Renderer as _, Style};

    let mut renderer =
        iced::Renderer::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")).await.unwrap();
    let size = Size::new(STATUS_SIZE, STATUS_SIZE);
    let viewport = Rectangle::with_size(size);
    let limits = layout::Limits::new(Size::ZERO, size);
    const SCALE: u32 = 4;
    let pixels = STATUS_SIZE as u32 * SCALE;

    for theme in [Theme::Light, Theme::Dark] {
        let mut render = |frame, breathe| {
            let mut indicator = running_indicator(frame, breathe);
            let mut tree = Tree::new(indicator.as_widget());
            let node = indicator.as_widget_mut().layout(&mut tree, &renderer, &limits);
            assert_eq!(node.size(), size);
            renderer.reset(viewport);
            indicator.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &Style::default(),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &viewport,
            );
            renderer.screenshot(Size::new(pixels, pixels), SCALE as f32, theme.palette().background)
        };
        let contracted = render(0, 0.0);
        let expanded = render(RUNNING_RING_FRAMES / 2, 1.0);
        let contrast = |image: &[u8], x, y| {
            let offset = ((y * pixels + x) * 4) as usize;
            let background = &image[..3];
            image[offset..offset + 3]
                .iter()
                .zip(background)
                .map(|(channel, background)| u32::from(channel.abs_diff(*background)))
                .sum::<u32>()
        };
        let center = pixels / 2;
        let ring_radius = 22;
        let top = |image: &[u8]| contrast(image, center, center - ring_radius);
        let bottom = |image: &[u8]| contrast(image, center, center + ring_radius);
        assert!(top(&contracted) < bottom(&contracted));
        assert!(top(&expanded) > bottom(&expanded));
        for image in [&contracted, &expanded] {
            assert!(contrast(image, center, center) > 0);
            assert_eq!(contrast(image, center - 16, center), 0);
        }
        assert_eq!(contrast(&contracted, center + 12, center), 0);
        assert!(contrast(&expanded, center + 12, center) > 0);
    }
}

#[test]
fn session_agent_description_distinguishes_native_and_acp_bindings() {
    let i18n = desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese);
    assert_eq!(super::agent_description(i18n, None, None, None), "识矩原生");
    assert_eq!(
        super::agent_description(i18n, Some("reviewer"), Some("代码审查"), None),
        "代码审查 · 识矩原生"
    );
    assert_eq!(
        super::agent_description(i18n, Some("reviewer"), Some("代码审查"), Some("codex")),
        "代码审查 · ACP · codex"
    );
}

#[tokio::test]
async fn session_hover_details_shape_chinese_workspace_and_agents_in_both_themes() {
    let i18n = desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese);
    let details = SessionRowDetails {
        title: "检查悬浮详情".into(),
        directory: "/项目/工作区".into(),
        updated: 0,
        project: "示例项目".into(),
        project_id: "project".into(),
        created: 0,
        changes: None,
        pending: false,
    };
    let size = Size::new(320.0, 360.0);
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .unwrap();
    for theme in [Theme::Light, Theme::Dark] {
        for workspace in [Some(true), Some(false), None] {
            let mut element = super::details_card(
                i18n,
                workspace,
                "审查智能体 · ACP · codex".into(),
                details.title.clone(),
                &details,
                SessionRowStatus::Idle,
                "刚刚",
            );
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, size),
            );
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &iced::advanced::renderer::Style { text_color: theme.palette().text },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &iced::Rectangle::with_size(size),
            );
            assert!(
                !renderer
                    .screenshot(Size::new(320, 360), 1.0, theme.palette().background)
                    .is_empty()
            );
        }
    }
}