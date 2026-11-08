//! 历史日志和只读文件手风琴；所有仓库数据由网关提供。
use super::repository_ops::{RemoteAction, remote_action_label};
use crate::app::components::system_settings_common::{
    icon_svg, primary_action_btn_style, rounded_action_btn_style, settings_muted_text_style,
};
use crate::app::{App, Message, assets::Icon, message::GitMessage};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, responsive, row, scrollable, text, tooltip};
use iced::{Alignment, Border, Color, Element, Length};
use unicode_width::UnicodeWidthChar;

pub(super) fn ellipsize(value: &str, cells: usize) -> String {
    let length: usize = value.chars().map(|c| c.width().unwrap_or(0)).sum();
    if length <= cells {
        return value.into();
    }
    let mut used = 0;
    let mut result = String::new();
    for c in value.chars() {
        let width = c.width().unwrap_or(0);
        if used + width >= cells {
            break;
        }
        used += width;
        result.push(c);
    }
    result.push('…');
    result
}

fn hint<'a>(
    trigger: impl Into<Element<'a, Message>>,
    label: String,
    available_width: f32,
) -> Element<'a, Message> {
    const GAP: f32 = 6.0;
    const PADDING: f32 = 5.0;
    let max_width = (available_width - GAP - 2.0 * PADDING).clamp(0.0, 520.0);
    tooltip(
        trigger,
        container(text(label).size(12))
            .padding(10)
            .max_width(max_width)
            .style(container::bordered_box),
        tooltip::Position::Right,
    )
    .gap(GAP)
    .padding(PADDING)
    .into()
}

fn action(
    icon: Icon,
    label: &str,
    message: GitMessage,
    enabled: bool,
    primary: bool,
) -> Element<'static, Message> {
    button(
        row![
            icon_svg(icon, 14.0).style(move |theme: &iced::Theme, _| iced::widget::svg::Style {
                color: Some(if !enabled {
                    theme.palette().text.scale_alpha(0.45)
                } else if primary {
                    Color::WHITE
                } else {
                    theme.palette().text
                }),
            }),
            text(label.to_owned()).size(12)
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    )
    .padding([7, 8])
    .style(move |theme, status| {
        if primary && enabled {
            primary_action_btn_style(theme, status)
        } else {
            let mut style = rounded_action_btn_style(theme, status);
            if !enabled {
                style.text_color = style.text_color.scale_alpha(0.45);
            }
            style
        }
    })
    .on_press_maybe(enabled.then_some(Message::Git(message)))
    .into()
}

pub(super) fn toolbar(app: &App) -> Element<'_, Message> {
    let state = &app.git_repository;
    let valid = state.repo_path.as_deref()
        == crate::app::message::git::git_context_path_for_app(app).as_deref();
    let enabled = valid && !state.loading && !state.remote_busy && !app.git_commit_in_progress;
    let snapshot = &state.snapshot;
    let counts = if valid {
        snapshot
            .ahead_behind
            .map(|(ahead, behind)| format!("↑{ahead} ↓{behind}"))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let controls = row![
        action(Icon::Pencil, "变更", GitMessage::ShowHistory(false), true, !state.history),
        action(Icon::Clock, "历史", GitMessage::ShowHistory(true), true, state.history),
        Space::new().width(Length::Fill),
        text(counts).size(12).style(settings_muted_text_style),
        action(
            Icon::ArrowRepeat,
            "获取",
            GitMessage::RunRemote(RemoteAction::Fetch),
            enabled && snapshot.remote.is_some(),
            false
        ),
        action(
            Icon::CloudDownload,
            "拉取",
            GitMessage::RunRemote(RemoteAction::Pull),
            enabled && snapshot.upstream,
            false
        ),
        action(
            Icon::CloudUpload,
            if snapshot.upstream { "推送" } else { "发布分支" },
            GitMessage::RunRemote(RemoteAction::Push),
            enabled && snapshot.remote.is_some() && snapshot.branch.is_some(),
            true
        ),
        action(
            Icon::ArrowRepeat,
            "刷新",
            GitMessage::RefreshRepository,
            !state.loading || !valid,
            false
        ),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    let mut content = column![controls].spacing(9);
    if state.remote_busy {
        let label = format!(
            "正在{}远端更新…",
            state.remote_action.map(remote_action_label).unwrap_or("同步")
        );
        let position = state.remote_animation_frame as u32 % 40;
        let left = if position <= 20 { position } else { 40 - position } + 1;
        let activity = row![
            Space::new().width(Length::FillPortion(left as u16)),
            container(Space::new().height(3)).width(Length::FillPortion(6)).style(
                |theme: &iced::Theme| container::Style {
                    background: Some(theme.extended_palette().primary.base.color.into()),
                    ..Default::default()
                }
            ),
            Space::new().width(Length::FillPortion((22 - left) as u16)),
        ];
        content = content.push(
            container(
                column![
                    row![
                        text(crate::app::components::status_animation::spinner_frame(
                            state.remote_animation_frame as usize
                        ))
                        .size(22)
                        .width(24),
                        text(label).size(14)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    activity,
                ]
                .spacing(10),
            )
            .padding(12)
            .width(Length::Fill)
            .style(|theme: &iced::Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.primary.weak.color.scale_alpha(0.22).into()),
                    border: Border {
                        width: 1.0,
                        radius: 9.0.into(),
                        color: palette.primary.base.color.scale_alpha(0.5),
                    },
                    ..Default::default()
                }
            }),
        );
    }
    if let Some(error) = &state.error {
        content = content.push(text(error).size(12));
    }
    container(content).padding([4, 7]).into()
}

fn scroll<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    scrollable(container(content).padding([0, 7]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill)
        .into()
}

fn selected_style(selected: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |theme, status| {
        let palette = theme.extended_palette();
        let mut style = rounded_action_btn_style(theme, status);
        style.border.radius = 7.0.into();
        if selected {
            style.background = Some(palette.primary.weak.color.into());
            style.text_color = palette.primary.weak.text;
        }
        style
    }
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    if app.git_repository.repo_path.as_deref()
        != crate::app::message::git::git_context_path_for_app(app).as_deref()
    {
        return container(text("点击刷新读取当前仓库历史"))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    }
    responsive(move |size| columns(app, size.width)).into()
}

fn columns(app: &App, width: f32) -> Element<'_, Message> {
    let log_width = (width * 0.23).clamp(140.0, 240.0);
    let state = &app.git_repository;
    let cells = ((log_width - 34.0) / 7.0).max(10.0) as usize;
    let mut commits =
        column![text("提交历史").size(12).style(settings_muted_text_style)].spacing(6);
    for commit in &state.snapshot.commits {
        let selected = state.selected.as_deref() == Some(&commit.sha);
        let time = chrono::DateTime::from_timestamp(commit.time, 0)
            .map(|time| time.with_timezone(&chrono::Local).format("%m-%d %H:%M").to_string())
            .unwrap_or_default();
        let caption = format!("{} · {time}", commit.author);
        let full = format!("{}\n\n{} · {}\n{}", commit.summary, commit.author, time, commit.sha);
        commits = commits.push(hint(
            button(
                column![
                    text(ellipsize(&commit.summary, cells)).size(13).wrapping(text::Wrapping::None),
                    text(ellipsize(&caption, cells)).size(11).wrapping(text::Wrapping::None),
                ]
                .spacing(5),
            )
            .width(Length::Fill)
            .padding(9)
            .style(selected_style(selected))
            .on_press(Message::Git(GitMessage::SelectHistoryCommit(commit.sha.clone()))),
            full,
            width - log_width,
        ));
    }
    if state.loading {
        commits = commits.push(text("正在加载历史…").size(12));
    } else if state.snapshot.commits.is_empty() {
        commits = commits.push(text("暂无提交历史").size(12));
    }
    if state.snapshot.has_more && state.limit < 10_000 {
        commits = commits.push(action(
            Icon::ArrowDown,
            "加载更多",
            GitMessage::LoadMoreHistory,
            !state.loading,
            false,
        ));
    }
    let details: Element<'_, Message> = if state.detail_loading {
        container(text("正在加载提交详情…")).into()
    } else if let Some(detail) = &state.detail {
        let mut files = column![
            text(&detail.message).size(13),
            text(format!("{} 个变更文件 · 点击展开差异", detail.files.len()))
                .size(11)
                .style(settings_muted_text_style)
        ]
        .spacing(10);
        for (index, file) in detail.files.iter().enumerate() {
            files = files.push(super::diff_view::view_history_diff(app, index, file));
        }
        scroll(files)
    } else {
        container(text("选择提交查看变更文件")).into()
    };
    row![container(scroll(commits)).width(log_width), container(details).width(Length::Fill)]
        .spacing(8)
        .height(Length::Fill)
        .into()
}
