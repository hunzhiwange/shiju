use std::collections::BTreeSet;

use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, rounded_action_btn_style,
    settings_pick_list_menu_style, settings_pick_list_style, settings_text_input,
};
use iced::widget::{Column, Row, Space, button, column, container, pick_list, row};
use iced::{Alignment, Length, border};

use super::view::{
    Item, Palette, Tone, action, card, chip, icon, label, panel_style, strong, tint_style,
};
use super::workflows::{Department, List, ListEvent, PAGE_SIZE, Sort, Status, Workflow, message};

pub(super) fn view(state: &List, width: f32) -> Item<'_> {
    let items = state.items.as_deref().map_or(&[][..], Vec::as_slice);
    let scoped: Vec<_> = items.iter().filter(|item| state.department.matches(item)).collect();
    let count = |status| scoped.iter().filter(|item| item.status() == status).count();
    let total = |count: usize| state.items.as_ref().map(|_| count as u64);
    let [published, drafts, offline] =
        [Status::Published, Status::Draft, Status::Offline].map(count);
    let active = if scoped.is_empty() {
        total(0)
    } else {
        scoped.iter().filter_map(|item| item.active_task_count).reduce(u64::saturating_add)
    };
    let stats = [
        stat("已发布流程", total(published), "可用于派任务", Icon::GitBranch, Tone::Green),
        stat(
            "草稿",
            total(drafts),
            "发布后才能用",
            Icon::FileText,
            if drafts > 0 { Tone::Warning } else { Tone::Green },
        ),
        stat(
            "按流程跑的任务",
            active,
            if active.is_some() || state.items.is_none() {
                "未结束且选了流程"
            } else {
                "暂时没统计出来"
            },
            Icon::ArrowRepeat,
            Tone::Blue,
        ),
        stat(
            "已下线",
            total(offline),
            "不接新任务，可随时上线",
            Icon::Clock,
            if offline > 0 { Tone::Muted } else { Tone::Green },
        ),
    ];
    let [published_card, drafts_card, running_card, offline_card] = stats;
    let stats: Item<'_> = if width >= 950.0 {
        row![published_card, drafts_card, running_card, offline_card].spacing(14).into()
    } else if width >= 490.0 {
        column![
            row![published_card, drafts_card].spacing(12),
            row![running_card, offline_card].spacing(12),
        ]
        .spacing(12)
        .into()
    } else {
        column![published_card, drafts_card, running_card, offline_card].spacing(10).into()
    };

    let page = column![
        column![
            strong("工作流程", 24, Tone::Ink),
            label("把多个共享员工串成一条能跑的流程", 14, Tone::Muted),
        ]
        .spacing(8),
        stats,
        filters(state, items, [scoped.len(), published, drafts, offline], width),
    ]
    .spacing(16)
    .width(Length::Fill);

    let results = if state.loading {
        notice("正在加载工作流程…", Tone::Muted)
    } else if let Some(error) = &state.error {
        notice(error.as_str(), Tone::Red)
    } else {
        let filtered = state.filtered(&scoped);
        if filtered.is_empty() {
            let title = if state.search.trim().is_empty()
                && state.status.is_none()
                && state.department == Department::All
            {
                "还没有工作流程"
            } else {
                "没有符合筛选条件的流程"
            };
            notice(title, Tone::Muted)
        } else {
            results(state, &filtered, width)
        }
    };
    page.push(Space::new().height(20)).push(results).into()
}

fn notice<'a>(text: &'a str, tone: Tone) -> Item<'a> {
    card(container(label(text, 14, tone)).padding(16))
}

fn stat(
    title: &'static str,
    value: Option<u64>,
    hint: &'static str,
    glyph: Icon,
    tone: Tone,
) -> Item<'static> {
    let badge = container(icon(glyph, 19, tone))
        .center(38)
        .style(move |theme| tint_style(theme, tone, 8.0));
    let count = value.map_or_else(|| "—".into(), |n| n.to_string());
    card(
        row![
            badge,
            column![
                label(title, 12, Tone::Muted),
                row![strong(count, 23, Tone::Ink), strong(hint, 12, Tone::Green)]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .wrap(),
            ]
            .spacing(6),
        ]
        .spacing(12)
        .padding(16),
    )
}

fn filters<'a>(state: &'a List, items: &[Workflow], counts: [usize; 4], width: f32) -> Item<'a> {
    let wide = width >= 1110.0;
    let refresh = (!state.loading).then(|| message(ListEvent::Refresh));
    let search = column![
        label("关键字", 12, Tone::Muted),
        settings_text_input("搜索流程名称、说明或部门", &state.search)
            .on_input(|value| message(ListEvent::Search(value)))
            .on_submit_maybe(refresh.clone())
            .width(Length::Fill),
    ]
    .spacing(6);
    let statuses = [None, Some(Status::Published), Some(Status::Draft), Some(Status::Offline)];
    let status_picker =
        Row::with_children(statuses.into_iter().zip(counts).map(|(status, count)| {
            let title = status.map_or("全部", Status::label);
            let title =
                if state.items.is_some() { format!("{title} {count}") } else { title.into() };
            let selected = state.status == status;
            let tone = if selected { Tone::Accent } else { Tone::Muted };
            button(strong(title, 13, tone))
                .on_press(message(ListEvent::Status(status)))
                .padding([7, 10])
                .style(move |theme, status| {
                    let mut style = rounded_action_btn_style(theme, status);
                    style.border = border::rounded(8);
                    let opacity = if selected {
                        0.13
                    } else if status == button::Status::Active {
                        0.0
                    } else {
                        0.10
                    };
                    style.background = Some(tone.color(theme).scale_alpha(opacity).into());
                    style
                })
                .into()
        }))
        .spacing(4)
        .wrap();
    let status_picker = container(status_picker).padding(4).style(|theme| {
        let palette = Palette::new(theme);
        let mut style = panel_style(theme);
        style.background = Some(palette.muted.scale_alpha(0.05).into());
        style
    });
    let departments: Vec<_> = std::iter::once(Department::All)
        .chain(items.iter().map(Workflow::department).collect::<BTreeSet<_>>())
        .collect();
    let department = column![
        label("筛选部门", 12, Tone::Muted),
        pick_list(departments, Some(state.department.clone()), |value| message(
            ListEvent::Department(value)
        ))
        .padding(SETTINGS_CONTROL_PADDING)
        .text_size(SETTINGS_CONTROL_TEXT_SIZE)
        .width(164)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style),
    ]
    .spacing(6);
    let sort = column![
        label("排序", 12, Tone::Muted),
        pick_list([Sort::Updated, Sort::Name, Sort::Runs], Some(state.sort), |value| message(
            ListEvent::Sort(value)
        ))
        .padding(SETTINGS_CONTROL_PADDING)
        .text_size(SETTINGS_CONTROL_TEXT_SIZE)
        .width(150)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style),
    ]
    .spacing(6);
    let query =
        action(if state.loading { "加载中" } else { "查询" }, Some(Icon::Search), refresh, true);
    if wide {
        card(
            row![
                container(search).width(320),
                status_picker,
                department,
                sort,
                Space::new().width(Length::Fill),
                query
            ]
            .align_y(Alignment::End)
            .spacing(12)
            .padding(16),
        )
    } else {
        card(
            column![
                search,
                row![status_picker, department, sort, query]
                    .align_y(Alignment::End)
                    .spacing(12)
                    .wrap(),
            ]
            .spacing(12)
            .padding(16),
        )
    }
}

fn results<'a>(state: &List, items: &[&'a Workflow], width: f32) -> Item<'a> {
    let pages = items.len().div_ceil(PAGE_SIZE);
    let page = state.page.min(pages - 1);
    let start = page * PAGE_SIZE;
    let end = (start + PAGE_SIZE).min(items.len());
    let columns = if width >= 740.0 { 2 } else { 1 };
    let card_width = (width - 16.0 * (columns - 1) as f32) / columns as f32;
    let mut grid = Column::new().spacing(16).width(Length::Fill);
    for cards in items[start..end].chunks(columns) {
        let mut group =
            Row::with_children(cards.iter().map(|item| workflow_card(item, card_width)))
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
            .push(Space::new().width(Length::Fill))
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
    column![grid, footer.wrap()].spacing(32).into()
}

fn workflow_card(item: &Workflow, width: f32) -> Item<'_> {
    let status = item.status();
    let tone = match status {
        Status::Published => Tone::Green,
        Status::Draft => Tone::Warning,
        Status::Offline | Status::Unknown => Tone::Muted,
    };
    let mut content = column![
        row![
            label(item.name.as_str(), 15, Tone::Ink).width(Length::Fill),
            chip(strong(status.label(), 12, tone), tone),
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

    let mut steps = Row::new().align_y(Alignment::Center).spacing(8);
    for (index, title) in item
        .step_titles
        .iter()
        .map(|title| title.trim())
        .filter(|title| !title.is_empty())
        .enumerate()
    {
        let mut step = Row::new().align_y(Alignment::Center).spacing(8);
        if index > 0 {
            step = step.push(strong("→", 14, Tone::Muted));
        }
        let number = container(strong((index + 1).to_string(), 12, Tone::Muted))
            .center(24)
            .style(|theme| tint_style(theme, Tone::Muted, 16.0));
        steps = steps.push(step.push(number).push(
            container(strong(title, 13, Tone::Muted)).max_width((width - 110.0).clamp(70.0, 190.0)),
        ));
    }
    if item.step_titles.iter().any(|title| !title.trim().is_empty()) {
        content = content.push(card(container(steps.wrap()).padding([12, 14])));
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
    content = content.push(
        row![
            label(version, 12, Tone::Muted),
            label(scope, 12, Tone::Muted),
            label(item.department().to_string(), 12, Tone::Muted),
            label(format!("近 30 天跑了 {runs} 次"), 12, Tone::Muted),
            label(format!("成功率 {success}"), 12, Tone::Muted),
        ]
        .spacing(8)
        .wrap(),
    );
    if let Some(active) = item.active_task_count.filter(|count| *count > 0) {
        content = content.push(label(format!("{active} 个任务正在按它跑"), 12, Tone::Blue));
    }
    card(content.padding(13))
}
