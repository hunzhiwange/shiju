use super::list::SCROLLBAR_WIDTH;
use super::styles;
use crate::app::assets::{self, Icon};
use crate::app::message::SkillHubMessage;
use crate::app::skill_hub::{
    InstalledSkill, LoadState, PublicDetailMode, SkillDetail, SkillDetailState, SkillVersion,
    SkillVersionAction, SkillVersionDirection,
};
use crate::app::{App, Message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg::{self, Svg};
use iced::widget::{Space, button, column, container, markdown, row, scrollable, text};
use iced::{Alignment, Element, Font, Length, Theme};
use std::cmp::Ordering;

pub(super) fn close_message() -> Message {
    Message::SkillHub(SkillHubMessage::CloseSkillDetail)
}

pub(super) fn download_message(version_id: u64) -> Message {
    Message::SkillHub(SkillHubMessage::DownloadVersion(version_id))
}

fn icon(icon: Icon, size: f32) -> Svg<'static> {
    Svg::new(assets::get_icon(icon)).width(Length::Fixed(size)).height(Length::Fixed(size)).style(
        |theme: &Theme, _status| svg::Style { color: Some(theme.palette().text.scale_alpha(0.74)) },
    )
}

fn back_button() -> Element<'static, Message> {
    button(
        row![icon(Icon::ChevronLeft, 14.0), text("技能").size(13)]
            .spacing(5)
            .align_y(Alignment::Center),
    )
    .on_press(close_message())
    .padding([7, 10])
    .style(styles::quiet_button_style)
    .into()
}

fn installed_skill(app: &App, skill_id: u64) -> Option<&InstalledSkill> {
    match &app.expert_hub.installed {
        LoadState::Loaded(items) => items.iter().find(|skill| skill.skill_id == skill_id),
        _ => None,
    }
}

fn is_installed(app: &App, skill_id: u64) -> bool {
    installed_skill(app, skill_id).is_some()
        || match &app.expert_hub.marketplace {
            LoadState::Loaded(page) => page
                .items
                .iter()
                .find(|skill| skill.id == skill_id)
                .is_some_and(|skill| skill.is_installed),
            _ => false,
        }
}

fn action_message(skill_id: u64, installed: bool) -> Message {
    Message::SkillHub(if installed {
        SkillHubMessage::UninstallSkill(skill_id)
    } else {
        SkillHubMessage::InstallSkill(skill_id)
    })
}

fn field(label: &'static str, value: String) -> Element<'static, Message> {
    column![text(label).size(11).style(styles::muted_text_style), text(value).size(14),]
        .spacing(5)
        .width(Length::Fixed(210.0))
        .into()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum VersionRowAction {
    Current,
    Change { label: &'static str, action: SkillVersionAction },
}

fn numeric_version(version: &str) -> Option<Vec<u64>> {
    let version = version.strip_prefix('v').unwrap_or(version);
    if version.is_empty() {
        return None;
    }
    version
        .split('.')
        .map(|part| {
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                None
            } else {
                part.parse().ok()
            }
        })
        .collect()
}

fn compare_numeric_versions(left: &str, right: &str) -> Option<Ordering> {
    let left = numeric_version(left)?;
    let right = numeric_version(right)?;
    let length = left.len().max(right.len());
    Some(
        (0..length)
            .map(|index| {
                left.get(index)
                    .copied()
                    .unwrap_or_default()
                    .cmp(&right.get(index).copied().unwrap_or_default())
            })
            .find(|ordering| *ordering != Ordering::Equal)
            .unwrap_or(Ordering::Equal),
    )
}

pub(super) fn version_action_spec(
    installed_version: &str,
    version: &SkillVersion,
) -> VersionRowAction {
    let direction = match compare_numeric_versions(installed_version, &version.version) {
        Some(Ordering::Equal) => return VersionRowAction::Current,
        Some(Ordering::Less) => SkillVersionDirection::Upgrade,
        Some(Ordering::Greater) => SkillVersionDirection::Rollback,
        None if installed_version == version.version => return VersionRowAction::Current,
        None => SkillVersionDirection::Switch,
    };
    let label = match direction {
        SkillVersionDirection::Upgrade => "升级到",
        SkillVersionDirection::Rollback => "回退到",
        SkillVersionDirection::Switch => "切换到",
    };
    VersionRowAction::Change {
        label,
        action: SkillVersionAction {
            skill_id: version.skill_id,
            version_id: version.id,
            target_version: version.version.clone(),
            direction,
        },
    }
}

fn version_row(
    version: &SkillVersion,
    installed_version: Option<&str>,
    busy: bool,
) -> Element<'static, Message> {
    let changelog = version.changelog.as_deref().unwrap_or("无更新说明");
    let download_available = version.download_url.is_some();
    let mut download_button = button(
        row![
            icon(Icon::CloudDownload, 14.0),
            text(if download_available { "下载" } else { "不可下载" }).size(12),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([8, 12])
    .style(styles::quiet_button_style);
    if download_available {
        download_button = download_button.on_press(download_message(version.id));
    }
    let mut actions = row![download_button].spacing(8).align_y(Alignment::Center);
    if let Some(installed_version) = installed_version {
        match version_action_spec(installed_version, version) {
            VersionRowAction::Current => {
                actions = actions.push(
                    button(text("当前版本").size(12))
                        .padding([8, 12])
                        .style(styles::quiet_button_style),
                );
            }
            VersionRowAction::Change { label, action } => {
                let action_label = if busy {
                    "处理中…".to_string()
                } else {
                    format!("{label} v{}", version.version)
                };
                let mut action_button = button(text(action_label).size(12))
                    .padding([8, 12])
                    .style(styles::pill_button_style);
                if !busy {
                    action_button = action_button
                        .on_press(Message::SkillHub(SkillHubMessage::ChangeSkillVersion(action)));
                }
                actions = actions.push(action_button);
            }
        }
    }
    container(
        row![
            container(text(format!("v{}", version.version)).size(12))
                .padding([5, 9])
                .style(styles::meta_badge_style),
            column![
                text(changelog.to_string()).size(13),
                text(format!("{} · {}", version.status, version.change_type))
                    .size(11)
                    .style(styles::muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            actions,
        ]
        .align_y(Alignment::Center)
        .spacing(12),
    )
    .padding(13)
    .width(Length::Fill)
    .style(styles::inset_panel_style)
    .into()
}

fn readme_mode_button(
    label: &'static str,
    mode: PublicDetailMode,
    selected: bool,
) -> Element<'static, Message> {
    button(text(label).size(12))
        .on_press(Message::SkillHub(SkillHubMessage::SetSkillReadmeMode(mode)))
        .padding([7, 12])
        .style(move |theme, status| styles::selected_tab_style(theme, status, selected))
        .into()
}

fn readme_panel<'a>(app: &'a App, state: &'a SkillDetailState) -> Element<'a, Message> {
    let body: Element<'a, Message> = match &state.readme {
        LoadState::Loaded(content) if content.trim().is_empty() => {
            text("当前版本暂无 README。").size(13).style(styles::muted_text_style).into()
        }
        LoadState::Loaded(content) if state.readme_mode == PublicDetailMode::Markdown => {
            text(content).size(13).font(Font::default()).width(Length::Fill).into()
        }
        LoadState::Loaded(_) => {
            let mut markdown_style = markdown::Style::from(&app.theme());
            markdown_style.inline_code_font = Font::default();
            markdown_style.code_block_font = Font::default();
            let settings = markdown::Settings::with_text_size(14, markdown_style);
            markdown::view(state.readme_items.iter(), settings).map(|_| Message::None).into()
        }
        LoadState::Error(error) => text(format!("README 加载失败：{error}"))
            .size(13)
            .style(styles::muted_text_style)
            .into(),
        LoadState::NeedsLogin => text("登录识矩账户后查看 README")
            .size(13)
            .style(styles::muted_text_style)
            .into(),
        LoadState::Idle | LoadState::Loading { .. } => text("README 正在加载…")
            .size(13)
            .style(styles::muted_text_style)
            .into(),
    };

    container(
        column![
            row![
                text("README")
                    .size(18)
                    .font(Font { weight: iced::font::Weight::Semibold, ..Font::default() })
                    .width(Length::Fill),
                readme_mode_button(
                    "可视化",
                    PublicDetailMode::Visual,
                    state.readme_mode == PublicDetailMode::Visual,
                ),
                readme_mode_button(
                    "Markdown",
                    PublicDetailMode::Markdown,
                    state.readme_mode == PublicDetailMode::Markdown,
                ),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            container(body).padding(18).width(Length::Fill),
        ]
        .spacing(12),
    )
    .padding(22)
    .width(Length::Fill)
    .style(styles::detail_panel_style)
    .into()
}

fn loaded_view<'a>(
    app: &'a App,
    state: &'a SkillDetailState,
    detail: &SkillDetail,
    versions: &[SkillVersion],
    versions_notice: Option<String>,
) -> Element<'a, Message> {
    let installed = is_installed(app, detail.id);
    let installed_version =
        installed_skill(app, detail.id).map(|skill| skill.installed_version.as_str());
    let pending = app.expert_hub.action.as_ref().is_some_and(|action| action.skill_id == detail.id);
    let confirming_rollback = app
        .expert_hub
        .version_confirmation
        .as_ref()
        .is_some_and(|action| action.skill_id == detail.id);
    let action_label = if pending {
        "处理中…"
    } else if confirming_rollback {
        "等待确认…"
    } else if installed {
        "卸载"
    } else {
        "安装"
    };
    let mut action_button =
        button(text(action_label).size(13)).padding([9, 18]).style(styles::pill_button_style);
    if !pending && !confirming_rollback {
        action_button = action_button.on_press(action_message(detail.id, installed));
    }

    let tags = detail.tags.iter().map(|tag| tag.name.as_str()).collect::<Vec<_>>().join(" · ");
    let current_version = detail
        .current_version
        .as_ref()
        .map(|version| version.version.clone())
        .unwrap_or_else(|| format!("#{}", detail.current_version_id));

    let information = container(
        column![
            row![
                text("技能信息")
                    .size(18)
                    .font(Font { weight: iced::font::Weight::Semibold, ..Font::default() }),
                Space::new().width(Length::Fill),
                container(text(detail.status.clone()).size(11))
                    .padding([5, 10])
                    .style(styles::meta_badge_style),
            ]
            .align_y(Alignment::Center),
            container(Space::new())
                .height(Length::Fixed(1.0))
                .width(Length::Fill)
                .style(styles::divider_style),
            row![
                field("名称", detail.name.clone()),
                field("当前版本", current_version),
                field("格式", detail.format.clone()),
                field("标识", detail.slug.clone()),
            ]
            .spacing(18)
            .wrap(),
            row![
                field("层级", detail.level.clone()),
                field("模态", detail.modality.clone()),
                field(
                    "访问范围",
                    if detail.is_all_company_accessible == 1 {
                        "全公司".to_string()
                    } else {
                        "受限".to_string()
                    },
                ),
                field("标签", if tags.is_empty() { "暂无".to_string() } else { tags },),
            ]
            .spacing(18)
            .wrap(),
            column![
                text("介绍").size(11).style(styles::muted_text_style),
                text(detail.description.clone()).size(14).width(Length::Fill),
            ]
            .spacing(6),
        ]
        .spacing(18),
    )
    .padding(22)
    .width(Length::Fill)
    .style(styles::detail_panel_style);

    let mut versions_content = column![
        row![
            text("版本")
                .size(18)
                .font(Font { weight: iced::font::Weight::Semibold, ..Font::default() }),
            container(text(versions.len().to_string()).size(11))
                .padding([4, 8])
                .style(styles::meta_badge_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(10);
    if let Some(action) =
        app.expert_hub.version_confirmation.as_ref().filter(|action| action.skill_id == detail.id)
    {
        versions_content = versions_content.push(
            container(
                row![
                    text(format!(
                        "确认回退到 v{}？服务端安装版本将被切换。",
                        action.target_version
                    ))
                    .size(13)
                    .width(Length::Fill),
                    button(text("取消").size(12))
                        .on_press(Message::SkillHub(SkillHubMessage::CancelSkillVersionChange))
                        .padding([7, 12])
                        .style(styles::quiet_button_style),
                    button(text("确认回退").size(12))
                        .on_press(Message::SkillHub(SkillHubMessage::ConfirmSkillVersionChange))
                        .padding([7, 12])
                        .style(styles::pill_button_style),
                ]
                .align_y(Alignment::Center)
                .spacing(8),
            )
            .padding(12)
            .style(styles::inset_panel_style),
        );
    }
    if let Some(notice) = versions_notice {
        versions_content =
            versions_content.push(text(notice).size(13).style(styles::muted_text_style));
    } else if versions.is_empty() {
        versions_content =
            versions_content.push(text("暂无可下载版本").size(13).style(styles::muted_text_style));
    } else {
        for version in versions {
            versions_content = versions_content.push(version_row(
                version,
                installed_version,
                pending || confirming_rollback,
            ));
        }
    }

    let page = column![
        back_button(),
        row![
            column![
                text(detail.name.clone())
                    .size(25)
                    .font(Font { weight: iced::font::Weight::Bold, ..Font::default() }),
                text(detail.description.clone()).size(14).style(styles::muted_text_style),
            ]
            .spacing(8)
            .width(Length::Fill),
            action_button,
        ]
        .align_y(Alignment::Center)
        .spacing(18),
        container(Space::new())
            .height(Length::Fixed(1.0))
            .width(Length::Fill)
            .style(styles::divider_style),
        information,
        readme_panel(app, state),
        versions_content,
    ]
    .spacing(20)
    .width(Length::Fill);

    scrollable(container(page).width(Length::Fill).max_width(1180).center_x(Length::Fill))
        .direction(Direction::Vertical(
            Scrollbar::new().width(SCROLLBAR_WIDTH).scroller_width(SCROLLBAR_WIDTH),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn status_view(title: &'static str, detail: String) -> Element<'static, Message> {
    container(
        column![
            back_button(),
            container(
                column![
                    text(title)
                        .size(18)
                        .font(Font { weight: iced::font::Weight::Semibold, ..Font::default() }),
                    text(detail).size(13).style(styles::muted_text_style),
                ]
                .spacing(8),
            )
            .padding(22)
            .width(Length::Fill)
            .style(styles::detail_panel_style),
        ]
        .spacing(18),
    )
    .width(Length::Fill)
    .max_width(1180)
    .center_x(Length::Fill)
    .into()
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let Some(state) = &app.expert_hub.detail else {
        return status_view("未选择技能", "请返回技能列表选择一个技能".to_string());
    };
    match (&state.detail, &state.versions) {
        (LoadState::Loaded(detail), LoadState::Loaded(versions)) => {
            loaded_view(app, state, detail, versions, None)
        }
        (LoadState::Loaded(detail), LoadState::Error(error)) => {
            loaded_view(
                app,
                state,
                detail,
                &[],
                Some(format!("版本列表加载失败：{error}")),
            )
        }
        (LoadState::Loaded(detail), LoadState::NeedsLogin) => {
            loaded_view(
                app,
                state,
                detail,
                &[],
                Some("登录识矩账户后查看版本列表".to_string()),
            )
        }
        (LoadState::Loaded(detail), LoadState::Idle | LoadState::Loading { .. }) => {
            loaded_view(
                app,
                state,
                detail,
                &[],
                Some("版本列表正在加载…".to_string()),
            )
        }
        (LoadState::Error(error), _) => status_view("技能详情加载失败", error.clone()),
        _ => status_view("正在加载技能详情", "正在同步技能信息与可下载版本…".to_string()),
    }
}
