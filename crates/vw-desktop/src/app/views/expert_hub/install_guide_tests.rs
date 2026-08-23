//! 技能安装指南测试，覆盖平台目标、安装范围、进度阶段和卸载确认层。
//! 明暗主题需满足可读对比度，忙碌状态和重复操作必须正确禁用。

use super::install_guide::{
    SCROLLBAR_WIDTH, agent_install_preview, copy_action_message, direct_install_action_message,
    direct_install_button_label, direct_install_progress, install_project_options,
    install_target_presentations, manual_download_action_message, open_target_directory_message,
    scope_action_message, sync_action_message, sync_button_label, target_action_message,
    target_button_style, target_directory_action_label, target_path_label,
    uninstall_action_message, upgrade_available,
};
use crate::app::message::SkillHubMessage;
use crate::app::skill_hub::{
    LoadState, ShijuDirectInstallState, ShijuInstallGuideState, ShijuInstallation,
    ShijuInstallationsReport, ShijuManualDownloadState, ShijuPackageId, ShijuTargetInstallation,
    SkillInstallScope, SkillInstallSelection, SkillInstallTarget, resolve_skill_destinations,
};
use crate::app::{App, Message};
use iced::{Background, Color};

fn contrast_ratio(first: Color, second: Color) -> f32 {
    fn luminance(color: Color) -> f32 {
        fn channel(value: f32) -> f32 {
            if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
        }
        0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
    }
    let first = luminance(first);
    let second = luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

#[test]
fn install_guide_builds_all_methods_and_platform_actions() {
    let (mut app, _) = App::new();
    app.expert_hub.install_guide.open = true;
    app.expert_hub.install_guide.current_package = Some(ShijuPackageId::DarwinArm64);
    app.expert_hub.install_guide.agent_package = LoadState::Error("retry".to_string());

    let element: iced::Element<'_, Message> = super::install_guide::view(&app);
    std::hint::black_box(element);
    assert!(matches!(
        direct_install_action_message(&app.expert_hub.install_guide),
        Some(Message::SkillHub(SkillHubMessage::DirectInstallShiju))
    ));
    assert!(matches!(
        copy_action_message(&app.expert_hub.install_guide),
        Some(Message::SkillHub(SkillHubMessage::CopyAgentInstall))
    ));
    for package_id in ShijuPackageId::ALL {
        assert!(matches!(
            manual_download_action_message(&app.expert_hub.install_guide, package_id),
            Some(Message::SkillHub(SkillHubMessage::DownloadShijuPackage(active)))
                if active == package_id
        ));
    }
    assert_eq!(SCROLLBAR_WIDTH, 4);
}

#[test]
fn install_guide_disables_duplicate_actions_and_styles_both_themes() {
    let mut state = ShijuInstallGuideState {
        current_package: Some(ShijuPackageId::DarwinArm64),
        ..ShijuInstallGuideState::default()
    };
    state.direct_install = ShijuDirectInstallState::Installing {
        generation: 1,
        package_id: ShijuPackageId::DarwinArm64,
    };
    state.manual_download =
        Some(ShijuManualDownloadState { generation: 2, package_id: ShijuPackageId::LinuxAmd64 });
    state.agent_package = LoadState::Loading { generation: 3 };

    assert!(direct_install_action_message(&state).is_none());
    assert!(copy_action_message(&state).is_none());
    assert!(manual_download_action_message(&state, ShijuPackageId::WindowsAmd64).is_none());
    for theme in [iced::Theme::Light, iced::Theme::Dark] {
        let card = super::styles::card_style(&theme);
        let inset = super::styles::inset_panel_style(&theme);
        assert!(card.background.is_some());
        assert!(inset.background.is_some());
    }
}

#[test]
fn install_guide_lists_all_supported_agent_targets() {
    let targets = install_target_presentations(SkillInstallScope::Global);

    assert_eq!(targets.len(), 10);
    assert_eq!(
        targets.iter().map(|target| target.target).collect::<Vec<_>>(),
        vec![
            SkillInstallTarget::VibeWindow,
            SkillInstallTarget::AgentSkills,
            SkillInstallTarget::Codex,
            SkillInstallTarget::WorkBuddy,
            SkillInstallTarget::ClaudeCode,
            SkillInstallTarget::OpenCode,
            SkillInstallTarget::GeminiCli,
            SkillInstallTarget::GitHubCopilot,
            SkillInstallTarget::Cursor,
            SkillInstallTarget::Windsurf,
        ]
    );
    let shared = targets
        .iter()
        .find(|target| target.target == SkillInstallTarget::AgentSkills)
        .expect("shared Agent Skills target");
    assert_eq!(shared.name, "通用 Agent Skills");
    for agent in ["Cursor", "Gemini CLI", "OpenCode", "Copilot", "Windsurf"] {
        assert!(shared.compatibility.contains(agent));
    }
    assert!(!shared.compatibility.contains("Codex"));
    assert_eq!(shared.path, "~/.agents/skills/shiju");
    let codex = targets
        .iter()
        .find(|target| target.target == SkillInstallTarget::Codex)
        .expect("Codex target");
    assert_eq!(codex.name, "Codex");
    assert_eq!(codex.path, "~/.codex/skills/shiju");
}

#[test]
fn scope_and_target_actions_follow_busy_and_project_state() {
    let mut state = ShijuInstallGuideState::default();

    assert!(scope_action_message(&state, SkillInstallScope::Project, false).is_none());
    assert!(matches!(
        scope_action_message(&state, SkillInstallScope::Project, true),
        Some(Message::SkillHub(SkillHubMessage::SelectShijuInstallScope(
            SkillInstallScope::Project
        )))
    ));
    assert!(matches!(
        target_action_message(&state, SkillInstallTarget::ClaudeCode),
        Some(Message::SkillHub(SkillHubMessage::ToggleShijuInstallTarget(
            SkillInstallTarget::ClaudeCode
        )))
    ));
    assert_eq!(direct_install_button_label(&state), "直接安装到 1 个目标");
    state.selected_targets.push(SkillInstallTarget::ClaudeCode);
    assert_eq!(direct_install_button_label(&state), "直接安装到 2 个目标");

    state.direct_install = ShijuDirectInstallState::Installing {
        generation: 1,
        package_id: ShijuPackageId::DarwinArm64,
    };
    assert!(scope_action_message(&state, SkillInstallScope::Project, true).is_none());
    assert!(target_action_message(&state, SkillInstallTarget::Cursor).is_none());

    state.direct_install = ShijuDirectInstallState::Installed;
    let home = tempfile::tempdir().expect("home fixture");
    let selection = SkillInstallSelection {
        target: SkillInstallTarget::AgentSkills,
        scope: SkillInstallScope::Global,
    };
    let destination = resolve_skill_destinations(home.path(), None, &[selection])
        .expect("resolve fixture target")
        .remove(0);
    state.set_installations(ShijuInstallationsReport {
        targets: vec![ShijuTargetInstallation {
            installation: Some(ShijuInstallation {
                installed_dir: destination.installed_dir.clone(),
                version: Some("3.0.3".to_string()),
            }),
            destination,
        }],
    });
    assert!(matches!(
        open_target_directory_message(&state, SkillInstallTarget::AgentSkills),
        Some(Message::SkillHub(SkillHubMessage::OpenShijuInstallDirFor(
            SkillInstallTarget::AgentSkills
        )))
    ));
}

#[test]
fn install_project_options_put_current_first_and_deduplicate_recent_paths() {
    let (mut app, _) = App::new();
    app.project_path = Some("/repo/current".to_string());
    app.recent_projects = vec![
        "/repo/other".to_string(),
        "/repo/current".to_string(),
        "  /repo/third  ".to_string(),
        String::new(),
    ];
    app.expert_hub.install_guide.selected_project_path = Some("/repo/other".to_string());

    assert_eq!(
        install_project_options(&app),
        vec!["/repo/current".to_string(), "/repo/other".to_string(), "/repo/third".to_string(),]
    );
}

#[test]
fn install_project_action_routes_selected_path() {
    let state = ShijuInstallGuideState::default();

    assert!(matches!(
        super::install_guide::install_project_action_message(
            state.selected_project_path.as_deref(),
            state.is_busy(),
            "/repo/other".to_string()
        ),
        Some(Message::SkillHub(
            SkillHubMessage::SelectShijuInstallProject(path)
        )) if path == "/repo/other"
    ));
}

#[test]
fn target_paths_are_ascii_and_themes_have_contrast() {
    for scope in [SkillInstallScope::Global, SkillInstallScope::Project] {
        for target in install_target_presentations(scope) {
            assert!(target.path.is_ascii());
            assert_eq!(target.path, target_path_label(target.target, scope));
        }
    }

    for theme in [iced::Theme::Light, iced::Theme::Dark] {
        for selected in [false, true] {
            let style = target_button_style(&theme, iced::widget::button::Status::Active, selected);
            let Background::Color(background) = style.background.expect("target background") else {
                panic!("target background should be a solid color");
            };
            assert!(contrast_ratio(style.text_color, background) >= 3.0);
        }
    }
    assert_eq!(SCROLLBAR_WIDTH, 4);
}

#[test]
fn direct_install_progress_and_location_action_cover_all_stages() {
    let mut state = ShijuInstallGuideState::default();
    assert_eq!(direct_install_progress(&state), 0.0);
    assert!(open_target_directory_message(&state, SkillInstallTarget::AgentSkills).is_none());
    assert_eq!(target_directory_action_label(&state, SkillInstallTarget::AgentSkills), "未安装");

    state.direct_install = ShijuDirectInstallState::Fetching {
        generation: 1,
        package_id: ShijuPackageId::DarwinArm64,
    };
    assert_eq!(direct_install_progress(&state), 25.0);

    state.direct_install = ShijuDirectInstallState::Installing {
        generation: 1,
        package_id: ShijuPackageId::DarwinArm64,
    };
    assert_eq!(direct_install_progress(&state), 75.0);

    state.direct_install = ShijuDirectInstallState::Installed;
    let home = tempfile::tempdir().expect("home fixture");
    let selection = SkillInstallSelection {
        target: SkillInstallTarget::AgentSkills,
        scope: SkillInstallScope::Global,
    };
    let destination = resolve_skill_destinations(home.path(), None, &[selection])
        .expect("resolve fixture target")
        .remove(0);
    state.set_installations(ShijuInstallationsReport {
        targets: vec![ShijuTargetInstallation {
            installation: Some(ShijuInstallation {
                installed_dir: destination.installed_dir.clone(),
                version: Some("3.0.3".to_string()),
            }),
            destination,
        }],
    });
    assert_eq!(direct_install_progress(&state), 100.0);
    assert!(direct_install_action_message(&state).is_some());
    assert_eq!(
        target_directory_action_label(&state, SkillInstallTarget::AgentSkills),
        "打开该目标目录"
    );
    assert!(matches!(
        open_target_directory_message(&state, SkillInstallTarget::AgentSkills),
        Some(Message::SkillHub(SkillHubMessage::OpenShijuInstallDirFor(
            SkillInstallTarget::AgentSkills
        )))
    ));
    assert!(matches!(
        uninstall_action_message(&state),
        Some(Message::SkillHub(SkillHubMessage::UninstallShiju))
    ));

    state.direct_install = ShijuDirectInstallState::Uninstalling { generation: 2 };
    assert_eq!(direct_install_progress(&state), 50.0);
    assert!(open_target_directory_message(&state, SkillInstallTarget::AgentSkills).is_none());
    assert!(uninstall_action_message(&state).is_none());
}

#[test]
fn installed_guide_exposes_sync_action_after_uninstall() {
    let mut state = ShijuInstallGuideState {
        is_installed: true,
        direct_install: ShijuDirectInstallState::Installed,
        ..ShijuInstallGuideState::default()
    };

    assert!(matches!(
        uninstall_action_message(&state),
        Some(Message::SkillHub(SkillHubMessage::UninstallShiju))
    ));
    assert!(matches!(
        sync_action_message(&state),
        Some(Message::SkillHub(SkillHubMessage::SyncShijuKey))
    ));
    assert_eq!(sync_button_label(&state), "同步登录信息");

    state.login_sync_in_progress = true;
    assert!(sync_action_message(&state).is_none());
    assert_eq!(sync_button_label(&state), "同步中...");

    state.login_sync_in_progress = false;
    state.direct_install = ShijuDirectInstallState::Installing {
        generation: 1,
        package_id: ShijuPackageId::DarwinArm64,
    };
    assert!(sync_action_message(&state).is_none());
}

#[test]
fn installed_version_detects_available_upgrade() {
    let mut state = ShijuInstallGuideState {
        is_installed: true,
        installed_version: Some("3.0.3".to_string()),
        ..ShijuInstallGuideState::default()
    };
    state.agent_package = LoadState::Loaded(crate::app::skill_hub::ShijuPackageRelease {
        download_url: crate::app::skill_hub::SkillDownloadUrl::parse(
            "https://skill.shiju.cloud/hub/releases/3.1.0/shiju.zip?signature=secret",
        )
        .expect("trusted download URL"),
    });

    assert!(upgrade_available(&state));
    let (mut app, _) = App::new();
    app.expert_hub.install_guide = state;
    std::hint::black_box(super::install_guide::view(&app));
}

#[test]
fn uninstall_confirmation_builds_as_full_expert_hub_overlay() {
    let (mut app, _) = App::new();
    app.expert_hub.skill_tab = crate::app::skill_hub::SkillTab::Install;
    app.expert_hub.install_guide.open = true;
    app.expert_hub.install_guide.is_installed = true;
    app.expert_hub.install_guide.installed_version = Some("3.0.3".to_string());
    app.expert_hub.install_guide.uninstall_confirmation_open = true;

    let modal: iced::Element<'_, Message> = super::view(&app);
    std::hint::black_box(modal);
    assert!(crate::app::skill_hub::shiju_install_path_label().is_ascii());
}

#[test]
fn install_page_never_renders_the_signed_download_url() {
    let mut state = ShijuInstallGuideState::default();
    state.agent_package = LoadState::Loaded(crate::app::skill_hub::ShijuPackageRelease {
        download_url: crate::app::skill_hub::SkillDownloadUrl::parse(
            "https://skill.shiju.cloud/hub/releases/shiju.zip?x-oss-signature=secret",
        )
        .expect("trusted download URL"),
    });

    let preview = agent_install_preview(&state);
    assert!(preview.contains("已生成"));
    assert!(!preview.contains("x-oss-signature"));
    assert!(!preview.contains("secret"));
}
