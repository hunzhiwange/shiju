//! 专家中心列表测试，覆盖技能标签、访问状态、搜索分页和响应式卡片布局。
//! 多选筛选与商业技能操作必须依赖服务器安装状态，并在两种主题下保持可读。

use super::Message;
use super::fixture::Fixture;
use super::list::{SCROLLBAR_GUTTER, SCROLLBAR_WIDTH, access_action_message, skill_tab_specs};
use super::styles::card_style;
use crate::views::workspace::{AccountMessage, SkillHubMessage};
use desktop_skill_hub::skill_hub::{CursorPage, LoadState, Page, SkillTab, SkillTag};
use iced::Theme;

fn set_state(app: &mut Fixture, tab: SkillTab, state: usize) {
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
fn expert_hub_view_builds_all_visible_skill_tabs_and_access_states() {
    let (mut app, _) = Fixture::new();

    assert!(!skill_tab_specs().iter().any(|(_, tab)| *tab == SkillTab::System));
    assert!(!skill_tab_specs().iter().any(|(label, _)| *label == "公司技能"));
    for &(_, tab) in skill_tab_specs() {
        for state in 0..4 {
            set_state(&mut app, tab, state);
            let element: iced::Element<'_, Message> = super::view(&app.input());
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

#[tokio::test]
async fn skill_form_controls_render_system_tag_menu() {
    use super::public_skills_tests::{CONTROL_LOCALES, SkillControls, control_renderer};
    use desktop_skill_hub::skill_hub::SystemSkill;
    use iced::keyboard::key::Named;

    let mut renderer = control_renderer().await;
    let (mut app, _) = Fixture::new();
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
                    SkillControls::new(super::list::view(&app.input()), &mut renderer, width);
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
