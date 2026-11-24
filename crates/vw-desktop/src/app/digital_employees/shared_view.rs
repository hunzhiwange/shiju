use std::collections::BTreeSet;

use desktop_assets::assets::Icon;
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::{
    settings_close_button, settings_error_banner, settings_modal_card,
    settings_modal_overlay_with_escape, settings_select, settings_text_input,
};
use iced::widget::tooltip::Position;
use iced::widget::{Column, Row, Space, column, container, image, row, tooltip};
use iced::{Alignment, Length};

use crate::app::{Message, message::AccountMessage};

use super::{departments, portraits};
use super::shared::{Kind, ListEvent, Runtime, Shared, message};
use super::shared_api::{Access, Directory, Operation, SharedEmployee};
use super::view::{
    Item, Tone, action, divider, format_time, icon, label, page_notice, panel_style, segment,
    segmented, status_label, strong, table_header_style, tint_style,
};

pub(super) fn view<'a>(i18n: desktop_i18n::I18n,
    state: &'a Shared,
    departments: Option<&departments::Directory>,
    signed_in: bool,
    width: f32,
) -> Item<'a> {
    let mut heading = row![strong(i18n.__("共享员工"), 24, Tone::Ink), Space::new().width(Length::Fill)]
        .spacing(10)
        .align_y(Alignment::Center);
    if signed_in {
        heading = heading.push(action(
            if state.loading { i18n.__("刷新中…") } else { i18n.__("刷新") },
            Some(Icon::ArrowRepeat),
            (!state.loading).then(|| message(ListEvent::Refresh)),
            false,
        ));
    }
    let mut page = column![heading].spacing(20).width(Length::Fill);
    if !signed_in {
        return page
            .push(page_notice(
                column![
                    label(i18n.__("登录识矩后查看共享员工"), 15, Tone::Ink),
                    action(
                        i18n.__("登录识矩"),
                        None,
                        Some(Message::Account(AccountMessage::StartBrowserLogin)),
                        true
                    )
                ]
                .spacing(12)
                .align_x(Alignment::Center),
            ))
            .into();
    }
    if let Some(error) = &state.error {
        page = page.push(settings_error_banner(error.render(i18n)));
    }
    let Some(directory) = &state.directory else {
        if state.loading {
            page = page.push(page_notice(label(i18n.__("正在加载共享员工…"), 14, Tone::Muted)));
        }
        return page.into();
    };

    let query = state.query.trim().to_lowercase();
    let path = state.department.as_deref();
    let scoped = directory.employees.iter().filter(|employee| {
        path.is_none_or(|prefix| {
            employee.department().is_some_and(|path| departments::contains(prefix, path))
        })
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
    page = page.push(filters(i18n,
        state,
        directory,
        departments,
        [all.len(), managers.len(), workers.len(), archived],
        width,
    ));

    let selected = match state.kind {
        Kind::All => &all,
        Kind::Manager => &managers,
        Kind::Worker => &workers,
    };
    if selected.is_empty() {
        return page.push(page_notice(label(i18n.__("当前筛选条件下没有共享员工"), 14, Tone::Muted))).into();
    }
    if state.table {
        let pagination = Pagination::new(selected.len(), state.page, state.table_size);
        return page
            .push(table(i18n, pagination.slice(selected), &directory.access, departments, width))
            .push(pager(i18n, pagination, false, true))
            .into();
    }
    if state.kind == Kind::All {
        for (title, employees, is_manager) in
            [(i18n.__("任务主管"), &managers, true), (i18n.__("执行员工"), &workers, false)]
        {
            if !employees.is_empty() {
                let index = if is_manager { state.manager_page } else { state.page };
                let pagination = Pagination::new(employees.len(), index, state.card_size);
                page = page.push(
                    column![
                        strong(title, 16, Tone::Ink),
                        cards(i18n, pagination.slice(employees), directory, width),
                        pager(i18n, pagination, is_manager, false),
                    ]
                    .spacing(12),
                );
            }
        }
    } else {
        let pagination = Pagination::new(selected.len(), state.page, state.card_size);
        page = page
            .push(cards(i18n, pagination.slice(selected), directory, width))
            .push(pager(i18n, pagination, false, false));
    }
    page.into()
}

fn filters<'a>(i18n: desktop_i18n::I18n,
    state: &'a Shared,
    directory: &Directory,
    department_directory: Option<&departments::Directory>,
    counts: [usize; 4],
    width: f32,
) -> Item<'a> {
    let [all, managers, workers, archived] = counts;
    let departments: BTreeSet<_> =
        directory.employees.iter().filter_map(SharedEmployee::department).collect();
    let choice = |path: Option<&str>| departments::Choice {
        value: path.map(str::to_owned),
        label: path
            .map_or(i18n.__("全部部门"), |path| department_label(i18n, department_directory, path))
            .to_owned(),
    };
    let options = std::iter::once(choice(None))
        .chain(departments.into_iter().map(|path| choice(Some(path))));
    let department = settings_select(options, Some(choice(state.department.as_deref())), |option| {
        message(ListEvent::Department(option.value))
    })
    .width(248);
    let runtime = settings_select(
        [Runtime::All, Runtime::OnDuty, Runtime::Abnormal].map(|value| super::LocalizedOption::new(value, i18n)),
        Some(super::LocalizedOption::new(state.runtime, i18n)),
        |runtime| message(ListEvent::Runtime(runtime.value)),
    )
    .width(128);
    let search = settings_text_input(i18n.__("搜索名称、岗位或能力"), &state.query)
        .width(if width >= 660.0 {
            Length::Fixed((width - 486.0).min(340.0))
        } else {
            Length::Fill
        })
        .on_input(|query| message(ListEvent::Search(query)))
        .on_submit(message(ListEvent::Refresh));
    let search_and_filters = row![
        search,
        department,
        runtime,
        label(i18n.format("岗位模板 %s", &[format!("{}", value(directory.templates)).into()]).expect("valid workbench template"), 13, Tone::Muted),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap();

    let segments = group([
        (i18n.format("全部 %s", &[format!("{}", all).into()]).expect("valid workbench template"), state.kind == Kind::All, ListEvent::Kind(Kind::All)),
        (
            i18n.format("任务主管 %s", &[format!("{}", managers).into()]).expect("valid workbench template"),
            state.kind == Kind::Manager,
            ListEvent::Kind(Kind::Manager),
        ),
        (i18n.format("执行员工 %s", &[format!("{}", workers).into()]).expect("valid workbench template"), state.kind == Kind::Worker, ListEvent::Kind(Kind::Worker)),
    ]);
    let archive = group([
        (i18n.__("在用").into(), !state.archived, ListEvent::Archived(false)),
        (i18n.format("已归档 %s", &[format!("{}", archived).into()]).expect("valid workbench template"), state.archived, ListEvent::Archived(true)),
    ]);
    let views = group([
        (i18n.__("卡片").into(), !state.table, ListEvent::Table(false)),
        (i18n.__("列表").into(), state.table, ListEvent::Table(true)),
    ]);
    let mut modes = row![segments].spacing(8).align_y(Alignment::Center);
    if width >= 700.0 {
        modes = modes.push(Space::new().width(Length::Fill));
    }
    let modes = modes.push(archive).push(views);
    let modes: Item<'_> = if width >= 700.0 { modes.into() } else { modes.wrap().into() };
    column![modes, search_and_filters].spacing(10).width(Length::Fill).into()
}

fn group(options: impl IntoIterator<Item = (String, bool, ListEvent)>) -> Item<'static> {
    segmented(
        Row::with_children(
            options
                .into_iter()
                .map(|(title, selected, event)| segment(title, selected, message(event))),
        )
        .spacing(2)
        .wrap()
        .into(),
    )
}

fn cards<'a>(i18n: desktop_i18n::I18n, employees: &[&'a SharedEmployee], directory: &Directory, width: f32) -> Item<'a> {
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
            line = line.push(employee_card(i18n, employee, directory));
        }
        for _ in employees.len()..columns {
            line = line.push(Space::new().width(Length::Fill));
        }
        grid = grid.push(line);
    }
    grid.into()
}

fn employee_card<'a>(i18n: desktop_i18n::I18n, employee: &'a SharedEmployee, directory: &Directory) -> Item<'a> {
    let role = employee.role();
    let portrait: Item<'_> = match portraits::shared_avatar(
        &employee.name,
        employee.manager(),
        role.unwrap_or_default(),
    ) {
        Some(handle) => image(handle).width(44).height(44).into(),
        None => container(icon(Icon::Person, 20, Tone::Accent))
            .center(44)
            .style(|theme| tint_style(theme, Tone::Accent, 22.0))
            .into(),
    };
    let identity = row![
        portrait,
        column![
            strong(employee.name.as_str(), 17, Tone::Ink),
            metadata(role.unwrap_or_else(|| kind_title(i18n, employee)).to_owned()),
        ]
        .spacing(3)
        .width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let team = if employee.manager() {
        i18n.format("协作员工 %s 名", &[format!("{}", value(employee.worker_count)).into()]).expect("valid workbench template")
    } else {
        i18n.format("任务主管 %s", &[format!("{}", employee.manager_name.as_deref().unwrap_or("—")).into()]).expect("valid workbench template")
    };
    let counts = directory.counts.as_ref();
    let local = counts.and_then(|counts| counts.get(&employee.employee_id));
    let today = employee
        .today
        .or(employee.today_executed)
        .or_else(|| counts.map(|_| local.map_or(0, |c| c.today)));
    let active = counts.map(|_| local.map_or(0, |c| c.active));

    let mut details = column![metadata(team)].spacing(5);
    if let Some(template) = employee.template_name.as_deref().filter(|name| !name.trim().is_empty())
    {
        details = details.push(metadata(i18n.format("岗位模板 · %s", &[format!("{}", template).into()]).expect("valid workbench template")));
    }
    let capabilities = employee.capability_tags().collect::<Vec<_>>();
    if !capabilities.is_empty() {
        details = details.push(metadata(capabilities.join(" · ")));
    }

    let mut content = column![
        identity,
        runtime_status(i18n, employee, true),
        details,
        divider(),
        row![card_metric(today, i18n.__("今日执行")), card_metric(active, i18n.__("进行中任务"))]
            .spacing(16)
            .align_y(Alignment::Center)
            .wrap(),
    ]
    .spacing(10);
    if let Some(actions) = employee_actions(i18n, employee, &directory.access) {
        content = content.push(actions);
    }
    container(content).padding(16).width(Length::Fill).style(panel_style).into()
}

fn metadata(value: String) -> Item<'static> {
    tooltip(
        container(SingleLineLabel::new(value.clone(), 12.0))
            .style(|theme| container::Style::default().color(Tone::Muted.color(theme))),
        container(label(value, 12, Tone::Ink)).max_width(340).padding([6, 9]).style(panel_style),
        Position::Top,
    )
    .gap(5)
    .into()
}

fn employee_actions(i18n: desktop_i18n::I18n, employee: &SharedEmployee, access: &Access) -> Option<Item<'static>> {
    let id = &employee.employee_id;
    let can_dispatch = access.can_dispatch(employee);
    let can_configure = access.can_configure(employee);
    let options: Vec<_> =
        Operation::ALL.into_iter().filter(|op| access.can_manage(employee, *op)).collect();
    if !employee.can_chat() && !can_dispatch && !can_configure && options.is_empty() {
        return None;
    }
    let mut actions = Row::new().spacing(6).align_y(Alignment::Center);
    if employee.can_chat() {
        actions = actions.push(super::view::chat_action(i18n, Some(message(ListEvent::Chat(id.clone())))));
    }
    if can_dispatch {
        actions = actions.push(action(
            i18n.__("派任务"),
            None,
            Some(message(ListEvent::Dispatch(id.clone()))),
            true,
        ));
    }
    if can_configure {
        actions = actions.push(action(
            i18n.__("配置"),
            None,
            Some(message(ListEvent::Configure(id.clone()))),
            false,
        ));
    }
    if !options.is_empty() {
        let id = id.clone();
        actions = actions.push(
            settings_select(options.into_iter().map(|value| super::LocalizedOption::new(value, i18n)), None::<super::LocalizedOption<Operation>>, move |operation| {
                message(ListEvent::OpenOperation(id.clone(), operation.value))
            })
            .placeholder(i18n.__("更多操作"))
            .width(176),
        );
    }
    Some(actions.wrap().into())
}

pub(super) fn overlay<'a>(i18n: desktop_i18n::I18n, base: Item<'a>, state: &'a Shared, width: f32) -> Item<'a> {
    let Some(dialog) = &state.confirmation else { return base };
    let close = message(ListEvent::CloseOperation);
    let operation = dialog.operation;
    let description = match operation {
        Operation::SaveTemplate => i18n.__("将员工当前已发布的配置保存为企业岗位模板。"),
        Operation::Enable => i18n.__("恢复员工为在用状态，是否可接任务以云端状态为准。"),
        Operation::Disable => i18n.__("停用后员工无法接收新任务，配置和历史将保留。"),
        Operation::Archive => i18n.__("归档后员工无法接收新任务，可在已归档列表中恢复。"),
        Operation::Delete => i18n.__("删除不可撤销。若有未完成任务或流程引用，云端会拒绝删除。"),
    };
    let heading = row![
        strong(format!("{} · {}", i18n.__(&operation.to_string()), dialog.employee_name), 17, Tone::Ink),
        Space::new().width(Length::Fill),
        settings_close_button(close.clone()),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let mut content = column![heading, label(description, 14, Tone::Muted)].spacing(14);
    if operation == Operation::SaveTemplate {
        let mut name = settings_text_input(i18n.__("模板名称"), &dialog.template_name);
        if !dialog.submitting && dialog.error.is_none() {
            name = name.on_input(|name| message(ListEvent::TemplateName(name)));
        }
        content = content.push(column![label(i18n.__("模板名称"), 12, Tone::Muted), name].spacing(7));
    }
    if let Some(error) = &dialog.error {
        content = content.push(label(error.render(i18n), 13, Tone::Red));
    }
    let ready = !dialog.submitting
        && dialog.error.is_none()
        && (operation != Operation::SaveTemplate || !dialog.template_name.trim().is_empty());
    let confirm = match operation {
        _ if dialog.submitting => i18n.__("提交中…"),
        Operation::SaveTemplate => i18n.__("保存模板"),
        Operation::Enable => i18n.__("确认启用"),
        Operation::Disable => i18n.__("确认停用"),
        Operation::Archive => i18n.__("确认归档"),
        Operation::Delete => i18n.__("确认删除"),
    };
    let modal = settings_modal_card(
        content.push(
            row![
                Space::new().width(Length::Fill),
                action(i18n.__("取消"), None, (!dialog.submitting).then(|| close.clone()), false),
                action(confirm, None, ready.then(|| message(ListEvent::SubmitOperation)), true),
            ]
            .spacing(8),
        ),
    )
    .width((width - 32.0).clamp(230.0, 430.0));
    settings_modal_overlay_with_escape(base, close, modal)
}

fn card_metric(number: Option<usize>, title: &'static str) -> Item<'static> {
    row![label(title, 12, Tone::Muted), strong(value(number), 13, Tone::Ink)]
        .spacing(5)
        .align_y(Alignment::Center)
        .into()
}

fn value(number: Option<usize>) -> String {
    number.map_or_else(|| "—".into(), |number| number.to_string())
}

fn publish(i18n: desktop_i18n::I18n, employee: &SharedEmployee) -> (&'static str, Tone) {
    match employee.publish_status.as_deref() {
        Some("PUBLISHED") => (i18n.__("已发布"), Tone::Green),
        Some("PUBLISHING") => (i18n.__("发布中"), Tone::Accent),
        Some("TESTING") => (i18n.__("测试中"), Tone::Accent),
        Some("PUBLISH_FAILED") => (i18n.__("发布失败"), Tone::Red),
        _ => (i18n.__("草稿"), Tone::Muted),
    }
}

pub(super) fn kind_title(i18n: desktop_i18n::I18n, employee: &SharedEmployee) -> &'static str {
    if employee.manager() { i18n.__("任务主管") } else { i18n.__("执行员工") }
}

pub(super) fn runtime_badge(i18n: desktop_i18n::I18n, employee: &SharedEmployee) -> (&str, Tone) {
    let name = employee.runtime_status.as_deref();
    let (fallback, tone) = match name {
        Some("on_duty") => (i18n.__("在岗"), Tone::Green),
        Some("abnormal") => (i18n.__("异常"), Tone::Red),
        Some("enabling") => (i18n.__("启用中"), Tone::Accent),
        Some("disabling") => (i18n.__("停用中"), Tone::Accent),
        Some("none") | None if employee.archived() => (i18n.__("已归档"), Tone::Muted),
        Some("none") if employee.status.as_deref() == Some("ACTIVE") => (i18n.__("容器未运行"), Tone::Muted),
        Some("none") => (i18n.__("已停用"), Tone::Muted),
        _ if employee
            .status
            .as_deref()
            .is_some_and(|status| status.eq_ignore_ascii_case("DISABLED")) =>
        {
            (i18n.__("已停用"), Tone::Muted)
        }
        _ => ("—", Tone::Muted),
    };
    let title = if (fallback == "—" || fallback == i18n.__("已归档") || fallback == i18n.__("容器未运行")) || name.is_none() {
        fallback
    } else {
        employee
            .runtime_status_label
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(fallback)
    };
    (title, tone)
}

fn runtime_status(i18n: desktop_i18n::I18n, employee: &SharedEmployee, show_publish: bool) -> Item<'_> {
    let (title, tone) = runtime_badge(i18n, employee);
    let mut statuses = row![status_label(title, tone)].spacing(10).align_y(Alignment::Center);
    if employee.occupancy.as_ref().is_some_and(|occupancy| occupancy.busy) {
        statuses = statuses.push(status_label(i18n.__("占用中"), Tone::Warning));
    }
    if show_publish {
        let (published, tone) = publish(i18n, employee);
        let tone = if matches!(tone, Tone::Red) { Tone::Red } else { Tone::Muted };
        statuses = statuses.push(label(i18n.format("配置 · %s", &[format!("{}", published).into()]).expect("valid workbench template"), 12, tone));
    }
    let mut lines = column![statuses.wrap()].spacing(5);
    if matches!(tone, Tone::Red) {
        let detail = [employee.runtime_cause.as_deref(), employee.runtime_next_action.as_deref()]
            .into_iter()
            .flatten()
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        if !detail.is_empty() {
            lines = lines.push(label(detail, 12, Tone::Muted));
        }
    }
    lines.into()
}

fn department_label<'a>(i18n: desktop_i18n::I18n, directory: Option<&'a departments::Directory>, path: &str) -> &'a str {
    departments::name(directory, path).unwrap_or(i18n.__("部门名称未提供"))
}

fn table<'a>(i18n: desktop_i18n::I18n,
    employees: &[&'a SharedEmployee],
    access: &Access,
    departments: Option<&departments::Directory>,
    width: f32,
) -> Item<'a> {
    let HEADERS: [&str; 6] = [i18n.__("名称"), i18n.__("类型"), i18n.__("部门"), i18n.__("发布状态"), i18n.__("运行状态"), i18n.__("更新时间")];
    let actions =
        employees.iter().map(|employee| employee_actions(i18n, employee, access)).collect::<Vec<_>>();
    let show_actions = actions.iter().any(Option::is_some);
    let compact = width < if show_actions { 1020.0 } else { 720.0 };
    let mut body = Column::new().width(Length::Fill);
    if !compact {
        let header = HEADERS.map(|title| strong(title, 12, Tone::Muted).into());
        body = body.push(
            container(table_row(
                header,
                show_actions.then(|| strong(i18n.__("操作"), 12, Tone::Muted).into()),
            ))
            .style(table_header_style),
        );
    }
    for (index, (employee, actions)) in employees.iter().zip(actions).enumerate() {
        let (published, tone) = publish(i18n, employee);
        let items = [
            strong(employee.name.as_str(), 14, Tone::Ink).into(),
            label(kind_title(i18n, employee), 13, Tone::Muted).into(),
            label(
                employee
                    .department()
                    .map_or(i18n.__("全公司"), |path| department_label(i18n, departments, path))
                    .to_owned(),
                14,
                Tone::Ink,
            )
            .into(),
            status_label(published, tone),
            runtime_status(i18n, employee, false),
            label(format_time(employee.updated_at.as_deref()), 14, Tone::Ink).into(),
        ];
        if !compact || index > 0 {
            body = body.push(divider());
        }
        body = body.push(if compact {
            let [name, kind, department, published, runtime, updated] = items;
            let field = |title, value| column![label(title, 12, Tone::Muted), value].spacing(3);
            let mut entry = column![
                name,
                row![
                    column![field(HEADERS[1], kind), field(HEADERS[2], department)]
                        .spacing(8)
                        .width(Length::Fill),
                    column![field(HEADERS[3], published), field(HEADERS[4], runtime)]
                        .spacing(8)
                        .width(Length::Fill),
                ]
                .spacing(12),
                field(HEADERS[5], updated),
            ]
            .spacing(10);
            if let Some(actions) = actions {
                entry = entry.push(actions);
            }
            container(entry).padding([12, 16]).width(Length::Fill).into()
        } else {
            table_row(items, show_actions.then(|| actions.unwrap_or_else(|| Space::new().into())))
        });
    }
    container(body).width(Length::Fill).style(panel_style).into()
}

fn table_row<'a>(items: [Item<'a>; 6], actions: Option<Item<'a>>) -> Item<'a> {
    let mut row =
        Row::new().width(Length::Fill).spacing(12).padding([12, 16]).align_y(Alignment::Center);
    for (item, portion) in items.into_iter().zip([5, 3, 4, 3, 5, 5]) {
        row = row.push(container(item).width(Length::FillPortion(portion)));
    }
    if let Some(actions) = actions {
        row = row.push(container(actions).width(320));
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

fn pager(i18n: desktop_i18n::I18n, pagination: Pagination, managers: bool, table: bool) -> Item<'static> {
    let Pagination { total, index, size } = pagination;
    let pages = total.div_ceil(size);
    let status = if pages > 1 {
        i18n.format("%s–%s / %s 人", &[format!("{}", index * size + 1).into(), format!("{}", ((index + 1) * size).min(total)).into(), format!("{}", total).into()]).expect("valid workbench template")
    } else {
        i18n.format("共 %s 人", &[format!("{}", total).into()]).expect("valid workbench template")
    };
    let sizes = if table { vec![5, 10] } else { vec![3, 6, 9] };
    let page = |index| message(ListEvent::Page { managers, index });
    let mut controls = row![
        label(status, 12, Tone::Muted),
        row![
            label(i18n.__("每页"), 12, Tone::Muted),
            settings_select(sizes, Some(size), |size| message(ListEvent::PageSize(size)))
                .width(104)
                .height(34.0),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    if pages > 1 {
        controls = controls.push(
            row![
                action(i18n.__("上一页"), None, index.checked_sub(1).map(page), false),
                label(format!("{} / {pages}", index + 1), 12, Tone::Muted),
                action(i18n.__("下一页"), None, (index + 1 < pages).then(|| page(index + 1)), false),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }
    controls.wrap().into()
}
