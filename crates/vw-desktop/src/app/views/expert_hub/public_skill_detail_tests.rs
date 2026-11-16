//! 公共技能详情测试，覆盖 Markdown 内容及按安装范围生成的平台动作。
//! 项目上下文缺失或安装正忙时，操作可用性必须收敛到安全状态。

use gateway_skill_market::{SkillMarketCoordinateDto, SkillMarketSkillDetailDto, SkillMarketSkillDto, SkillMarketSkillFileDto, SkillMarketSkillLocatorDto, SkillMarketSkillSetDetailDto, SkillMarketSkillSetDto};



use super::public_skill_detail::{detail_project_action_message, detail_scope_action_message, detail_target_action_message};
use crate::app::message::skill_hub::{PublicSkillMessage, SkillHubMessage};
use crate::app::skill_hub::{ExpertHubSection, LoadState, PublicDetailMode, PublicDetailState, PublicDetailTarget, PublicInstallState, SkillInstallScope, SkillInstallTarget, parse_public_markdown};
use crate::app::{App, Message};

fn locator() -> SkillMarketSkillLocatorDto {
    SkillMarketSkillLocatorDto {
        source: "skillhub".to_string(),
        namespace: "public".to_string(),
        slug: "中文技能".to_string(),
    }
}

fn app_with_markdown(mode: PublicDetailMode) -> App {
    let (mut app, _) = App::new();
    app.expert_hub.section = ExpertHubSection::Skills;
    let coordinate = SkillMarketCoordinateDto { locator: locator(), version: "1.0.0".to_string() };
    let markdown = concat!(
        "# 中文标题\n\n",
        "- 第一项\n",
        "- 第二项\n\n",
        "行内代码：`中文变量`\n\n",
        "```bash\n",
        "# 中文注释会被高亮器设为斜体等宽字体\n",
        "value=\"中文\"\n",
        "```\n",
    );
    app.expert_hub.public_market.detail = Some(PublicDetailState {
        target: PublicDetailTarget::Skill(locator()),
        generation: 1,
        mode,
        skill_detail: LoadState::Loaded(SkillMarketSkillDetailDto {
            skill: SkillMarketSkillDto {
                coordinate: coordinate.clone(),
                name: "中文技能".to_string(),
                description: "安全地处理中文内容".to_string(),
                author: "作者".to_string(),
                category: "开发".to_string(),
                tags: vec!["中文".to_string()],
                icon: None,
                downloads: 10,
                installs: 5,
                rating: 4.5,
                updated_at: "2026-07-30".to_string(),
            },
            license: Some("MIT".to_string()),
            homepage: None,
            repository: None,
            requires_api_key: false,
            security: None,
        }),
        skill_file: LoadState::Loaded(SkillMarketSkillFileDto {
            coordinate,
            content: markdown.to_string(),
        }),
        suite_detail: LoadState::Idle,
        markdown_items: parse_public_markdown(markdown),
        install: PublicInstallState::Idle,
        install_confirmation_open: false,
        install_scope: SkillInstallScope::Project,
        selected_install_project_path: Some("/repo/current".to_string()),
        selected_install_targets: vec![SkillInstallTarget::VibeWindow],
    });
    app
}

#[test]
fn public_skill_detail_platform_actions_follow_scope_and_busy_state() {
    let mut app = app_with_markdown(PublicDetailMode::Visual);
    let detail = app.expert_hub.public_market.detail.as_ref().expect("detail");
    assert!(detail_scope_action_message(detail, SkillInstallScope::Project, false).is_none());
    assert!(matches!(
        detail_scope_action_message(detail, SkillInstallScope::Global, false),
        Some(Message::SkillHub(SkillHubMessage::Public(
            PublicSkillMessage::SelectDetailInstallScope(SkillInstallScope::Global)
        )))
    ));
    assert!(matches!(
        detail_target_action_message(detail, SkillInstallTarget::Codex),
        Some(Message::SkillHub(SkillHubMessage::Public(
            PublicSkillMessage::ToggleDetailInstallTarget(SkillInstallTarget::Codex)
        )))
    ));
    assert!(matches!(
        detail_project_action_message(
            detail.selected_install_project_path.as_deref(),
            detail.install_is_busy(),
            "/repo/other".to_string(),
        ),
        Some(Message::SkillHub(SkillHubMessage::Public(
            PublicSkillMessage::SelectDetailInstallProject(path)
        ))) if path == "/repo/other"
    ));

    app.expert_hub.public_market.detail.as_mut().expect("detail").install =
        PublicInstallState::Installing { generation: 2 };
    let detail = app.expert_hub.public_market.detail.as_ref().expect("detail");
    assert!(detail_target_action_message(detail, SkillInstallTarget::ClaudeCode).is_none());
    assert!(
        detail_project_action_message(
            detail.selected_install_project_path.as_deref(),
            detail.install_is_busy(),
            "/repo/other".to_string(),
        )
        .is_none()
    );
}
