//! 构建识矩技能安装向导及多目标安装路径说明。

use super::styles;
use crate::app::components::system_settings_common::{
    danger_action_btn_style, primary_action_btn_style, rounded_action_btn_style,
    settings_close_button, settings_divider, settings_field_label, settings_modal_header,
    settings_modal_layout, settings_modal_overlay, settings_muted_text_style,
    settings_segment_button_style, settings_select,
};
use crate::app::message::SkillHubMessage;
use crate::app::skill_hub::{
    LoadState, ShijuDirectInstallState, ShijuInstallGuideState, ShijuPackageId, SkillInstallScope,
    SkillInstallTarget, version_is_newer,
};
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::text::Wrapping;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, progress_bar, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Font, Length, Theme};
use vw_config_types::paths::HOME_CONFIG_DIR_NAME;

/// 安装向导纵向滚动条的固定宽度。
pub(super) const SCROLLBAR_WIDTH: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
/// 安装目标在向导中的名称、兼容性和路径说明。
pub(super) struct InstallTargetPresentation {
    pub(super) target: SkillInstallTarget,
    pub(super) name: &'static str,
    pub(super) compatibility: &'static str,
    pub(super) path: String,
}

const INSTALL_TARGETS: [(SkillInstallTarget, &str, &str); 10] = [
    (SkillInstallTarget::VibeWindow, "ShiJu", "ShiJu 原生技能目录，仅供当前 ShiJu 运行时使用。"),
    (
        SkillInstallTarget::AgentSkills,
        "通用 Agent Skills",
        "覆盖 Cursor、Gemini CLI、OpenCode、Copilot、Windsurf 等兼容工具。",
    ),
    (SkillInstallTarget::Codex, "Codex", "Codex 原生技能目录。"),
    (SkillInstallTarget::WorkBuddy, "WorkBuddy", "WorkBuddy 原生技能目录。"),
    (SkillInstallTarget::ClaudeCode, "Claude Code", "Claude Code 原生技能目录。"),
    (SkillInstallTarget::OpenCode, "OpenCode", "OpenCode 原生技能目录。"),
    (SkillInstallTarget::GeminiCli, "Gemini CLI", "Gemini CLI 原生技能目录。"),
    (SkillInstallTarget::GitHubCopilot, "GitHub Copilot", "GitHub Copilot 原生技能目录。"),
    (SkillInstallTarget::Cursor, "Cursor", "Cursor 原生技能目录。"),
    (SkillInstallTarget::Windsurf, "Windsurf", "Windsurf 原生技能目录。"),
];

pub(super) fn target_path_label(target: SkillInstallTarget, scope: SkillInstallScope) -> String {
    format!("{}/shiju", target_root_label(target, scope))
}

pub(super) fn target_root_label(target: SkillInstallTarget, scope: SkillInstallScope) -> String {
    let root = match (target, scope) {
        (SkillInstallTarget::VibeWindow, _) => format!("{HOME_CONFIG_DIR_NAME}/skills"),
        (SkillInstallTarget::AgentSkills, _) => ".agents/skills".to_string(),
        (SkillInstallTarget::Codex, _) => ".codex/skills".to_string(),
        (SkillInstallTarget::WorkBuddy, _) => ".workbuddy/skills".to_string(),
        (SkillInstallTarget::ClaudeCode, _) => ".claude/skills".to_string(),
        (SkillInstallTarget::OpenCode, SkillInstallScope::Global) => {
            ".config/opencode/skills".to_string()
        }
        (SkillInstallTarget::OpenCode, SkillInstallScope::Project) => {
            ".opencode/skills".to_string()
        }
        (SkillInstallTarget::GeminiCli, _) => ".gemini/skills".to_string(),
        (SkillInstallTarget::GitHubCopilot, SkillInstallScope::Global) => {
            ".copilot/skills".to_string()
        }
        (SkillInstallTarget::GitHubCopilot, SkillInstallScope::Project) => {
            ".github/skills".to_string()
        }
        (SkillInstallTarget::Cursor, _) => ".cursor/skills".to_string(),
        (SkillInstallTarget::Windsurf, SkillInstallScope::Global) => {
            ".codeium/windsurf/skills".to_string()
        }
        (SkillInstallTarget::Windsurf, SkillInstallScope::Project) => {
            ".windsurf/skills".to_string()
        }
    };
    match scope {
        SkillInstallScope::Global => format!("~/{root}"),
        SkillInstallScope::Project => format!("./{root}"),
    }
}

/// 根据当前范围和项目路径生成全部安装目标说明。
pub(super) fn install_target_presentations(
    scope: SkillInstallScope,
) -> Vec<InstallTargetPresentation> {
    INSTALL_TARGETS
        .into_iter()
        .map(|(target, name, compatibility)| InstallTargetPresentation {
            target,
            name,
            compatibility,
            path: target_path_label(target, scope),
        })
        .collect()
}

pub(super) fn scope_action_message(
    state: &ShijuInstallGuideState,
    scope: SkillInstallScope,
    project_available: bool,
) -> Option<Message> {
    (!state.is_busy()
        && state.install_scope != scope
        && (scope == SkillInstallScope::Global || project_available))
        .then_some(Message::SkillHub(SkillHubMessage::SelectShijuInstallScope(scope)))
}

pub(super) fn install_project_options(app: &App) -> Vec<String> {
    let mut options = Vec::new();
    for path in app.project_path.iter().chain(app.recent_projects.iter()) {
        let path = path.trim();
        if !path.is_empty() && !options.iter().any(|option: &String| option.as_str() == path) {
            options.push(path.to_string());
        }
    }
    options
}

pub(super) fn install_project_action_message(
    selected_project_path: Option<&str>,
    is_busy: bool,
    project_path: String,
) -> Option<Message> {
    (!is_busy && selected_project_path != Some(project_path.as_str()))
        .then_some(Message::SkillHub(SkillHubMessage::SelectShijuInstallProject(project_path)))
}

pub(super) fn target_action_message(
    state: &ShijuInstallGuideState,
    target: SkillInstallTarget,
) -> Option<Message> {
    let removing_last = state.selected_targets.len() == 1
        && state.selected_targets.first().is_some_and(|selected| *selected == target);
    (!state.is_busy() && !removing_last)
        .then_some(Message::SkillHub(SkillHubMessage::ToggleShijuInstallTarget(target)))
}

pub(super) fn open_target_directory_message(
    state: &ShijuInstallGuideState,
    target: SkillInstallTarget,
) -> Option<Message> {
    (state.installation(state.install_scope, target).is_some()
        && !matches!(state.direct_install, ShijuDirectInstallState::Uninstalling { .. }))
    .then_some(Message::SkillHub(SkillHubMessage::OpenShijuInstallDirFor(target)))
}

pub(super) fn target_directory_action_label(
    state: &ShijuInstallGuideState,
    target: SkillInstallTarget,
) -> &'static str {
    if state.installation(state.install_scope, target).is_some() {
        "打开该目标目录"
    } else {
        "未安装"
    }
}

pub(super) fn target_button_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    styles::selected_tab_style(theme, status, selected)
}

pub(super) fn direct_install_button_label(
    i18n: crate::app::i18n::I18n,
    state: &ShijuInstallGuideState,
) -> String {
    let count = state.selected_targets.len();
    match state.direct_install {
        ShijuDirectInstallState::Idle | ShijuDirectInstallState::Installed
            if upgrade_available(state) =>
        {
            i18n.format("升级 %s 个目标", &[count.to_string().into()])
                .expect("valid display template")
        }
        ShijuDirectInstallState::Idle | ShijuDirectInstallState::Installed => i18n
            .format("直接安装到 %s 个目标", &[count.to_string().into()])
            .expect("valid display template"),
        ShijuDirectInstallState::Fetching { .. } => i18n.__("正在获取安装包…").to_string(),
        ShijuDirectInstallState::Installing { .. } => i18n.__("正在安装…").to_string(),
        ShijuDirectInstallState::Uninstalling { .. } => i18n.__("正在卸载…").to_string(),
    }
}

/// 仅在向导空闲且存在目标选择时提供直接安装消息。
pub(super) fn direct_install_action_message(state: &ShijuInstallGuideState) -> Option<Message> {
    (state.current_package.is_some()
        && !state.selected_targets.is_empty()
        && !state.login_sync_in_progress
        && matches!(
            state.direct_install,
            ShijuDirectInstallState::Idle | ShijuDirectInstallState::Installed
        ))
    .then_some(Message::SkillHub(SkillHubMessage::DirectInstallShiju))
}

pub(super) fn direct_install_progress(state: &ShijuInstallGuideState) -> f32 {
    match state.direct_install {
        ShijuDirectInstallState::Idle => 0.0,
        ShijuDirectInstallState::Fetching { .. } => 25.0,
        ShijuDirectInstallState::Installing { .. } => 75.0,
        ShijuDirectInstallState::Uninstalling { .. } => 50.0,
        ShijuDirectInstallState::Installed => 100.0,
    }
}

/// 仅在空闲且所选目标中存在安装时提供卸载确认消息。
pub(super) fn uninstall_action_message(state: &ShijuInstallGuideState) -> Option<Message> {
    (state.is_installed
        && matches!(
            state.direct_install,
            ShijuDirectInstallState::Idle | ShijuDirectInstallState::Installed
        ))
    .then_some(Message::SkillHub(SkillHubMessage::UninstallShiju))
}

/// 仅在存在已安装目标且没有在途操作时提供密钥同步消息。
pub(super) fn sync_action_message(state: &ShijuInstallGuideState) -> Option<Message> {
    (state.is_installed
        && !state.login_sync_in_progress
        && matches!(
            state.direct_install,
            ShijuDirectInstallState::Idle | ShijuDirectInstallState::Installed
        ))
    .then_some(Message::SkillHub(SkillHubMessage::SyncShijuKey))
}

pub(super) fn sync_button_label(state: &ShijuInstallGuideState) -> &'static str {
    if state.login_sync_in_progress { "同步中..." } else { "同步登录信息" }
}

fn sync_tooltip_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(Color::WHITE),
        background: Some(Background::Color(Color::from_rgba8(18, 18, 20, 0.96))),
        border: Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn sync_tooltip(i18n: crate::app::i18n::I18n) -> Element<'static, Message> {
    container(
        text(i18n.__(
            "登录、切换账号或安装识矩技能后会自动同步；若因文件权限等原因未成功，可点击此处重试。",
        ))
        .size(12)
        .width(Length::Fixed(360.0)),
    )
    .padding([8, 10])
    .style(sync_tooltip_style)
    .into()
}

/// 当服务端版本为有效且严格更新的数字版本时提示升级。
pub(super) fn upgrade_available(state: &ShijuInstallGuideState) -> bool {
    let Some(current) = state.installed_version.as_deref() else {
        return false;
    };
    let LoadState::Loaded(release) = &state.agent_package else {
        return false;
    };
    release.version().is_some_and(|latest| version_is_newer(current, latest))
}

fn latest_version_label(i18n: crate::app::i18n::I18n, state: &ShijuInstallGuideState) -> String {
    match &state.agent_package {
        LoadState::Loaded(release) => release
            .version()
            .map(|version| format!("v{version}"))
            .unwrap_or_else(|| i18n.__("未知").to_string()),
        LoadState::Loading { .. } | LoadState::Idle => i18n.__("检测中…").to_string(),
        LoadState::NeedsLogin => i18n.__("登录后检测").to_string(),
        LoadState::Error(_) => i18n.__("检测失败").to_string(),
    }
}

fn upgrade_status_label(i18n: crate::app::i18n::I18n, state: &ShijuInstallGuideState) -> String {
    if !state.is_installed {
        return i18n.__("尚未安装").to_string();
    }
    let Some(current) = state.installed_version.as_deref() else {
        return i18n.__("当前版本未知").to_string();
    };
    match &state.agent_package {
        LoadState::Loaded(release) => match release.version() {
            Some(latest) if version_is_newer(current, latest) => i18n
                .format("可升级到 v%s", &[latest.to_string().into()])
                .expect("valid display template"),
            Some(_) => i18n.__("已是最新版").to_string(),
            None => i18n.__("最新版未知").to_string(),
        },
        LoadState::Loading { .. } | LoadState::Idle => i18n.__("正在检测升级…").to_string(),
        LoadState::NeedsLogin => i18n.__("登录后检测升级").to_string(),
        LoadState::Error(_) => i18n.__("升级检测失败").to_string(),
    }
}

fn status_row(
    i18n: crate::app::i18n::I18n,
    label: &'static str,
    value: String,
) -> Element<'static, Message> {
    row![
        text(i18n.__(label)).size(12).style(styles::muted_text_style).width(Length::Fixed(90.0)),
        text(value).size(13),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn copy_action_message(state: &ShijuInstallGuideState) -> Option<Message> {
    (!matches!(state.agent_package, LoadState::Loading { .. } | LoadState::NeedsLogin))
        .then_some(Message::SkillHub(SkillHubMessage::CopyAgentInstall))
}

pub(super) fn manual_download_action_message(
    state: &ShijuInstallGuideState,
    package_id: ShijuPackageId,
) -> Option<Message> {
    state
        .manual_download
        .is_none()
        .then_some(Message::SkillHub(SkillHubMessage::DownloadShijuPackage(package_id)))
}

pub(super) fn agent_install_preview(state: &ShijuInstallGuideState) -> &'static str {
    match state.agent_package {
        LoadState::Loaded(_) => {
            "请帮我安装识矩技能。\n\n下载地址：已生成，复制时会获取最新地址。\n\n请下载该 zip，将解压后的技能包放入当前 Agent 的技能目录。"
        }
        LoadState::Loading { .. } => "正在生成当前设备的安装地址…",
        LoadState::NeedsLogin => "登录识矩账户后即可生成安装文字。",
        LoadState::Error(_) => "安装地址生成失败，请稍后重试。",
        LoadState::Idle => "正在准备安装文字…",
    }
}

fn method_badge(i18n: crate::app::i18n::I18n, label: &'static str) -> Element<'static, Message> {
    container(text(i18n.__(label)).size(12))
        .padding([4, 10])
        .style(styles::skill_badge_style)
        .into()
}

fn section_header(
    i18n: crate::app::i18n::I18n,
    method: Option<&'static str>,
    title: &'static str,
    description: &'static str,
) -> Element<'static, Message> {
    let mut heading = row![].spacing(8).align_y(Alignment::Center);
    if let Some(method) = method {
        heading = heading.push(text(i18n.__(method)).size(13).style(styles::muted_text_style));
    }
    heading = heading.push(
        text(i18n.__(title))
            .size(18)
            .font(Font { weight: iced::font::Weight::Semibold, ..Font::default() }),
    );
    column![
        heading,
        text(i18n.__(description))
            .size(13)
            .style(styles::muted_text_style)
            .wrapping(Wrapping::Word),
    ]
    .spacing(6)
    .into()
}

fn scope_selector(
    i18n: crate::app::i18n::I18n,
    state: &ShijuInstallGuideState,
    project_available: bool,
) -> Element<'static, Message> {
    let mut choices = row![].spacing(8).align_y(Alignment::Center);
    for (scope, label) in [
        (SkillInstallScope::Global, i18n.__("全局")),
        (SkillInstallScope::Project, i18n.__("项目")),
    ] {
        let selected = state.install_scope == scope;
        let mut choice =
            button(text(i18n.__(label)).size(13)).padding([8, 16]).style(move |theme, status| {
                let mut style = settings_segment_button_style(theme, status, selected);
                if selected {
                    style.text_color = theme.palette().text;
                }
                style
            });
        if let Some(message) = scope_action_message(state, scope, project_available) {
            choice = choice.on_press(message);
        }
        choices = choices.push(choice);
    }
    let availability = if project_available {
        i18n.__("项目级技能安装可从已打开或最近使用的项目中选择。")
    } else {
        i18n.__("打开项目后可选择项目级安装。")
    };
    column![
        text(i18n.__("安装范围")).size(13),
        choices,
        text(availability).size(12).style(styles::muted_text_style),
    ]
    .spacing(8)
    .into()
}

fn project_selector(app: &App, options: Vec<String>) -> Element<'static, Message> {
    let state = &app.expert_hub.install_guide;
    let is_busy = state.is_busy();
    let selected_project_path = state.selected_project_path.clone();
    row![
        settings_field_label(crate::app::i18n::I18n::new(app.desktop_locale).__("目标项目")),
        settings_select(options, state.selected_project_path.clone(), move |project_path| {
            install_project_action_message(selected_project_path.as_deref(), is_busy, project_path)
                .unwrap_or(Message::None)
        })
        .placeholder(crate::app::i18n::I18n::new(app.desktop_locale).__("选择项目"))
        .width(Length::Fill)
        .disabled(is_busy),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn target_card(
    i18n: crate::app::i18n::I18n,
    state: &ShijuInstallGuideState,
    presentation: InstallTargetPresentation,
) -> Element<'static, Message> {
    let selected = state.selected_targets.contains(&presentation.target);
    let installation = state.installation(state.install_scope, presentation.target);
    let status = match installation.and_then(|installed| installed.version.as_deref()) {
        Some(version) => i18n
            .format("已安装 · v%s", &[version.to_string().into()])
            .expect("valid display template"),
        None if installation.is_some() => i18n.__("已安装 · 版本未知").to_string(),
        None => i18n.__("未安装").to_string(),
    };
    let mut select = button(
        column![
            row![
                text(i18n.__(presentation.name))
                    .size(15)
                    .font(Font { weight: iced::font::Weight::Semibold, ..Font::default() })
                    .width(Length::Fill),
                text(i18n.__(if selected { i18n.__("已选择") } else { i18n.__("未选择") }))
                    .size(12),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text(i18n.__(presentation.compatibility)).size(12).wrapping(Wrapping::Word),
            text(presentation.path).size(12).font(Font::MONOSPACE).wrapping(Wrapping::Word),
            text(i18n.__(&status).to_string()).size(12),
        ]
        .spacing(8)
        .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(14)
    .style(move |theme, button_status| target_button_style(theme, button_status, selected));
    if let Some(message) = target_action_message(state, presentation.target) {
        select = select.on_press(message);
    }

    let mut directory_action =
        button(text(i18n.__(target_directory_action_label(state, presentation.target))).size(12))
            .width(Length::Fill)
            .padding([8, 12])
            .style(styles::quiet_button_style);
    if let Some(message) = open_target_directory_message(state, presentation.target) {
        directory_action = directory_action.on_press(message);
    }
    container(column![select, directory_action].spacing(8))
        .width(Length::Fixed(350.0))
        .padding(2)
        .style(styles::inset_panel_style)
        .into()
}

fn direct_install_actions(app: &App) -> Element<'static, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let state = &app.expert_hub.install_guide;
    let button_label = direct_install_button_label(i18n, state);
    let mut install_button =
        button(text(button_label).size(14)).padding([10, 24]).style(primary_action_btn_style);
    if let Some(message) = direct_install_action_message(state) {
        install_button = install_button.on_press(message);
    }
    let mut actions = row![install_button].spacing(10).align_y(Alignment::Center);
    if let Some(message) = uninstall_action_message(state) {
        actions = actions.push(
            button(text(crate::app::i18n::I18n::new(app.desktop_locale).__("卸载")).size(13))
                .on_press(message)
                .padding([10, 18])
                .style(rounded_action_btn_style),
        );
    }
    if state.installed_selection_count() > 0 {
        let mut sync_button = button(text(i18n.__(sync_button_label(state))).size(13))
            .padding([10, 18])
            .style(rounded_action_btn_style);
        if let Some(message) = sync_action_message(state) {
            sync_button = sync_button.on_press(message);
        }
        actions = actions
            .push(Tooltip::new(sync_button, sync_tooltip(i18n), TooltipPosition::Top).gap(8));
    }
    actions.wrap().into()
}

fn direct_install_card(app: &App) -> Element<'static, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let state = &app.expert_hub.install_guide;
    let project_options = install_project_options(app);
    let platform = state
        .current_package
        .map(|package| {
            crate::app::i18n::I18n::new(app.desktop_locale)
                .format(
                    "当前设备：%s · %s",
                    &[
                        (package.name()).to_string().into(),
                        i18n.__(package.description()).to_string().into(),
                    ],
                )
                .expect("valid display template")
        })
        .unwrap_or_else(|| {
            crate::app::i18n::I18n::new(app.desktop_locale)
                .__("当前系统或架构暂不支持直接安装")
                .to_string()
        });
    let (progress_label, progress) = match state.direct_install {
        ShijuDirectInstallState::Idle => (
            crate::app::i18n::I18n::new(app.desktop_locale).__("等待安装"),
            direct_install_progress(state),
        ),
        ShijuDirectInstallState::Fetching { .. } => (
            crate::app::i18n::I18n::new(app.desktop_locale).__("正在获取并校验安装包"),
            direct_install_progress(state),
        ),
        ShijuDirectInstallState::Installing { .. } => (
            crate::app::i18n::I18n::new(app.desktop_locale).__("正在写入技能目录"),
            direct_install_progress(state),
        ),
        ShijuDirectInstallState::Uninstalling { .. } => (
            crate::app::i18n::I18n::new(app.desktop_locale).__("正在移除技能目录"),
            direct_install_progress(state),
        ),
        ShijuDirectInstallState::Installed => (
            crate::app::i18n::I18n::new(app.desktop_locale).__("安装完成"),
            direct_install_progress(state),
        ),
    };
    let current_version =
        state.installed_version.as_deref().map(|version| format!("v{version}")).unwrap_or_else(
            || {
                if state.is_installed {
                    crate::app::i18n::I18n::new(app.desktop_locale).__("未知").to_string()
                } else {
                    "—".to_string()
                }
            },
        );

    let target_cards = install_target_presentations(state.install_scope)
        .into_iter()
        .map(|presentation| target_card(i18n, state, presentation))
        .collect::<Vec<_>>();
    let selected_count = state.selected_targets.len();
    let installed_count = state.installed_selection_count();
    let mut scope_controls =
        column![scope_selector(i18n, state, !project_options.is_empty())].spacing(10);
    if state.install_scope == SkillInstallScope::Project {
        scope_controls = scope_controls.push(project_selector(app, project_options));
    }

    container(
        column![
            section_header(i18n,
                Some(crate::app::i18n::I18n::new(app.desktop_locale).__("方法 1")),
                crate::app::i18n::I18n::new(app.desktop_locale).__("直接安装"),
                crate::app::i18n::I18n::new(app.desktop_locale).__("选择全局或项目范围，再选择目标项目和一个或多个 Agent 目标。ShiJu 会下载、校验并以独立副本统一安装。"),
            ),
            text(platform).size(13),
            scope_controls,
            column![
                row![
                    text(crate::app::i18n::I18n::new(app.desktop_locale).__("安装目标")).size(13).width(Length::Fill),
                    text(crate::app::i18n::I18n::new(app.desktop_locale).format("已选择 %s 个", &[selected_count.to_string().into()]).expect("valid display template"))
                        .size(12)
                        .style(styles::muted_text_style),
                ]
                .align_y(Alignment::Center),
                row(target_cards).spacing(10).wrap(),
            ]
            .spacing(10),
            container(
                column![
                    status_row(i18n,
                        crate::app::i18n::I18n::new(app.desktop_locale).__("安装状态"),
                        if installed_count > 0 {
                            crate::app::i18n::I18n::new(app.desktop_locale).format("所选目标中已安装 %s 个", &[installed_count.to_string().into()]).expect("valid display template")
                        } else {
                            crate::app::i18n::I18n::new(app.desktop_locale).__("所选目标均未安装").to_string()
                        },
                    ),
                    status_row(i18n, crate::app::i18n::I18n::new(app.desktop_locale).__("当前版本"), current_version),
                    status_row(i18n, crate::app::i18n::I18n::new(app.desktop_locale).__("最新版本"), latest_version_label(crate::app::i18n::I18n::new(app.desktop_locale), state)),
                    status_row(i18n, crate::app::i18n::I18n::new(app.desktop_locale).__("升级状态"), upgrade_status_label(crate::app::i18n::I18n::new(app.desktop_locale), state)),
                ]
                .spacing(8),
            )
            .width(Length::Fill)
            .padding(14)
            .style(styles::inset_panel_style),
            column![
                row![
                    text(i18n.__(progress_label)).size(12).width(Length::Fill),
                    text(format!("{progress:.0}%")).size(12),
                ]
                .align_y(Alignment::Center),
                progress_bar(0.0..=100.0, progress).girth(Length::Fixed(8.0)),
            ]
            .spacing(7),
        ]
        .spacing(14),
    )
    .width(Length::Fill)
    .padding(20)
    .style(styles::card_style)
    .into()
}

fn step_card(
    i18n: crate::app::i18n::I18n,
    number: &'static str,
    title: &'static str,
    description: &'static str,
) -> Element<'static, Message> {
    container(
        row![
            container(text(number).size(13))
                .width(Length::Fixed(30.0))
                .height(Length::Fixed(30.0))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(styles::skill_badge_style),
            column![
                text(i18n.__(title))
                    .size(14)
                    .font(Font { weight: iced::font::Weight::Semibold, ..Font::default() }),
                text(i18n.__(description))
                    .size(12)
                    .style(styles::muted_text_style)
                    .wrapping(Wrapping::Word),
            ]
            .spacing(5),
        ]
        .spacing(12)
        .align_y(Alignment::Start),
    )
    .width(Length::FillPortion(1))
    .padding(14)
    .style(styles::inset_panel_style)
    .into()
}

fn agent_install_card(
    i18n: crate::app::i18n::I18n,
    state: &ShijuInstallGuideState,
) -> Element<'static, Message> {
    let prompt = i18n.__(agent_install_preview(state));
    let mut copy_button =
        button(text(i18n.__(if matches!(state.agent_package, LoadState::Loading { .. }) {
            i18n.__("正在准备…")
        } else {
            i18n.__("复制安装文字")
        })))
        .padding([10, 22])
        .style(styles::action_button_style);
    if let Some(message) = copy_action_message(state) {
        copy_button = copy_button.on_press(message);
    }
    let status = match &state.agent_package {
        LoadState::Error(error) => Some(
            text(
                i18n.format("地址生成失败：%s", &[error.render(i18n).into()])
                    .expect("valid display template"),
            )
            .size(12)
            .style(styles::muted_text_style),
        ),
        LoadState::NeedsLogin => {
            Some(text(i18n.__("请先登录识矩账户。")).size(12).style(styles::muted_text_style))
        }
        _ => None,
    };
    let mut actions = row![copy_button].spacing(10).align_y(Alignment::Center);
    if let Some(status) = status {
        actions = actions.push(status);
    }

    container(
        column![
            row![
                method_badge(i18n, i18n.__("推荐")),
                section_header(i18n,
                    Some(i18n.__("方法 2")),
                    i18n.__("让 Agent 帮你安装"),
                    i18n.__("只需 2 步：复制安装文字，再粘贴给 Agent。不同 Agent 的技能目录可能不同，请让 Agent 选择自身支持的目录。"),
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Start),
            row![
                step_card(i18n, "1", i18n.__("复制"), i18n.__("安装文字准备好后，点击“复制”。")),
                step_card(i18n,
                    "2",
                    i18n.__("粘贴给 Agent"),
                    i18n.__("切换到当前使用的 Agent 对话，粘贴并发送；Agent 会自动下载安装。"),
                ),
            ]
            .spacing(12),
            container(
                text(prompt)
                    .size(12)
                    .wrapping(Wrapping::Word),
            )
            .width(Length::Fill)
            .height(Length::Fixed(155.0))
            .padding(14)
            .style(styles::inset_panel_style),
            actions,
        ]
        .spacing(16),
    )
    .width(Length::Fill)
    .padding(20)
    .style(styles::card_style)
    .into()
}

fn platform_card(
    i18n: crate::app::i18n::I18n,
    state: &ShijuInstallGuideState,
    package_id: ShijuPackageId,
) -> Element<'static, Message> {
    let is_pending = state.manual_download.is_some_and(|pending| pending.package_id == package_id);
    let mut download_button = button(text(i18n.__(if is_pending {
        i18n.__("正在生成…")
    } else if state.manual_download.is_some() {
        i18n.__("请稍候")
    } else {
        i18n.__("下载安装包")
    })))
    .width(Length::Fill)
    .padding([9, 12])
    .style(styles::quiet_button_style);
    if let Some(message) = manual_download_action_message(state, package_id) {
        download_button = download_button.on_press(message);
    }

    container(
        column![
            text(package_id.name())
                .size(16)
                .font(Font { weight: iced::font::Weight::Semibold, ..Font::default() }),
            text(i18n.__(package_id.description())).size(12).style(styles::muted_text_style),
            text(i18n.__("平台安装包（zip）")).size(12).style(styles::muted_text_style),
            text(
                i18n.__(
                    "安装步骤：\n1. 下载该平台安装包。\n2. 解压后将技能包根目录放入当前 Agent 的技能目录。"
                )
            )
            .size(12)
            .style(styles::muted_text_style)
            .wrapping(Wrapping::Word),
            Space::new().height(Length::Fill),
            download_button,
        ]
        .spacing(10),
    )
    .width(Length::Fixed(270.0))
    .height(Length::Fixed(260.0))
    .padding(18)
    .style(styles::card_style)
    .into()
}

fn manual_install_section(
    i18n: crate::app::i18n::I18n,
    state: &ShijuInstallGuideState,
) -> Element<'static, Message> {
    let cards = ShijuPackageId::ALL
        .into_iter()
        .map(|package_id| platform_card(i18n, state, package_id))
        .collect::<Vec<_>>();
    container(
        column![
            section_header(i18n,
                None,
                i18n.__("需要手动安装？"),
                i18n.__("如果无法使用自动安装，或你希望自行安装，请下载匹配的安装包，并放入目标 Agent 对应的技能目录。"),
            ),
            row(cards).spacing(12).wrap(),
        ]
        .spacing(14),
    )
    .width(Length::Fill)
    .padding(20)
    .style(styles::card_style)
    .into()
}

/// 构建安装范围、目标、直接安装与手动安装的完整向导。
pub(super) fn view(app: &App) -> Element<'_, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let state = &app.expert_hub.install_guide;
    let header = column![
        text(crate::app::i18n::I18n::new(app.desktop_locale).__("安装识矩技能"))
            .size(26)
            .font(Font { weight: iced::font::Weight::Bold, ..Font::default() }),
        text(crate::app::i18n::I18n::new(app.desktop_locale).__("识矩技能连接 Agent 与技能中心，让 Agent 能获取并使用当前账号已安装的业务技能。推荐把安装文字复制到你正在使用的 Agent 对话中，由 Agent 完成安装。"))
            .size(13)
            .style(styles::muted_text_style)
            .wrapping(Wrapping::Word),
    ]
    .spacing(7)
    .width(Length::Fill);
    let content = container(
        column![
            header,
            direct_install_card(app),
            agent_install_card(i18n, state),
            manual_install_section(i18n, state),
        ]
        .spacing(20),
    )
    .width(Length::Fill)
    .max_width(1240.0)
    .center_x(Length::Fill);

    let body = scrollable(container(content).padding([0, 7]).width(Length::Fill))
        .direction(Direction::Vertical(
            Scrollbar::new().width(SCROLLBAR_WIDTH).scroller_width(SCROLLBAR_WIDTH),
        ))
        .width(Length::Fill)
        .height(Length::Fill);
    column![
        body,
        settings_divider(),
        container(direct_install_actions(app)).padding([4, 7]).align_right(Length::Fill),
    ]
    .spacing(10)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// 在用户请求卸载时为向导叠加确认对话框。
pub(super) fn with_uninstall_confirmation<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    if !app.expert_hub.install_guide.uninstall_confirmation_open {
        return base;
    }
    let cancel = Message::SkillHub(SkillHubMessage::CancelShijuUninstall);
    let current_version = app
        .expert_hub
        .install_guide
        .installed_version
        .as_deref()
        .map(|version| {
            crate::app::i18n::I18n::new(app.desktop_locale)
                .format("当前版本：v%s", &[version.to_string().into()])
                .expect("valid display template")
        })
        .unwrap_or_else(|| {
            crate::app::i18n::I18n::new(app.desktop_locale).__("当前版本：未知").to_string()
        });
    let selected_targets = &app.expert_hub.install_guide.selected_targets;
    let selected_count = selected_targets.len();
    let path_rows = install_target_presentations(app.expert_hub.install_guide.install_scope)
        .into_iter()
        .filter(|target| selected_targets.contains(&target.target))
        .map(|target| {
            row![
                text(format!(
                    "{}：",
                    crate::app::i18n::I18n::new(app.desktop_locale).__(target.name)
                ))
                .size(12),
                text(target.path).size(12).font(Font::MONOSPACE),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
            .into()
        })
        .collect::<Vec<Element<'static, Message>>>();
    let header = settings_modal_header(
        column![
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("确认卸载识矩技能")).size(22),
            text(
                crate::app::i18n::I18n::new(app.desktop_locale)
                    .format(
                        "将从当前选择的 %s 个目标移除本机技能包。",
                        &[selected_count.to_string().into()]
                    )
                    .expect("valid display template")
            )
            .size(12)
            .style(settings_muted_text_style),
        ]
        .spacing(6),
        settings_close_button(cancel.clone()),
    );
    let actions = row![
        Space::new().width(Length::Fill),
        button(text(crate::app::i18n::I18n::new(app.desktop_locale).__("取消")).size(13))
            .on_press(cancel.clone())
            .padding([9, 14])
            .style(rounded_action_btn_style),
        button(text(crate::app::i18n::I18n::new(app.desktop_locale).__("确认卸载")).size(13))
            .on_press(Message::SkillHub(SkillHubMessage::ConfirmShijuUninstall))
            .padding([9, 14])
            .style(danger_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let body = column![
        text(current_version).size(13),
        text(crate::app::i18n::I18n::new(app.desktop_locale).__("卸载位置")).size(13),
        column(path_rows).spacing(6),
        text(
            crate::app::i18n::I18n::new(app.desktop_locale)
                .__("卸载不会删除识矩账号中的业务技能；之后可以重新安装。")
        )
        .size(12)
        .style(settings_muted_text_style),
    ]
    .spacing(10);
    let card = settings_modal_layout(header, body, Some(actions.into()))
        .width(Length::Fixed(520.0))
        .height(Length::Fixed(420.0));

    settings_modal_overlay(Some(base), cancel, container(card).padding(12))
}
