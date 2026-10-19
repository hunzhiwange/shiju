//! 专家中心列表测试，覆盖技能标签、访问状态、搜索分页和响应式卡片布局。
//! 多选筛选与商业技能操作必须依赖服务器安装状态，并在两种主题下保持可读。

use super::list::{
    SCROLLBAR_GUTTER, SCROLLBAR_WIDTH, access_action_message, description_preview,
    description_tooltip, detail_action_message, grid_column_count, install_action_message,
    installed_quick_upgrade_message, market_search_messages, marketplace_quick_upgrade_action,
    mine_filter_menu_entry_style, mine_filter_menu_style, mine_status_filter_label,
    mine_tag_filter_label, mine_tag_filter_options, pagination_messages, skill_tab_specs,
};
use super::styles::{
    action_button_style, card_style, search_input_style, selected_tab_style, skill_badge_style,
};
use crate::app::message::{AccountMessage, SkillHubMessage};
use crate::app::skill_hub::{
    CursorPage, InstalledSkill, LoadState, MarketplaceSkill, MineSkillStatus, Page, SkillTab,
    SkillTag, SkillVersionDirection,
};
use crate::app::{App, Message};
use iced::Theme;
use iced::widget::{button, text_input};

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
                2 => LoadState::Error("network error".to_string()),
                _ => LoadState::NeedsLogin,
            };
        }
        SkillTab::Installed => {
            app.expert_hub.installed = match state {
                0 => LoadState::Loading { generation: 1 },
                1 => LoadState::Loaded(Vec::new()),
                2 => LoadState::Error("network error".to_string()),
                _ => LoadState::NeedsLogin,
            };
        }
        SkillTab::System => {
            app.expert_hub.system = match state {
                0 => LoadState::Loading { generation: 1 },
                1 => {
                    LoadState::Loaded(Page { items: Vec::new(), page: 1, page_size: 20, total: 0 })
                }
                2 => LoadState::Error("network error".to_string()),
                _ => LoadState::NeedsLogin,
            };
        }
        SkillTab::Mine => {
            app.expert_hub.mine = match state {
                0 => LoadState::Loading { generation: 1 },
                1 => {
                    LoadState::Loaded(Page { items: Vec::new(), page: 1, page_size: 20, total: 0 })
                }
                2 => LoadState::Error("network error".to_string()),
                _ => LoadState::NeedsLogin,
            };
        }
        SkillTab::Install => {
            app.expert_hub.install_guide.open = true;
        }
    }
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
fn shiju_install_is_the_last_skill_tab() {
    assert_eq!(skill_tab_specs().last(), Some(&("安装识矩技能", SkillTab::Install)));
}

#[test]
fn expert_hub_market_search_and_pagination_emit_scoped_messages() {
    let (changed, submitted) = market_search_messages("视觉".to_string());
    assert!(matches!(
        changed,
        Message::SkillHub(SkillHubMessage::SearchDraftChanged(query)) if query == "视觉"
    ));
    assert!(matches!(submitted, Message::SkillHub(SkillHubMessage::SearchSubmitted)));

    let first: Page<()> = Page { items: Vec::new(), page: 1, page_size: 20, total: 21 };
    let middle: Page<()> = Page { items: Vec::new(), page: 2, page_size: 20, total: 41 };
    let last: Page<()> = Page { items: Vec::new(), page: 3, page_size: 20, total: 41 };
    let (previous, next) = pagination_messages(&first);
    assert!(previous.is_none());
    assert!(matches!(next, Some(Message::SkillHub(SkillHubMessage::NextPage))));
    let (previous, next) = pagination_messages(&middle);
    assert!(matches!(previous, Some(Message::SkillHub(SkillHubMessage::PreviousPage))));
    assert!(matches!(next, Some(Message::SkillHub(SkillHubMessage::NextPage))));
    let (_, next) = pagination_messages(&last);
    assert!(next.is_none());

    assert!(matches!(
        detail_action_message(7),
        Message::SkillHub(SkillHubMessage::OpenSkillDetail(7))
    ));
    assert!(matches!(
        install_action_message(7, false),
        Message::SkillHub(SkillHubMessage::InstallSkill(7))
    ));
    assert!(matches!(
        install_action_message(7, true),
        Message::SkillHub(SkillHubMessage::UninstallSkill(7))
    ));
}

#[test]
fn mine_multi_select_labels_and_tag_fallbacks_are_stable() {
    let tags = [
        SkillTag { id: 9, name: "设计".to_string(), sort_order: 0 },
        SkillTag { id: 2, name: "开发".to_string(), sort_order: 1 },
        SkillTag { id: 9, name: "重复名称".to_string(), sort_order: 2 },
    ];
    let options = mine_tag_filter_options(tags.iter(), &[2, 7]);

    assert_eq!(options.iter().map(|option| option.id).collect::<Vec<_>>(), vec![9, 2, 7]);
    assert_eq!(
        options.iter().map(|option| option.label.as_str()).collect::<Vec<_>>(),
        vec!["设计", "开发", "标签 7"]
    );
    assert_eq!(mine_tag_filter_label(&[]), "选择标签");
    assert_eq!(mine_tag_filter_label(&[9, 2]), "已选 2 个标签");
    assert_eq!(mine_status_filter_label(&MineSkillStatus::ALL), "已选 4 个状态");
}

#[test]
fn mine_filter_menu_styles_are_readable_in_light_and_dark_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        let menu = mine_filter_menu_style(&theme);
        assert!(menu.background.is_some());
        assert!(menu.text_color.is_some());
        assert_ne!(menu.background.unwrap(), menu.text_color.unwrap().into());

        let selected = mine_filter_menu_entry_style(&theme, true);
        let unselected = mine_filter_menu_entry_style(&theme, false);
        assert!(selected.background.is_some());
        assert_ne!(selected.background, unselected.background);
        assert_eq!(selected.text_color, Some(theme.palette().text));
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

#[test]
fn expert_hub_tabs_buttons_and_descriptions_remain_readable() {
    let long = "这是一个很长的技能介绍，用来验证卡片不会再被内容撑高或遮住底部操作按钮。".repeat(4);
    let preview = description_preview(&long);
    assert!(preview.ends_with('…'));
    assert!(preview.chars().count() <= 73);
    assert_eq!(description_preview("简短介绍"), "简短介绍");
    let tooltip: iced::Element<'_, Message> = description_tooltip(long.clone());
    std::hint::black_box(tooltip);

    for theme in [Theme::Light, Theme::Dark] {
        for selected in [false, true] {
            let style = selected_tab_style(&theme, button::Status::Active, selected);
            let background = style.background.expect("tab must have an opaque surface");
            assert_ne!(background, style.text_color.into());
            if selected {
                assert_eq!(style.text_color, iced::Color::WHITE);
            }
            assert_eq!(style.border.radius.top_left, 8.0);
        }
        let action = action_button_style(&theme, button::Status::Active);
        assert!(action.background.is_some());
        assert_eq!(action.text_color, iced::Color::WHITE);
        assert_ne!(action.background.unwrap(), action.text_color.into());
        assert_eq!(action.border.radius.top_left, 8.0);
        let input = search_input_style(&theme, text_input::Status::Active);
        assert_eq!(input.border.radius.top_left, 12.0);
        assert!(input.border.width > 0.0);
        let badge = skill_badge_style(&theme);
        assert_eq!(badge.border.radius.top_left, 999.0);
    }
}

#[test]
fn skill_cards_use_responsive_equal_width_columns() {
    assert_eq!(grid_column_count(299.0), 1);
    assert_eq!(grid_column_count(612.0), 2);
    assert_eq!(grid_column_count(924.0), 3);
}
