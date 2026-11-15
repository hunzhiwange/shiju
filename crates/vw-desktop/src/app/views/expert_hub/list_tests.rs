//! 专家中心列表测试，覆盖技能标签、访问状态、搜索分页和响应式卡片布局。
//! 多选筛选与商业技能操作必须依赖服务器安装状态，并在两种主题下保持可读。

use super::list::{SCROLLBAR_GUTTER, SCROLLBAR_WIDTH, access_action_message, installed_quick_upgrade_message, market_catalog_filter_options, market_tag_option_style, marketplace_quick_upgrade_action, skill_tab_specs};
use super::styles::{card_style};
use crate::app::message::{AccountMessage, SkillHubMessage};
use crate::app::skill_hub::{CursorPage, InstalledSkill, LoadState, MarketplaceSkill, Page, SkillTab, SkillTag, SkillVersionDirection};
use crate::app::{App, Message};
use iced::Theme;


fn set_state(app: &mut App, tab: SkillTab, state: usize) {
    app.expert_hub.skill_tab = tab;
    match tab {
        SkillTab::Marketplace => {
            app.expert_hub.marketplace = match state {
                0 => LoadState::Loading { generation: 1 },
                1 => LoadState::Loaded(CursorPage {
                    items: Vec::new(),
                    next_cursor: None,
                    has_more: false,
                }),
                2 => LoadState::Error(("network error".to_string()).into()),
                _ => LoadState::NeedsLogin,
            };
        }
        SkillTab::Installed => {
            app.expert_hub.installed = match state {
                0 => LoadState::Loading { generation: 1 },
                1 => LoadState::Loaded(Vec::new()),
                2 => LoadState::Error(("network error".to_string()).into()),
                _ => LoadState::NeedsLogin,
            };
        }
        SkillTab::System => {
            app.expert_hub.system = match state {
                0 => LoadState::Loading { generation: 1 },
                1 => {
                    LoadState::Loaded(Page { items: Vec::new(), page: 1, page_size: 20, total: 0 })
                }
                2 => LoadState::Error(("network error".to_string()).into()),
                _ => LoadState::NeedsLogin,
            };
        }
        SkillTab::Mine => {
            app.expert_hub.mine = match state {
                0 => LoadState::Loading { generation: 1 },
                1 => {
                    LoadState::Loaded(Page { items: Vec::new(), page: 1, page_size: 20, total: 0 })
                }
                2 => LoadState::Error(("network error".to_string()).into()),
                _ => LoadState::NeedsLogin,
            };
        }
        SkillTab::Install => {
            app.expert_hub.install_guide.open = true;
        }
    }
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
        crate::app::i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &platform,
    );
    let (company_options, company_feedback) = market_catalog_filter_options(
        crate::app::i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
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
        crate::app::i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &LoadState::Loaded(Vec::new()),
    );
    assert_eq!(empty.len(), 1);
    assert_eq!(empty_feedback, Some("暂无可选标签"));
    let (_, failed_feedback) = market_catalog_filter_options(
        crate::app::i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
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

    let (mut app, _) = App::new();
    app.expert_hub.skill_tab = SkillTab::Marketplace;
    app.expert_hub.platform_tag_catalog = platform;
    app.expert_hub.company_tag_catalog = company;
    app.expert_hub.market_tag_id = Some(1);
    app.expert_hub.market_company_tag_id = Some(7);
    let element: iced::Element<'_, Message> = super::view(&app);
    std::hint::black_box(element);
}

#[test]
fn expert_hub_view_builds_all_visible_skill_tabs_and_access_states() {
    let (mut app, _) = App::new();

    assert!(!skill_tab_specs().iter().any(|(_, tab)| *tab == SkillTab::System));
    assert!(!skill_tab_specs().iter().any(|(label, _)| *label == "公司技能"));
    for &(_, tab) in skill_tab_specs() {
        for state in 0..4 {
            set_state(&mut app, tab, state);
            let element: iced::Element<'_, Message> = super::view(&app);
            std::hint::black_box(element);
        }
    }

    assert_eq!(SCROLLBAR_WIDTH, 4);
    assert_eq!(SCROLLBAR_GUTTER, 12);
    assert!(SCROLLBAR_GUTTER > SCROLLBAR_WIDTH);
    assert!(matches!(access_action_message(), Message::Account(AccountMessage::StartBrowserLogin)));
    for theme in [Theme::Light, Theme::Dark] {
        let style = card_style(&theme);
        assert!(style.background.is_some());
        assert!(style.text_color.is_some());
        assert_ne!(style.background.unwrap(), style.text_color.unwrap().into());
    }
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

    let (mut app, _) = App::new();
    app.expert_hub.marketplace =
        LoadState::Loaded(CursorPage { items: vec![market], next_cursor: None, has_more: false });
    app.expert_hub.installed = LoadState::Loaded(vec![installed]);
    let view: iced::Element<'_, Message> = super::view(&app);
    std::hint::black_box(view);
}

#[tokio::test]
async fn skill_form_controls_render_system_tag_menu() {
    use super::public_skills_tests::{CONTROL_LOCALES, SkillControls, control_renderer};
    use crate::app::skill_hub::SystemSkill;
    use iced::keyboard::key::Named;

    let mut renderer = control_renderer().await;
    let (mut app, _) = App::new();
    app.expert_hub.skill_tab = SkillTab::System;
    app.expert_hub.system_filter.keyword_draft = "fixture".into();
    for (locale, name) in CONTROL_LOCALES {
        app.desktop_locale = locale;
        app.expert_hub.system = LoadState::Loaded(Page {
            items: vec![SystemSkill {
                id: 1,
                slug: "fixture".into(),
                name: name.into(),
                description: name.into(),
                is_all_company_accessible: 1,
                tags: vec![SkillTag { id: 57, name: name.into(), sort_order: 0 }],
            }],
            page: 1,
            page_size: 20,
            total: 1,
        });
        for theme in [Theme::Light, Theme::Dark] {
            for width in [420, 760] {
                let mut controls =
                    SkillControls::new(super::list::view(&app), &mut renderer, width);
                let input = controls.bounds(&[3, 0]);
                let tag = controls.bounds(&[3, 1]);
                let submit = controls.bounds(&[3, 2, 0]);
                let reset = controls.bounds(&[3, 2, 1]);
                assert!(input.width >= 300.0);
                assert!(matches!(
                    controls.choose_second(&mut renderer, &theme, tag, "tag").as_slice(),
                    [Message::SkillHub(SkillHubMessage::FilterTagChanged(Some(57)))]
                ));
                assert!(controls.enter(&mut renderer, tag).is_empty());
                assert!(matches!(
                    controls.key(&mut renderer, Named::Enter).as_slice(),
                    [Message::SkillHub(SkillHubMessage::FilterTagChanged(None))]
                ));
                for messages in [
                    controls.enter(&mut renderer, input),
                    controls.click(&mut renderer, submit.center()),
                ] {
                    assert!(matches!(
                        messages.as_slice(),
                        [Message::SkillHub(SkillHubMessage::FiltersSubmitted)]
                    ));
                }
                assert!(matches!(
                    controls.click(&mut renderer, reset.center()).as_slice(),
                    [Message::SkillHub(SkillHubMessage::FiltersReset)]
                ));
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn installed_upgrade_actions_render_chinese_targets_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::widget::Tree;
    use iced::advanced::{Layout, layout, mouse, renderer};
    use iced::{Color, Font, Pixels, Rectangle, Size};

    let mut app = App::new().0;
    app.expert_hub.install_guide.selected_targets =
        vec![crate::app::skill_hub::SkillInstallTarget::Codex];
    for theme in [Theme::Dark, Theme::Light] {
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("headless renderer");
        for width in [320.0, 900.0] {
            let mut element = super::upgrade_actions::view(&app);
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
