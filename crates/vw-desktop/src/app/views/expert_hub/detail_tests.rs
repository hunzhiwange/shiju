//! 专家中心详情页测试，覆盖关闭、下载、版本操作与工作助手视觉契约。
//! 已安装、可升级、可回滚及未知版本都应映射到明确且有限的界面动作。

use super::detail::{VersionRowAction, close_message, download_message, version_action_spec};
use crate::app::message::SkillHubMessage;
use crate::app::skill_hub::{
    InstalledSkill, LoadState, PublicDetailMode, SkillActionKind, SkillActionState, SkillDetail,
    SkillDetailState, SkillDownloadUrl, SkillVersion, SkillVersionDirection, parse_public_markdown,
};
use crate::app::{App, Message};
use iced::Theme;
use iced::widget::button;

#[test]
fn expert_hub_detail_view_binds_only_close_download_and_skill_actions() {
    assert!(matches!(close_message(), Message::SkillHub(SkillHubMessage::CloseSkillDetail)));
    assert!(matches!(
        download_message(42),
        Message::SkillHub(SkillHubMessage::DownloadVersion(42))
    ));

    let (mut app, _) = App::new();
    app.expert_hub.detail = Some(SkillDetailState {
        skill_id: 7,
        generation: 1,
        detail: LoadState::Loading { generation: 1 },
        readme: LoadState::Loading { generation: 1 },
        readme_mode: PublicDetailMode::Visual,
        readme_items: Vec::new(),
        versions: LoadState::Loading { generation: 1 },
    });
    let loading: iced::Element<'_, Message> = super::view(&app);
    std::hint::black_box(loading);
    app.expert_hub.detail.as_mut().unwrap().detail = LoadState::Error("fixture error".to_string());
    let error: iced::Element<'_, Message> = super::view(&app);
    std::hint::black_box(error);

    let detail = SkillDetail {
        id: 7,
        slug: "safe-skill".to_string(),
        name: "安全技能".to_string(),
        description: "详情".to_string(),
        level: "basic".to_string(),
        modality: "text".to_string(),
        format: "markdown".to_string(),
        company_id: "company-1".to_string(),
        created_uid: "user-1".to_string(),
        maintainer_uid: "user-1".to_string(),
        status: "published".to_string(),
        is_all_company_accessible: 1,
        current_version_id: 17,
        current_version: None,
        created_at: String::new(),
        updated_at: String::new(),
        tags: Vec::new(),
    };
    let version = SkillVersion {
        id: 17,
        skill_id: 7,
        version: "1.0.0".to_string(),
        status: "published".to_string(),
        change_type: "initial".to_string(),
        changelog: None,
        checksum: "sha256:test".to_string(),
        content_type: "application/zip".to_string(),
        download_expires_at: 1_800_000_000,
        download_url: Some(
            SkillDownloadUrl::parse("https://skill.shiju.cloud/download?s=secret").unwrap(),
        ),
        created_at: String::new(),
        updated_at: String::new(),
    };
    let state = app.expert_hub.detail.as_mut().unwrap();
    state.detail = LoadState::Loaded(detail);
    state.versions = LoadState::Loaded(vec![version]);
    let readme_loading: iced::Element<'_, Message> = super::view(&app);
    std::hint::black_box(readme_loading);

    let state = app.expert_hub.detail.as_mut().unwrap();
    state.readme = LoadState::Loaded("# 使用说明\n支持 UTF-8 正文。".to_string());
    state.readme_items = parse_public_markdown("# 使用说明\n支持 UTF-8 正文。");
    let readme_loaded: iced::Element<'_, Message> = super::view(&app);
    std::hint::black_box(readme_loaded);

    app.expert_hub.detail.as_mut().unwrap().readme_mode = PublicDetailMode::Markdown;
    let readme_markdown: iced::Element<'_, Message> = super::view(&app);
    std::hint::black_box(readme_markdown);

    app.expert_hub.detail.as_mut().unwrap().readme = LoadState::Loaded(String::new());
    let readme_empty: iced::Element<'_, Message> = super::view(&app);
    std::hint::black_box(readme_empty);

    app.expert_hub.detail.as_mut().unwrap().readme =
        LoadState::Error("fixture README error".to_string());
    let readme_error: iced::Element<'_, Message> = super::view(&app);
    std::hint::black_box(readme_error);

    app.expert_hub.detail.as_mut().unwrap().versions =
        LoadState::Error("fixture versions error".to_string());
    let detail_with_versions_error: iced::Element<'_, Message> = super::view(&app);
    std::hint::black_box(detail_with_versions_error);
}

#[test]
fn expert_hub_workbuddy_visual_contract_uses_soft_cards_and_navigation_actions() {
    for theme in [Theme::Light, Theme::Dark] {
        let card = super::styles::card_style(&theme);
        assert_eq!(card.border.radius.top_left, 16.0);
        assert!(card.shadow.blur_radius >= 16.0);

        let panel = super::styles::detail_panel_style(&theme);
        assert_eq!(panel.border.radius.top_left, 16.0);
        assert!(panel.background.is_some());

        let quiet = super::styles::quiet_button_style(&theme, button::Status::Active);
        assert_eq!(quiet.border.radius.top_left, 10.0);
        assert!(quiet.background.is_some());

        let pill = super::styles::pill_button_style(&theme, button::Status::Active);
        assert_eq!(pill.border.radius.top_left, 999.0);
        assert!(pill.background.is_some());
    }
}

#[test]
fn business_skill_detail_labels_current_upgrade_rollback_and_unknown_versions() {
    let mut version = SkillVersion {
        id: 42,
        skill_id: 7,
        version: "1.2.0".to_string(),
        status: "published".to_string(),
        change_type: "feature".to_string(),
        changelog: None,
        checksum: "sha256:test".to_string(),
        content_type: "application/zip".to_string(),
        download_expires_at: 0,
        download_url: None,
        created_at: String::new(),
        updated_at: String::new(),
    };

    assert_eq!(version_action_spec("1.2.0", &version), VersionRowAction::Current);
    assert!(matches!(
        version_action_spec("1.0.0", &version),
        VersionRowAction::Change { label: "升级到", action }
            if action.version_id == 42
                && action.target_version == "1.2.0"
                && action.direction == SkillVersionDirection::Upgrade
    ));
    version.version = "0.9.0".to_string();
    assert!(matches!(
        version_action_spec("1.0.0", &version),
        VersionRowAction::Change {
            label: "回退到",
            action: crate::app::skill_hub::SkillVersionAction {
                direction: SkillVersionDirection::Rollback,
                ..
            },
        }
    ));
    version.version = "release-next".to_string();
    assert!(matches!(
        version_action_spec("1.0.0", &version),
        VersionRowAction::Change {
            label: "切换到",
            action: crate::app::skill_hub::SkillVersionAction {
                direction: SkillVersionDirection::Switch,
                ..
            },
        }
    ));

    let (mut app, _) = App::new();
    app.expert_hub.installed = LoadState::Loaded(vec![InstalledSkill {
        skill_id: 7,
        slug: "safe-skill".to_string(),
        name: "安全技能".to_string(),
        description: "fixture".to_string(),
        installed_version: "1.0.0".to_string(),
        current_version: "1.2.0".to_string(),
        format: "markdown".to_string(),
        upgrade_available: true,
    }]);
    app.expert_hub.detail = Some(SkillDetailState {
        skill_id: 7,
        generation: 1,
        detail: LoadState::Loaded(SkillDetail {
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
            current_version: None,
            created_at: String::new(),
            updated_at: String::new(),
            tags: Vec::new(),
        }),
        readme: LoadState::Loaded(String::new()),
        readme_mode: PublicDetailMode::Visual,
        readme_items: Vec::new(),
        versions: LoadState::Loaded(vec![version.clone()]),
    });
    app.expert_hub.action = Some(SkillActionState {
        skill_id: 7,
        generation: 2,
        kind: SkillActionKind::Version(match version_action_spec("1.0.0", &version) {
            VersionRowAction::Change { action, .. } => action,
            VersionRowAction::Current => panic!("fixture must be switchable"),
        }),
    });
    let busy_view: iced::Element<'_, Message> = super::view(&app);
    std::hint::black_box(busy_view);
}
