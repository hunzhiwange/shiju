//! 智能体、技能和模型的定时任务选择面板。
use super::{AgentChoice, EDITOR_PICKER_SCROLL_ID, ViewData};
use crate::message::{Message, SettingsMessage as S};
use crate::state::{CronEditorPicker as Picker, CronEditorPickerAction as Action, CronJobDraft};
use desktop_assets::assets::Icon;
use desktop_chat_input::input_panel::{
    attachment::composer_add_button,
    layout::composer_toolbar,
    model_views::composer_model_button,
    panel::agent_toggle,
    surface_styles::{
        BOTTOM_BAR_ICON_SIZE, popover_style, round_icon_button_style, selectable_list_button_style,
        tooltip_dark_style,
    },
};
use desktop_i18n::I18n;
use desktop_ui_controls::composer_icons::ComposerIcon;
use desktop_ui_controls::icons::{
    acp_agent_icon, auto_icon, provider_logo_handle, themed_svg_handle,
};
use desktop_ui_controls::overlays::{AboveOverlay, BelowOverlay};
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::icons::icon_btn_svg_style;
use desktop_ui_controls::system_settings_common::{
    icon_svg, settings_divider, settings_muted_text_style, settings_text_input,
};
use iced::widget::{Space, button, column, container, row, scrollable, text, tooltip};
use iced::{Alignment, Element, Length};

pub(super) fn controls<'a>(
    app: &ViewData<'_>,
    draft: &CronJobDraft,
    editing: bool,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let agent_id = if draft.agent.trim().is_empty() && !draft.acp_agent.trim().is_empty() {
        format!("acp-{}", draft.acp_agent.trim())
    } else if draft.agent.trim().is_empty() {
        "main".to_owned()
    } else {
        draft.agent.clone()
    };
    let agent = app.agents.iter().find(|agent| agent.id == agent_id);
    let model_open = draft.editor_picker == Some(Picker::Model);
    let model = draft.model.trim();
    let model_choice = app.models.iter().find_map(|group| {
        group.models.iter().find(|choice| choice.id == model).map(|choice| (group, choice))
    });
    let native_model =
        agent.is_some_and(|agent| agent.backend.is_some()) || !draft.acp_agent.trim().is_empty();
    let default_model =
        if native_model { i18n.__("智能体原生模型") } else { i18n.__("自动模型") };

    let model_icon = if is_auto_model(model) {
        if native_model {
            ComposerIcon::Agent.svg(BOTTOM_BAR_ICON_SIZE)
        } else {
            themed_svg_handle(auto_icon(), BOTTOM_BAR_ICON_SIZE)
        }
    } else if let Some((group, _)) = model_choice {
        themed_svg_handle(provider_logo_handle(&group.id), BOTTOM_BAR_ICON_SIZE)
    } else {
        icon_svg(Icon::Sliders, BOTTOM_BAR_ICON_SIZE)
    };
    let model_label = if is_auto_model(model) {
        default_model
    } else {
        model_choice.map_or(model, |(_, choice)| choice.label.as_str())
    };
    let model_trigger = composer_model_button(
        model_icon.into(),
        model_label.to_owned(),
        None,
        model_open,
        action(editing, if model_open { Action::Close } else { Action::Open(Picker::Model) }),
    );
    let model_control = if model_open {
        window_picker(
            model_trigger,
            model_panel(app, draft, default_model, editing),
            true,
            action(editing, Action::Close),
        )
    } else {
        model_trigger
    };

    composer_toolbar(row![
        skill_control(i18n, app, draft, editing),
        agent_control(i18n, app, draft, &agent_id, agent, editing),
        Space::new().width(Length::Fill),
        model_control,
    ])
    .into()
}

fn agent_control<'a>(
    i18n: I18n,
    app: &ViewData<'_>,
    draft: &CronJobDraft,
    agent_id: &str,
    agent: Option<&AgentChoice>,
    editing: bool,
) -> Element<'a, Message> {
    let open = draft.editor_picker == Some(Picker::Agent);
    let close = action(editing, Action::Close);
    let label = agent.map_or_else(
        || if agent_id == "main" { i18n.__("智能体").to_owned() } else { agent_id.to_owned() },
        |agent| i18n.__(&agent.label).to_owned(),
    );
    let backend = agent.and_then(|agent| agent.backend.as_deref()).or_else(|| {
        let backend = draft.acp_agent.trim();
        (!backend.is_empty()).then_some(backend)
    });
    let trigger = container(agent_toggle(
        open,
        label,
        backend,
        None,
        if open { close.clone() } else { action(editing, Action::Open(Picker::Agent)) },
    ))
    .max_width(160);
    if !open {
        return trigger.into();
    }
    let query = draft.editor_picker_query.trim().to_lowercase();
    let header = search(i18n.__("搜索智能体"), &draft.editor_picker_query, editing);
    let items = agent_list(i18n, &app.agents, agent_id, &query, editing);
    window_picker(trigger, panel(header, items, 500.0), false, close)
}

fn window_picker<'a>(
    trigger: impl Into<Element<'a, Message>>,
    content: Element<'a, Message>,
    align_right: bool,
    close: Message,
) -> Element<'a, Message> {
    BelowOverlay::new(trigger, content)
        .show(true)
        .within_window()
        .align_right(align_right)
        .gap(7.0)
        .dismiss_on_escape()
        .on_close(close)
        .into()
}

fn skill_control<'a>(
    i18n: I18n,
    app: &ViewData<'_>,
    draft: &CronJobDraft,
    editing: bool,
) -> Element<'a, Message> {
    let open = draft.editor_picker == Some(Picker::Skills);
    let close = action(editing, Action::Close);
    let trigger = composer_add_button(
        open,
        round_icon_button_style,
        if open { close.clone() } else { action(editing, Action::Open(Picker::Skills)) },
    );
    if !open {
        return tooltip(trigger, text(i18n.__("技能")).size(12), tooltip::Position::Top)
            .style(tooltip_dark_style)
            .into();
    }
    let query = draft.editor_picker_query.trim().to_lowercase();
    let header = search(i18n.__("搜索技能"), &draft.editor_picker_query, editing);
    let items = skill_list(i18n, app, draft, &query, editing);
    AboveOverlay::new(trigger, panel(header, items, 360.0))
        .show(true)
        .block_background_events(true)
        .flip_if_clipped()
        .gap(7.0)
        .on_close(close)
        .into()
}

fn action(editing: bool, action: Action) -> Message {
    Message::Settings(S::CronEditorPickerChanged { editing, action })
}

fn panel<'a>(
    header: Element<'a, Message>,
    items: iced::widget::Column<'a, Message>,
    height: f32,
) -> Element<'a, Message> {
    let list = scrollable(container(items).padding([0, 7]).width(Length::Fill))
        .id(EDITOR_PICKER_SCROLL_ID)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .height(Length::Fill)
        .width(Length::Fill);
    container(column![header, list].spacing(8).height(Length::Fill))
        .padding(8)
        .width(326)
        .height(Length::Fixed(height))
        .style(popover_style)
        .into()
}

fn search<'a>(placeholder: &str, query: &str, editing: bool) -> Element<'a, Message> {
    settings_text_input(placeholder, query)
        .on_input(move |query| action(editing, Action::Search(query)))
        .into()
}

fn matches_query(query: &str, fields: &[&str]) -> bool {
    query.is_empty() || fields.iter().any(|field| field.to_lowercase().contains(query))
}

fn agent_icon<'a>(agent: Option<&AgentChoice>) -> Element<'a, Message> {
    if agent.is_some_and(|agent| agent.requires_employee_target) {
        return icon_svg(Icon::Robot, 16.0).into();
    }
    match agent.and_then(|agent| agent.backend.as_deref()) {
        Some(backend) => acp_agent_icon(backend, 16.0),
        None => icon_svg(Icon::Hexagon, 16.0).into(),
    }
}

fn agent_list<'a>(
    i18n: I18n,
    agents: &[AgentChoice],
    selected: &str,
    query: &str,
    editing: bool,
) -> iced::widget::Column<'a, Message> {
    let mut list = column![].spacing(4);
    let mut any = false;
    for agent in agents {
        let mut detail = match agent.backend.as_deref() {
            Some(_) if agent.requires_employee_target => i18n.__("内置 ACP").to_owned(),
            Some(backend) => {
                let status = match agent.installed {
                    Some(true) => i18n.__("已安装"),
                    Some(false) => i18n.__("未安装"),
                    None => i18n.__("安装状态待检测"),
                };
                format!("ACP · {backend} · {status}")
            }
            None => i18n.__("识矩原生").to_owned(),
        };
        if !agent.model.trim().is_empty() {
            detail.push_str(" / ");
            detail.push_str(&agent.model);
        }
        if !matches_query(query, &[&agent.id, &agent.label, &detail]) {
            continue;
        }
        any = true;
        if agent.requires_employee_target {
            if cfg!(target_arch = "wasm32") {
                continue;
            }
            let detail = if editing {
                i18n.__("可在员工排程中编辑已保存的数字员工任务。")
            } else {
                i18n.__("选择共享员工和流程，创建数字员工排程。")
            };
            list = list.push(option(
                agent_icon(Some(agent)),
                agent.label.clone(),
                detail.to_owned(),
                false,
                (!editing).then_some(Message::Settings(S::CronOpenEmployeeSchedule)),
            ));
            continue;
        }
        let enabled = agent.installed != Some(false);
        list = list.push(option(
            agent_icon(Some(agent)),
            agent.label.clone(),
            detail,
            agent.id == selected,
            enabled.then(|| {
                action(
                    editing,
                    Action::AgentSelected { id: agent.id.clone(), backend: agent.backend.clone() },
                )
            }),
        ));
    }
    if !any {
        list = list
            .push(text(i18n.__("没有匹配的智能体。")).size(13).style(settings_muted_text_style));
    }
    list
}

fn skill_list<'a>(
    i18n: I18n,
    app: &ViewData<'_>,
    draft: &CronJobDraft,
    query: &str,
    editing: bool,
) -> iced::widget::Column<'a, Message> {
    let available = app
        .skills
        .iter()
        .map(|skill| (skill.id.as_str(), skill.label.as_str(), skill.description.as_str()));
    let saved = draft
        .skills
        .iter()
        .filter(|id| !app.skills.iter().any(|skill| &skill.id == *id))
        .map(|id| (id.as_str(), id.as_str(), ""));
    let mut list = column![].spacing(4);
    let mut any = false;
    for (id, label, description) in available.chain(saved) {
        if !matches_query(query, &[id, label, description]) {
            continue;
        }
        any = true;
        list = list.push(option(
            ComposerIcon::Skill.svg(14.0).into(),
            label.to_owned(),
            description.to_owned(),
            draft.skills.iter().any(|saved| saved == id),
            Some(Message::Settings(S::CronEditorSkillToggled { editing, id: id.to_owned() })),
        ));
    }
    let empty = if app.cron_settings.skill_catalog_request.is_some() {
        Some(i18n.__("正在刷新技能目录 ..."))
    } else if app.cron_settings.skill_catalog_failed {
        Some(i18n.__("技能目录加载失败"))
    } else if !any {
        Some(if query.is_empty() {
            i18n.__("没有可添加的技能")
        } else {
            i18n.__("没有匹配的技能。")
        })
    } else {
        None
    };
    if let Some(empty) = empty {
        list = list.push(text(empty).size(13).style(settings_muted_text_style));
    }
    list
}

fn is_auto_model(model: &str) -> bool {
    model.is_empty() || model.eq_ignore_ascii_case("auto")
}

fn model_panel<'a>(
    app: &ViewData<'_>,
    draft: &CronJobDraft,
    default_model: &'static str,
    editing: bool,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let query = draft.editor_picker_query.trim().to_lowercase();
    let current = draft.model.trim();
    let header = column![
        search(i18n.__("搜索模型…"), &draft.editor_picker_query, editing),
        settings_divider(),
    ]
    .spacing(6);
    let mut list = column![].spacing(4);
    let show_default = matches_query(&query, &[default_model, "auto"]);
    if show_default {
        list = list.push(option(
            themed_svg_handle(auto_icon(), 16.0).into(),
            default_model.to_owned(),
            String::new(),
            is_auto_model(current),
            Some(action(editing, Action::ModelSelected(String::new()))),
        ));
    }
    let mut any = false;
    for group in &app.models {
        let models = group.models.iter().filter(|model| {
            matches_query(&query, &[&group.id, &group.label, &model.id, &model.label])
        });
        let mut models = models.peekable();
        if models.peek().is_none() {
            continue;
        }
        any = true;
        list = list.push(
            container(
                row![
                    themed_svg_handle(provider_logo_handle(&group.id), 16.0),
                    text(group.label.clone()).size(13).style(settings_muted_text_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([7, 8]),
        );
        for model in models {
            list = list.push(option(
                Space::new().width(16).into(),
                model.label.clone(),
                String::new(),
                model.id == current,
                Some(action(editing, Action::ModelSelected(model.id.clone()))),
            ));
        }
    }
    if !any && (!show_default || app.models.is_empty()) {
        let empty = if app.models.is_empty() {
            i18n.__("暂无可选模型（请先在系统设置里启用模型）")
        } else {
            i18n.__("没有匹配的模型")
        };
        list = list.push(text(empty).size(13).style(settings_muted_text_style));
    }
    panel(header.into(), list, 500.0)
}

fn option<'a>(
    icon: Element<'a, Message>,
    title: String,
    detail: String,
    selected: bool,
    message: Option<Message>,
) -> Element<'a, Message> {
    let enabled = message.is_some();
    let mut label = column![text(title).size(13)].spacing(3).width(Length::Fill);
    if !detail.is_empty() {
        label = label.push(container(SingleLineLabel::new(detail, 11.0)).style(|theme| {
            container::Style {
                text_color: settings_muted_text_style(theme).color,
                ..Default::default()
            }
        }));
    }
    let check: Element<'a, Message> = if selected {
        icon_svg(Icon::Check, 14.0).style(icon_btn_svg_style).into()
    } else {
        Space::new().width(14).into()
    };
    button(row![icon, label, check].spacing(9).align_y(Alignment::Center))
        .padding([8, 9])
        .width(Length::Fill)
        .style(move |theme, status| {
            let mut style = selectable_list_button_style(theme, status, selected);
            if !enabled {
                style.text_color = theme.palette().text.scale_alpha(0.65);
            }
            style
        })
        .on_press_maybe(message)
        .into()
}
