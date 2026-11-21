use std::collections::BTreeSet;

use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, settings_pick_list_menu_style,
    settings_pick_list_style, settings_text_input_style,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Column, Row, Space, button, column, container, image, pick_list, row, scrollable, text_input,
};
use iced::{Alignment, Length, Theme, border};

use crate::app::{Message, message::AccountMessage};

use super::portraits;
use super::shared::{Kind, ListEvent, Runtime, Shared, message};
use super::shared_api::{Directory, SharedEmployee};
use super::view::{
    Item, Palette, Tone, action, card, chip, divider, format_time, icon, label, panel_style,
    strong, tint_style,
};

pub(super) fn view(state: &Shared, signed_in: bool, width: f32) -> Item<'_> {
    let mut page = column![
        row![
            column![
                strong("共享员工", 24, Tone::Ink),
                label("任务主管接单派工，执行员工按流程执行", 14, Tone::Muted)
            ]
            .spacing(8)
            .width(Length::Fill),
            action(
                if state.loading { "刷新中" } else { "刷新" },
                None,
                (signed_in && !state.loading).then(|| message(ListEvent::Refresh)),
                false
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
    ]
    .spacing(16)
    .width(Length::Fill);

    if !signed_in {
        return page
            .push(card(
                column![
                    label("登录识矩后查看共享员工", 14, Tone::Muted),
                    action(
                        "登录识矩",
                        None,
                        Some(Message::Account(AccountMessage::StartBrowserLogin)),
                        true
                    )
                ]
                .spacing(12)
                .padding(16),
            ))
            .into();
    }
    if let Some(error) = &state.error {
        page = page.push(card(container(label(error.as_str(), 14, Tone::Red)).padding(16)));
    }
    let Some(directory) = &state.directory else {
        if state.loading {
            page =
                page.push(card(container(label("正在加载共享员工…", 14, Tone::Muted)).padding(16)));
        }
        return page.into();
    };

    let query = state.query.trim().to_lowercase();
    let path = state.department.as_deref();
    let scoped = directory.employees.iter().filter(|employee| {
        path.is_none_or(|prefix| department_matches(prefix, employee.department()))
    });
    let archived = scoped.clone().filter(|employee| employee.archived()).count();
    let mut all = Vec::new();
    let mut managers = Vec::new();
    let mut workers = Vec::new();
    for employee in scoped.filter(|e| e.archived() == state.archived && e.matches(&query)) {
        if state.runtime.key().is_some_and(|key| employee.runtime_status.as_deref() != Some(key)) {
            continue;
        }
        all.push(employee);
        if employee.manager() {
            managers.push(employee);
        } else {
            workers.push(employee);
        }
    }
    page = page
        .push(metrics([all.len(), managers.len(), workers.len()], directory.templates, width))
        .push(filters(
            state,
            directory,
            [all.len(), managers.len(), workers.len(), archived],
            width,
        ))
        .push(Space::new().height(18));

    let selected = match state.kind {
        Kind::All => &all,
        Kind::Manager => &managers,
        Kind::Worker => &workers,
    };
    if selected.is_empty() {
        return page
            .push(card(
                container(label("当前筛选条件下没有共享员工", 14, Tone::Muted))
                    .padding(24)
                    .center_x(Length::Fill),
            ))
            .into();
    }
    if state.table {
        let pagination = Pagination::new(selected.len(), state.page, state.table_size);
        return page
            .push(table(pagination.slice(selected), width))
            .push(pager(pagination, false, true))
            .into();
    }
    if state.kind == Kind::All {
        for (title, description, employees, is_manager) in [
            ("任务主管", "接收任务、安排员工、验收成果", &managers, true),
            ("执行员工", "由任务主管按工作流程安排", &workers, false),
        ] {
            if !employees.is_empty() {
                let index = if is_manager { state.manager_page } else { state.page };
                let pagination = Pagination::new(employees.len(), index, state.card_size);
                page = page
                    .push(
                        row![
                            strong(title, 16, Tone::Ink),
                            Space::new().width(Length::Fill),
                            label(description, 13, Tone::Muted)
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .push(cards(pagination.slice(employees), directory, width))
                    .push(pager(pagination, is_manager, false));
            }
        }
    } else {
        let pagination = Pagination::new(selected.len(), state.page, state.card_size);
        page = page
            .push(cards(pagination.slice(selected), directory, width))
            .push(pager(pagination, false, false));
    }
    page.into()
}

fn department_matches(prefix: &str, path: Option<&str>) -> bool {
    let Some(path) = path else { return false };
    let prefix = prefix.trim_end_matches('/');
    let path = path.trim_end_matches('/');
    let prefix = prefix.trim_start_matches('/');
    let path = path.trim_start_matches('/');
    path == prefix
        || (!prefix.is_empty()
            && path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/')))
}

fn metrics(counts: [usize; 3], templates: Option<usize>, width: f32) -> Item<'static> {
    let [all, managers, workers] = counts.map(|count| count.to_string());
    let tiles = [
        ("全部", all, Icon::Robot, Tone::Accent),
        ("任务主管", managers, Icon::ChatTextFill, Tone::Blue),
        ("执行员工", workers, Icon::Gear, Tone::Violet),
        ("岗位模板", value(templates), Icon::Box, Tone::Warning),
    ];
    let columns = if width >= 780.0 {
        4
    } else if width >= 350.0 {
        2
    } else {
        1
    };
    let mut grid = Column::new().spacing(14);
    for line in tiles.chunks(columns) {
        let mut items = Row::new().spacing(14);
        for (title, count, glyph, tone) in line {
            let tone = *tone;
            let badge = container(icon(*glyph, 18, tone))
                .center(38)
                .style(move |theme| tint_style(theme, tone, 8.0));
            items = items.push(card(
                row![
                    badge,
                    column![label(*title, 13, Tone::Muted), strong(count.clone(), 23, Tone::Ink),]
                        .spacing(6)
                ]
                .spacing(12)
                .padding(16)
                .height(84),
            ));
        }
        grid = grid.push(items);
    }
    grid.into()
}

#[derive(Clone, PartialEq, Eq)]
struct Department(Option<String>);

impl std::fmt::Display for Department {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(path) if path.starts_with('/') => f.write_str(path),
            Some(path) => write!(f, "/{path}"),
            None => f.write_str("全部可查看部门"),
        }
    }
}

fn filters<'a>(
    state: &'a Shared,
    directory: &Directory,
    counts: [usize; 4],
    width: f32,
) -> Item<'a> {
    let [all, managers, workers, archived] = counts;
    let departments: BTreeSet<_> =
        directory.employees.iter().filter_map(SharedEmployee::department).collect();
    let departments = std::iter::once(Department(None))
        .chain(departments.into_iter().map(|path| Department(Some(path.to_owned()))))
        .collect::<Vec<_>>();

    let department = pick_list(departments, Some(Department(state.department.clone())), |option| {
        message(ListEvent::Department(option.0))
    });
    let runtime = pick_list(
        [Runtime::All, Runtime::OnDuty, Runtime::Abnormal],
        Some(state.runtime),
        |runtime| message(ListEvent::Runtime(runtime)),
    );
    let search = text_input("搜索员工名称、岗位或能力", &state.query)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .on_input(|query| message(ListEvent::Search(query)))
        .on_submit(message(ListEvent::Refresh));
    let segments = group([
        (format!("全部 {all}"), state.kind == Kind::All, ListEvent::Kind(Kind::All)),
        (
            format!("任务主管 {managers}"),
            state.kind == Kind::Manager,
            ListEvent::Kind(Kind::Manager),
        ),
        (format!("执行员工 {workers}"), state.kind == Kind::Worker, ListEvent::Kind(Kind::Worker)),
    ]);
    let archive = group([
        ("在用".into(), !state.archived, ListEvent::Archived(false)),
        (format!("已归档 {archived}"), state.archived, ListEvent::Archived(true)),
    ]);
    let views = group([
        ("卡片".into(), !state.table, ListEvent::Table(false)),
        ("列表".into(), state.table, ListEvent::Table(true)),
    ]);
    let query = action(
        "查询",
        Some(Icon::Search),
        (!state.loading).then(|| message(ListEvent::Refresh)),
        true,
    );
    let controls = row![
        field("关键字", search.width((width * 0.22).clamp(200.0, 320.0))),
        segments,
        archive,
        field(
            "筛选部门",
            department
                .width(165)
                .padding(SETTINGS_CONTROL_PADDING)
                .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_pick_list_style)
                .menu_style(settings_pick_list_menu_style)
        ),
        field(
            "运行状态",
            runtime
                .width(110)
                .padding(SETTINGS_CONTROL_PADDING)
                .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_pick_list_style)
                .menu_style(settings_pick_list_menu_style)
        ),
        views,
        query,
    ]
    .spacing(12)
    .align_y(Alignment::End)
    .wrap();
    container(controls).width(Length::Fill).padding(16).style(header_style).into()
}

fn field<'a>(title: &'static str, input: impl Into<Item<'a>>) -> Item<'a> {
    column![label(title, 12, Tone::Muted), input.into()].spacing(7).into()
}

fn group<'a>(options: impl IntoIterator<Item = (String, bool, ListEvent)>) -> Item<'a> {
    let mut buttons = Row::new().spacing(4);
    for (title, selected, event) in options {
        buttons = buttons.push(
            button(strong(title, 13, if selected { Tone::Accent } else { Tone::Muted }))
                .padding([8, 9])
                .on_press(message(event))
                .style(move |theme, status| {
                    let active = selected || status != button::Status::Active;
                    button::Style {
                        background: active
                            .then(|| Tone::Accent.color(theme).scale_alpha(0.10).into()),
                        border: border::rounded(8),
                        ..Default::default()
                    }
                }),
        );
    }
    container(buttons).padding(4).style(header_style).into()
}

fn header_style(theme: &Theme) -> container::Style {
    panel_style(theme).background(Palette::new(theme).header)
}

fn cards<'a>(employees: &[&'a SharedEmployee], directory: &Directory, width: f32) -> Item<'a> {
    let columns = if width >= 890.0 {
        3
    } else if width >= 550.0 {
        2
    } else {
        1
    };
    let mut grid = Column::new().spacing(16);
    for employees in employees.chunks(columns) {
        let mut line = Row::new().spacing(16);
        for employee in employees {
            line = line.push(employee_card(employee, directory));
        }
        for _ in employees.len()..columns {
            line = line.push(Space::new().width(Length::Fill));
        }
        grid = grid.push(line);
    }
    grid.into()
}

fn employee_card<'a>(employee: &'a SharedEmployee, directory: &Directory) -> Item<'a> {
    let role = employee.role();
    let portrait: Item<'_> = match portraits::shared_avatar(
        &employee.name,
        employee.manager(),
        role.unwrap_or_default(),
    ) {
        Some(handle) => image(handle).width(136).height(136).into(),
        None => container(icon(Icon::Person, 42, Tone::Muted))
            .center(136)
            .style(|theme| tint_style(theme, Tone::Muted, 68.0))
            .into(),
    };
    let team = if employee.manager() {
        format!("协作员工 {} 名", value(employee.worker_count))
    } else {
        format!("任务主管 {}", employee.manager_name.as_deref().unwrap_or("—"))
    };
    let counts = directory.counts.as_ref();
    let local = counts.and_then(|counts| counts.get(&employee.employee_id));
    let today = employee
        .today
        .or(employee.today_executed)
        .or_else(|| counts.map(|_| local.map_or(0, |c| c.today)));
    let active = counts.map(|_| local.map_or(0, |c| c.active));
    let published = card_publish(employee);
    let mut content = column![portrait, strong(employee.name.as_str(), 18, Tone::Ink)]
        .spacing(9)
        .align_x(Alignment::Center)
        .width(Length::Fill);
    if let Some(role) = role {
        content = content.push(label(role, 13, Tone::Muted));
    }
    content = content
        .push(
            row![runtime_badges(employee), badge(published.0, published.1)]
                .spacing(6)
                .align_y(Alignment::Center)
                .wrap()
                .align_x(iced::alignment::Horizontal::Center),
        )
        .push(kind_badge(employee));
    let mut tags = Row::new().spacing(5);
    let mut count = 0;
    for (index, tag) in employee.capability_tags().enumerate() {
        if index < 2 {
            tags = tags.push(badge(tag, Tone::Muted));
        }
        count += 1;
    }
    if count > 2 {
        tags = tags.push(badge(format!("+{}", count - 2), Tone::Muted));
    }
    if count > 0 {
        content = content.push(tags.wrap().align_x(iced::alignment::Horizontal::Center));
    }
    content = content.push(label(team, 13, Tone::Muted));
    if let Some(template) = employee.template_name.as_deref().filter(|name| !name.trim().is_empty())
    {
        content = content.push(label(format!("岗位模板 {template}"), 12, Tone::Muted));
    }
    card(
        content
            .push(
                container(row![card_metric(today, "今日执行"), card_metric(active, "进行中任务")])
                    .padding([10, 6])
                    .width(Length::Fill)
                    .style(|theme| {
                        container::Style::default()
                            .background(Palette::new(theme).header)
                            .border(border::rounded(8))
                    }),
            )
            .padding(16),
    )
}

fn card_metric(number: Option<usize>, title: &'static str) -> Item<'static> {
    column![strong(value(number), 16, Tone::Ink), label(title, 11, Tone::Muted)]
        .spacing(6)
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .into()
}

fn value(number: Option<usize>) -> String {
    number.map_or_else(|| "—".into(), |number| number.to_string())
}

fn card_publish(employee: &SharedEmployee) -> (String, Tone) {
    let (label, tone) = if employee.archived() {
        ("已归档", Tone::Muted)
    } else if employee
        .status
        .as_deref()
        .is_some_and(|status| status.eq_ignore_ascii_case("DISABLED"))
    {
        ("已停用", Tone::Muted)
    } else {
        match employee.runtime_status.as_deref() {
            Some("enabling") => ("启用中", Tone::Accent),
            Some("disabling") => ("停用中", Tone::Accent),
            _ => publish(employee),
        }
    };
    let label = if employee.archived() { label.to_owned() } else { format!("配置 {label}") };
    (label, tone)
}

fn publish(employee: &SharedEmployee) -> (&'static str, Tone) {
    match employee.publish_status.as_deref() {
        Some("PUBLISHED") => ("已发布", Tone::Green),
        Some("PUBLISHING") => ("发布中", Tone::Accent),
        Some("TESTING") => ("测试中", Tone::Accent),
        Some("PUBLISH_FAILED") => ("异常", Tone::Red),
        _ => ("草稿", Tone::Accent),
    }
}

fn kind_badge(employee: &SharedEmployee) -> Item<'static> {
    if employee.manager() {
        badge("任务主管", Tone::Blue)
    } else {
        badge("执行员工", Tone::Violet)
    }
}

fn badge<'a>(title: impl iced::widget::text::IntoFragment<'a>, tone: Tone) -> Item<'a> {
    chip(strong(title, 12, tone), tone)
}

fn runtime_badges(employee: &SharedEmployee) -> Item<'_> {
    let (fallback, tone) = match employee.runtime_status.as_deref() {
        Some("on_duty") => ("在岗", Tone::Green),
        Some("abnormal") => ("异常", Tone::Red),
        Some("enabling") => ("启用中", Tone::Accent),
        Some("disabling") => ("停用中", Tone::Accent),
        Some("none") if employee.status.as_deref() == Some("ACTIVE") => ("容器未运行", Tone::Muted),
        Some("none") => ("已停用", Tone::Muted),
        _ => ("—", Tone::Muted),
    };
    let title = if matches!(fallback, "—" | "容器未运行") {
        fallback
    } else {
        employee
            .runtime_status_label
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(fallback)
    };
    let status = badge(title, tone);
    let mut badges = row![status].spacing(4);
    if employee.occupancy.as_ref().is_some_and(|occupancy| occupancy.busy) {
        badges = badges.push(badge("占用中", Tone::Warning));
    }
    let mut lines = column![badges.wrap()].spacing(3);
    if matches!(tone, Tone::Red) {
        let detail = [employee.runtime_cause.as_deref(), employee.runtime_next_action.as_deref()]
            .into_iter()
            .flatten()
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        if !detail.is_empty() {
            lines = lines.push(label(detail, 11, Tone::Muted));
        }
    }
    lines.into()
}

fn table<'a>(employees: &[&'a SharedEmployee], width: f32) -> Item<'a> {
    let header = ["名称", "类型", "部门", "发布态", "运行状态", "更新时间"]
        .map(|title| strong(title, 12, Tone::Muted).into());
    let mut body = column![container(table_row(header)).style(header_style)];
    for employee in employees {
        let (published, tone) = publish(employee);
        body = body.push(divider()).push(table_row([
            strong(employee.name.as_str(), 14, Tone::Ink).into(),
            kind_badge(employee),
            label(
                employee.department().map_or_else(
                    || "全公司".into(),
                    |path| Department(Some(path.to_owned())).to_string(),
                ),
                14,
                Tone::Ink,
            )
            .into(),
            badge(published, tone),
            runtime_badges(employee),
            label(format_time(employee.updated_at.as_deref()), 14, Tone::Ink).into(),
        ]));
    }
    let sheet = container(body).width(width.max(720.0)).style(panel_style);
    scrollable(sheet)
        .width(Length::Fill)
        .direction(Direction::Horizontal(Scrollbar::new().width(4).scroller_width(4).margin(3)))
        .into()
}

fn table_row(items: [Item<'_>; 6]) -> Item<'_> {
    let mut row = Row::new().spacing(10).padding([15, 10]).align_y(Alignment::Center);
    for (item, portion) in items.into_iter().zip([5, 3, 3, 3, 3, 6]) {
        row = row.push(container(item).width(Length::FillPortion(portion)));
    }
    row.into()
}

struct Pagination {
    total: usize,
    index: usize,
    size: usize,
}

impl Pagination {
    fn new(total: usize, index: usize, size: usize) -> Self {
        Self { total, index: index.min(total.div_ceil(size).saturating_sub(1)), size }
    }

    fn slice<'a, T>(&self, entries: &'a [T]) -> &'a [T] {
        &entries[self.index * self.size..((self.index + 1) * self.size).min(self.total)]
    }
}

fn pager(pagination: Pagination, managers: bool, table: bool) -> Item<'static> {
    let Pagination { total, index, size } = pagination;
    let status =
        format!("共 {total} 条，当前显示 {}-{}", index * size + 1, ((index + 1) * size).min(total));
    let sizes = if table { vec![5, 10] } else { vec![3, 6, 9] };
    let page = |index| message(ListEvent::Page { managers, index });
    column![
        divider(),
        row![
            label(status, 13, Tone::Muted),
            Space::new().width(Length::Fill),
            row![
                label("每页", 12, Tone::Muted),
                pick_list(sizes, Some(size), |size| message(ListEvent::PageSize(size)))
                    .padding(SETTINGS_CONTROL_PADDING)
                    .text_size(SETTINGS_CONTROL_TEXT_SIZE)
                    .style(settings_pick_list_style)
                    .menu_style(settings_pick_list_menu_style),
                action("上一页", None, index.checked_sub(1).map(page), false),
                chip(label((index + 1).to_string(), 13, Tone::Ink), Tone::Muted),
                action(
                    "下一页",
                    None,
                    ((index + 1) * size < total).then(|| page(index + 1)),
                    false
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .wrap()
    ]
    .spacing(12)
    .into()
}
