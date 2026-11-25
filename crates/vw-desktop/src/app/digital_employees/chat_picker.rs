use super::view::{Item, Palette, Tone, icon, label, panel_style, status_label, tint_style};
use super::{
    Event as EmployeesEvent, assistants, assistants_view, chat, departments, shared,
    shared_view, tasks,
};
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_modal_header, settings_text_input,
};
use crate::app::{App, Message, Screen};
use desktop_assets::assets::Icon;
use desktop_session::session::agent::{EMPLOYEE_CHAT_PROFILE, EmployeeKind, EmployeeTarget};
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::panels::{
    settings_modal_layout, settings_modal_overlay_with_escape,
};
use iced::widget::{Space, button, column, container, image, row, text};
use iced::{Alignment, Element, Length, Task};

pub(super) struct Picker {
    navigation_generation: u64,
    session_id: Option<String>,
    screen: Screen,
    kind: EmployeeKind,
    query: String,
    pending: Option<u64>,
}

#[derive(Clone)]
pub(crate) enum Event {
    Scope(EmployeeKind),
    Search(String),
    Select(EmployeeTarget),
    Retry,
    Close,
}

fn message(event: Event) -> Message {
    Message::DigitalEmployees(EmployeesEvent::ChatPicker(event))
}

pub(crate) fn employee_locked(app: &App) -> bool {
    app.current_session_runtime_ref().is_some_and(|runtime| {
        runtime.is_employee_chat()
            && (app.current_session_has_started()
                || runtime.request_lifecycle.request_started_ms != 0)
    })
}

fn current(app: &App, picker: &Picker) -> bool {
    picker.navigation_generation == app.session_state.session_view_cache.navigation_generation
        && picker.session_id == app.session_state.active_session_id
        && picker.screen == app.navigation_tabs.screen
        && !app.navigation_panels.show_system_settings
        && !app.current_session_has_started()
        && !employee_locked(app)
}

pub(crate) fn is_open(app: &App) -> bool {
    app.digital_employees.chat_picker.as_ref().is_some_and(|picker| current(app, picker))
}

fn refresh(app: &mut App, kind: EmployeeKind) -> Task<Message> {
    match kind {
        EmployeeKind::Private => assistants::refresh(app),
        EmployeeKind::Shared => Task::batch([shared::refresh(app), tasks::load_departments(app)]),
    }
}

pub(crate) fn open(app: &mut App) -> Task<Message> {
    if app.current_session_has_started() || employee_locked(app) {
        return app.show_localized_toast(crate::app::state::ToastKind::Warning, "会话开始后不能更换数字员工", vec![]);
    }
    if app.account.active_account().is_none() {
        return app.show_localized_toast(crate::app::state::ToastKind::Error, "请先登录识矩", vec![]);
    }
    let kind = app
        .current_session_runtime_ref()
        .and_then(|runtime| runtime.employee_target.as_ref())
        .map_or(EmployeeKind::Private, |target| target.kind);
    app.popover_visibility.session_tool_selector = false;
    app.popover_visibility.model = false;
    app.popover_visibility.acp = false;
    app.digital_employees.chat_picker = Some(Picker {
        navigation_generation: app.session_state.session_view_cache.navigation_generation,
        session_id: app.session_state.active_session_id.clone(),
        screen: app.navigation_tabs.screen,
        kind,
        query: String::new(),
        pending: None,
    });
    refresh(app, kind)
}

pub(super) struct Choice<'a> {
    pub(super) selection: chat::Selection,
    role: &'a str,
    department: Option<&'a str>,
    status: (&'a str, Tone),
}

pub(super) fn choices(app: &App, kind: EmployeeKind) -> Vec<Choice<'_>> {
    choices_for(app, kind, false)
}

pub(super) fn schedule_choices(app: &App) -> Vec<Choice<'_>> {
    choices_for(app, EmployeeKind::Shared, true)
}

fn choices_for(app: &App, kind: EmployeeKind, scheduling: bool) -> Vec<Choice<'_>> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    match kind {
        EmployeeKind::Private => app
            .digital_employees
            .assistants
            .data
            .iter()
            .flat_map(|snapshot| snapshot.mine.iter().map(move |employee| (snapshot, employee)))
            .filter_map(|(snapshot, employee)| {
                let id = employee.chat_id()?;
                let department = snapshot
                    .members
                    .get(&employee.owner_user_id)
                    .and_then(|member| member.department.as_deref());
                Some(Choice {
                    selection: chat::Selection::private(id.to_owned(), employee.name.clone()),
                    role: i18n.__("私有执行员工"),
                    department,
                    status: assistants_view::runtime_badge(i18n, employee),
                })
            })
            .collect(),
        EmployeeKind::Shared => app
            .digital_employees
            .shared
            .directory
            .iter()
            .flat_map(|directory| directory.employees.iter().filter(move |employee| {
                if scheduling { directory.access.can_dispatch(employee) } else { employee.can_chat() }
            }))
            .map(|employee| {
                let role = employee.role().unwrap_or_else(|| shared_view::kind_title(i18n, employee));
                let department = employee
                    .department()
                    .filter(|path| !path.trim_matches('/').is_empty())
                    .map_or("全公司", |path| {
                        departments::name(app.digital_employees.tasks.departments.as_deref(), path)
                            .unwrap_or(i18n.__("部门共享"))
                    });
                Choice {
                    selection: chat::Selection::shared(employee),
                    role,
                    department: Some(department),
                    status: shared_view::runtime_badge(i18n, employee),
                }
            })
            .collect(),
    }
}

pub(super) fn finish_verification(app: &mut App, request: u64) -> bool {
    let Some(picker) = app.digital_employees.chat_picker.as_ref() else {
        return false;
    };
    if picker.pending != Some(request) {
        return false;
    }
    if !current(app, picker) {
        app.digital_employees.chat_picker = None;
        return false;
    }
    if let Some(picker) = app.digital_employees.chat_picker.as_mut() {
        picker.pending = None;
    }
    app.agents_settings
        .entries
        .iter()
        .any(|entry| entry.key == EMPLOYEE_CHAT_PROFILE && entry.can_start_session())
}

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Close) {
        app.digital_employees.chat_picker = None;
        return Task::none();
    }
    let Some(picker) = app.digital_employees.chat_picker.as_ref() else {
        return Task::none();
    };
    if !current(app, picker) {
        app.digital_employees.chat_picker = None;
        return Task::none();
    }
    if picker.pending.is_some() {
        return Task::none();
    }
    let kind = picker.kind;
    match event {
        Event::Close => Task::none(),
        Event::Scope(next) => {
            if let Some(picker) = app.digital_employees.chat_picker.as_mut() {
                picker.kind = next;
            }
            refresh(app, next)
        }
        Event::Search(query) => {
            if let Some(picker) = app.digital_employees.chat_picker.as_mut() {
                picker.query = query;
            }
            Task::none()
        }
        Event::Retry => refresh(app, kind),
        Event::Select(target) => {
            let Some(selection) = choices(app, kind)
                .into_iter()
                .find_map(|choice| (choice.selection.target == target).then_some(choice.selection))
            else {
                return app.show_localized_toast(crate::app::state::ToastKind::Error, "该数字员工当前不可聊天，请刷新列表", vec![]);
            };
            let before = app.digital_employees.chat_scope_request;
            let task = chat::check_scope(app, Some(chat::ChatIntent::Bind(selection)));
            let request = app.digital_employees.chat_scope_request;
            if request != before
                && let Some(picker) = app.digital_employees.chat_picker.as_mut()
            {
                picker.pending = Some(request);
            }
            task
        }
    }
}

pub(crate) fn with_modal<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let i18n = desktop_i18n::I18n::new(app.general_settings.desktop_locale);
    let Some(picker) =
        app.digital_employees.chat_picker.as_ref().filter(|picker| current(app, picker))
    else {
        return base;
    };
    let close = message(Event::Close);
    let pending = picker.pending.is_some();
    let scope_button = |label, kind| {
        button(text(label).size(14))
            .padding([8, 12])
            .style(if picker.kind == kind {
                primary_action_btn_style
            } else {
                rounded_action_btn_style
            })
            .on_press_maybe(
                (!pending && picker.kind != kind).then_some(message(Event::Scope(kind))),
            )
    };
    let header = column![
        settings_modal_header(text(i18n.__("选择数字员工")).size(18), settings_close_button(close.clone())),
        row![
            scope_button(i18n.__("私有员工"), EmployeeKind::Private),
            scope_button(i18n.__("共享员工"), EmployeeKind::Shared),
            settings_text_input(i18n.__("搜索姓名、岗位或部门"), &picker.query)
                .width(Length::Fixed(208.0))
                .on_input_maybe((!pending).then_some(|value| message(Event::Search(value)))),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap(),
    ]
    .spacing(12);
    let selected =
        app.current_session_runtime_ref().and_then(|runtime| runtime.employee_target.as_ref());
    let query = picker.query.trim().to_lowercase();
    let choices = choices(app, picker.kind)
        .into_iter()
        .filter(|choice| {
            choice.matches(&query)
        })
        .collect::<Vec<_>>();
    let (loading, error) = match picker.kind {
        EmployeeKind::Private => {
            (app.digital_employees.assistants.loading, &app.digital_employees.assistants.error)
        }
        EmployeeKind::Shared => {
            (app.digital_employees.shared.loading, &app.digital_employees.shared.error)
        }
    };
    let mut body = column![].spacing(8).width(Length::Fill);
    if pending {
        body = body.push(label(i18n.__("正在验证识矩账号…"), 13, Tone::Muted));
    } else if let Some(error) = error {
        body = body.push(text(error.render(i18n))).push(
            button(text(i18n.__("重试")))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press(message(Event::Retry)),
        );
    } else if choices.is_empty() {
        body = body.push(label(
            if loading { i18n.__("正在加载员工…") } else { i18n.__("没有匹配的可聊天员工") },
            13,
            Tone::Muted,
        ));
    }
    let mut grid = row![].spacing(8).width(Length::Fill);
    for choice in choices {
        let selected = selected == Some(&choice.selection.target);
        let select = (!pending).then(|| message(Event::Select(choice.selection.target.clone())));
        grid = grid.push(employee_card(choice, selected, select));
    }
    body = body.push(grid.wrap());
    let card = settings_modal_layout(header, body, None)
        .width(Length::Fill)
        .max_width(620)
        .height(Length::Fill)
        .max_height(480);
    settings_modal_overlay_with_escape(base, close, card)
}

impl Choice<'_> {
    pub(super) fn matches(&self, query: &str) -> bool {
        [Some(self.selection.name.as_str()), Some(self.role), self.department]
            .into_iter()
            .flatten()
            .any(|value| value.to_lowercase().contains(query))
    }
}

pub(super) fn employee_card(choice: Choice<'_>, selected: bool, select: Option<Message>) -> Item<'_> {
    let avatar: Item<'_> = match choice.selection.portrait {
        Some(handle) => image(handle).width(32).height(32).into(),
        None => container(icon(Icon::Person, 15, Tone::Accent))
            .center(32)
            .style(|theme| tint_style(theme, Tone::Accent, 16.0))
            .into(),
    };
    let details = choice.department.map_or_else(
        || choice.role.to_owned(),
        |department| format!("{} · {department}", choice.role),
    );
    let name = container(SingleLineLabel::new(choice.selection.name, 13.0))
        .width(Length::Fill)
        .style(|theme| container::Style::default().color(Tone::Ink.color(theme)));
    let details = container(SingleLineLabel::new(details, 11.0))
        .width(Length::Fill)
        .style(|theme| container::Style::default().color(Tone::Muted.color(theme)));
    let check: Item<'_> = if selected {
        icon(Icon::Check, 13, Tone::Accent).into()
    } else {
        Space::new().width(13).into()
    };
    let info = column![
        row![name, check].spacing(3).align_y(Alignment::Center),
        details,
        status_label(choice.status.0, choice.status.1),
    ]
    .spacing(2)
    .width(Length::Fill);
    button(row![avatar, info].spacing(7).align_y(Alignment::Center))
        .width(176)
        .padding([7, 8])
        .style(move |theme, status| {
            let palette = Palette::new(theme);
            let panel = panel_style(theme);
            let mut style = rounded_action_btn_style(theme, status);
            style.background = match status {
                button::Status::Hovered | button::Status::Pressed => Some(palette.header.into()),
                _ => panel.background,
            };
            style.border = panel.border;
            if selected || status == button::Status::Hovered {
                style.border.color = palette.accent;
            }
            style
        })
        .on_press_maybe(select)
        .into()
}
