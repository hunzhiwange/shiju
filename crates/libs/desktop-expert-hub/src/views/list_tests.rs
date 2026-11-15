//! 专家中心列表测试，覆盖技能标签、访问状态、搜索分页和响应式卡片布局。
//! 多选筛选与商业技能操作必须依赖服务器安装状态，并在两种主题下保持可读。

use super::list::{description_preview, description_tooltip, detail_action_message, grid_column_count, install_action_message, market_search_messages, mine_catalog_filter_options, mine_filter_menu_entry_style, mine_filter_menu_style, mine_named_tag_filter_label, mine_status_filter_label, mine_tag_filter_options, pagination_messages, skill_tab_specs};
use super::styles::{action_button_style, search_input_style, selected_tab_style, skill_badge_style};
use crate::message::skill_hub::SkillHubMessage;
use desktop_skill_hub::skill_hub::{LoadState, MineSkillStatus, Page, SkillTab, SkillTag};
use super::Message;
use iced::Theme;
use iced::widget::{button, text_input};

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

#[test]
fn mine_multi_select_labels_and_tag_fallbacks_are_stable() {
    let tags = [
        SkillTag { id: 9, name: "设计".to_string(), sort_order: 2 },
        SkillTag { id: 2, name: "开发".to_string(), sort_order: 1 },
        SkillTag { id: 9, name: "重复名称".to_string(), sort_order: 0 },
    ];
    let options = mine_tag_filter_options(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        tags.iter(),
        &[2, 7],
    );

    assert_eq!(options.iter().map(|option| option.id).collect::<Vec<_>>(), vec![2, 9, 7]);
    assert_eq!(
        options.iter().map(|option| option.label.as_str()).collect::<Vec<_>>(),
        vec!["开发", "设计", "标签 7"]
    );
    assert_eq!(mine_named_tag_filter_label("平台标签", &[]), "平台标签");
    assert_eq!(mine_named_tag_filter_label("公司标签", &[2, 7]), "公司标签 2");
    assert_eq!(
        mine_status_filter_label(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            &MineSkillStatus::DEFAULT
        ),
        "已选 3 个状态"
    );
    assert!(!MineSkillStatus::DEFAULT.contains(&MineSkillStatus::Unpublished));
}

#[test]
fn mine_tag_catalogs_are_independent_sorted_and_keep_selected_fallbacks() {
    let platform = LoadState::Loaded(vec![
        SkillTag { id: 3, name: "平台三".to_string(), sort_order: 30 },
        SkillTag { id: 1, name: "平台一".to_string(), sort_order: 10 },
    ]);
    let company = LoadState::Loaded(vec![
        SkillTag { id: 8, name: "公司八".to_string(), sort_order: 20 },
        SkillTag { id: 7, name: "公司七".to_string(), sort_order: 10 },
    ]);

    let (platform_options, platform_empty) = mine_catalog_filter_options(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &platform,
        &[9],
    );
    let (company_options, company_empty) = mine_catalog_filter_options(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &company,
        &[],
    );
    assert_eq!(
        platform_options.iter().map(|option| option.label.as_str()).collect::<Vec<_>>(),
        vec!["平台一", "平台三", "标签 9"]
    );
    assert_eq!(
        company_options.iter().map(|option| option.label.as_str()).collect::<Vec<_>>(),
        vec!["公司七", "公司八"]
    );
    assert_eq!(platform_empty, None);
    assert_eq!(company_empty, None);

    let (failed_options, failed_empty) = mine_catalog_filter_options(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &LoadState::Error(("目录不可用".to_string()).into()),
        &[12],
    );
    assert_eq!(failed_options[0].label, "标签 12");
    assert_eq!(failed_empty, Some("标签目录加载失败"));

    let (empty_options, empty_label) = mine_catalog_filter_options(
        desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
        &LoadState::Loaded(Vec::new()),
        &[],
    );
    assert!(empty_options.is_empty());
    assert_eq!(empty_label, Some("暂无可选标签"));
}
