use super::{display_author, event, heading, icon, muted, time};
use crate::app::components::system_settings_common::{
    icon_svg, primary_action_btn_style, rounded_action_btn_style, settings_divider,
    settings_error_banner, settings_text_editor, settings_text_editor_style,
};
use crate::app::i18n::I18n;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_task_board::detail::{COMMENT_EDITOR_ID, Event, Field, Operation, State};
use desktop_task_model::Task;
use desktop_ui_controls::icons::acp_agent_icon;
use desktop_ui_controls::overlays::AboveOverlay;
use desktop_ui_controls::system_settings_common::panels::with_settings_modal_escape;
use desktop_ui_controls::system_settings_common::settings_icon_tile_style;
use iced::keyboard::{Key, key::Named};
use iced::widget::{Space, button, column, container, row, svg, text, text_editor, tooltip};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Theme, Vector};
use vw_api_agent_scheduler::{
    AgentSchedulerTaskAgentTurnStatus as TurnStatus, AgentSchedulerTaskCommentDto as Comment,
    AgentSchedulerTaskReplyMode as Mode, AgentSchedulerTaskReplySession as Session,
};

pub(super) fn composer<'a>(app: &'a App, i18n: I18n, task: &Task) -> Element<'a, Message> {
    let state = &app.task_board.detail;
    if task.archived || task.deleted {
        return Space::new().into();
    }
    if state.load_failed {
        return settings_error_banner(i18n.__("无法加载任务详情，请刷新后重试。"));
    }
    let pending = state.request.is_some_and(|(_, operation)| operation == Operation::PostComment);
    let attach = icon_button(
        Icon::Link45Deg,
        i18n.__("添加附件链接"),
        (!state.write_pending() && state.data.is_some() && state.attachments.len() < 5)
            .then(|| event(Event::EditField(Some(Field::Attachment)))),
    );
    let local_user = display_author(i18n, vw_api_transport::id::DEFAULT_USER_ID);
    if !state.comment_expanded {
        let reply = button(
            row![avatar(local_user), text(i18n.__("回复…")).size(14)]
                .spacing(9)
                .align_y(Alignment::Center),
        )
        .style(quiet_button)
        .width(Length::Fill)
        .padding([5, 7])
        .on_press_maybe(state.data.is_some().then(|| event(Event::ExpandComment)));
        return container(row![reply, attach].spacing(6).align_y(Alignment::Center))
            .padding(5)
            .width(Length::Fill)
            .style(|theme| composer_surface(theme, false))
            .into();
    }

    let placeholder =
        i18n.__(if state.reply_to.is_some() { "输入回复…" } else { "回复…" });
    let can_post = state.can_post();
    let can_collapse = state.reply_to.is_none()
        && state.attachments.is_empty()
        && state.comment.text().trim().is_empty();
    let mut input = settings_text_editor(&state.comment)
        .id(COMMENT_EDITOR_ID)
        .placeholder(placeholder)
        .min_height(32)
        .max_height(140)
        .line_height(1.4)
        .style(composer_editor)
        .key_binding(move |press| match press.key.as_ref() {
            Key::Named(Named::Enter) if press.modifiers.command() => {
                can_post.then(|| text_editor::Binding::Custom(event(Event::PostComment)))
            }
            Key::Named(Named::Escape) if can_collapse => {
                Some(text_editor::Binding::Custom(event(Event::CollapseComment)))
            }
            _ => text_editor::Binding::from_key_press(press),
        });
    if !pending {
        input = input.on_action(|action| event(Event::EditComment(action)));
    }
    let mut content = column![].spacing(8).width(Length::Fill);
    if let Some(parent) = state.data.as_ref().and_then(|detail| {
        detail
            .collaboration
            .comments
            .iter()
            .find(|comment| Some(&comment.id) == state.reply_to.as_ref())
    }) {
        let label = i18n
            .format("回复 %s", &[display_author(i18n, &parent.author).into()])
            .expect("valid task reply label");
        content = content.push(
            row![
                container(muted(label)).width(Length::Fill).clip(true),
                icon_button(
                    Icon::X,
                    i18n.__("取消回复"),
                    (!pending).then(|| event(Event::ReplyTo(None)))
                ),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }
    content = content.push(row![container(avatar(local_user)).padding([3, 0]), input].spacing(5));
    if state.comment.text().trim().chars().count() > 4000 {
        content = content.push(muted(i18n.__("回复最多 4,000 字符。")));
    }
    if !state.reply_selection_current() {
        content = content.push(muted(i18n.__("会话状态已变化，请重新选择发送方式。")));
    }
    if !state.attachments.is_empty() {
        content = content.push(
            column(state.attachments.iter().enumerate().map(|(index, link)| {
                row![
                    link_button(link),
                    icon_button(
                        Icon::X,
                        i18n.__("移除附件链接"),
                        (!pending).then(|| event(Event::RemoveAttachment(index)))
                    ),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
                .into()
            }))
            .spacing(3),
        );
    }
    let target: Element<'_, Message> = state
        .data
        .as_ref()
        .and_then(|detail| detail.reply_agent.as_deref().map(|agent| (agent, detail.reply_session)))
        .map_or_else(
            || Space::new().into(),
            |(agent, session)| reply_menu(i18n, state, agent, session, pending),
        );
    let send_label = if pending { "正在发送…" } else { "发送回复" };
    let send = button(icon_svg(Icon::ArrowUp, 17.0).style(move |theme, _| {
        svg::Style {
            color: Some(
                primary_action_btn_style(
                    theme,
                    if can_post { button::Status::Active } else { button::Status::Disabled },
                )
                .text_color,
            ),
        }
    }))
    .padding(9)
    .style(|theme, status| {
        let mut style = primary_action_btn_style(theme, status);
        style.border = style.border.rounded(20);
        style
    })
    .on_press_maybe(can_post.then(|| event(Event::PostComment)));
    content = content.push(
        row![
            container(target).width(Length::Fill),
            attach,
            with_tooltip(send, i18n.__(send_label)),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    );
    container(content)
        .padding([8, 10])
        .width(Length::Fill)
        .style(|theme| composer_surface(theme, true))
        .into()
}

fn reply_menu<'a>(
    i18n: I18n,
    state: &State,
    agent: &'a str,
    session: Session,
    pending: bool,
) -> Element<'a, Message> {
    let name = desktop_task_board::draft::task_acp_agent_label(Some(agent));
    let status = match session {
        Session::Missing => "尚未启动会话",
        Session::Idle => "会话空闲",
        Session::Busy => "运行中",
        Session::Unavailable => "ACP 会话不可用",
    };
    let action = mode_label(state.reply_mode, session);
    let trigger = button(
        row![
            acp_agent_icon(agent, 14.0),
            text(format!("{name} · {}", i18n.__(action)))
                .size(12)
                .wrapping(text::Wrapping::WordOrGlyph),
            icon(Icon::ChevronDown),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([6, 8])
    .style(rounded_action_btn_style)
    .on_press_maybe((!pending).then(|| event(Event::ToggleReplyMenu)));
    let mut options = column![
        container(
            row![acp_agent_icon(agent, 18.0), heading(name, 13), muted(i18n.__(status)),]
                .spacing(8)
                .align_y(Alignment::Center)
        )
        .padding([7, 9])
    ]
    .spacing(4);
    for mode in [Mode::Start, Mode::Queue, Mode::Restart]
        .into_iter()
        .filter(|mode| state.reply_mode_allowed(*mode))
    {
        let description = match (mode, session) {
            (Mode::Start, Session::Idle) => "将回复发送到已有的 ACP 会话。",
            (Mode::Start, _) => "用这条回复开启 ACP 会话。",
            (Mode::Queue, _) => "当前轮结束后，发送这条回复。",
            (Mode::Restart, _) => "停止当前轮，再发送这条回复。",
        };
        options = options.push(mode_option(
            i18n,
            Some(mode),
            session,
            description,
            state.reply_mode == Some(mode),
        ));
    }
    let menu = container(options.push(settings_divider()).push(mode_option(
        i18n,
        None,
        session,
        "回复仅保存在任务讨论中。",
        state.reply_mode.is_none(),
    )))
    .width(300)
    .padding(6)
    .style(menu_surface);
    let menu = with_settings_modal_escape(menu.into(), event(Event::CloseReplyMenu));
    AboveOverlay::new(trigger, menu)
        .show(state.reply_menu_open)
        .gap(6.0)
        .on_close(event(Event::CloseReplyMenu))
        .block_background_events(true)
        .into()
}

fn mode_label(mode: Option<Mode>, session: Session) -> &'static str {
    match mode {
        Some(Mode::Start) if session == Session::Idle => "发送到会话",
        Some(Mode::Start) => "发送后开始",
        Some(Mode::Queue) => "本轮结束后开始",
        Some(Mode::Restart) => "停止并重新开始",
        None => "仅评论",
    }
}

fn mode_option(
    i18n: I18n,
    mode: Option<Mode>,
    session: Session,
    description: &str,
    selected: bool,
) -> Element<'static, Message> {
    let label = heading(i18n.__(mode_label(mode, session)), 13).width(Length::Fill);
    let check: Element<'_, Message> =
        if selected { icon(Icon::Check).into() } else { Space::new().width(14).into() };
    button(
        column![
            row![label, check].spacing(8).align_y(Alignment::Center),
            muted(i18n.__(description)).wrapping(text::Wrapping::WordOrGlyph)
        ]
        .spacing(3),
    )
    .width(Length::Fill)
    .padding([7, 9])
    .style(move |theme, status| {
        let mut style = quiet_button(theme, status);
        if selected && status == button::Status::Active {
            style.background = Some(theme.palette().primary.scale_alpha(0.09).into());
        }
        style
    })
    .on_press(event(Event::SelectReplyMode(mode)))
    .into()
}

pub(super) fn comments<'a>(app: &'a App, i18n: I18n, task: &Task) -> Element<'a, Message> {
    let state = &app.task_board.detail;
    let Some(detail) = &state.data else {
        return if state.load_failed {
            Space::new().into()
        } else {
            muted(i18n.__("正在加载任务详情…")).into()
        };
    };
    if detail.collaboration.comments.is_empty() {
        return Space::new().into();
    }
    let mut replies = std::collections::HashMap::<&str, Vec<_>>::new();
    for comment in &detail.collaboration.comments {
        if let Some(parent) = &comment.reply_to {
            replies.entry(parent.as_str()).or_default().push(comment);
        }
    }
    let editable = !task.archived
        && !task.deleted
        && !state.request.is_some_and(|(_, operation)| operation == Operation::PostComment);
    let mut threads = column![heading(i18n.__("讨论"), 16)].spacing(10).width(Length::Fill);
    for comment in detail.collaboration.comments.iter().filter(|comment| comment.reply_to.is_none())
    {
        let replying = state.reply_to.as_ref() == Some(&comment.id);
        let action = if editable {
            Some(
                button(text(i18n.__(if replying { "正在回复" } else { "回复" })).size(12))
                    .padding([4, 8])
                    .style(quiet_button)
                    .on_press_maybe(
                        (!replying).then(|| event(Event::ReplyTo(Some(comment.id.clone())))),
                    )
                    .into(),
            )
        } else {
            None
        };
        let active = detail.reply_active_turn_id.as_deref();
        let mut thread =
            column![comment_entry(i18n, comment, action, active, state.write_pending())]
                .spacing(10);
        if let Some(replies) = replies.get(comment.id.as_str()) {
            thread = thread.push(settings_divider()).extend(replies.iter().map(|reply| {
                container(comment_entry(i18n, reply, None, active, state.write_pending()))
                    .padding(iced::Padding::ZERO.left(18))
                    .into()
            }));
        }
        threads =
            threads.push(container(thread).padding(12).width(Length::Fill).style(super::surface));
    }
    threads.into()
}

fn comment_entry<'a>(
    i18n: I18n,
    comment: &'a Comment,
    action: Option<Element<'a, Message>>,
    active_id: Option<&str>,
    pending: bool,
) -> Element<'a, Message> {
    let author = display_author(i18n, &comment.author);
    let timestamp = u64::try_from(comment.created_at_ms.0).unwrap_or_default();
    let mut header = row![heading(author, 13).width(Length::Fill), muted(time(timestamp))]
        .spacing(8)
        .align_y(Alignment::Center);
    if let Some(action) = action {
        header = header.push(action);
    }
    let mut content = column![header].spacing(6).width(Length::Fill);
    if !comment.body.is_empty() {
        content = content.push(body_text(&comment.body));
    }
    content = content.extend(comment.attachments.iter().map(link_button));
    let entry = row![avatar(author), content].spacing(9);
    let Some(turn) = comment.agent_turn.as_ref() else { return entry.into() };
    let label = match turn.status {
        TurnStatus::Queued => "等待 ACP 会话",
        TurnStatus::Running => "运行中",
        TurnStatus::Completed => "已完成",
        TurnStatus::Stopped => "已停止",
        TurnStatus::Failed => "ACP 回复失败",
    };
    let agent = desktop_task_board::draft::task_acp_agent_label(Some(&turn.agent));
    let mark = match turn.status {
        TurnStatus::Queued => Icon::Clock,
        TurnStatus::Running => Icon::ArrowClockwise,
        TurnStatus::Completed => Icon::Check,
        TurnStatus::Stopped => Icon::Square,
        TurnStatus::Failed => Icon::X,
    };
    let mut header = row![heading(agent, 13), icon(mark), muted(i18n.__(label))]
        .spacing(7)
        .align_y(Alignment::Center);
    if active_id == Some(comment.id.as_str()) {
        header = header.push(Space::new().width(Length::Fill)).push(icon_button(
            Icon::Square,
            i18n.__("停止 ACP 回复"),
            (!pending).then(|| event(Event::StopReply(comment.id.clone()))),
        ));
    }
    let mut reply = column![header].spacing(6).width(Length::Fill);
    if !turn.response.is_empty() {
        reply = reply.push(body_text(&turn.response));
    }
    column![
        entry,
        row![
            container(acp_agent_icon(&turn.agent, 16.0))
                .center_x(26)
                .center_y(26)
                .style(settings_icon_tile_style),
            reply,
        ]
        .spacing(9)
    ]
    .spacing(10)
    .into()
}

fn body_text(value: &str) -> iced::widget::Text<'_> {
    text(value).size(14).line_height(1.45).width(Length::Fill).wrapping(text::Wrapping::WordOrGlyph)
}

fn avatar<'a>(name: &str) -> Element<'a, Message> {
    let initial = name.chars().next().unwrap_or('?').to_uppercase().to_string();
    container(heading(initial, 12))
        .center_x(26)
        .center_y(26)
        .style(|theme: &Theme| container::Style {
            text_color: Some(theme.palette().text),
            background: Some(theme.palette().primary.scale_alpha(0.16).into()),
            border: Border::default().rounded(13),
            ..Default::default()
        })
        .into()
}

fn link_button(link: &String) -> Element<'_, Message> {
    use crate::app::message::ViewMessage;
    button(
        row![
            icon(Icon::Link45Deg),
            text(link).size(12).width(Length::Fill).wrapping(text::Wrapping::WordOrGlyph)
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([4, 6])
    .style(quiet_button)
    .on_press(Message::View(ViewMessage::OpenUrlExternal(link.clone())))
    .into()
}

fn icon_button(
    glyph: Icon,
    label: &'static str,
    message: Option<Message>,
) -> Element<'static, Message> {
    let opacity = if message.is_some() { 0.85 } else { 0.36 };
    with_tooltip(
        button(icon_svg(glyph, 16.0).style(move |theme, _| svg::Style {
            color: Some(theme.palette().text.scale_alpha(opacity)),
        }))
        .padding(8)
        .style(quiet_button)
        .on_press_maybe(message),
        label,
    )
}

fn with_tooltip<'a>(
    control: impl Into<Element<'a, Message>>,
    label: &'static str,
) -> Element<'a, Message> {
    let content = container(text(label).size(12))
        .padding([4, 7])
        .style(desktop_ui_controls::system_settings_common::icons::icon_btn_tooltip_style);
    tooltip(control, content, tooltip::Position::Top).gap(5).padding(0).into()
}

fn quiet_button(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    if matches!(status, button::Status::Active | button::Status::Disabled) {
        style.background = None;
    }
    style.border = Border::default().rounded(8);
    style
}

fn composer_editor(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    let mut style = settings_text_editor_style(theme, status);
    style.background = Color::TRANSPARENT.into();
    style.border = Border::default().rounded(7);
    style
}

fn composer_surface(theme: &Theme, expanded: bool) -> container::Style {
    let palette = theme.palette();
    container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: Border {
            color: if expanded {
                palette.primary.scale_alpha(0.46)
            } else {
                palette.text.scale_alpha(0.17)
            },
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

fn menu_surface(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: Border {
            color: theme.palette().text.scale_alpha(0.2),
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: Color::BLACK.scale_alpha(0.22),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        },
        ..Default::default()
    }
}
