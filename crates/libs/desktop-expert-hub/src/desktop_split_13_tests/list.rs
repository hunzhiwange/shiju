use crate::message::skill_hub::SkillHubMessage;
use crate::views::{
    Message,
    list::{
        installed_quick_upgrade_message, market_catalog_filter_options, market_tag_option_style,
        marketplace_quick_upgrade_action,
    },
};
use desktop_skill_hub::skill_hub::*;
use iced::Theme;
fn view(state: &ExpertHubState) -> iced::Element<'_, Message> {
    let input = crate::views::ViewInput {
        expert_hub: state,
        desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
        app_theme: &Theme::Dark,
    };
    crate::views::list::view(
        &input,
        crate::views::install_guide::view(&crate::views::install_guide::InstallInput {
            state: &state.install_guide,
            desktop_locale: input.desktop_locale,
            project_path: &None,
            recent_projects: &[],
        }),
    )
}
#[test]
fn market_tag_rows_show_full_catalogs() {
    let platform = LoadState::Loaded(vec![
        SkillTag { id: 3, name: "平台三".to_string(), sort_order: 30 },
        SkillTag { id: 1, name: "平台一".to_string(), sort_order: 10 },
    ]);
    let company = LoadState::Loaded(vec![
        SkillTag { id: 8, name: "公司八".to_string(), sort_order: 20 },
        SkillTag { id: 7, name: "公司七".to_string(), sort_order: 10 },
    ]);

    let (platform_options, platform_feedback) = market_catalog_filter_options(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &platform,
    );
    let (company_options, company_feedback) = market_catalog_filter_options(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &company,
    );
    assert_eq!(
        platform_options
            .iter()
            .map(|option| (option.id, option.label.as_str()))
            .collect::<Vec<_>>(),
        vec![(None, "全部"), (Some(1), "平台一"), (Some(3), "平台三")]
    );
    assert_eq!(
        company_options.iter().map(|option| (option.id, option.label.as_str())).collect::<Vec<_>>(),
        vec![(None, "全部"), (Some(7), "公司七"), (Some(8), "公司八")]
    );
    assert_eq!(platform_feedback, None);
    assert_eq!(company_feedback, None);

    let (empty, empty_feedback) = market_catalog_filter_options(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &LoadState::Loaded(Vec::new()),
    );
    assert_eq!(empty.len(), 1);
    assert_eq!(empty_feedback, Some("暂无可选标签"));
    let (_, failed_feedback) = market_catalog_filter_options(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &LoadState::Error(("目录不可用".to_string()).into()),
    );
    assert_eq!(failed_feedback, Some("标签目录加载失败"));

    for theme in [Theme::Light, Theme::Dark] {
        let selected = market_tag_option_style(&theme, true);
        let unselected = market_tag_option_style(&theme, false);
        assert_ne!(selected.background, unselected.background);
        for style in [selected, unselected] {
            let Some(iced::Background::Color(background)) = style.background else {
                panic!("tag filters need a solid background over wallpaper");
            };
            assert_eq!(background.a, 1.0);
        }
    }

    let mut app = ExpertHubState::default();
    app.skill_tab = SkillTab::Marketplace;
    app.platform_tag_catalog = platform;
    app.company_tag_catalog = company;
    app.market_tag_id = Some(1);
    app.market_company_tag_id = Some(7);
    let element: iced::Element<'_, Message> = view(&app);
    std::hint::black_box(element);
}

#[test]
fn business_skill_list_actions_use_installed_server_state() {
    let market = MarketplaceSkill {
        id: 7,
        slug: "safe-skill".to_string(),
        name: "安全技能".to_string(),
        description: "fixture".to_string(),
        level: "basic".to_string(),
        modality: "text".to_string(),
        format: "markdown".to_string(),
        company_id: "company-1".to_string(),
        created_uid: "user-1".to_string(),
        maintainer_uid: "user-1".to_string(),
        status: "published".to_string(),
        is_all_company_accessible: 1,
        current_version_id: 42,
        created_at: String::new(),
        updated_at: String::new(),
        deleted_at: None,
        is_installed: true,
        tags: Vec::new(),
    };
    let mut installed = InstalledSkill {
        skill_id: 7,
        slug: "safe-skill".to_string(),
        name: "安全技能".to_string(),
        description: "fixture".to_string(),
        installed_version: "1.0.0".to_string(),
        current_version: "1.2.0".to_string(),
        format: "markdown".to_string(),
        upgrade_available: false,
    };

    assert!(marketplace_quick_upgrade_action(&market, &[installed.clone()]).is_none());
    assert!(installed_quick_upgrade_message(&installed).is_none());

    installed.upgrade_available = true;
    let target = marketplace_quick_upgrade_action(&market, &[installed.clone()])
        .expect("server upgrade flag should expose marketplace upgrade");
    assert_eq!(target.skill_id, 7);
    assert_eq!(target.version_id, 42);
    assert_eq!(target.target_version, "1.2.0");
    assert_eq!(target.direction, SkillVersionDirection::Upgrade);
    assert!(matches!(
        installed_quick_upgrade_message(&installed),
        Some(Message::SkillHub(SkillHubMessage::UpgradeSkill(7)))
    ));

    let mut versionless_market = market.clone();
    versionless_market.current_version_id = 0;
    assert!(marketplace_quick_upgrade_action(&versionless_market, &[installed.clone()]).is_none());

    let mut app = ExpertHubState::default();
    app.marketplace =
        LoadState::Loaded(CursorPage { items: vec![market], next_cursor: None, has_more: false });
    app.installed = LoadState::Loaded(vec![installed]);
    let view: iced::Element<'_, Message> = view(&app);
    std::hint::black_box(view);
}

#[tokio::test(flavor = "multi_thread")]
async fn installed_upgrade_actions_render_chinese_targets_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::widget::Tree;
    use iced::advanced::{Layout, layout, mouse, renderer};
    use iced::{Color, Font, Pixels, Rectangle, Size};

    let mut app = ExpertHubState::default();
    app.install_guide.selected_targets =
        vec![desktop_skill_hub::skill_hub::SkillInstallTarget::Codex];
    for theme in [Theme::Dark, Theme::Light] {
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("headless renderer");
        for width in [320.0, 900.0] {
            let mut element = crate::views::upgrade_actions::view(&crate::views::ViewInput {
                expert_hub: &app,
                desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
                app_theme: &theme,
            });
            let bounds = Size::new(width, 280.0);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
            assert!(node.bounds().width <= bounds.width);
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style { text_color: theme.palette().text },
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &Rectangle::with_size(bounds),
            );
            let pixels = renderer.screenshot(Size::new(width as u32, 280), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        }
    }
}
