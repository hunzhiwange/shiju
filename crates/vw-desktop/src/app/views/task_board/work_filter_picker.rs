use crate::app::components::system_settings_common::{
    icon_svg, primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_divider, settings_muted_text_style, settings_panel_style,
    settings_segment_button_style, settings_text_input,
};
use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_task_board::message::AdvancedWorkFilterAction;
use desktop_task_board::work_views::{WorkFilterField, WorkPriorityLevel, WorkScopeField, WorkToolbarMenu};
use desktop_task_model::{DEFAULT_TASK_WORKFLOW_ID, TASK_AGENT_MAIN, TaskStatus};
use desktop_ui_controls::{
    overlays::SideOverlay, single_line_label::SingleLineLabel,
    system_settings_common::icons::icon_btn_svg_style,
};
use iced::widget::{button, checkbox, column, container, mouse_area, row, scrollable, text};
use iced::{Alignment, Color, Element, Length};
use std::collections::{BTreeMap, HashMap};

pub(in crate::app) const WORK_FILTER_SEARCH_INPUT_ID: &str = "task-board-filter-search";

pub(in crate::app) fn inherited_filter_query(app: &App, field: &WorkFilterField) -> String {
    let query = app.task_board.work_filter_query.trim();
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let label = field_label(app, i18n, field);
    if field.has_search() && !label.to_lowercase().contains(&query.to_lowercase()) {
        query.to_owned()
    } else {
        String::new()
    }
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let board = &app.task_board;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let query = board.work_filter_query.trim().to_lowercase();
    let search = row![
        icon_svg(Icon::Search, 15.0).style(icon_btn_svg_style),
        settings_text_input(i18n.__("按字段或值筛选…"), &board.work_filter_query)
            .id(WORK_FILTER_SEARCH_INPUT_ID)
            .width(Length::Fill)
            .on_input(|value| TaskBoardMessage::SearchWorkFilter(value).into()),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let footer = row![
        menu_row(
            Icon::Sliders,
            i18n.__("切换到高级筛选").to_owned(),
            TaskBoardMessage::AdvancedWorkFilter(AdvancedWorkFilterAction::Open),
            false,
        ),
        button(text(i18n.__("应用筛选")).size(13))
            .padding([6, 12])
            .style(primary_action_btn_style)
            .on_press(TaskBoardMessage::ApplyQuickWorkFilter.into()),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    container(
        column![
            search,
            settings_divider(),
            scroll_options(field_list(app, i18n, &query), 350),
            settings_divider(),
            footer,
            menu_row(
                Icon::ArrowCounterClockwise,
                i18n.__("重置全部筛选").to_owned(),
                TaskBoardMessage::ClearWorkFilter,
                false,
            ),
        ]
        .spacing(8),
    )
    .width(324)
    .padding(11)
    .style(settings_panel_style)
    .into()
}

pub(super) fn scroll_options<'a>(options: Element<'a, Message>, height: u32) -> Element<'a, Message> {
    container(
        scrollable(container(options).width(Length::Fill).padding([0, 7]))
            .height(Length::Shrink)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            )),
    )
    .max_height(height)
    .into()
}

fn field_panel<'a>(app: &'a App, i18n: I18n, field: &WorkFilterField) -> Element<'a, Message> {
    let query = &app.task_board.work_filter_value_query;
    let mut content = if let WorkFilterField::Scope(scope) = field {
        column![scope_operator(app, i18n, *scope)]
    } else if matches!(field, WorkFilterField::Label) {
        column![label_operator(app, i18n)]
    } else {
        column![text(field_label(app, i18n, field)).size(14)]
    }.spacing(8);
    if field.has_search() {
        content = content.push(
            settings_text_input(i18n.__("搜索筛选值…"), query)
                .on_input(|value| TaskBoardMessage::SearchWorkFilterValue(value).into()),
        );
    }
    let height = if matches!(field, WorkFilterField::Date) { 348 } else { 264 };
    content = match field {
        WorkFilterField::Status => content.push(status_operator(app, i18n)),
        WorkFilterField::Priority => content.push(priority_operator(app, i18n)),
        _ => content,
    };
    let choices = field_options(app, i18n, field, &query.trim().to_lowercase());
    container(content.push(scroll_options(choices, height)))
        .width(256)
        .padding(10)
        .style(settings_panel_style)
        .into()
}

pub(super) fn field_label(app: &App, i18n: I18n, field: &WorkFilterField) -> String {
    match field {
        WorkFilterField::Status => i18n.__("状态"),
        WorkFilterField::AgentRunning => i18n.__("智能体状态"),
        WorkFilterField::ReadState => i18n.__("阅读状态"),
        WorkFilterField::Agent => i18n.__("执行智能体"),
        WorkFilterField::Priority => i18n.__("优先级"),
        WorkFilterField::Date => i18n.__("日期"),
        WorkFilterField::Scope(scope) => i18n.__(scope.label()),
        WorkFilterField::Label => i18n.__("标签"),
        WorkFilterField::Advanced => i18n.__("高级筛选"),
        WorkFilterField::Property(id) => app
            .task_board
            .task_properties
            .iter()
            .find(|property| &property.id == id)
            .map_or(i18n.__("属性已不存在"), |property| property.name.as_str()),
    }
    .to_owned()
}

pub(super) fn fields(app: &App) -> Vec<(WorkFilterField, Icon)> {
    let board = &app.task_board;
    let builtins = [
        (WorkFilterField::Status, Icon::Circle),
        (WorkFilterField::AgentRunning, Icon::Robot),
        (WorkFilterField::ReadState, Icon::Eye),
        (WorkFilterField::Priority, Icon::BarChartFill),
        (WorkFilterField::Date, Icon::Calendar),
    ];
    let scopes = [WorkScopeField::Assignee, WorkScopeField::Creator, WorkScopeField::Relation, WorkScopeField::Project].map(|field| {
        (WorkFilterField::Scope(field), super::work_filter_scope::field_icon(field))
    });
    let other = [
        (WorkFilterField::Scope(WorkScopeField::ProjectStatus), Icon::Circle),
        (WorkFilterField::Label, Icon::ListUl),
        (WorkFilterField::Scope(WorkScopeField::DueDate), Icon::Calendar),
        (WorkFilterField::Agent, Icon::Robot),
    ];
    let properties = board
        .task_properties
        .iter()
        .filter(|property| {
            !property.archived || board.work_property_values.contains_key(&property.id)
                || board.work_unset_properties.contains(&property.id)
        })
        .map(|property| (WorkFilterField::Property(property.id.clone()), Icon::Sliders));
    builtins.into_iter().chain(scopes).chain(other).chain(properties).collect()
}

fn field_list<'a>(app: &'a App, i18n: I18n, query: &str) -> Element<'a, Message> {
    let board = &app.task_board;
    let draft = crate::app::message::task_board::work_filter_draft(app).ok();
    let filter = draft.as_ref().unwrap_or(&board.work_filter);
    let mut options = column![].spacing(2).width(Length::Fill);
    let mut has_rows = false;
    let mut has_properties = false;
    for (field, icon) in fields(app) {
        let label = field_label(app, i18n, &field);
        let summary = super::work_filter_bar::value_summary(app, i18n, &field, filter);
        if !label.to_lowercase().contains(query)
            && !summary.as_ref().is_some_and(|value| value.to_lowercase().contains(query))
            && !value_matches(app, i18n, &field, query)
        {
            continue;
        }
        if !has_properties && matches!(field, WorkFilterField::Property(_)) {
            if has_rows {
                options = options.push(settings_divider());
            }
            has_properties = true;
        }
        has_rows = true;
        let active = board.work_filter_field.as_ref() == Some(&field);
        let message = TaskBoardMessage::SelectWorkFilterField(field.clone());
        let trigger = mouse_area(menu_button(icon, label, message.clone(), true, active, summary))
            .on_enter(message.into());
        let option: Element<'_, Message> = if active {
            SideOverlay::new(trigger, field_panel(app, i18n, &field))
                .show(true)
                .gap(5.0)
                .on_mouse_exit(TaskBoardMessage::DismissWorkFilterField(field).into())
                .into()
        } else {
            trigger.into()
        };
        options = options.push(option);
    }
    if has_rows { options.into() } else { option_list(i18n, std::iter::empty()) }
}

fn value_matches(app: &App, i18n: I18n, field: &WorkFilterField, query: &str) -> bool {
    use desktop_task_board::work_views::{WorkDateField, WorkDateRange};
    let board = &app.task_board;
    let matches = |value: &str| value.to_lowercase().contains(query);
    match field {
        WorkFilterField::Status => board.task_board_settings.workflow()
            .or_else(|| desktop_task_model::task_workflow_by_id(DEFAULT_TASK_WORKFLOW_ID))
            .is_some_and(|workflow| workflow.statuses.iter().any(|status| {
                status.enabled && (matches(status.key()) || matches(&desktop_i18n::tasks::status_label(
                    i18n, &status.label, matches!(status.status, TaskStatus::Custom(_)),
                )))
            })),
        WorkFilterField::AgentRunning => ["运行中", "空闲"].iter().any(|label| matches(i18n.__(label))),
        WorkFilterField::ReadState => matches(i18n.__("未读")),
        WorkFilterField::Agent => agent_choices(app, i18n).iter().any(|(key, label)| matches(key) || matches(label)),
        WorkFilterField::Priority => WorkPriorityLevel::ALL.iter().any(|level| matches(i18n.__(level.label()))),
        WorkFilterField::Date => [WorkDateField::Created, WorkDateField::Updated].iter().any(|field| matches(i18n.__(field.label())))
            || WorkDateRange::PRESETS.iter().any(|range| matches(i18n.__(range.label())))
            || matches(i18n.__("自定义日期或范围")),
        WorkFilterField::Scope(scope) => super::work_filter_scope::value_matches(app, i18n, *scope, query),
        WorkFilterField::Label => board.task_labels.iter().any(|label| !label.archived && matches(&label.name)),
        WorkFilterField::Property(id) => matches(i18n.__("未设置"))
            || board.task_properties.iter().find(|property| &property.id == id)
                .is_some_and(|property| property.options.iter().any(|option| matches(&option.name))),
        WorkFilterField::Advanced => false,
    }
}

pub(super) fn advanced_values<'a>(app: &'a App, i18n: I18n, field: &WorkFilterField) -> Element<'a, Message> {
    let query = &app.task_board.work_filter_value_query;
    let mut content = column![].spacing(6);
    if field.has_search() {
        content = content.push(
            settings_text_input(i18n.__("搜索筛选值…"), query)
                .on_input(|value| TaskBoardMessage::SearchWorkFilterValue(value).into()),
        );
    }
    let height = if matches!(field, WorkFilterField::Date) { 320 } else { 170 };
    content.push(scroll_options(field_options(app, i18n, field, &query.trim().to_lowercase()), height)).into()
}

fn field_options<'a>(
    app: &'a App,
    i18n: I18n,
    field: &WorkFilterField,
    query: &str,
) -> Element<'a, Message> {
    let board = &app.task_board;
    match field {
        WorkFilterField::Status => status_choices(app, i18n, query),
        WorkFilterField::AgentRunning => running_choices(app, i18n),
        WorkFilterField::ReadState => unread_choice(app, i18n),
        WorkFilterField::Agent => {
            csv_choices(i18n, agent_choices(app, i18n), &board.work_agent_input, 4, query)
        }
        WorkFilterField::Priority => priority_choices(app, i18n),
        WorkFilterField::Date => super::work_filter_date::choices(board, i18n),
        WorkFilterField::Scope(scope) => super::work_filter_scope::choices(app, i18n, *scope, query),
        WorkFilterField::Label => {
            let options = board.task_labels.iter().filter_map(|label| {
                let selected = board.work_label_ids.contains(&label.id);
                let name = if label.archived {
                    format!("{} · {}", label.name, i18n.__("已归档"))
                } else {
                    label.name.clone()
                };
                if (!selected && label.archived) || !name.to_lowercase().contains(query) {
                    return None;
                }
                let mut content = row![desktop_task_board::board::task_label_badge(label)]
                    .spacing(6)
                    .align_y(Alignment::Center);
                if label.archived {
                    content = content.push(text(i18n.__("已归档")).size(11).style(settings_muted_text_style));
                }
                Some(filter_choice(
                    content.into(),
                    selected,
                    TaskBoardMessage::ToggleWorkViewLabel(label.id.clone()),
                ))
            });
            option_list(i18n, options)
        }
        WorkFilterField::Property(id) => {
            board.task_properties.iter().find(|property| &property.id == id).map_or_else(
                || text(i18n.__("属性已不存在")).size(13).style(settings_muted_text_style).into(),
                |property| super::properties::quick_filter_property(app, property),
            )
        }
        WorkFilterField::Advanced => iced::widget::Space::new().into(),
    }
}

pub(super) fn status_operator(app: &App, i18n: I18n) -> Element<'static, Message> {
    operator(
        i18n,
        app.task_board.work_status_excluded,
        TaskBoardMessage::SetWorkFilterStatusExcluded,
    )
}

pub(super) fn priority_operator(app: &App, i18n: I18n) -> Element<'static, Message> {
    operator(
        i18n,
        app.task_board.work_priority_excluded,
        TaskBoardMessage::SetWorkFilterPriorityExcluded,
    )
}

pub(super) fn scope_operator(app: &App, i18n: I18n, field: WorkScopeField) -> Element<'static, Message> {
    let excluded = app.task_board.work_scope_selections.get(&field)
        .is_some_and(|selection| selection.excluded);
    operator(i18n, excluded, |excluded| TaskBoardMessage::SetWorkScopeExcluded(field, excluded))
}

pub(super) fn label_operator(app: &App, i18n: I18n) -> Element<'static, Message> {
    operator(i18n, app.task_board.work_labels_excluded, TaskBoardMessage::SetWorkFilterLabelsExcluded)
}

fn operator(
    i18n: I18n,
    is_excluded: bool,
    set_excluded: impl Fn(bool) -> TaskBoardMessage,
) -> Element<'static, Message> {
    let buttons = [(false, "是"), (true, "不是")].into_iter().map(|(excluded, label)| {
        let selected = is_excluded == excluded;
        button(text(i18n.__(label)).size(13))
            .padding([5, 10])
            .width(Length::Fill)
            .style(move |theme, status| settings_segment_button_style(theme, status, selected))
            .on_press(set_excluded(excluded).into())
            .into()
    });
    row(buttons).spacing(5).into()
}

pub(super) fn unread_choice(app: &App, i18n: I18n) -> Element<'static, Message> {
    let checked = app.task_board.work_unread_only;
    marked_choice(
        i18n.__("未读"),
        accent_dot(),
        checked,
        TaskBoardMessage::SetWorkFilterUnread(!checked),
    )
}

pub(super) fn priority_choices(app: &App, i18n: I18n) -> Element<'static, Message> {
    column(WorkPriorityLevel::ALL.map(|level| {
        marked_choice(
            i18n.__(level.label()),
            priority_marker(level),
            app.task_board.work_priority_levels.contains(&level),
            TaskBoardMessage::ToggleWorkFilterPriority(level),
        )
    }))
    .spacing(3)
    .into()
}

pub(super) fn priority_label(i18n: I18n, levels: &[WorkPriorityLevel], excluded: bool) -> String {
    let label = match levels {
        [level] => i18n.__(level.label()).to_owned(),
        _ => i18n.format("%u 个优先级", &[levels.len().into()]).expect("valid priority count"),
    };
    if excluded { format!("{} {label}", i18n.__("排除")) } else { label }
}

pub(super) fn priority_marker(level: WorkPriorityLevel) -> Element<'static, Message> {
    let (color, symbol) = match level {
        WorkPriorityLevel::None => return text("–").size(14).style(settings_muted_text_style).into(),
        WorkPriorityLevel::Urgent => (Color::from_rgb8(218, 59, 65), Some("!")),
        WorkPriorityLevel::High => (Color::from_rgb8(226, 116, 25), None),
        WorkPriorityLevel::Medium => (Color::from_rgb8(203, 145, 20), None),
        WorkPriorityLevel::Low => (Color::from_rgb8(75, 140, 212), None),
    };
    if let Some(symbol) = symbol {
        return container(text(symbol).size(11).color(Color::WHITE))
            .width(13)
            .height(13)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(move |_| container::Style::default().background(color).border(
                iced::Border { radius: 3.0.into(), ..Default::default() },
            ))
            .into();
    }
    icon_svg(Icon::BarChartFill, 13.0)
        .style(move |_, _| iced::widget::svg::Style { color: Some(color) })
        .into()
}

fn accent_dot() -> Element<'static, Message> {
    container(iced::widget::Space::new().width(6).height(6))
        .style(|theme: &iced::Theme| {
            container::Style::default().background(theme.palette().primary).border(
                iced::Border { radius: 3.0.into(), ..Default::default() },
            )
        })
        .into()
}

pub(super) fn marked_choice(
    label: &str,
    marker: Element<'static, Message>,
    checked: bool,
    message: TaskBoardMessage,
) -> Element<'static, Message> {
    filter_choice(
        row![
            container(marker).width(15).align_x(Alignment::Center),
            SingleLineLabel::new(label.to_owned(), 13.0),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into(),
        checked,
        message,
    )
}

pub(super) fn filter_choice<'a>(
    content: Element<'a, Message>,
    checked: bool,
    message: TaskBoardMessage,
) -> Element<'a, Message> {
    let check = if checked { Icon::CheckSquare } else { Icon::Square };
    button(row![icon_svg(check, 15.0).style(icon_btn_svg_style), content]
        .spacing(8).align_y(Alignment::Center))
    .padding([6, 5])
    .width(Length::Fill)
    .style(move |theme, status| super::style::navigation_button(theme, status, checked))
    .on_press(message.into())
    .into()
}

pub(super) fn running_choices(app: &App, i18n: I18n) -> Element<'static, Message> {
    let rows = [(true, "运行中"), (false, "空闲")].into_iter().map(|(running, label)| {
        checkbox(app.task_board.work_filter_agent_running == Some(running))
            .label(i18n.__(label))
            .style(settings_checkbox_style)
            .on_toggle(move |checked| {
                TaskBoardMessage::SetWorkFilterAgentRunning(checked.then_some(running)).into()
            })
            .into()
    });
    column(rows).spacing(9).into()
}

fn status_choices(app: &App, i18n: I18n, query: &str) -> Element<'static, Message> {
    let board = &app.task_board;
    let selected = csv_values(&board.work_status_input);
    let workflow = board
        .task_board_settings
        .workflow()
        .or_else(|| desktop_task_model::task_workflow_by_id(DEFAULT_TASK_WORKFLOW_ID));
    let mut counts = HashMap::<TaskStatus, usize>::new();
    for task in board.task_board_tasks.iter().filter(|task| !task.deleted && !task.archived) {
        *counts.entry(task.status).or_default() += 1;
    }
    let mut options = workflow
        .into_iter()
        .flat_map(|workflow| workflow.statuses.into_iter())
        .filter(|status| status.enabled || selected.iter().any(|key| key == status.key()))
        .map(|status| {
            let key = status.key().to_owned();
            let label = desktop_i18n::tasks::status_label(
                i18n,
                &status.label,
                matches!(status.status, TaskStatus::Custom(_)),
            );
            (key, label, Some(status.status))
        })
        .collect::<Vec<_>>();
    for value in &selected {
        if !options.iter().any(|(key, _, _)| key == value) {
            options.push((value.clone(), value.clone(), None));
        }
    }
    let rows = options
        .into_iter()
        .filter(|(key, label, _)| {
            label.to_lowercase().contains(query) || key.to_lowercase().contains(query)
        })
        .map(|(key, label, status)| {
            let checked = selected.contains(&key);
            let mark = if checked { Icon::CheckSquare } else { Icon::Square };
            let mut item = row![icon_svg(mark, 15.0).style(icon_btn_svg_style)]
                .spacing(8)
                .align_y(Alignment::Center);
            if let Some(status) = status {
                item = item.push(desktop_task_board::board::task_status_marker(status));
            }
            item = item.push(SingleLineLabel::new(label, 13.0));
            if let Some(count) = status.and_then(|status| counts.get(&status)) {
                let summary =
                    i18n.format("%u 个任务", &[(*count).into()]).expect("valid task count");
                item = item.push(text(summary).size(11).style(settings_muted_text_style));
            }
            button(item)
                .padding([5, 4])
                .width(Length::Fill)
                .style(move |theme, status| super::style::navigation_button(theme, status, checked))
                .on_press(change_csv_choice(1, &selected, &key, !checked))
                .into()
        });
    option_list(i18n, rows)
}

pub(super) fn agent_choices(app: &App, i18n: I18n) -> Vec<(String, String)> {
    let mut agents = BTreeMap::new();
    agents.insert(format!("builtin:{TASK_AGENT_MAIN}"), i18n.__("主智能体").to_owned());
    for entry in &app.agents_settings.entries {
        agents.insert(format!("builtin:{}", entry.key), entry.label.clone());
    }
    let acp_ids =
        app.acp_settings.catalog.keys().map(String::as_str).chain(
            app.task_board.task_board_tasks.iter().filter_map(|task| task.acp_agent.as_deref()),
        );
    for id in acp_ids.filter_map(desktop_task_model::normalize_task_acp_agent_input) {
        agents
            .entry(format!("acp:{id}"))
            .or_insert_with(|| desktop_task_board::draft::task_acp_agent_label(Some(&id)));
    }
    for id in app.task_board.task_board_tasks.iter().filter_map(|task| task.agent.as_deref()) {
        agents.entry(format!("builtin:{id}")).or_insert_with(|| id.to_owned());
    }
    agents.into_iter().collect()
}

fn csv_choices(
    i18n: I18n,
    mut options: Vec<(String, String)>,
    input: &str,
    index: u8,
    query: &str,
) -> Element<'static, Message> {
    let selected = csv_values(input);
    for value in &selected {
        if !options.iter().any(|(key, _)| key == value) {
            options.push((value.clone(), value.clone()));
        }
    }
    let rows = options
        .into_iter()
        .filter(|(key, label)| {
            label.to_lowercase().contains(query) || key.to_lowercase().contains(query)
        })
        .map(|(key, label)| {
            let checked = selected.contains(&key);
            let selected = selected.clone();
            checkbox(checked)
                .label(label)
                .style(settings_checkbox_style)
                .on_toggle(move |checked| change_csv_choice(index, &selected, &key, checked))
                .into()
        });
    option_list(i18n, rows)
}

fn csv_values(input: &str) -> Vec<String> {
    input.split(',').map(str::trim).filter(|value| !value.is_empty()).map(str::to_owned).collect()
}

fn change_csv_choice(index: u8, selected: &[String], key: &str, checked: bool) -> Message {
    let mut values = selected.to_vec();
    values.retain(|value| value != key);
    if checked {
        values.push(key.to_owned());
    }
    TaskBoardMessage::WorkViewFieldChanged(index, values.join(", ")).into()
}

fn option_list<'a>(
    i18n: I18n,
    options: impl Iterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut options = options.peekable();
    if options.peek().is_none() {
        return container(
            text(i18n.__("没有匹配的筛选项")).size(13).style(settings_muted_text_style),
        )
        .padding([7, 4])
        .into();
    }
    column(options).spacing(7).width(Length::Fill).into()
}

pub(super) fn menu_row(
    icon: Icon,
    label: String,
    message: TaskBoardMessage,
    submenu: bool,
) -> Element<'static, Message> {
    menu_button(icon, label, message, submenu, false, None)
}

fn menu_button(
    icon: Icon,
    label: String,
    message: TaskBoardMessage,
    submenu: bool,
    active: bool,
    summary: Option<String>,
) -> Element<'static, Message> {
    let mut content =
        row![icon_svg(icon, 15.0).style(icon_btn_svg_style), SingleLineLabel::new(label, 13.0),]
            .spacing(8)
            .align_y(Alignment::Center);
    if let Some(summary) = summary {
        content = content
            .push(container(SingleLineLabel::new(summary, 12.0).shrink()).max_width(108))
            .push(accent_dot());
    }
    if submenu {
        content = content.push(icon_svg(Icon::ChevronRight, 12.0).style(icon_btn_svg_style));
    }
    button(content)
        .padding([6, 8])
        .width(Length::Fill)
        .style(move |theme, status| super::style::navigation_button(theme, status, active))
        .on_press(message.into())
        .into()
}
