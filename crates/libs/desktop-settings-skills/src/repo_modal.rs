use desktop_provider_apps::AgentAppsContext;
use vw_api_app_skills::repos_api::SkillReposResponse;
#[derive(Debug, Clone, Default)]
pub struct State {
    pub open: bool,
    pub busy: bool,
    pub identity: String,
    pub source_link: String,
    pub source_preview: Option<desktop_platform::skill_deeplink::Request>,
    pub branch: String,
    pub identity_error: Option<String>,
    pub branch_error: Option<String>,
    pub remove_pending: Option<usize>,
    pub snapshot: Option<SkillReposResponse>,
    pub error: Option<String>,
    pub generation: u64,
    pub directory: Option<String>,
    pub context: Option<AgentAppsContext>,
    pub project: Option<String>,
    pub host: String,
}
#[derive(Debug, Clone)]
pub enum Event {
    Open,
    Close,
    Identity(String),
    Branch(String),
    Refresh,
    Add,
    Toggle(usize),
    Remove(usize),
    ConfirmRemove,
    CancelRemove,
    OpenSource(usize),
    SourceLink(String),
    PreviewSource,
    CancelSource,
    ConfirmSource,
    Completed { generation: u64, added: bool, result: Result<SkillReposResponse, String> },
}

// Repository settings share the standard theme-aware, scrollable modal layout.

use crate::third_party::style;
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{Space, button, checkbox, column, container, row, rule, text};
use iced::{Element, Length};
pub fn view<'a, M: Clone + 'static>(
    s: &'a State,
    base: Element<'a, M>,
    message: fn(Event) -> M,
) -> Element<'a, M> {
    view_with_discovery(s, base, message, None)
}

pub fn view_with_discovery<'a, M: Clone + 'static>(
    s: &'a State,
    base: Element<'a, M>,
    message: fn(Event) -> M,
    discovery: Option<&'a vw_api_app_skills::discovery_api::DiscoverSkillsResponse>,
) -> Element<'a, M> {
    if !s.open {
        return base;
    }
    let mut body = column![
        text("添加仓库").size(15),
        settings_field_label("仓库 URL"),
        settings_text_input("owner/name 或 https://github.com/owner/name", &s.identity)
            .style(style::input)
            .on_input_maybe((!s.busy).then_some(move |v| message(Event::Identity(v)))),
    ]
    .spacing(10);
    if let Some(error) = &s.identity_error {
        body = body.push(settings_error_banner(error.clone()));
    }
    body = body.push(settings_field_label("分支")).push(
        settings_text_input("使用默认分支", &s.branch)
            .style(style::input)
            .on_input_maybe((!s.busy).then_some(move |v| message(Event::Branch(v)))),
    );
    if let Some(error) = &s.branch_error {
        body = body.push(settings_error_banner(error.clone()));
    }
    body = body.push(
        button("＋ 添加仓库")
            .padding([8, 10])
            .style(style::primary)
            .on_press_maybe((!s.busy && s.snapshot.is_some()).then_some(message(Event::Add))),
    );
    body = body.push(rule::horizontal(1)).push(text("已添加的仓库").size(15));
    if let Some(snapshot) = &s.snapshot {
        let mut entries = column![];
        for (index, repo) in snapshot.repos.iter().enumerate() {
            let status = match discovery.filter(|report| report.version == snapshot.version) {
                Some(report)
                    if report.failures.iter().any(|failure| {
                        failure.owner == repo.owner
                            && failure.name == repo.name
                            && failure.branch == repo.branch
                    }) =>
                {
                    "读取失败".to_owned()
                }
                Some(report) => format!(
                    "已发现 {} 项",
                    report
                        .skills
                        .iter()
                        .filter(|skill| skill.repo_owner == repo.owner
                            && skill.repo_name == repo.name
                            && skill.repo_branch == repo.branch)
                        .count()
                ),
                None => "数量未读取".to_owned(),
            };
            entries = entries.push(
                row![
                    checkbox(repo.enabled).size(15).style(style::checkbox).on_toggle_maybe(
                        (!s.busy).then_some(move |_| message(Event::Toggle(index)))
                    ),
                    container(
                        column![
                            text(format!("{}/{}", repo.owner, repo.name))
                                .size(13)
                                .wrapping(text::Wrapping::None),
                            text(format!("{} · {status}", repo.branch))
                                .size(12)
                                .style(settings_muted_text_style),
                        ]
                        .spacing(3)
                    )
                    .width(Length::Fill)
                    .clip(true),
                    button(text("打开").size(12))
                        .padding([5, 7])
                        .style(rounded_action_btn_style)
                        .on_press_maybe((!s.busy).then_some(message(Event::OpenSource(index)))),
                    button(text("删除").size(12))
                        .padding([5, 7])
                        .style(rounded_action_btn_style)
                        .on_press_maybe((!s.busy).then_some(message(Event::Remove(index)))),
                ]
                .spacing(7)
                .padding([8, 10])
                .align_y(iced::Alignment::Center),
            );
            if index + 1 < snapshot.repos.len() {
                entries = entries.push(rule::horizontal(1));
            }
        }
        if snapshot.repos.is_empty() {
            body = body.push(text("尚未配置仓库").size(13).style(settings_muted_text_style));
        } else {
            body = body.push(container(entries).width(Length::Fill).style(style::table));
        }
    }
    if let Some(index) = s.remove_pending {
        if let Some(repo) = s.snapshot.as_ref().and_then(|v| v.repos.get(index)) {
            body = body
                .push(text(format!(
                    "确认删除仓库 {}/{}？已安装技能将保留。",
                    repo.owner, repo.name
                )))
                .push(
                    row![
                        button("确认删除")
                            .padding([8, 10])
                            .style(style::primary)
                            .on_press(message(Event::ConfirmRemove)),
                        button("取消")
                            .padding([8, 10])
                            .style(rounded_action_btn_style)
                            .on_press(message(Event::CancelRemove))
                    ]
                    .spacing(8),
                );
        }
    }

    if s.busy {
        body = body.push(text("正在同步仓库…"));
    }
    if let Some(error) = &s.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    body = body.push(
        crate::deeplink::view(
            crate::deeplink::ViewState {
                source_link: &s.source_link,
                busy: s.busy,
                source_preview: &s.source_preview,
                snapshot: &s.snapshot,
            },
            |event| event,
        )
        .map(move_deeplink)
        .map(message),
    );
    let close = message(Event::Close);
    let header =
        settings_modal_header(text("管理技能仓库").size(20), settings_close_button(close.clone()));
    let footer = row![
        Space::new().width(Length::Fill),
        button("刷新")
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(message(Event::Refresh))),
        button("关闭")
            .padding([8, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(close.clone())),
    ]
    .spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(style::dialog)
        .width(440)
        .height(Length::Fill);
    settings_right_drawer_overlay(base, Some((close, container(card).padding(12).into())))
}

fn move_deeplink(e: crate::deeplink::Event) -> Event {
    match e {
        crate::deeplink::Event::SourceLink(v) => Event::SourceLink(v),
        crate::deeplink::Event::PreviewSource => Event::PreviewSource,
        crate::deeplink::Event::CancelSource => Event::CancelSource,
        crate::deeplink::Event::ConfirmSource => Event::ConfirmSource,
    }
}
