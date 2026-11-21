use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    rounded_action_btn_style, settings_close_button, settings_error_banner, settings_modal_layout,
    settings_modal_overlay_with_escape, settings_select, settings_text_input,
};
use iced::widget::{Column, Row, Space, button, column, container, image, row};
use iced::{Alignment, Length};

use super::assistants::{Assistant, List, ListEvent, PAGE_SIZE, RunFilter, Snapshot, message};
use super::portraits;
use super::view::{
    Item, Tone, action, card, divider, icon, label, page_notice, segment, segmented, status_label,
    strong, tint_style,
};

pub(super) fn view<'a>(state: &'a List, width: f32, actor: Option<&'a str>) -> Item<'a> {
    let mut page = column![
        row![
            strong("私有员工", 24, Tone::Ink).width(Length::Fill),
            action(
                if state.loading { "刷新中…" } else { "刷新" },
                Some(Icon::ArrowRepeat),
                (!state.loading).then(|| message(ListEvent::Refresh)),
                false,
            ),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(20)
    .width(Length::Fill);
    let Some(data) = state.data.as_deref() else {
        let content = match &state.error {
            Some(error) => settings_error_banner(error.as_str()),
            None => page_notice(label("正在加载私有员工…", 14, Tone::Muted)),
        };
        return page.push(content).into();
    };

    let governance = data.governance.as_ref().ok().and_then(Option::as_deref);
    let source = if state.organization { governance.unwrap_or_default() } else { &data.mine };
    let search = state.search.trim().to_lowercase();
    let matches: Vec<_> = source
        .iter()
        .filter(|assistant| {
            state.filter.matches(assistant)
                && (search.is_empty()
                    || [
                        Some(assistant.name.as_str()),
                        Some("私有执行员工"),
                        assistant.owner_name(&data.members, actor),
                        assistant.creator_name(&data.members, actor),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|value| value.to_lowercase().contains(&search)))
        })
        .collect();

    page = page.push(filters(state, data, source, width));
    if let Err(error) = &data.governance {
        page = page.push(settings_error_banner(format!("组织内私有员工加载失败。{error}")));
    }
    if matches.is_empty() {
        let empty = if state.organization && data.governance.is_err() {
            "组织内私有员工暂不可用"
        } else if !search.is_empty() || state.filter != RunFilter::All {
            "没有符合筛选条件的私有员工"
        } else if state.organization {
            "可查看的范围内暂无私有员工"
        } else {
            "你还没有私有员工"
        };
        return page.push(page_notice(label(empty, 14, Tone::Muted))).into();
    }

    let total_pages = matches.len().div_ceil(PAGE_SIZE);
    let selected_page = state.page.min(total_pages - 1);
    let start = selected_page * PAGE_SIZE;
    let end = (start + PAGE_SIZE).min(matches.len());
    let columns = if width >= 990.0 {
        3
    } else if width >= 620.0 {
        2
    } else {
        1
    };
    let mut grid = Column::new().spacing(16);
    for employees in matches[start..end].chunks(columns) {
        let mut group = Row::with_children(
            employees.iter().map(|person| employee_card(person, data, actor, !state.loading)),
        )
        .spacing(16)
        .width(Length::Fill);
        for _ in employees.len()..columns {
            group = group.push(Space::new().width(Length::Fill));
        }
        grid = grid.push(group);
    }
    let mut footer =
        row![label(format!("{}–{} / {} 人", start + 1, end, matches.len()), 12, Tone::Muted,)]
            .spacing(10)
            .align_y(Alignment::Center);
    if total_pages > 1 {
        footer = footer
            .push(action(
                "上一页",
                Some(Icon::ChevronLeft),
                selected_page.checked_sub(1).map(|page| message(ListEvent::Page(page))),
                false,
            ))
            .push(action(
                "下一页",
                Some(Icon::ChevronRight),
                (selected_page + 1 < total_pages)
                    .then(|| message(ListEvent::Page(selected_page + 1))),
                false,
            ));
    }
    page.push(grid).push(footer.wrap()).into()
}

pub(super) fn overlay<'a>(
    base: Item<'a>,
    state: &'a List,
    actor: Option<&'a str>,
) -> Item<'a> {
    let (Some(transfer), Some(employee), Some(data)) =
        (&state.transfer, state.transfer_employee(), state.data.as_deref())
    else {
        return base;
    };
    let close = message(ListEvent::CloseTransfer);
    let owner = employee
        .owner_name(&data.members, actor)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("当前使用人");
    let description = format!(
        "当前使用人是{owner}。转交后，原使用人将立即失去对话、配置、发布和连接办公软件的权限，已连接的消息渠道会一并解绑。"
    );

    let mut form = column![label(description, 14, Tone::Muted), strong("新使用人", 14, Tone::Ink),]
        .spacing(10);
    if let Some(recipients) = &transfer.recipients {
        if recipients.is_empty() {
            form = form.push(label("可指派的范围内暂无其他成员", 14, Tone::Muted));
        } else {
            let query = transfer.search.trim().to_lowercase();
            let mut choices =
                recipients.iter().filter(|recipient| recipient.matches(&query)).peekable();
            let search = settings_text_input("搜索成员姓名或账号", &transfer.search)
                .on_input_maybe(
                    (!transfer.submitting)
                        .then_some(|search| message(ListEvent::TransferSearch(search))),
                )
                .width(Length::Fill);
            form = form.push(search);
            if choices.peek().is_none() {
                form = form.push(label("没有匹配的成员", 14, Tone::Muted));
            }
            let mut options = Column::new().spacing(4);
            for recipient in choices {
                let selected = transfer.selected.as_ref() == Some(recipient);
                let mark: Item<'_> = if selected {
                    icon(Icon::Check, 16, Tone::Ink).into()
                } else {
                    Space::new().width(16).into()
                };
                let option = button(
                    row![mark, label(recipient.to_string(), 14, Tone::Ink)]
                        .spacing(8)
                        .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([10, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe(
                    (!transfer.submitting)
                        .then(|| message(ListEvent::TransferSelect(recipient.clone()))),
                );
                options = options.push(option);
            }
            form = form.push(options);
        }
    } else if transfer.error.is_none() {
        form = form.push(label("正在读取成员…", 14, Tone::Muted));
    }
    let header = row![
        strong("转交使用人", 20, Tone::Ink).width(Length::Fill),
        (!transfer.submitting).then(|| settings_close_button(close.clone())),
    ]
    .align_y(Alignment::Center)
    .spacing(12);
    let actions = row![
        container(transfer.selected.as_ref().map(|recipient| {
            label(format!("已选：{recipient}"), 13, Tone::Muted)
        }))
        .width(Length::Fill),
        action("取消", None, (!transfer.submitting).then(|| close.clone()), false),
        action(
            if transfer.submitting { "正在转交…" } else { "确认转交" },
            None,
            (transfer.selected.is_some() && !transfer.submitting)
                .then(|| message(ListEvent::SubmitTransfer)),
            true,
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let footer = column![
        transfer.error.as_deref().map(|error| label(error, 13, Tone::Red)),
        actions,
    ]
    .spacing(8);
    let dialog = settings_modal_layout(header, form, Some(footer.into()))
        .width(Length::Fill)
        .max_width(640)
        .height(Length::Fill)
        .max_height(640);
    settings_modal_overlay_with_escape(base, close, dialog)
}

fn filters<'a>(state: &'a List, data: &Snapshot, source: &[Assistant], width: f32) -> Item<'a> {
    let mut scopes = row![segment(
        format!("我的私有员工 {}", data.mine.len()),
        !state.organization,
        message(ListEvent::Scope(false)),
    )]
    .spacing(2);
    if let Ok(Some(governance)) = &data.governance {
        scopes = scopes.push(segment(
            format!("组织内私有员工 {}", governance.len()),
            state.organization,
            message(ListEvent::Scope(true)),
        ));
    }
    let on_duty =
        RunFilter::OnDuty(source.iter().filter(|item| item.runtime() == "on_duty").count());
    let abnormal =
        RunFilter::Abnormal(source.iter().filter(|item| item.runtime() == "abnormal").count());
    let current = match state.filter {
        RunFilter::All => RunFilter::All,
        RunFilter::OnDuty(_) => on_duty,
        RunFilter::Abnormal(_) => abnormal,
    };
    let search = settings_text_input("搜索私有员工", &state.search)
        .on_input(|input| message(ListEvent::Search(input)))
        .width(if width >= 595.0 { Length::Fixed(285.0) } else { Length::Fill });
    let run = settings_select([RunFilter::All, on_duty, abnormal], Some(current), |value| {
        message(ListEvent::Run(value))
    })
    .width(156);
    let published = data.mine.iter().filter(|item| item.publish_status == "PUBLISHED").count();
    let runs = data.mine.iter().map(|item| item.usage_count).sum::<u64>();
    let fields = row![
        search,
        run,
        label(
            format!(
                "本人已发布 {published} · 草稿 {} · 今日执行 {runs}",
                data.mine.len() - published
            ),
            12,
            Tone::Muted,
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap();
    column![segmented(scopes.wrap().into()), fields].spacing(10).into()
}

fn employee_card<'a>(
    person: &'a Assistant,
    data: &'a Snapshot,
    actor: Option<&'a str>,
    can_open: bool,
) -> Item<'a> {
    let (runtime, runtime_tone) = match person.runtime() {
        "on_duty" => ("在岗", Tone::Green),
        "abnormal" => ("异常", Tone::Red),
        "enabling" => ("启用中", Tone::Warning),
        "disabling" => ("停用中", Tone::Warning),
        "none" => ("已停用", Tone::Muted),
        _ => ("—", Tone::Muted),
    };
    let (published, publish_tone) = match person.publish_status.as_str() {
        "PUBLISHED" => ("已发布", Tone::Green),
        "PUBLISHING" => ("发布中", Tone::Warning),
        "DRAFT" => ("草稿", Tone::Warning),
        "TESTING" => ("测试中", Tone::Warning),
        status if status.contains("FAIL") => ("发布异常", Tone::Red),
        _ => ("未知", Tone::Muted),
    };
    let publish_tone = if matches!(publish_tone, Tone::Red) { Tone::Red } else { Tone::Muted };
    let status = row![
        status_label(runtime, runtime_tone),
        label(format!("配置 · {published}"), 12, publish_tone),
        (!person.owned_by_caller).then(|| label("组织内", 12, Tone::Muted)),
    ]
    .align_y(Alignment::Center)
    .spacing(10)
    .wrap();
    let picture: Item<'_> = match portraits::avatar(&person.name, false) {
        Some(handle) => image(handle).width(44).height(44).into(),
        None => container(icon(Icon::Person, 20, Tone::Accent))
            .center(44)
            .style(|theme| tint_style(theme, Tone::Accent, 22.0))
            .into(),
    };
    let identity = row![
        picture,
        column![
            strong(person.name.as_str(), 17, Tone::Ink),
            label("私有执行员工", 12, Tone::Muted),
        ]
        .spacing(3)
        .width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let department = |id: &str| {
        data.members.get(id).and_then(|member| member.department.as_deref()).unwrap_or("")
    };
    let attribution = row![
        member(
            "使用人",
            person.owner_name(&data.members, actor),
            department(&person.owner_user_id)
        ),
        member("创建人", person.creator_name(&data.members, actor), department(&person.created_by)),
    ]
    .spacing(10);
    let note = if person.owned_by_caller {
        "仅你可使用 · 通过已连接的办公软件交互"
    } else {
        "仅使用人可使用 · 不参与团队任务"
    };
    card(
        column![
            identity,
            status,
            label(
                format!(
                    "技能 {} · 系统工具 {}",
                    person.capability_summary.skill_count, person.capability_summary.mcp_count
                ),
                12,
                Tone::Muted,
            ),
            attribution,
            label(note, 12, Tone::Muted),
            divider(),
            row![
                label("今日执行", 12, Tone::Muted),
                strong(person.usage_count.to_string(), 13, Tone::Ink),
                person.transfer_id().map(|id| action(
                    "转交使用人",
                    None,
                    can_open.then(|| message(ListEvent::OpenTransfer(id.to_owned()))),
                    false,
                )),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap(),
        ]
        .spacing(10)
        .padding(16),
    )
}

fn member<'a>(title: &'static str, name: Option<&'a str>, department: &'a str) -> Item<'a> {
    column![
        label(title, 11, Tone::Muted),
        strong(name.unwrap_or("—"), 13, Tone::Ink),
        label(if department.is_empty() { "未登记" } else { department }, 11, Tone::Muted),
    ]
    .spacing(2)
    .width(Length::Fill)
    .into()
}
