use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    settings_close_button, settings_error_banner, settings_modal_card,
    settings_modal_overlay_with_escape, settings_select, settings_text_input,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Column, Row, Space, column, container, image, row, scrollable};
use iced::{Alignment, Length};

use super::assistants::{Assistant, List, ListEvent, PAGE_SIZE, RunFilter, Snapshot, message};
use super::portraits;
use super::view::{
    Item, Tone, action, card, divider, icon, label, page_notice, segment, segmented, status_label,
    strong, tint_style,
};

pub(super) fn view<'a>(i18n: desktop_i18n::I18n, state: &'a List, width: f32, actor: Option<&'a str>) -> Item<'a> {
    let mut page = column![
        row![
            strong(i18n.__("私有员工"), 24, Tone::Ink).width(Length::Fill),
            action(
                if state.loading { i18n.__("刷新中…") } else { i18n.__("刷新") },
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
            Some(error) => settings_error_banner(error.render(i18n)),
            None => page_notice(label(i18n.__("正在加载私有员工…"), 14, Tone::Muted)),
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
                        Some(i18n.__("私有执行员工")),
                        assistant.owner_name(&data.members, actor),
                        assistant.creator_name(&data.members, actor),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|value| value.to_lowercase().contains(&search)))
        })
        .collect();

    page = page.push(filters(i18n, state, data, width));
    if let Err(error) = &data.governance {
        page = page.push(settings_error_banner(i18n.format("组织内私有员工加载失败。%s", &[error.render(i18n).into()]).expect("valid workbench template")));
    }
    if matches.is_empty() {
        let empty = if state.organization && data.governance.is_err() {
            i18n.__("组织内私有员工暂不可用")
        } else if !search.is_empty() || state.filter != RunFilter::All {
            i18n.__("没有符合筛选条件的私有员工")
        } else if state.organization {
            i18n.__("可查看的范围内暂无私有员工")
        } else {
            i18n.__("你还没有私有员工")
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
            employees.iter().map(|person| employee_card(i18n, person, data, actor, !state.loading)),
        )
        .spacing(16)
        .width(Length::Fill);
        for _ in employees.len()..columns {
            group = group.push(Space::new().width(Length::Fill));
        }
        grid = grid.push(group);
    }
    let mut footer =
        row![label(i18n.format("%s–%s / %s 人", &[format!("{}", start + 1).into(), format!("{}", end).into(), format!("{}", matches.len()).into()]).expect("valid workbench template"), 12, Tone::Muted,)]
            .spacing(10)
            .align_y(Alignment::Center);
    if total_pages > 1 {
        footer = footer
            .push(action(
                i18n.__("上一页"),
                Some(Icon::ChevronLeft),
                selected_page.checked_sub(1).map(|page| message(ListEvent::Page(page))),
                false,
            ))
            .push(action(
                i18n.__("下一页"),
                Some(Icon::ChevronRight),
                (selected_page + 1 < total_pages)
                    .then(|| message(ListEvent::Page(selected_page + 1))),
                false,
            ));
    }
    page.push(grid).push(footer.wrap()).into()
}

pub(super) fn overlay<'a>(i18n: desktop_i18n::I18n,
    base: Item<'a>,
    state: &'a List,
    actor: Option<&'a str>,
    height: f32,
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
        .unwrap_or(i18n.__("当前使用人"));
    let description = i18n.format("当前使用人是%s。转交后，原使用人将立即失去对话、配置、发布和连接办公软件的权限，已连接的消息渠道会一并解绑。", &[format!("{}", owner).into()]).expect("valid workbench template");

    let mut form = column![strong(i18n.__("新使用人"), 14, Tone::Ink)].spacing(10);
    if let Some(recipients) = &transfer.recipients {
        if recipients.is_empty() {
            form = form.push(label(i18n.__("可指派的范围内暂无其他成员"), 14, Tone::Muted));
        } else {
            let query = transfer.search.trim().to_lowercase();
            let choices: Vec<_> =
                recipients.iter().filter(|recipient| recipient.matches(&query)).cloned().collect();
            let placeholder =
                if choices.is_empty() { i18n.__("没有匹配的成员") } else { i18n.__("请选择使用人") };
            let search = settings_text_input(i18n.__("搜索成员姓名或账号"), &transfer.search)
                .on_input_maybe(
                    (!transfer.submitting)
                        .then_some(|search| message(ListEvent::TransferSearch(search))),
                )
                .width(Length::Fill);
            form = form.push(search).push(
                settings_select(choices, transfer.selected.clone(), |recipient| {
                    message(ListEvent::TransferSelect(recipient))
                })
                .placeholder(placeholder)
                .disabled(transfer.submitting)
                .width(Length::Fill),
            );
        }
    } else if transfer.error.is_none() {
        form = form.push(label(i18n.__("正在读取成员…"), 14, Tone::Muted));
    }
    let notice = column![
        label(description, 14, Tone::Muted),
        transfer.error.as_ref().map(|error| label(i18n.format("转交失败。%s。如员工资料已更新，请关闭弹窗并刷新列表。", &[error.render(i18n).into()]).expect("valid transfer error template"), 13, Tone::Red)),
    ]
    .spacing(10);
    let notice = container(
        scrollable(container(notice.width(Length::Fill)).padding([0, 7]))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
    )
    .max_height((height - 280.0).clamp(32.0, 180.0))
    .width(Length::Fill);
    let header = row![
        strong(i18n.__("转交使用人"), 20, Tone::Ink).width(Length::Fill),
        (!transfer.submitting).then(|| settings_close_button(close.clone())),
    ]
    .align_y(Alignment::Center)
    .spacing(12);
    let footer = row![
        Space::new().width(Length::Fill),
        action(i18n.__("取消"), None, (!transfer.submitting).then(|| close.clone()), false),
        action(
            if transfer.submitting { i18n.__("正在转交…") } else { i18n.__("确认转交") },
            None,
            (transfer.selected.is_some() && !transfer.submitting)
                .then(|| message(ListEvent::SubmitTransfer)),
            true,
        ),
    ]
    .spacing(8);
    // Keep the select outside the scrollable so its menu can use the window viewport.
    let dialog = settings_modal_card(column![header, notice, form, footer].spacing(14))
        .width(Length::Fill)
        .max_width(640);
    settings_modal_overlay_with_escape(base, close, dialog)
}

fn filters<'a>(i18n: desktop_i18n::I18n, state: &'a List, data: &Snapshot, width: f32) -> Item<'a> {
    let mut scopes = row![segment(
        i18n.format("我的私有员工 %s", &[format!("{}", data.mine.len()).into()]).expect("valid workbench template"),
        !state.organization,
        message(ListEvent::Scope(false)),
    )]
    .spacing(2);
    if let Ok(Some(governance)) = &data.governance {
        scopes = scopes.push(segment(
            i18n.format("组织内私有员工 %s", &[format!("{}", governance.len()).into()]).expect("valid workbench template"),
            state.organization,
            message(ListEvent::Scope(true)),
        ));
    }
    let search = settings_text_input(i18n.__("搜索私有员工"), &state.search)
        .on_input(|input| message(ListEvent::Search(input)))
        .width(if width >= 595.0 { Length::Fixed(285.0) } else { Length::Fill });
    let run = settings_select(
        [RunFilter::All, RunFilter::OnDuty, RunFilter::Abnormal].map(|value| super::LocalizedOption::new(value, i18n)),
        Some(super::LocalizedOption::new(state.filter, i18n)),
        |value| message(ListEvent::Run(value.value)),
    )
    .width(156);
    let published = data.mine.iter().filter(|item| item.publish_status == "PUBLISHED").count();
    let runs = data.mine.iter().map(|item| item.usage_count).sum::<u64>();
    let fields = row![
        search,
        run,
        label(
            i18n.format("本人已发布 %s · 草稿 %s · 今日执行 %s", &[format!("{}", published).into(), format!("{}", data.mine.len() - published).into(), format!("{}", runs).into()]).expect("valid workbench template"),
            12,
            Tone::Muted,
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap();
    column![segmented(scopes.wrap().into()), fields].spacing(10).into()
}

pub(super) fn runtime_badge(i18n: desktop_i18n::I18n, person: &Assistant) -> (&'static str, Tone) {
    match person.runtime() {
        "on_duty" => (i18n.__("在岗"), Tone::Green),
        "abnormal" => (i18n.__("异常"), Tone::Red),
        "enabling" => (i18n.__("启用中"), Tone::Warning),
        "disabling" => (i18n.__("停用中"), Tone::Warning),
        "none" => (i18n.__("已停用"), Tone::Muted),
        _ => ("—", Tone::Muted),
    }
}

fn employee_card<'a>(i18n: desktop_i18n::I18n,
    person: &'a Assistant,
    data: &'a Snapshot,
    actor: Option<&'a str>,
    can_open: bool,
) -> Item<'a> {
    let (runtime, runtime_tone) = runtime_badge(i18n, person);
    let (published, publish_tone) = match person.publish_status.as_str() {
        "PUBLISHED" => (i18n.__("已发布"), Tone::Green),
        "PUBLISHING" => (i18n.__("发布中"), Tone::Warning),
        "DRAFT" => (i18n.__("草稿"), Tone::Warning),
        "TESTING" => (i18n.__("测试中"), Tone::Warning),
        status if status.contains("FAIL") => (i18n.__("发布异常"), Tone::Red),
        _ => (i18n.__("未知"), Tone::Muted),
    };
    let publish_tone = if matches!(publish_tone, Tone::Red) { Tone::Red } else { Tone::Muted };
    let status = row![
        status_label(runtime, runtime_tone),
        label(i18n.format("配置 · %s", &[format!("{}", published).into()]).expect("valid workbench template"), 12, publish_tone),
        (!person.owned_by_caller).then(|| label(i18n.__("组织内"), 12, Tone::Muted)),
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
            label(i18n.__("私有执行员工"), 12, Tone::Muted),
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
        member(i18n,
            i18n.__("使用人"),
            person.owner_name(&data.members, actor),
            department(&person.owner_user_id)
        ),
        member(i18n, i18n.__("创建人"), person.creator_name(&data.members, actor), department(&person.created_by)),
    ]
    .spacing(10);
    let note = if person.owned_by_caller {
        i18n.__("仅你可使用")
    } else {
        i18n.__("仅使用人可使用 · 不参与团队任务")
    };
    card(
        column![
            identity,
            status,
            label(
                i18n.format("技能 %s · 系统工具 %s", &[format!("{}", person.capability_summary.skill_count).into(), format!("{}", person.capability_summary.mcp_count).into()]).expect("valid workbench template"),
                12,
                Tone::Muted,
            ),
            attribution,
            label(note, 12, Tone::Muted),
            divider(),
            row![
                label(i18n.__("今日执行"), 12, Tone::Muted),
                strong(person.usage_count.to_string(), 13, Tone::Ink),
                person.chat_id().map(|id| super::view::chat_action(i18n,
                    can_open.then(|| message(ListEvent::Chat(id.to_owned()))),
                )),
                person.transfer_id().map(|id| action(
                    i18n.__("转交使用人"),
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

fn member<'a>(i18n: desktop_i18n::I18n, title: &'static str, name: Option<&'a str>, department: &'a str) -> Item<'a> {
    column![
        label(title, 11, Tone::Muted),
        strong(name.unwrap_or("—"), 13, Tone::Ink),
        label(if department.is_empty() { i18n.__("未登记") } else { department }, 11, Tone::Muted),
    ]
    .spacing(2)
    .width(Length::Fill)
    .into()
}
