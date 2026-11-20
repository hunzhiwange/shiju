use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, settings_pick_list_menu_style,
    settings_pick_list_style, settings_text_input,
};
use iced::widget::{Column, Row, Space, column, container, image, pick_list, row};
use iced::{Alignment, Length};

use super::assistants::{Assistant, List, ListEvent, PAGE_SIZE, RunFilter, Snapshot, message};
use super::portraits;
use super::view::{Item, Tone, action, card, chip, icon, label, strong, tint_style};

pub(super) fn view<'a>(state: &'a List, width: f32, actor: Option<&'a str>) -> Item<'a> {
    let mut page = column![strong("私有员工", 25, Tone::Ink)].spacing(16).width(Length::Fill);
    let Some(data) = state.data.as_deref() else {
        let content = match &state.error {
            Some(error) => column![
                label(error.as_str(), 14, Tone::Red),
                action("重试", None, Some(message(ListEvent::Refresh)), false),
            ]
            .spacing(12),
            None => column![label("正在加载私有员工…", 14, Tone::Muted)],
        };
        return page.push(card(container(content).padding(16))).into();
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

    page = page.push(kpis(data, width)).push(filters(state, data, source, width));
    if let Err(error) = &data.governance {
        page = page.push(label(format!("组织内私有员工加载失败。{error}"), 13, Tone::Red));
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
        return page
            .push(card(container(label(empty, 14, Tone::Muted)).center_x(Length::Fill).padding(32)))
            .into();
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
        let mut group =
            Row::with_children(employees.iter().map(|person| employee_card(person, data, actor)))
                .spacing(16)
                .width(Length::Fill);
        for _ in employees.len()..columns {
            group = group.push(Space::new().width(Length::Fill));
        }
        grid = grid.push(group);
    }
    page.push(grid)
        .push(
            row![
                label(
                    format!("第 {}–{} 项，共 {} 项", start + 1, end, matches.len()),
                    13,
                    Tone::Muted
                ),
                Space::new().width(Length::Fill),
                action(
                    "上一页",
                    Some(Icon::ChevronLeft),
                    selected_page.checked_sub(1).map(|page| message(ListEvent::Page(page))),
                    false,
                ),
                action(
                    "下一页",
                    Some(Icon::ChevronRight),
                    (selected_page + 1 < total_pages)
                        .then(|| message(ListEvent::Page(selected_page + 1))),
                    false,
                ),
            ]
            .align_y(Alignment::Center)
            .spacing(8)
            .wrap(),
        )
        .into()
}

fn kpis(data: &Snapshot, width: f32) -> Item<'_> {
    let mine = data.mine.len();
    let published = data.mine.iter().filter(|item| item.publish_status == "PUBLISHED").count();
    let runs = data.mine.iter().map(|item| item.usage_count).sum::<u64>();
    let drafts = mine - published;
    let visible = data.governance.as_ref().map(|rows| mine + rows.as_ref().map_or(0, Vec::len));
    let owned = stat(
        "我的私有员工",
        mine.to_string(),
        format!("{published} 个已发布"),
        Icon::Robot,
        Tone::Violet,
    );
    let today =
        stat("今日执行", runs.to_string(), "仅本人私有员工".into(), Icon::Check, Tone::Green);
    let organization = stat(
        "组织内私有员工",
        visible.as_ref().map_or_else(|_| "—".into(), |count| count.to_string()),
        if matches!(&data.governance, Ok(None)) { "仅本人" } else { "账号可见范围" }.into(),
        Icon::Robot,
        Tone::Blue,
    );
    let draft = stat(
        "草稿待发布",
        drafts.to_string(),
        "发布后才能开始用".into(),
        Icon::Clock,
        if drafts > 0 { Tone::Warning } else { Tone::Green },
    );
    if width >= 900.0 {
        row![owned, today, organization, draft].spacing(16).into()
    } else if width >= 490.0 {
        column![row![owned, today].spacing(12), row![organization, draft].spacing(12)]
            .spacing(12)
            .into()
    } else {
        column![owned, today, organization, draft].spacing(10).into()
    }
}

fn stat(
    title: &'static str,
    value: String,
    hint: String,
    glyph: Icon,
    tone: Tone,
) -> Item<'static> {
    let badge = container(icon(glyph, 19, tone))
        .center(38)
        .style(move |theme| tint_style(theme, tone, 8.0));
    card(
        row![
            badge,
            column![
                label(title, 13, Tone::Muted),
                strong(value, 24, Tone::Ink),
                label(hint, 12, Tone::Muted)
            ]
            .spacing(4),
        ]
        .align_y(Alignment::Start)
        .spacing(12)
        .padding(16),
    )
}

fn filters<'a>(state: &'a List, data: &Snapshot, source: &[Assistant], width: f32) -> Item<'a> {
    let mut scopes =
        row![scope("我的私有员工", data.mine.len(), !state.organization, false)].spacing(8);
    if let Ok(Some(governance)) = &data.governance {
        scopes = scopes.push(scope("组织内私有员工", governance.len(), state.organization, true));
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
    let search = column![
        label("关键字", 12, Tone::Muted),
        settings_text_input("搜索私有员工名称、岗位或能力", &state.search)
            .on_input(|input| message(ListEvent::Search(input)))
            .width(if width >= 620.0 { Length::Fixed(285.0) } else { Length::Fill }),
    ]
    .spacing(6);
    let run = column![
        label("运行状态", 12, Tone::Muted),
        pick_list([RunFilter::All, on_duty, abnormal], Some(current), |value| message(
            ListEvent::Run(value)
        ))
        .padding(SETTINGS_CONTROL_PADDING)
        .text_size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style),
    ]
    .spacing(6);
    let refresh = action(
        if state.loading { "加载中" } else { "查询" },
        Some(Icon::Search),
        (!state.loading).then(|| message(ListEvent::Refresh)),
        true,
    );
    let fields: Item<'_> = if width >= 620.0 {
        row![search, run, Space::new().width(Length::Fill), refresh]
            .align_y(Alignment::End)
            .spacing(12)
            .into()
    } else {
        column![search, row![run, refresh].align_y(Alignment::End).spacing(12)].spacing(12).into()
    };
    card(column![scopes.wrap(), fields].spacing(12).padding(16))
}

fn scope(title: &str, count: usize, selected: bool, organization: bool) -> Item<'static> {
    use desktop_ui_controls::system_settings_common::{
        primary_action_btn_style, rounded_action_btn_style,
    };
    iced::widget::button(iced::widget::text(format!("{title} {count}")).size(13))
        .padding([8, 12])
        .style(if selected { primary_action_btn_style } else { rounded_action_btn_style })
        .on_press(message(ListEvent::Scope(organization)))
        .into()
}

fn employee_card<'a>(
    person: &'a Assistant,
    data: &'a Snapshot,
    actor: Option<&'a str>,
) -> Item<'a> {
    let (runtime, runtime_tone) = match person.runtime() {
        "on_duty" => ("● 在岗", Tone::Green),
        "abnormal" => ("● 异常", Tone::Red),
        "enabling" => ("● 启用中", Tone::Warning),
        "disabling" => ("● 停用中", Tone::Warning),
        "none" => ("已停用", Tone::Muted),
        _ => ("—", Tone::Muted),
    };
    let (published, publish_tone) = match person.publish_status.as_str() {
        "PUBLISHED" => ("已发布", Tone::Green),
        "PUBLISHING" => ("启用中", Tone::Warning),
        "DRAFT" => ("草稿", Tone::Warning),
        "TESTING" => ("测试中", Tone::Warning),
        status if status.contains("FAIL") => ("发布异常", Tone::Red),
        _ => ("未知", Tone::Muted),
    };
    let mut status =
        row![strong(runtime, 12, runtime_tone), tag(format!("配置：{published}"), publish_tone)]
            .align_y(Alignment::Center)
            .spacing(6);
    if !person.owned_by_caller {
        status = status.push(tag("组织内".into(), Tone::Violet));
    }
    let tags = row![
        tag("私有员工".into(), Tone::Violet),
        tag(format!("技能 {}", person.capability_summary.skill_count), Tone::Muted),
        tag(format!("系统工具 {}", person.capability_summary.mcp_count), Tone::Muted),
    ]
    .spacing(6);
    let picture: Item<'_> = match portraits::avatar(&person.name, false) {
        Some(handle) => image(handle).width(136).height(136).into(),
        None => container(icon(Icon::Robot, 44, Tone::Violet)).center(136).into(),
    };
    let portrait = container(picture)
        .center_x(Length::Fill)
        .style(|theme| tint_style(theme, Tone::Violet, 8.0));
    let department = |id: &str| {
        data.members.get(id).and_then(|member| member.department.as_deref()).unwrap_or("")
    };
    let attribution = container(
        row![
            member(
                "使用人",
                person.owner_name(&data.members, actor),
                department(&person.owner_user_id)
            ),
            member(
                "创建人",
                person.creator_name(&data.members, actor),
                department(&person.created_by)
            ),
        ]
        .spacing(8),
    )
    .padding(10)
    .width(Length::Fill)
    .style(|theme| tint_style(theme, Tone::Violet, 8.0));
    let note = if person.owned_by_caller {
        "仅你可使用 · 通过已连接的办公软件交互"
    } else {
        "仅使用人可使用 · 不参与团队任务"
    };
    let executions = container(
        column![
            strong(person.usage_count.to_string(), 15, Tone::Ink),
            label("今日执行", 11, Tone::Muted)
        ]
        .align_x(Alignment::Center)
        .spacing(2),
    )
    .center_x(Length::Fill)
    .padding(8)
    .style(|theme| tint_style(theme, Tone::Muted, 8.0));
    card(
        column![
            portrait,
            strong(person.name.as_str(), 17, Tone::Ink),
            label("私有执行员工", 13, Tone::Muted),
            status.wrap().align_x(Alignment::Center),
            tags.wrap().align_x(Alignment::Center),
            attribution,
            label(note, 11, Tone::Violet),
            executions,
        ]
        .align_x(Alignment::Center)
        .spacing(8)
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

fn tag(value: String, tone: Tone) -> Item<'static> {
    chip(label(value, 11, tone), tone)
}
