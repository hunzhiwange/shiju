//! Agent Cloud schedules embedded in the desktop Cron screen.
use super::{chat_picker, shared, task_api, tasks};
use crate::app::components::system_settings_cron;
use crate::app::message::SettingsMessage;
use crate::app::{App, Message};
use chrono::{DateTime, Utc};
use desktop_i18n::{I18n, workflow::Feedback};
use desktop_settings_cron::state::{CronScheduleRule, CronSettingsTab};
use desktop_settings_cron::system_settings_cron::date_time::{
    DateTimePicker, PickerEvent, picker_overlay_with_i18n,
};
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, primary_action_btn_style, rounded_action_btn_style,
    settings_close_button, settings_error_banner, settings_field_label, settings_modal_header,
    settings_modal_layout, settings_modal_overlay_with_escape, settings_muted_text_style,
    settings_select, settings_tab_button, settings_text_editor, settings_text_input,
};
use iced::widget::{button, column, container, row, scrollable, text, text_editor};
use iced::{Alignment, Element, Length, Task};
use runtime_time::zones::NamedTimeZone;
use std::sync::Arc;

#[derive(Default)]
pub(super) struct State {
    account_revision: u64,
    page: usize,
    plans: Option<Arc<task_api::Schedules>>,
    list_loading: bool,
    list_revision: u64,
    list_error: Option<Feedback>,
    directory: Option<Arc<task_api::Directory>>,
    directory_loading: bool,
    directory_error: Option<Feedback>,
    reading: Option<String>,
    editor: Option<Editor>,
}

impl State {
    pub(super) fn reset_account(&self) -> Self {
        Self { account_revision: self.account_revision.wrapping_add(1), ..Self::default() }
    }
}

struct Editor {
    from_cron_form: bool,
    existing: Option<task_api::Schedule>,
    draft: tasks::Draft,
    cron: String,
    timezone: String,
    employee_search: Option<String>,
    saving: bool,
    error: Option<Feedback>,
}

#[derive(Clone)]
pub(crate) enum Event {
    Refresh,
    ReloadDirectory,
    Page(usize),
    New { from_cron_form: bool },
    Edit(String),
    Close,
    DirectoryLoaded(u64, Result<Arc<task_api::Directory>, Feedback>),
    ListLoaded(u64, u64, Result<Arc<task_api::Schedules>, Feedback>),
    Loaded(u64, String, Result<task_api::Schedule, Feedback>),
    Title(String),
    Prompt(text_editor::Action),
    Workflow(String),
    Cron(String),
    Timezone(String),
    Trigger(tasks::Trigger),
    EmployeePicker(bool),
    EmployeeSearch(String),
    Employee(String),
    OpenTime,
    Time(PickerEvent),
    Save,
    Saved(u64, Result<(), Feedback>),
}

fn message(event: Event) -> Message {
    Message::DigitalEmployees(super::Event::CronSchedules(event))
}

pub(crate) fn refresh(app: &mut App) -> Task<Message> {
    let state = &mut app.digital_employees.cron_schedules;
    let Some(account) = app.account.active_account() else { return Task::none() };
    if state.list_loading {
        return Task::none();
    }
    state.list_loading = true;
    state.list_error = None;
    state.list_revision = state.list_revision.wrapping_add(1);
    let (revision, list_revision, page) = (state.account_revision, state.list_revision, state.page);
    let key = account.skill_hub_credential();
    Task::perform(
        async move { task_api::schedules(key.expose(), page).await.map(Arc::new) },
        move |result| message(Event::ListLoaded(revision, list_revision, result)),
    )
}

fn load_directory(app: &mut App) -> Task<Message> {
    let state = &mut app.digital_employees.cron_schedules;
    let Some(account) = app.account.active_account() else { return Task::none() };
    if state.directory_loading {
        return Task::none();
    }
    state.directory_loading = true;
    state.directory_error = None;
    let revision = state.account_revision;
    let key = account.skill_hub_credential();
    Task::perform(
        async move { task_api::directory(key.expose()).await.map(Arc::new) },
        move |result| message(Event::DirectoryLoaded(revision, result)),
    )
}

fn new_editor(app: &App, from_cron_form: bool) -> Editor {
    let mut draft = tasks::Draft::new(tasks::Trigger::Cron, None, String::new());
    let mut cron = "0 9 * * *".to_owned();
    let mut timezone = super::task_schedule::TIMEZONE.name().to_owned();
    if from_cron_form {
        let source = &app.cron_settings.add_draft;
        draft.title.clone_from(&source.name);
        draft.prompt = text_editor::Content::with_text(&source.prompt);
        cron.clear();
        let schedule = source.effective_schedule();
        if let Ok(rule) = schedule.request()
            && !rule.starts_at.as_ref().is_some_and(Option::is_some)
            && !rule.ends_at.as_ref().is_some_and(Option::is_some)
            && !rule.start_date.as_ref().is_some_and(Option::is_some)
            && !rule.end_date.as_ref().is_some_and(Option::is_some)
        {
            if let Some(zone) = rule.timezone {
                timezone = zone;
            }
            if rule.schedule_kind == "at" {
                draft.trigger = tasks::Trigger::Once;
                let zone = timezone.parse::<NamedTimeZone>().ok();
                draft.once_at = rule
                    .local_at
                    .as_deref()
                    .and_then(|value| {
                        chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M").ok()
                    })
                    .or_else(|| {
                        let value = DateTime::parse_from_rfc3339(rule.at.as_deref()?).ok()?;
                        Some(value.with_timezone(&zone?).naive_local())
                    });
            } else if let CronScheduleRule::Weekly { weekdays, time } = schedule.rule {
                use chrono::Timelike;
                let days = weekdays.iter().map(|day| (day % 7).to_string()).collect::<Vec<_>>();
                cron = format!("{} {} * * {}", time.minute(), time.hour(), days.join(","));
            } else if let Some(expression) =
                rule.schedule.filter(|value| value.split_whitespace().count() == 5)
            {
                cron = expression;
            }
        }
    }
    Editor {
        from_cron_form,
        existing: None,
        draft,
        cron,
        timezone,
        employee_search: Some(String::new()),
        saving: false,
        error: None,
    }
}

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    let state = &mut app.digital_employees.cron_schedules;
    match event {
        Event::ListLoaded(revision, list_revision, result)
            if revision == state.account_revision
                && list_revision == state.list_revision
                && state.list_loading =>
        {
            state.list_loading = false;
            match result {
                Ok(plans) => state.plans = Some(plans),
                Err(error) => state.list_error = Some(error),
            }
        }
        Event::DirectoryLoaded(revision, result) if revision == state.account_revision => {
            state.directory_loading = false;
            match result {
                Ok(directory) => state.directory = Some(directory),
                Err(error) => {
                    state.directory = None;
                    state.directory_error = Some(error);
                }
            }
        }
        Event::Loaded(revision, id, result)
            if revision == state.account_revision
                && state.reading.as_deref() == Some(id.as_str()) =>
        {
            state.reading = None;
            let schedule = match result {
                Ok(schedule)
                    if schedule.schedule_id == id
                        && system_settings_cron::visible(app)
                        && app.cron_settings.active_tab == CronSettingsTab::Employee =>
                {
                    schedule
                }
                Ok(_) => return Task::none(),
                Err(error) => {
                    app.digital_employees.cron_schedules.list_error = Some(error);
                    return Task::none();
                }
            };
            let Some(cron) = schedule.cron.clone().filter(|value| !value.trim().is_empty()) else {
                return Task::none();
            };
            let timezone = schedule.timezone.clone().unwrap_or_else(|| "UTC".into());
            let mut draft = tasks::Draft::new(tasks::Trigger::Cron, None, String::new());
            draft.title.clone_from(&schedule.title);
            draft.employee = schedule.input_text("employee_id").unwrap_or_default().to_owned();
            draft.workflow = schedule.input_text("workflow_id").unwrap_or_default().to_owned();
            draft.prompt =
                text_editor::Content::with_text(schedule.input_text("prompt").unwrap_or_default());
            app.digital_employees.cron_schedules.editor = Some(Editor {
                from_cron_form: false,
                existing: Some(schedule),
                draft,
                cron,
                timezone,
                employee_search: None,
                saving: false,
                error: None,
            });
            return Task::batch([shared::refresh(app), load_directory(app)]);
        }
        Event::Saved(revision, result) if revision == state.account_revision => {
            let Some(editor) = state.editor.as_mut().filter(|editor| editor.saving) else {
                return Task::none();
            };
            if let Err(error) = result {
                editor.saving = false;
                editor.error = Some(error);
                return Task::none();
            }
            let from_cron_form = editor.from_cron_form;
            if editor.existing.is_none() {
                state.page = 0;
            }
            state.editor = None;
            state.list_loading = false;
            let toast = app.show_success_toast("员工排程已保存");
            if from_cron_form && system_settings_cron::visible(app) {
                return Task::batch([
                    toast,
                    Task::done(Message::Settings(SettingsMessage::CronJobEditCanceled)),
                    Task::done(Message::Settings(SettingsMessage::CronTabSelected(
                        CronSettingsTab::Employee,
                    ))),
                ]);
            }
            return Task::batch([toast, refresh(app)]);
        }
        Event::ListLoaded(..)
        | Event::DirectoryLoaded(..)
        | Event::Loaded(..)
        | Event::Saved(..) => {}
        event => {
            if !system_settings_cron::visible(app) {
                return Task::none();
            }
            return interact(app, event);
        }
    }
    Task::none()
}

fn interact(app: &mut App, event: Event) -> Task<Message> {
    let state = &mut app.digital_employees.cron_schedules;
    match event {
        Event::Refresh => return refresh(app),
        Event::ReloadDirectory => return load_directory(app),
        Event::Page(page) if !state.list_loading => {
            state.page = page;
            state.plans = None;
            return refresh(app);
        }
        Event::New { from_cron_form } if state.editor.is_none() && state.reading.is_none() => {
            if app.account.active_account().is_none() {
                return app.show_error_toast("请先登录识矩");
            }
            if from_cron_form
                && (!app.cron_settings.show_add_modal || app.cron_settings.editing_job_id.is_some())
            {
                return Task::none();
            }
            app.digital_employees.cron_schedules.editor = Some(new_editor(app, from_cron_form));
            return Task::batch([shared::refresh(app), load_directory(app)]);
        }
        Event::Edit(id) if state.editor.is_none() && state.reading.is_none() => {
            let Some(account) = app.account.active_account() else { return Task::none() };
            let revision = state.account_revision;
            let key = account.skill_hub_credential();
            state.reading = Some(id.clone());
            let request = id.clone();
            return Task::perform(
                async move { task_api::schedule(key.expose(), &request).await },
                move |result| message(Event::Loaded(revision, id, result)),
            );
        }
        Event::Save => return save(app),
        Event::Employee(id)
            if chat_picker::schedule_choices(app)
                .iter()
                .any(|choice| choice.selection.target.employee_id == id) =>
        {
            let department = app
                .digital_employees
                .shared
                .directory
                .as_ref()
                .and_then(|directory| {
                    directory.employees.iter().find(|employee| employee.employee_id == id)
                })
                .and_then(|employee| employee.department())
                .unwrap_or_default()
                .to_owned();
            let state = &mut app.digital_employees.cron_schedules;
            if let Some(editor) =
                state.editor.as_mut().filter(|editor| !editor.saving && editor.existing.is_none())
            {
                if editor.draft.employee != id || editor.draft.department != department {
                    editor.draft.employee = id;
                    editor.draft.department = department;
                    editor.draft.key = tasks::new_key();
                }
                editor.employee_search = None;
                editor.error = None;
            }
        }
        event => {
            let Some(editor) = app
                .digital_employees
                .cron_schedules
                .editor
                .as_mut()
                .filter(|editor| !editor.saving)
            else {
                return Task::none();
            };
            if matches!(event, Event::Close) {
                app.digital_employees.cron_schedules.editor = None;
                return Task::none();
            }
            match event {
                Event::Title(value) if editor.existing.is_none() && editor.draft.title != value => {
                    editor.draft.title = value
                }
                Event::Prompt(action) if editor.existing.is_none() => {
                    let before = action.is_edit().then(|| editor.draft.prompt.text());
                    editor.draft.prompt.perform(action);
                    if before.is_none_or(|before| before == editor.draft.prompt.text()) {
                        return Task::none();
                    }
                }
                Event::Workflow(id) if editor.existing.is_none() && editor.draft.workflow != id => {
                    editor.draft.workflow = id
                }
                Event::Cron(value) if editor.cron != value => editor.cron = value,
                Event::Timezone(value) if editor.timezone != value => editor.timezone = value,
                Event::Trigger(trigger)
                    if editor.existing.is_none() && editor.draft.trigger != trigger =>
                {
                    editor.draft.trigger = trigger
                }
                Event::EmployeePicker(open) if editor.existing.is_none() => {
                    editor.employee_search = open.then(String::new);
                    return if open { shared::refresh(app) } else { Task::none() };
                }
                Event::EmployeeSearch(query) if editor.employee_search.is_some() => {
                    editor.employee_search = Some(query);
                    return Task::none();
                }
                Event::OpenTime if editor.existing.is_none() => {
                    let Some(local) = editor.draft.once_at.or_else(|| {
                        let zone = editor.timezone.parse::<NamedTimeZone>().ok()?;
                        Some(Utc::now().with_timezone(&zone).naive_local())
                    }) else {
                        editor.error = Some("请选择有效的 IANA 时区".into());
                        return Task::none();
                    };
                    editor.draft.once_picker =
                        Some(DateTimePicker::new(false, local.date(), local.time()));
                    return Task::none();
                }
                Event::Time(PickerEvent::Cancel) => editor.draft.once_picker = None,
                Event::Time(PickerEvent::Confirm) => {
                    if let Some(picker) = editor.draft.once_picker.take() {
                        let selected = Some(picker.date.and_time(picker.time));
                        if selected == editor.draft.once_at {
                            return Task::none();
                        }
                        editor.draft.once_at = selected;
                    }
                }
                Event::Time(event) => {
                    if let Some(picker) = &mut editor.draft.once_picker {
                        use chrono::Timelike;
                        match event {
                            PickerEvent::Navigate(months) => picker.navigate(months),
                            PickerEvent::Date(date) => picker.select(date),
                            PickerEvent::Time(value) => {
                                picker.set_time(value.hour(), value.minute())
                            }
                            _ => {}
                        }
                    }
                    return Task::none();
                }
                _ => return Task::none(),
            }
            editor.draft.key = tasks::new_key();
            editor.error = None;
        }
    }
    Task::none()
}

fn save(app: &mut App) -> Task<Message> {
    use chrono::TimeZone;
    let Some(account) = app.account.active_account() else {
        return app.show_error_toast("请先登录识矩");
    };
    let key = account.skill_hub_credential();
    let eligible = app.digital_employees.cron_schedules.editor.as_ref().is_some_and(|editor| {
        chat_picker::schedule_choices(app)
            .iter()
            .any(|choice| choice.selection.target.employee_id == editor.draft.employee)
    });
    let state = &mut app.digital_employees.cron_schedules;
    let Some(editor) = state.editor.as_mut().filter(|editor| !editor.saving) else {
        return Task::none();
    };
    let zone = editor.timezone.trim().parse::<NamedTimeZone>();
    let Ok(zone) = zone else {
        editor.error = Some("请选择有效的 IANA 时区".into());
        return Task::none();
    };
    let timezone = zone.name().to_owned();
    let cron = editor.cron.split_whitespace().collect::<Vec<_>>().join(" ");
    if editor.draft.trigger == tasks::Trigger::Cron && cron.split_whitespace().count() != 5 {
        editor.error = Some("请输入五字段 Cron 表达式".into());
        return Task::none();
    }

    let revision = state.account_revision;
    if let Some(existing) = &editor.existing {
        let Some(version) = existing.version else { return Task::none() };
        let id = existing.schedule_id.clone();
        editor.saving = true;
        editor.error = None;
        return Task::perform(
            async move {
                task_api::update_schedule(key.expose(), &id, version, &cron, &timezone)
                    .await
                    .map(|_| ())
            },
            move |result| message(Event::Saved(revision, result)),
        );
    }
    let validate = || -> Result<(String, Option<String>), &'static str> {
        if editor.draft.title.trim().is_empty() {
            return Err("请填写任务名称");
        }
        if !eligible {
            return Err("请选择可派工的共享员工");
        }
        let version = state
            .directory
            .as_ref()
            .and_then(|directory| {
                directory
                    .workflows
                    .iter()
                    .find(|workflow| workflow.workflow_id == editor.draft.workflow)
            })
            .and_then(task_api::Workflow::version)
            .ok_or("请选择已发布的可用流程")?;
        let once = if editor.draft.trigger == tasks::Trigger::Once {
            let local = editor.draft.once_at.ok_or("请选择执行日期和时间")?;
            let at = zone
                .from_local_datetime(&local)
                .single()
                .ok_or("所选日期时间在该时区无唯一对应时间")?;
            if at.with_timezone(&Utc) <= Utc::now() {
                return Err("执行时间必须晚于当前时间");
            }
            Some(at.to_rfc3339())
        } else {
            None
        };
        Ok((version.to_owned(), once))
    };
    let (version, once) = match validate() {
        Ok(values) => values,
        Err(error) => {
            editor.error = Some(error.into());
            return Task::none();
        }
    };
    editor.saving = true;
    editor.error = None;
    let draft = editor.draft.clone();
    let cron = (draft.trigger == tasks::Trigger::Cron).then_some(cron);
    Task::perform(
        async move {
            task_api::create_with_timing(key.expose(), draft, version, once, cron, &timezone).await
        },
        move |result| message(Event::Saved(revision, result)),
    )
}

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.digital_employees.cron_schedules;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    if app.account.active_account().is_none() {
        return text(i18n.__("请先登录识矩")).style(settings_muted_text_style).into();
    }
    let actions = row![
        secondary(i18n.__("刷新"), Event::Refresh, !state.list_loading),
        button(text(i18n.__("新建定时任务")).size(13))
            .padding(SETTINGS_CONTROL_PADDING)
            .style(primary_action_btn_style)
            .on_press(message(Event::New { from_cron_form: false })),
    ]
    .spacing(8);
    let mut body = column![actions].spacing(12).width(Length::Fill);
    if let Some(error) = &state.list_error {
        body = body.push(settings_error_banner(error.render(i18n)));
    }
    if state.list_loading || state.reading.is_some() {
        body = body.push(text(i18n.__("加载中…")).size(13).style(settings_muted_text_style));
    }
    if let Some(plans) = &state.plans {
        if plans.schedules.is_empty() {
            body = body.push(text(i18n.__("暂无定时任务")).style(settings_muted_text_style));
        }
        let entries = column(plans.schedules.iter().map(|plan| {
            let can_edit = state.reading.is_none()
                && plan.version.is_some_and(|v| v > 0)
                && plan.cron.as_ref().is_some_and(|cron| !cron.is_empty());
            let detail =
                [plan.cron.as_deref(), plan.timezone.as_deref(), plan.next_fire_at.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" · ");
            let label = column![
                text(&plan.title).size(15),
                text(detail).size(12).style(settings_muted_text_style)
            ]
            .spacing(4)
            .width(Length::Fill);
            row![
                label,
                secondary(i18n.__("编辑"), Event::Edit(plan.schedule_id.clone()), can_edit),
            ].spacing(10).align_y(Alignment::Center).into()
        }))
        .spacing(12);
        body = body.push(
            scrollable(container(entries).padding([0, 7]))
                .direction(scrollable::Direction::Vertical(
                    scrollable::Scrollbar::new().width(4).scroller_width(4),
                ))
                .height(Length::Fill),
        );
        let next = plans.total.map_or(plans.schedules.len() >= task_api::PAGE_SIZE, |total| {
            (state.page + 1).saturating_mul(task_api::PAGE_SIZE) < total
        });
        if state.page > 0 || next {
            body = body.push(
                row![
                    secondary(
                        i18n.__("上一页"),
                        Event::Page(state.page.saturating_sub(1)),
                        state.page > 0
                    ),
                    secondary(i18n.__("下一页"), Event::Page(state.page + 1), next),
                ]
                .spacing(8),
            );
        }
    }
    body.into()
}

pub(crate) fn with_overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let state = &app.digital_employees.cron_schedules;
    let Some(editor) = &state.editor else { return base };
    if !editor.saving
        && (if editor.from_cron_form {
            !app.cron_settings.show_add_modal || app.cron_settings.editing_job_id.is_some()
        } else {
            app.cron_settings.active_tab != CronSettingsTab::Employee
        })
    {
        return base;
    }
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let new = editor.existing.is_none();
    let can_edit = !editor.saving;
    let close = if can_edit { message(Event::Close) } else { Message::None };
    let header = settings_modal_header(
        text(i18n.__(if new { "新建定时任务" } else { "编辑定时任务" })).size(20),
        settings_close_button(close.clone()),
    );
    let employee = app
        .digital_employees
        .shared
        .directory
        .as_ref()
        .and_then(|directory| {
            directory
                .employees
                .iter()
                .find(|employee| employee.employee_id == editor.draft.employee)
        })
        .map_or(editor.draft.employee.as_str(), |employee| employee.name.as_str());
    let employee = if employee.is_empty() { i18n.__("选择数字员工") } else { employee };
    let mut fields = column![
        settings_field_label(i18n.__("任务名称")),
        settings_text_input(i18n.__("输入任务名称"), &editor.draft.title)
            .on_input_maybe((can_edit && new).then_some(|value| message(Event::Title(value)))),
        settings_field_label(i18n.__("数字员工")),
        secondary(employee, Event::EmployeePicker(true), can_edit && new),
    ]
    .spacing(9)
    .width(Length::Fill);
    if new {
        let flows: Vec<_> = state
            .directory
            .iter()
            .flat_map(|directory| &directory.workflows)
            .filter(|workflow| workflow.version().is_some())
            .collect();
        let selected = (!editor.draft.workflow.is_empty()).then(|| editor.draft.workflow.clone());
        fields = fields.push(settings_field_label(i18n.__("工作流程"))).push(
            settings_select(flows.iter().map(|flow| flow.workflow_id.clone()), selected, |id| {
                message(Event::Workflow(id))
            })
            .format_labels(|id| {
                flows
                    .iter()
                    .find(|flow| &flow.workflow_id == id)
                    .map_or_else(|| id.clone(), |flow| flow.name.clone())
            })
            .placeholder(i18n.__("请选择已发布的可用流程"))
            .roomy_menu(true)
            .width(Length::Fill)
            .disabled(!can_edit),
        );
        if let Some(error) = &state.directory_error {
            fields = fields.push(settings_error_banner(error.render(i18n))).push(secondary(
                i18n.__("重试"),
                Event::ReloadDirectory,
                !state.directory_loading,
            ));
        }
        let mut prompt = settings_text_editor(&editor.draft.prompt)
            .height(92)
            .placeholder(i18n.__("要让 Agent 执行的任务"));
        if can_edit {
            prompt = prompt.on_action(|action| message(Event::Prompt(action)));
        }
        fields = fields.push(prompt);
        let current = editor.draft.trigger;
        fields = fields.push(
            row![
                settings_tab_button(
                    i18n.__("单次"),
                    current == tasks::Trigger::Once,
                    (can_edit && current != tasks::Trigger::Once)
                        .then(|| message(Event::Trigger(tasks::Trigger::Once)))
                ),
                settings_tab_button(
                    i18n.__("周期"),
                    current == tasks::Trigger::Cron,
                    (can_edit && current != tasks::Trigger::Cron)
                        .then(|| message(Event::Trigger(tasks::Trigger::Cron)))
                ),
            ]
            .spacing(6),
        );
    } else {
        let workflow = state
            .directory
            .as_ref()
            .and_then(|directory| {
                directory
                    .workflows
                    .iter()
                    .find(|workflow| workflow.workflow_id == editor.draft.workflow)
            })
            .map_or(editor.draft.workflow.as_str(), |workflow| workflow.name.as_str());
        fields = fields
            .push(settings_field_label(i18n.__("工作流程")))
            .push(text(workflow).size(13))
            .push(
                text(i18n.__("此排程可以修改周期和时区。"))
                    .size(12)
                    .style(settings_muted_text_style),
            );
    }
    fields = match editor.draft.trigger {
        tasks::Trigger::Once => fields.push(secondary(
            editor
                .draft
                .once_at
                .map(|at| at.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| i18n.__("选择日期和时间").to_owned()),
            Event::OpenTime,
            can_edit,
        )),
        _ => fields.push(
            settings_text_input("0 9 * * *", &editor.cron)
                .on_input_maybe(can_edit.then_some(|value| message(Event::Cron(value)))),
        ),
    };
    fields = fields.push(settings_field_label(i18n.__("时区（IANA）"))).push(
        settings_text_input("Asia/Shanghai", &editor.timezone)
            .on_input_maybe(can_edit.then_some(|value| message(Event::Timezone(value)))),
    );
    let mut footer = column![].spacing(8);
    if let Some(error) = &editor.error {
        footer = footer.push(settings_error_banner(error.render(i18n)));
    }
    let actions = row![
        secondary(i18n.__("取消"), Event::Close, can_edit),
        button(text(i18n.__(if can_edit { "保存" } else { "正在保存…" })).size(13))
            .padding(SETTINGS_CONTROL_PADDING)
            .style(primary_action_btn_style)
            .on_press_maybe(
                (can_edit && (!new || state.directory.is_some())).then(|| message(Event::Save))
            ),
    ]
    .spacing(8);
    let card = settings_modal_layout(
        header,
        fields,
        Some(footer.push(container(actions).align_right(Length::Fill)).into()),
    )
    .width(660)
    .height(620);
    let base = settings_modal_overlay_with_escape(base, close, card);
    if let Some(query) = &editor.employee_search {
        return employee_picker(app, editor, query, base);
    }
    if let Some(picker) = &editor.draft.once_picker {
        return picker_overlay_with_i18n(i18n, picker, editor.timezone.clone(), base, |event| {
            message(Event::Time(event))
        });
    }
    base
}

fn employee_picker<'a>(
    app: &'a App,
    editor: &Editor,
    query: &str,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let close = message(Event::EmployeePicker(false));
    let header = column![
        settings_modal_header(
            text(i18n.__("选择数字员工")).size(18),
            settings_close_button(close.clone())
        ),
        settings_field_label(i18n.__("共享员工")),
        settings_text_input(i18n.__("搜索姓名、岗位或部门"), query)
            .on_input(|query| message(Event::EmployeeSearch(query))),
    ]
    .spacing(8);
    let query = query.trim().to_lowercase();
    let choices: Vec<_> = chat_picker::schedule_choices(app)
        .into_iter()
        .filter(|choice| choice.matches(&query))
        .collect();
    let mut body = column![].spacing(10);
    let shared = &app.digital_employees.shared;
    if let Some(error) = &shared.error {
        body = body.push(settings_error_banner(error.render(i18n))).push(secondary(
            i18n.__("重试"),
            Event::EmployeePicker(true),
            true,
        ));
    } else if choices.is_empty() {
        body = body.push(
            text(i18n.__(if shared.loading {
                "正在加载员工…"
            } else {
                "没有匹配的可派工共享员工"
            }))
            .size(13)
            .style(settings_muted_text_style),
        );
    }
    let cards = row(choices.into_iter().map(|choice| {
        let id = choice.selection.target.employee_id.clone();
        chat_picker::employee_card(
            choice,
            editor.draft.employee == id,
            Some(message(Event::Employee(id))),
        )
    }))
    .spacing(8)
    .wrap();
    let card = settings_modal_layout(header, body.push(cards), None).width(620).height(480);
    settings_modal_overlay_with_escape(base, close, card)
}

fn secondary<'a>(label: impl Into<String>, event: Event, enabled: bool) -> Element<'a, Message> {
    button(text(label.into()).size(13))
        .padding(SETTINGS_CONTROL_PADDING)
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then(|| message(event)))
        .into()
}
