use std::collections::BTreeSet;

use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    settings_error_banner, settings_select, settings_text_input,
};
use iced::widget::{Column, Row, Space, column, container, row};
use iced::{Alignment, Length};

use super::departments;
use super::view::{
    Item, Tone, action, card, divider, label, page_notice, segment, segmented, status_label, strong,
};
use super::workflows::{Department, List, ListEvent, PAGE_SIZE, Sort, Status, Workflow, message};

pub(super) fn view<'a>(
    state: &'a List,
    directory: Option<&departments::Directory>,
    width: f32,
) -> Item<'a> {
    let items = state.items.as_deref().map_or(&[][..], Vec::as_slice);
    let scoped: Vec<_> = items.iter().filter(|item| state.department.matches(item)).collect();
    let count = |status| scoped.iter().filter(|item| item.status() == status).count();
    let [published, drafts, offline] =
        [Status::Published, Status::Draft, Status::Offline].map(count);
    let active = if scoped.is_empty() {
        state.items.as_ref().map(|_| 0)
    } else {
        scoped.iter().filter_map(|item| item.active_task_count).reduce(u64::saturating_add)
    };
    let page = column![
        row![
            strong("工作流程", 24, Tone::Ink).width(Length::Fill),
            action(
                if state.loading { "刷新中…" } else { "刷新" },
                Some(Icon::ArrowRepeat),
                (!state.loading).then(|| message(ListEvent::Refresh)),
                false,
            ),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        filters(state, items, directory, [scoped.len(), published, drafts, offline], active, width),
    ]
    .spacing(20)
    .width(Length::Fill);

    let results = if state.loading {
        page_notice(label("正在加载工作流程…", 14, Tone::Muted))
    } else if let Some(error) = &state.error {
        settings_error_banner(error.as_str())
    } else {
        let filtered = state.filtered(&scoped, directory);
        if filtered.is_empty() {
            let title = if state.search.trim().is_empty()
                && state.status.is_none()
                && state.department == Department::All
            {
                "还没有工作流程"
            } else {
                "没有符合筛选条件的流程"
            };
            page_notice(label(title, 14, Tone::Muted))
        } else {
            results(state, &filtered, directory, width)
        }
    };
    page.push(results).into()
}

fn filters<'a>(
    state: &'a List,
    items: &[Workflow],
    directory: Option<&departments::Directory>,
    counts: [usize; 4],
    active: Option<u64>,
    width: f32,
) -> Item<'a> {
    let statuses = [None, Some(Status::Published), Some(Status::Draft), Some(Status::Offline)];
    let status_picker =
        Row::with_children(statuses.into_iter().zip(counts).map(|(status, count)| {
            let title = status.map_or("全部", Status::label);
            let title =
                if state.items.is_some() { format!("{title} {count}") } else { title.into() };
            segment(title, state.status == status, message(ListEvent::Status(status)))
        }))
        .spacing(2)
        .wrap();
    let choice = |value: Department| departments::Choice {
        label: value.label(directory).to_owned(),
        value,
    };
    let departments = std::iter::once(Department::All)
        .chain(items.iter().map(Workflow::department).collect::<BTreeSet<_>>())
        .map(choice);
    let search = settings_text_input("搜索流程名称、说明或部门", &state.search)
        .on_input(|value| message(ListEvent::Search(value)))
        .on_submit_maybe((!state.loading).then(|| message(ListEvent::Refresh)))
        .width(if width >= 750.0 { Length::Fixed(285.0) } else { Length::Fill });
    let fields = row![
        search,
        settings_select(departments, Some(choice(state.department.clone())), |option| {
            message(ListEvent::Department(option.value))
        })
        .width(248),
        settings_select([Sort::Updated, Sort::Name, Sort::Runs], Some(state.sort), |value| {
            message(ListEvent::Sort(value))
        })
        .width(216),
        label(
            active.map_or_else(|| "执行中任务 —".to_owned(), |count| format!("执行中任务 {count}")),
            12,
            Tone::Muted,
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap();
    column![segmented(status_picker.into()), fields].spacing(10).into()
}

fn results<'a>(
    state: &List,
    items: &[&'a Workflow],
    directory: Option<&departments::Directory>,
    width: f32,
) -> Item<'a> {
    let pages = items.len().div_ceil(PAGE_SIZE);
    let page = state.page.min(pages - 1);
    let start = page * PAGE_SIZE;
    let end = (start + PAGE_SIZE).min(items.len());
    let columns = if width >= 740.0 { 2 } else { 1 };
    let card_width = (width - 16.0 * (columns - 1) as f32) / columns as f32;
    let mut grid = Column::new().spacing(16).width(Length::Fill);
    for cards in items[start..end].chunks(columns) {
        let mut group =
            Row::with_children(cards.iter().map(|item| workflow_card(item, directory, card_width)))
                .spacing(16)
                .width(Length::Fill);
        if cards.len() < columns {
            group = group.push(Space::new().width(Length::Fill));
        }
        grid = grid.push(group);
    }
    let mut footer = row![label(format!("共 {} 条", items.len()), 13, Tone::Muted)]
        .spacing(8)
        .align_y(Alignment::Center);
    if pages > 1 {
        footer = footer
            .push(action(
                "上一页",
                Some(Icon::ChevronLeft),
                page.checked_sub(1).map(|n| message(ListEvent::Page(n))),
                false,
            ))
            .push(label(format!("{} / {pages}", page + 1), 13, Tone::Muted))
            .push(action(
                "下一页",
                Some(Icon::ChevronRight),
                (page + 1 < pages).then(|| message(ListEvent::Page(page + 1))),
                false,
            ));
    }
    column![grid, footer.wrap()].spacing(12).into()
}

fn workflow_card<'a>(
    item: &'a Workflow,
    directory: Option<&departments::Directory>,
    width: f32,
) -> Item<'a> {
    let status = item.status();
    let tone = match status {
        Status::Published => Tone::Green,
        Status::Draft => Tone::Warning,
        Status::Offline | Status::Unknown => Tone::Muted,
    };
    let mut content = column![
        row![
            strong(item.name.as_str(), 17, Tone::Ink).width(Length::Fill),
            status_label(status.label(), tone),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(12);
    if let Some(description) =
        item.description.as_deref().map(str::trim).filter(|text| !text.is_empty())
    {
        content = content.push(label(description, 13, Tone::Muted));
    }

    let titles =
        item.step_titles.iter().map(|title| title.trim()).filter(|title| !title.is_empty());
    let mut steps = Row::new().align_y(Alignment::Center).spacing(10);
    for (index, title) in titles.enumerate() {
        steps = steps.push(
            row![
                strong((index + 1).to_string(), 12, Tone::Muted),
                container(label(title, 13, Tone::Ink)).max_width((width - 90.0).clamp(70.0, 210.0)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }
    if item.step_titles.iter().any(|title| !title.trim().is_empty()) {
        content = content.push(steps.wrap());
    }
    let version = item.version().map_or_else(|| "—".into(), |v| format!("v{v}"));
    let scope = match item.scope.as_deref() {
        Some("department") => "部门私有",
        Some("company") => "全公司可见",
        _ => "—",
    };
    let runs = item.runs_30d.map_or_else(|| "—".into(), |n| n.to_string());
    let success = match (item.succeeded_30d, item.runs_30d) {
        (Some(succeeded), Some(runs)) if runs > 0 => {
            format!("{:.0}%", succeeded as f64 / runs as f64 * 100.0)
        }
        _ => "—".into(),
    };
    content = content
        .push(divider())
        .push(
            row![
                label(version, 12, Tone::Muted),
                label(scope, 12, Tone::Muted),
                label(item.department().label(directory).to_owned(), 12, Tone::Muted),
            ]
            .spacing(8)
            .wrap(),
        )
        .push(
            row![
                label(format!("近 30 天执行 {runs} 次"), 12, Tone::Muted),
                label(format!("成功率 {success}"), 12, Tone::Muted),
                item.active_task_count.filter(|count| *count > 0).map(|count| label(
                    format!("{count} 个任务执行中"),
                    12,
                    Tone::Blue
                )),
            ]
            .spacing(8)
            .wrap(),
        );
    card(content.padding(16))
}
