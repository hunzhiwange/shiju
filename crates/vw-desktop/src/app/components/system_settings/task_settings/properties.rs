//! 当前项目的任务属性目录。

use super::catalog;
use crate::app::components::system_settings_common::*;
use crate::app::i18n::I18n;
use crate::app::message::task_board::{
    next_agent_scheduler_status_generation, resolve_task_board_scheduler_scope,
};
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::system_settings_common::panels::settings_responsive_row;
use iced::widget::{button, column, container, operation, row, text, text_editor};
use iced::{Alignment, Element, Length, Task};
use vw_api_agent_scheduler::{
    AgentSchedulerTaskPropertyDto as Property, AgentSchedulerTaskPropertyIcon as PropertyIcon,
    AgentSchedulerTaskPropertyKind as Kind, AgentSchedulerTaskPropertyMutation as Mutation,
    AgentSchedulerTaskPropertyOption as PropertyOption,
    AgentSchedulerTaskPropertyOptionChange as OptionChange,
    AgentSchedulerTaskPropertyOptionInput as OptionInput,
};

const NAME_INPUT: &str = "task-property-setting-name";

#[derive(Default)]
pub(in crate::app) struct State {
    project_path: Option<String>,
    properties: Vec<Property>,
    query: String,
    show_archived: bool,
    draft: Option<Draft>,
    pending: Option<u64>,
    saving: bool,
    error: Option<String>,
}

impl State {
    pub(super) fn saving(&self) -> bool {
        self.saving
    }
}

struct Draft {
    original: Option<Property>,
    name: String,
    kind: Kind,
    icon: PropertyIcon,
    description: text_editor::Content,
    options: Vec<OptionDraft>,
    picker: Option<Picker>,
}

#[derive(Default)]
struct OptionDraft {
    original: Option<PropertyOption>,
    name: String,
    color: Option<String>,
    archive: bool,
}

impl Draft {
    fn valid(&self) -> bool {
        !self.name.trim().is_empty()
            && (!self.kind.has_options()
                || ((!self.options.is_empty() || self.original.is_some())
                    && self.options.iter().all(|option| {
                        !option.name.trim().is_empty()
                            && option
                                .color
                                .as_deref()
                                .is_none_or(|color| catalog::palette_color(color).is_some())
                    })))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picker {
    Icon,
    Color(usize),
}

#[derive(Debug, Clone)]
pub enum Event {
    Refresh,
    New,
    Edit(String),
    NameChanged(String),
    KindChanged(Kind),
    IconChanged(PropertyIcon),
    DescriptionChanged(text_editor::Action),
    Picker(Option<Picker>),
    OptionColor(usize, String),
    Search(String),
    ShowArchived(bool),
    OptionChanged(usize, String),
    AddOption,
    RemoveOption(usize),
    ToggleOptionArchive(usize),
    Close,
    Save,
    Archive(String),
    Loaded {
        project_path: String,
        request_id: u64,
        changed: bool,
        result: Result<Vec<Property>, String>,
    },
}

pub(in crate::app) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Loaded { project_path, request_id, changed, result } = event {
        let selected = super::project_path(app) == Some(project_path.as_str());
        let state = &mut app.task_property_settings;
        if state.pending != Some(request_id)
            || state.project_path.as_deref() != Some(project_path.as_str())
        {
            return Task::none();
        }
        state.pending = None;
        state.saving = false;
        match result {
            Ok(properties) => {
                state.properties = properties;
                if changed {
                    state.draft = None;
                }
                let generation =
                    if changed { next_agent_scheduler_status_generation() } else { request_id };
                if app.navigation_tabs.project_path.as_deref() == Some(project_path.as_str())
                    && generation >= app.task_board.task_properties_generation
                {
                    app.task_board.task_properties.clone_from(&state.properties);
                    app.task_board.task_properties_generation = generation;
                }
                if changed && selected {
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Success,
                        "属性目录已保存",
                        vec![],
                    );
                }
            }
            Err(error) => {
                state.error = Some(error.clone());
                if changed && selected {
                    return app.show_error_toast(error);
                }
            }
        }
        return Task::none();
    }

    let Some(project_path) = super::project_path(app).map(str::to_owned) else {
        return Task::none();
    };
    let state = &mut app.task_property_settings;
    if state.project_path.as_deref() != Some(project_path.as_str()) {
        if !matches!(event, Event::Refresh) {
            return Task::none();
        }
        *state = State { project_path: Some(project_path.clone()), ..State::default() };
    }
    match event {
        Event::Search(value) => {
            state.query = value;
            return Task::none();
        }
        Event::ShowArchived(show) => {
            state.show_archived = show;
            return Task::none();
        }
        _ => {}
    }
    if state.pending.is_some() {
        return Task::none();
    }

    let mutation = match event {
        Event::Refresh => None,
        Event::Archive(id) => Some(Mutation::Archive { id }),
        Event::Save => {
            let Some(draft) = state.draft.as_ref() else {
                return Task::none();
            };
            if !draft.valid() {
                return Task::none();
            }
            let name = draft.name.trim().to_owned();
            let description = draft.description.text().trim().to_owned();
            let new_options = draft
                .options
                .iter()
                .filter(|option| option.original.is_none() && draft.kind.has_options())
                .map(|option| {
                    let name = option.name.trim().to_owned();
                    match option.color.as_ref() {
                        Some(color) => OptionInput::Colored { name, color: color.clone() },
                        None => OptionInput::Name(name),
                    }
                })
                .collect();
            Some(if let Some(original) = draft.original.as_ref() {
                let option_changes = draft
                    .options
                    .iter()
                    .filter_map(|option| {
                        let original = option.original.as_ref()?;
                        let name = (option.name.trim() != original.name)
                            .then(|| option.name.trim().to_owned());
                        let color = (option.color != original.color)
                            .then(|| option.color.clone())
                            .flatten();
                        (name.is_some() || color.is_some() || option.archive).then(|| {
                            (
                                original.id.clone(),
                                OptionChange { name, color, archive: option.archive },
                            )
                        })
                    })
                    .collect();
                Mutation::Update {
                    id: original.id.clone(),
                    name: (name != original.name).then_some(name),
                    icon: (draft.icon != original.icon).then_some(draft.icon),
                    description: (description != original.description).then_some(description),
                    new_options,
                    option_changes,
                }
            } else {
                Mutation::Create {
                    name,
                    kind: draft.kind,
                    icon: draft.icon,
                    description,
                    options: new_options,
                }
            })
        }
        Event::New => {
            state.error = None;
            state.draft = Some(Draft {
                original: None,
                name: String::new(),
                kind: Kind::SingleSelect,
                icon: PropertyIcon::default(),
                description: text_editor::Content::new(),
                options: vec![new_option()],
                picker: None,
            });
            return operation::focus(NAME_INPUT);
        }
        Event::Edit(id) => {
            if let Some(property) =
                state.properties.iter().find(|property| property.id == id && !property.archived)
            {
                state.error = None;
                state.draft = Some(Draft {
                    original: Some(property.clone()),
                    name: property.name.clone(),
                    kind: property.kind,
                    icon: property.icon,
                    description: text_editor::Content::with_text(&property.description),
                    picker: None,
                    options: property
                        .options
                        .iter()
                        .map(|option| OptionDraft {
                            original: Some(option.clone()),
                            name: option.name.clone(),
                            color: option.color.clone(),
                            archive: false,
                        })
                        .collect(),
                });
                return operation::focus(NAME_INPUT);
            }
            return Task::none();
        }
        Event::Close => {
            state.draft = None;
            state.error = None;
            return Task::none();
        }
        event => {
            if let Some(draft) = state.draft.as_mut() {
                match event {
                    Event::NameChanged(value) => draft.name = value,
                    Event::KindChanged(kind) if draft.original.is_none() => draft.kind = kind,
                    Event::IconChanged(icon) => {
                        draft.icon = icon;
                        draft.picker = None;
                    }
                    Event::Picker(picker) => draft.picker = picker,
                    Event::DescriptionChanged(action) => draft.description.perform(action),
                    Event::AddOption => draft.options.push(new_option()),
                    Event::OptionColor(index, color) => {
                        if let Some(option) = draft.options.get_mut(index) {
                            option.color = Some(color);
                        }
                    }
                    Event::RemoveOption(index) => {
                        if draft.options.get(index).is_some_and(|option| option.original.is_none())
                            && (draft.original.is_some() || draft.options.len() > 1)
                        {
                            draft.options.remove(index);
                            draft.picker = None;
                        }
                    }
                    Event::ToggleOptionArchive(index) => {
                        if let Some(option) = draft.options.get_mut(index).filter(|option| {
                            option.original.as_ref().is_some_and(|original| !original.archived)
                        }) {
                            option.archive = !option.archive;
                        }
                    }
                    Event::OptionChanged(index, value) => {
                        if let Some(option) = draft.options.get_mut(index) {
                            option.name = value;
                        }
                    }
                    _ => {}
                }
            }
            return Task::none();
        }
    };

    let request_id = next_agent_scheduler_status_generation();
    state.pending = Some(request_id);
    state.error = None;
    let changed = mutation.is_some();
    state.saving = changed;
    let result_project = project_path.clone();
    Task::perform(
        async move {
            let scope = resolve_task_board_scheduler_scope(&project_path).await?;
            let client = crate::app::config::gateway_client()?;
            if let Some(mutation) = mutation {
                client.agent_scheduler_mutate_task_properties(&scope, &mutation).await
            } else {
                client.agent_scheduler_task_properties(&scope).await
            }
        },
        move |result| {
            Message::TaskPropertySettings(Event::Loaded {
                project_path: result_project.clone(),
                request_id,
                changed,
                result,
            })
        },
    )
}

fn new_option() -> OptionDraft {
    OptionDraft { color: Some(catalog::DEFAULT_COLOR.to_owned()), ..Default::default() }
}

const KINDS: [Kind; 11] = [
    Kind::Text,
    Kind::Number,
    Kind::SingleSelect,
    Kind::MultiSelect,
    Kind::Date,
    Kind::Checkbox,
    Kind::Link,
    Kind::Member,
    Kind::MultiMember,
    Kind::TextList,
    Kind::LinkList,
];

fn kind_label(i18n: I18n, kind: Kind) -> &'static str {
    i18n.__(match kind {
        Kind::SingleSelect => "单选",
        Kind::MultiSelect => "多选",
        Kind::Date => "日期",
        Kind::Text => "文本",
        Kind::Number => "数字",
        Kind::Checkbox => "勾选",
        Kind::Link => "链接",
        Kind::Member => "成员",
        Kind::MultiMember => "成员（多选）",
        Kind::TextList => "文本列表",
        Kind::LinkList => "链接列表",
    })
}

const ICONS: [(PropertyIcon, Icon, &str); 12] = [
    (PropertyIcon::Sliders, Icon::Sliders, "属性"),
    (PropertyIcon::Star, Icon::Star, "星标"),
    (PropertyIcon::Person, Icon::Person, "成员"),
    (PropertyIcon::Calendar, Icon::Calendar, "日期"),
    (PropertyIcon::Check, Icon::CheckSquare, "勾选"),
    (PropertyIcon::Chart, Icon::BarChartFill, "图表"),
    (PropertyIcon::Link, Icon::Link, "链接"),
    (PropertyIcon::Clock, Icon::Clock, "时间"),
    (PropertyIcon::List, Icon::ListUl, "列表"),
    (PropertyIcon::Globe, Icon::Globe, "网页"),
    (PropertyIcon::Shield, Icon::ShieldLock, "安全"),
    (PropertyIcon::Box, Icon::Box, "组件"),
];

pub(in crate::app) fn property_icon(icon: PropertyIcon) -> Icon {
    ICONS.iter().find(|(key, _, _)| *key == icon).map_or(Icon::Sliders, |(_, glyph, _)| *glyph)
}

fn message(event: Event) -> Message {
    Message::TaskPropertySettings(event)
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let mut content = column![settings_page_heading(
        i18n.__("属性"),
        i18n.__("定义项目任务可填写和筛选的字段。"),
    )]
    .spacing(18);
    let Some(project) = super::project_path(app) else {
        return content.into();
    };
    let state = &app.task_property_settings;
    if state.project_path.as_deref() != Some(project) {
        return content
            .push(catalog::action(i18n.__("加载属性"), Some(message(Event::Refresh))))
            .into();
    }
    let enabled = state.pending.is_none();
    let query = state.query.trim().to_lowercase();
    let search = settings_text_input(i18n.__("按名称或描述搜索"), &state.query)
        .on_input(|value| message(Event::Search(value)));
    content = content.push(catalog::toolbar(
        search.into(),
        catalog::primary(
            i18n.__("新建属性"),
            Some(Icon::Plus),
            enabled.then_some(message(Event::New)),
        ),
    ));
    content = content.push(
        settings_toggle(state.show_archived)
            .label(i18n.__("显示已归档"))
            .text_size(13)
            .on_toggle(|show| message(Event::ShowArchived(show))),
    );
    if let Some(error) = state.error.as_ref() {
        content = content.push(
            row![
                container(settings_error_banner(error)).width(Length::Fill),
                catalog::action(i18n.__("刷新"), enabled.then_some(message(Event::Refresh))),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }
    let mut list = column![catalog::record(
        [
            (catalog::cell(i18n.__("名称")).into(), 3),
            (catalog::cell(i18n.__("类型")).into(), 2),
            (catalog::cell(i18n.__("选项")).into(), 3),
        ],
        None
    )];
    let mut has_results = false;
    for property in &state.properties {
        if (property.archived && !state.show_archived)
            || !(catalog::matches_search(&property.name, &query)
                || catalog::matches_search(&property.description, &query))
        {
            continue;
        }
        has_results = true;
        let mut title = column![
            row![catalog::glyph(property_icon(property.icon), 15.0), text(&property.name).size(14),]
                .spacing(8)
                .align_y(Alignment::Center)
        ]
        .spacing(4);
        if !property.description.is_empty() {
            title = title.push(catalog::cell(catalog::preview(&property.description)));
        }
        if property.archived {
            title = title.push(catalog::cell(i18n.__("已归档")));
        }
        let mut options = row![].spacing(8);
        let live = property.options.iter().filter(|option| !option.archived);
        for option in live.clone().take(3) {
            let color = option.color.as_deref().and_then(catalog::palette_color);
            let mut option_row = row![].spacing(5).align_y(Alignment::Center);
            if let Some(color) = color {
                option_row = option_row.push(catalog::color_dot(color, 8.0));
            }
            options = options.push(option_row.push(catalog::cell(&option.name)));
        }
        let count = live.count();
        if let Some(more) = count.checked_sub(3).filter(|more| *more > 0) {
            options = options.push(catalog::cell(format!("+{more}")));
        } else if count == 0 {
            options = options.push(catalog::cell("—"));
        }
        let menu = (!property.archived).then(|| {
            catalog::menu(
                vec![
                    (i18n.__("编辑"), message(Event::Edit(property.id.clone()))),
                    (i18n.__("归档"), message(Event::Archive(property.id.clone()))),
                ],
                enabled,
            )
        });
        list = list.push(settings_divider()).push(catalog::record(
            [
                (title.into(), 3),
                (catalog::cell(kind_label(i18n, property.kind)).into(), 2),
                (options.wrap().into(), 3),
            ],
            menu,
        ));
    }
    if !has_results {
        let status = if !enabled {
            "正在加载属性…"
        } else if state.properties.is_empty() && query.is_empty() {
            "项目暂无属性"
        } else {
            "没有匹配的结果"
        };
        list = list
            .push(settings_divider())
            .push(container(catalog::cell(i18n.__(status))).padding([18, 0]));
    }
    content.push(settings_group_card(list)).into()
}

fn icon_picker(i18n: I18n, draft: &Draft, enabled: bool) -> Element<'_, Message> {
    let open = enabled && draft.picker == Some(Picker::Icon);
    let trigger = button(
        row![
            catalog::glyph(property_icon(draft.icon), 18.0),
            catalog::glyph(Icon::ChevronDown, 11.0)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .style(rounded_action_btn_style)
    .padding([8, 12])
    .on_press_maybe(enabled.then_some(message(Event::Picker((!open).then_some(Picker::Icon)))));
    if !open {
        return trigger.into();
    }
    let buttons = ICONS.iter().map(|&(key, glyph, name)| {
        let selected = key == draft.icon;
        let preview = if selected {
            catalog::primary_glyph(glyph, 17.0)
        } else {
            catalog::glyph(glyph, 17.0)
        };
        iced::widget::tooltip(
            button(preview)
                .padding(9)
                .style(if selected { primary_action_btn_style } else { rounded_action_btn_style })
                .on_press(message(Event::IconChanged(key))),
            container(text(i18n.__(name)).size(12)).padding(6).style(settings_panel_style),
            iced::widget::tooltip::Position::Top,
        )
        .into()
    });
    let popup = container(
        column![settings_field_label(i18n.__("选择图标")), row(buttons).spacing(5).wrap(),]
            .spacing(8),
    )
    .padding(12)
    .width(265)
    .style(settings_modal_card_style);
    BelowOverlay::new(trigger, popup)
        .show(true)
        .gap(6.0)
        .within_window()
        .dismiss_on_escape()
        .on_close(message(Event::Picker(None)))
        .into()
}

fn option_editor<'a>(i18n: I18n, draft: &'a Draft, enabled: bool) -> Element<'a, Message> {
    let mut rows = column![].spacing(8).width(Length::Fill);
    for (index, option) in draft.options.iter().enumerate() {
        let archived = option.original.as_ref().is_some_and(|original| original.archived);
        let editable = enabled && !archived && !option.archive;
        let input = settings_text_input(i18n.__("选项名称"), &option.name).width(Length::Fill);
        let input = if editable {
            input.on_input(move |value| message(Event::OptionChanged(index, value)))
        } else {
            input
        };
        let picker = Picker::Color(index);
        let open = editable && draft.picker == Some(picker);
        let color = catalog::color_control(
            i18n,
            option.color.as_deref().unwrap_or(catalog::DEFAULT_COLOR),
            false,
            open,
            editable.then_some(message(Event::Picker((!open).then_some(picker)))),
            message(Event::Picker(None)),
            move |value| message(Event::OptionColor(index, value)),
        );
        let action: Element<'_, Message> = if archived {
            catalog::cell(i18n.__("已归档")).into()
        } else if option.original.is_some() {
            let label = if option.archive { "撤销归档" } else { "归档" };
            catalog::action(
                i18n.__(label),
                enabled.then_some(message(Event::ToggleOptionArchive(index))),
            )
            .into()
        } else {
            let removable = enabled && (draft.original.is_some() || draft.options.len() > 1);
            catalog::action(
                i18n.__("移除"),
                removable.then_some(message(Event::RemoveOption(index))),
            )
            .into()
        };
        rows = rows.push(row![color, input, action].spacing(8).align_y(Alignment::Center));
    }
    rows.push(catalog::action(i18n.__("添加选项"), enabled.then_some(message(Event::AddOption))))
        .into()
}

pub(in crate::app) fn overlay<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let state = &app.task_property_settings;
    let Some(draft) =
        state.draft.as_ref().filter(|_| super::project_path(app) == state.project_path.as_deref())
    else {
        return base;
    };
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let enabled = state.pending.is_none();
    let title = if draft.original.is_some() { "编辑属性" } else { "新建属性" };
    let mut body = column![
        settings_responsive_row(i18n.__("名称"), "", move || {
            let input = settings_text_input(i18n.__("属性名称"), &draft.name).id(NAME_INPUT);
            if enabled { input.on_input(|value| message(Event::NameChanged(value))) } else { input }
        }),
        settings_divider(),
        settings_responsive_row(i18n.__("类型"), "", move || {
            let selected = KINDS.iter().position(|kind| *kind == draft.kind);
            settings_select(0..KINDS.len(), selected, |index| {
                message(Event::KindChanged(KINDS[index]))
            })
            .format_labels(move |index| kind_label(i18n, KINDS[*index]).to_owned())
            .width(Length::Fill)
            .disabled(!enabled || draft.original.is_some())
        }),
        settings_divider(),
        settings_responsive_row(i18n.__("图标"), "", move || icon_picker(i18n, draft, enabled)),
        settings_divider(),
        settings_responsive_row(i18n.__("描述"), "", move || {
            let input = settings_text_editor(&draft.description)
                .height(80)
                .placeholder(i18n.__("说明这个属性的用途"));
            if enabled {
                input.on_action(|action| message(Event::DescriptionChanged(action)))
            } else {
                input
            }
        }),
    ]
    .spacing(8);
    if draft.kind.has_options() {
        body = body.push(settings_divider()).push(settings_responsive_row(
            i18n.__("选项"),
            "",
            move || option_editor(i18n, draft, enabled),
        ));
    }
    if let Some(error) = state.error.as_ref() {
        body = body.push(settings_error_banner(error));
    }
    let height = if draft.kind.has_options() { 560.0 } else { 460.0 };
    catalog::modal(
        i18n,
        base,
        i18n.__(title),
        body,
        height,
        message(Event::Close),
        (enabled && draft.valid()).then_some(message(Event::Save)),
    )
}
