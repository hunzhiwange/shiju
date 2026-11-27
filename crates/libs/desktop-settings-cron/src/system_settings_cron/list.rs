//! Current jobs own their scroll viewport so batch actions remain visible.
use super::*;
use crate::message::{ListMessage as Action, SettingsMessage as S};
use crate::state::{
    CronFrequency, CronJobKindFilter, CronListFilter, CronListMenu, CronScheduleDraft,
};
use desktop_assets::assets::Icon;
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::icons::icon_btn_style;
use desktop_ui_controls::system_settings_common::select::{menu_card_style, menu_option_style};
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_HEIGHT, SettingsSelectLabel, icon_svg, settings_tab_style,
    settings_text_input_style, settings_toggle, with_settings_modal_escape,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, column, mouse_area, responsive, scrollable, stack, svg, tooltip};

fn summary(job: &CronJobDto) -> String {
    let source =
        if job_is_agent(job) { job.prompt.as_deref().unwrap_or_default() } else { &job.command };
    let normalized = source.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = normalized.chars();
    let mut result: String = chars.by_ref().take(160).collect();
    if chars.next().is_some() {
        result.push('…');
    }
    result
}

fn schedule_label(i18n: I18n, job: &CronJobDto, system_timezone: NamedTimeZone) -> String {
    match job.schedule_kind.as_str() {
        "at" => format!(
            "{} · {}",
            i18n.__("指定时间"),
            format_cron_datetime(job.at.as_deref().unwrap_or(&job.expression), system_timezone),
        ),
        "every" => format!(
            "{} · {}",
            i18n.__("固定间隔"),
            job.every_ms.map(|ms| format!("{ms} ms")).unwrap_or_else(|| job.expression.clone())
        ),
        "biweekly" => match CronScheduleDraft::from_job(job) {
            Ok(draft) => super::schedule::rule_summary(i18n, &draft.rule),
            Err(error) => format!("{} · {error}", i18n.__("每双周")),
        },
        "cron" | "" => {
            let fields: Vec<_> = job.expression.split_whitespace().collect();
            let fields = match fields.as_slice() {
                ["0", rest @ ..] if rest.len() == 5 => rest,
                other => other,
            };
            let frequency = CronFrequency::from_expression(&fields.join(" "));
            if frequency == CronFrequency::Daily {
                format!("{} {:0>2}:{:0>2}", i18n.__("每天"), fields[1], fields[0])
            } else if frequency == CronFrequency::Advanced {
                job.expression.clone()
            } else {
                format!("{} · {}", i18n.__(&frequency.to_string()), job.expression)
            }
        }
        _ => job.expression.clone(),
    }
}

pub(super) fn event(action: Action) -> Message {
    Message::Settings(S::CronList(action))
}

pub(super) fn bold() -> iced::Font {
    iced::Font { weight: iced::font::Weight::Bold, ..iced::Font::DEFAULT }
}

fn tinted_icon(icon: Icon, size: f32) -> svg::Svg<'static> {
    icon_svg(icon, size).style(|theme, _| svg::Style { color: Some(theme.palette().text) })
}

pub(super) fn surface(theme: &Theme, emphasized: bool) -> container::Style {
    let palette = theme.extended_palette();
    let edge = if emphasized {
        palette.primary.base.color
    } else {
        theme.palette().text.scale_alpha(0.14)
    };
    container::Style {
        background: Some(
            if emphasized {
                palette.background.strong.color
            } else {
                palette.background.weak.color
            }
            .into(),
        ),
        border: iced::Border { color: edge, width: 1.0, radius: 12.0.into() },
        ..Default::default()
    }
}

fn tooltip_style(theme: &Theme) -> container::Style {
    let mut style = surface(theme, false);
    style.border.radius = 7.0.into();
    style.shadow = iced::Shadow {
        color: iced::Color::BLACK.scale_alpha(0.15),
        offset: iced::Vector::new(0.0, 2.0),
        blur_radius: 6.0,
    };
    style
}

fn filled_icon_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let mut style = rounded_action_btn_style(theme, status);
        let shade = match status {
            button::Status::Disabled => 0.035,
            button::Status::Pressed => 0.12,
            _ if active => 0.10,
            button::Status::Hovered => 0.085,
            _ => 0.055,
        };
        style.background =
            Some(iced::theme::palette::deviate(theme.palette().background, shade).into());
        style.border = iced::border::rounded(8.0);
        style
    }
}

fn icon_control(
    icon: svg::Svg<'static>,
    label: &'static str,
    size: f32,
    style: impl Fn(&Theme, button::Status) -> button::Style + 'static,
    on_press: Option<Message>,
) -> Element<'static, Message> {
    let control = button(container(icon).center(Length::Fill))
        .width(size)
        .height(size)
        .padding(4)
        .style(style)
        .on_press_maybe(on_press);
    tooltip(control, text(label).size(12), tooltip::Position::Top)
        .padding(6)
        .style(tooltip_style)
        .into()
}

pub(super) fn toolbar_icon(
    icon: Icon,
    icon_size: f32,
    label: &'static str,
    on_press: Option<Message>,
) -> Element<'static, Message> {
    let control = button(container(tinted_icon(icon, icon_size)).center(Length::Fill))
        .width(34)
        .height(34)
        .padding(4)
        .style(icon_btn_style)
        .on_press_maybe(on_press);
    tooltip(control, text(label).size(12), tooltip::Position::Bottom)
        .gap(5)
        .padding(8)
        .style(|_| container::Style {
            background: Some(iced::Color::BLACK.into()),
            text_color: Some(iced::Color::WHITE),
            border: iced::border::rounded(8.0),
            ..Default::default()
        })
        .into()
}

pub(super) fn toolbar_search(
    tr: I18n,
    open: bool,
    id: &'static str,
    query: &str,
    toggle: Message,
    on_input: impl Fn(String) -> Message + 'static,
) -> Element<'static, Message> {
    let label = tr.__("搜索定时任务/记录");
    if !open {
        return toolbar_icon(Icon::Search, 17.0, label, Some(toggle));
    }

    let input = settings_text_input(label, query)
        .id(id)
        .padding(iced::Padding::from(SETTINGS_CONTROL_PADDING).left(36))
        .style(|theme, status| {
            let mut style = settings_text_input_style(theme, status);
            style.border.radius = 8.0.into();
            style
        })
        .on_input(on_input);
    let icon = icon_svg(Icon::Search, 17.0)
        .style(|theme, _| svg::Style { color: Some(theme.palette().text.scale_alpha(0.65)) });
    let close = button(container(icon).center(Length::Fill))
        .width(36)
        .height(SETTINGS_CONTROL_HEIGHT)
        .padding(0)
        .style(|theme, status| {
            let mut style = rounded_action_btn_style(theme, status);
            style.background = None;
            style.border = iced::Border::default();
            style
        })
        .on_press(toggle.clone());

    with_settings_modal_escape(container(stack![input, close]).width(236).into(), toggle)
}

fn menu_item(
    icon: Icon,
    label: &'static str,
    warning: bool,
    on_press: Option<Message>,
) -> Element<'static, Message> {
    let enabled = on_press.is_some();
    let icon = icon_svg(icon, 15.0).style(move |theme, _| svg::Style {
        color: Some(if !enabled {
            theme.palette().text.scale_alpha(0.55)
        } else if warning {
            theme.extended_palette().danger.base.color
        } else {
            theme.palette().text
        }),
    });
    button(row![icon, text(label).size(13)].spacing(10).align_y(Alignment::Center))
        .padding([9, 10])
        .width(Length::Fill)
        .style(move |theme, status| {
            let mut style = menu_option_style(theme, status, false);
            if status == button::Status::Disabled {
                style.text_color = theme.palette().text.scale_alpha(0.55);
            } else if warning {
                style.text_color = theme.extended_palette().danger.base.color;
            }
            style
        })
        .on_press_maybe(on_press)
        .into()
}

pub(super) fn select_option(
    label: String,
    selected: bool,
    on_press: Message,
) -> Element<'static, Message> {
    let check: Element<'_, Message> = if selected {
        icon_svg(Icon::Check, 16.0)
            .style(|theme, _| svg::Style { color: Some(theme.palette().text.scale_alpha(0.7)) })
            .into()
    } else {
        Space::new().width(16).height(16).into()
    };
    button(
        row![SettingsSelectLabel::new(label), check]
            .spacing(12)
            .height(Length::Fill)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .height(36)
    .padding([0, 10])
    .style(|theme, status| menu_option_style(theme, status, false))
    .on_press(on_press)
    .into()
}

fn menu<'a>(
    trigger: Element<'a, Message>,
    items: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let panel = container(items).padding([6, 7]).width(200).style(menu_card_style);
    BelowOverlay::new(trigger, panel)
        .show(true)
        .within_window()
        .align_right(true)
        .gap(6.0)
        .dismiss_on_escape()
        .on_close(event(Action::MenuChanged(None)))
        .into()
}

fn task_actions(
    state: &crate::state::CronSettingsState,
    tr: I18n,
    job: &CronJobDto,
) -> Element<'static, Message> {
    let id = &job.id;
    let target = CronListMenu::Job(id.clone());
    let open = state.list_menu.as_ref() == Some(&target);
    let more = icon_control(
        tinted_icon(Icon::DotsThreeHorizontal, 15.0),
        tr.__("更多"),
        24.0,
        icon_btn_style,
        Some(event(Action::MenuChanged((!open).then_some(target)))),
    );
    let more = if open {
        menu(
            more,
            column![
                menu_item(
                    Icon::Eye,
                    tr.__("查看运行记录"),
                    false,
                    Some(event(Action::OpenHistory(id.clone())))
                ),
                menu_item(
                    Icon::Trash,
                    tr.__("删除"),
                    true,
                    (!state.deleting_jobs).then(|| Message::Settings(S::CronJobDelete(id.clone())))
                ),
            ]
            .spacing(2),
        )
    } else {
        more
    };
    row![
        icon_control(
            tinted_icon(Icon::Play, 15.0),
            tr.__("立即运行"),
            24.0,
            icon_btn_style,
            (!state.run_requests.contains(id)).then(|| event(Action::RunNow(id.clone())))
        ),
        icon_control(
            tinted_icon(Icon::Pencil, 10.0),
            tr.__("编辑"),
            24.0,
            icon_btn_style,
            Some(Message::Settings(S::CronJobEditStarted(id.clone())))
        ),
        more,
    ]
    .spacing(12)
    .padding([0, 5])
    .align_y(Alignment::Center)
    .into()
}

pub fn job_row<'a>(app: &ViewData<'a>, job: &'a CronJobDto) -> Element<'a, Message> {
    job_card(true, app.cron_settings, I18n::new(app.desktop_locale), job, app.system_timezone)
}

fn job_card<'a>(
    compact: bool,
    state: &'a crate::state::CronSettingsState,
    tr: I18n,
    job: &'a CronJobDto,
    system_timezone: NamedTimeZone,
) -> Element<'a, Message> {
    let id = &job.id;
    let selected = state.selected_job_ids.contains(id);
    let hovered = state.hovered_job_id.as_ref() == Some(id);
    let menu_open = matches!(&state.list_menu, Some(CronListMenu::Job(open)) if open == id);
    let mut header = row![].spacing(8).height(34).align_y(Alignment::Center);
    if state.batch_mode {
        header = header.push(
            checkbox(selected)
                .on_toggle(move |value| {
                    Message::Settings(S::CronJobSelectionToggled(id.clone(), value))
                })
                .style(settings_checkbox_style),
        );
    }
    let title =
        if job_title(job) == "未命名任务" { tr.__("未命名任务") } else { job_title(job) };
    let title =
        container(SingleLineLabel::new(title.to_owned(), 15.0).font(bold())).width(Length::Fill);
    let title = button(title)
        .padding(0)
        .width(Length::Fill)
        .style(|theme, status| {
            let mut style = cron_segment_style(theme, status, false);
            style.background = None;
            style.text_color = theme.palette().text;
            style.border = iced::Border::default();
            style
        })
        .on_press(if state.batch_mode {
            Message::Settings(S::CronJobSelectionToggled(id.clone(), !selected))
        } else {
            event(Action::OpenTask(id.clone()))
        });
    let header = header.push(title);
    let toggle = settings_toggle(job.enabled)
        .on_toggle(move |enabled| Message::Settings(S::CronJobEnabledChanged(id.clone(), enabled)));
    let header = if !state.batch_mode && (hovered || menu_open || compact) {
        header.push(task_actions(state, tr, job))
    } else {
        header
    }
    .push(toggle);

    let excerpt = container(
        text(summary(job))
            .size(13)
            .line_height(iced::widget::text::LineHeight::Absolute(iced::Pixels(20.0)))
            .style(settings_muted_text_style)
            .width(Length::Fill)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
    )
    .height(40)
    .clip(true);
    let (status, highlight) = if job_has_ended(job, chrono::Utc::now()) {
        (tr.__("已结束"), false)
    } else if job.enabled {
        (tr.__("已启用"), true)
    } else {
        (tr.__("已暂停"), false)
    };
    let timing = row![
        tinted_icon(Icon::Clock, 14.0),
        container(SingleLineLabel::new(schedule_label(tr, job, system_timezone), 12.0))
            .style(|theme| container::Style {
                text_color: settings_muted_text_style(theme).color,
                ..Default::default()
            })
            .width(Length::Fill),
        text(status).size(12).style(move |theme: &Theme| iced::widget::text::Style {
            color: Some(if highlight {
                theme.extended_palette().success.base.color
            } else {
                theme.palette().text.scale_alpha(0.70)
            }),
        }),
    ]
    .spacing(7)
    .align_y(Alignment::Center);
    let card = container(column![header, excerpt, settings_divider(), timing].spacing(10))
        .padding([12, 15])
        .width(Length::Fill)
        .style(move |theme| {
            let mut style = surface(theme, selected);
            if !selected && (hovered || menu_open) {
                style.background = Some(theme.extended_palette().background.strong.color.into());
                style.border.color = theme.palette().text.scale_alpha(0.29);
            }
            style
        });
    if state.batch_mode || compact {
        card.into()
    } else {
        mouse_area(card)
            .on_enter(event(Action::HoverChanged(id.clone(), true)))
            .on_exit(event(Action::HoverChanged(id.clone(), false)))
            .into()
    }
}

pub(super) fn toolbar(
    state: &crate::state::CronSettingsState,
    tr: I18n,
) -> Element<'static, Message> {
    let filter_open = state.list_menu == Some(CronListMenu::Filter);
    let filter = toolbar_icon(
        Icon::Funnel,
        15.0,
        tr.__("筛选"),
        Some(event(Action::MenuChanged((!filter_open).then_some(CronListMenu::Filter)))),
    );
    let filter = if filter_open {
        let items = column(
            [
                (CronJobKindFilter::All, "全部类型"),
                (CronJobKindFilter::Agent, "Agent"),
                (CronJobKindFilter::Shell, "Shell"),
            ]
            .map(|(kind, label)| {
                select_option(
                    tr.__(label).to_owned(),
                    state.kind_filter == kind,
                    event(Action::KindChanged(kind)),
                )
            }),
        )
        .spacing(2);
        menu(filter, items)
    } else {
        filter
    };
    let split = new_task_button(state.list_menu == Some(CronListMenu::New), tr);
    row![
        toolbar_icon(
            Icon::ArrowClockwise,
            15.0,
            tr.__(if state.jobs_loading { "刷新中…" } else { "刷新" }),
            (!state.jobs_loading).then_some(Message::Settings(S::CronJobsRefresh))
        ),
        toolbar_search(
            tr,
            state.show_job_search,
            "cron-jobs-search",
            &state.job_search,
            event(Action::SearchToggled),
            |query| event(Action::SearchChanged(query)),
        ),
        filter,
        button(
            container(
                text(tr.__(if state.batch_mode { "取消批量管理" } else { "批量管理" })).size(13)
            )
            .center_y(Length::Fill)
        )
        .height(SETTINGS_CONTROL_HEIGHT)
        .padding([0, 11])
        .style(rounded_action_btn_style)
        .on_press(Message::Settings(S::CronBatchModeToggled(!state.batch_mode))),
        split,
    ]
    .spacing(5)
    .align_y(Alignment::Center)
    .wrap()
    .into()
}

fn new_task_button(open: bool, tr: I18n) -> Element<'static, Message> {
    let create =
        button(container(text(tr.__("新建")).font(bold()).size(13)).center_y(Length::Fill))
            .height(SETTINGS_CONTROL_HEIGHT)
            .padding([0, 12])
            .style(|theme, status| create_button_style(theme, status, iced::border::left(8.0)))
            .on_press(Message::Settings(S::CronTabSelected(CronSettingsTab::Add)));
    let chevron = icon_svg(Icon::ChevronDown, 13.0)
        .style(|theme, _| svg::Style { color: Some(theme.palette().background) });
    let options = button(container(chevron).center(Length::Fill))
        .width(34)
        .height(SETTINGS_CONTROL_HEIGHT)
        .padding(4)
        .style(move |theme, status| {
            let status = if open { button::Status::Pressed } else { status };
            create_button_style(theme, status, iced::border::right(8.0))
        })
        .on_press(event(Action::MenuChanged((!open).then_some(CronListMenu::New))));
    let split = row![create, options].align_y(Alignment::Center).into();
    if !open {
        return split;
    }

    menu(
        split,
        column![
            menu_item(
                Icon::ChatTextFill,
                tr.__("从对话新建"),
                false,
                Some(Message::Settings(S::CronAddFromConversation)),
            ),
            menu_item(
                Icon::Grid1x2,
                tr.__("从模板新建"),
                false,
                Some(Message::Settings(S::CronTemplatesToggled(true))),
            ),
        ]
        .spacing(2),
    )
}

fn create_button_style(
    theme: &Theme,
    status: button::Status,
    radius: iced::border::Radius,
) -> button::Style {
    let mut style = primary_action_btn_style(theme, status);
    let palette = theme.palette();
    let shade = match status {
        button::Status::Hovered => 0.16,
        button::Status::Pressed => 0.26,
        _ => 0.0,
    };
    style.background =
        Some(iced::theme::palette::mix(palette.text, palette.background, shade).into());
    style.text_color = palette.background;
    style.border.radius = radius;
    style
}

pub(super) fn settings_menu(
    state: &crate::state::CronSettingsState,
    tr: I18n,
) -> Element<'static, Message> {
    let open = state.list_menu == Some(CronListMenu::Settings);
    let trigger = toolbar_icon(
        Icon::Gear,
        15.0,
        tr.__("配置"),
        Some(event(Action::MenuChanged((!open).then_some(CronListMenu::Settings)))),
    );
    if !open {
        return trigger;
    }
    let mut items = column![].spacing(2);
    for (icon, tab, label) in [
        (Icon::Person, CronSettingsTab::Employee, "员工排程"),
        (Icon::Gear, CronSettingsTab::Config, "配置"),
        (Icon::Sliders, CronSettingsTab::Scheduler, "调度器"),
    ] {
        if !cfg!(target_arch = "wasm32") || tab != CronSettingsTab::Employee {
            items = items.push(menu_item(
                icon,
                tr.__(label),
                false,
                Some(Message::Settings(S::CronTabSelected(tab))),
            ));
        }
    }
    items = items.push(menu_item(
        Icon::QuestionCircle,
        tr.__("帮助"),
        false,
        Some(Message::Settings(S::CronHelpOpen)),
    ));
    menu(trigger, items)
}

pub fn jobs_tab<'a>(app: &ViewData<'a>) -> Element<'a, Message> {
    let tr = I18n::new(app.desktop_locale);
    let state = app.cron_settings;
    let system_timezone = app.system_timezone;
    let mut groups = row![].spacing(6);
    for (filter, label) in [
        (CronListFilter::All, "全部"),
        (CronListFilter::Enabled, "进行中"),
        (CronListFilter::Paused, "已暂停"),
    ] {
        let selected = filter == state.list_filter;
        let label = format!("{} ({})", tr.__(label), state.job_count(filter));
        groups = groups.push(
            button(container(text(label).size(14).font(bold())).center_y(Length::Fill))
                .height(32)
                .padding([0, 12])
                .style(move |theme, status| {
                    let mut style = settings_tab_style(theme, status, selected);
                    let shade = match status {
                        button::Status::Pressed => Some(0.085),
                        button::Status::Hovered => Some(0.065),
                        _ if selected => Some(0.045),
                        _ => None,
                    };
                    let palette = theme.palette();
                    style.background = shade.map(|amount| {
                        iced::theme::palette::deviate(palette.background, amount).into()
                    });
                    style.border = iced::border::rounded(8.0);
                    if !selected && status == button::Status::Active {
                        style.text_color =
                            settings_muted_text_style(theme).color.unwrap_or(palette.text);
                    }
                    style
                })
                .on_press(Message::Settings(S::CronListFilterChanged(filter))),
        );
    }

    let mut page = column![groups.wrap()].spacing(14).width(Length::Fill).height(Length::Fill);

    let list = responsive(move |size| {
        let columns = super::grid_columns(size.width - 14.0, 300.0);
        let jobs: Vec<_> = state.visible_jobs().collect();
        let mut cards = column![].spacing(12).width(Length::Fill);
        if state.jobs_loading && state.jobs.is_empty() {
            cards = cards.push(empty_message(tr.__("正在加载定时任务...")));
        } else if jobs.is_empty() {
            let label = if state.jobs.is_empty() && state.save_error.is_none() {
                "当前没有定时任务。"
            } else if !state.job_search.trim().is_empty()
                || state.kind_filter != CronJobKindFilter::All
            {
                "没有匹配的定时任务"
            } else {
                "当前分组暂无任务"
            };
            cards = cards.push(empty_message(tr.__(label)));
            if state.jobs.is_empty() && state.save_error.is_none() {
                cards = cards.push(super::templates::view(tr));
            }
        } else {
            for group in jobs.chunks(columns) {
                let mut line = row![].spacing(12).width(Length::Fill);
                for job in group {
                    line = line.push(
                        container(job_card(
                            cfg!(target_arch = "wasm32") || size.width < 560.0,
                            state,
                            tr,
                            job,
                            system_timezone,
                        ))
                        .width(Length::FillPortion(1)),
                    );
                }
                for _ in group.len()..columns {
                    line = line.push(Space::new().width(Length::FillPortion(1)));
                }
                cards = cards.push(line);
            }
        }
        scrollable(container(cards).padding([1, 7]).width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill)
            .into()
    });
    page = page.push(list);
    if state.batch_mode {
        page = page.push(batch_footer(state, tr));
    }
    page.into()
}

fn empty_message(label: &'static str) -> Element<'static, Message> {
    container(
        row![tinted_icon(Icon::Clock, 16.0), text(label).size(14).style(settings_muted_text_style)]
            .spacing(9)
            .align_y(Alignment::Center),
    )
    .padding(16)
    .width(Length::Fill)
    .style(|theme| surface(theme, false))
    .into()
}

fn batch_footer(state: &crate::state::CronSettingsState, tr: I18n) -> Element<'static, Message> {
    let count = state.selected_job_ids.len();
    let visible = state.visible_jobs().count();
    let enabled = count > 0 && !state.deleting_jobs;
    let all_selected =
        visible > 0 && state.visible_jobs().all(|job| state.selected_job_ids.contains(&job.id));
    let open = state.list_menu == Some(CronListMenu::Batch);
    let more = icon_control(
        tinted_icon(Icon::DotsThreeHorizontal, 15.0),
        tr.__("更多"),
        34.0,
        filled_icon_style(open),
        enabled.then(|| event(Action::MenuChanged((!open).then_some(CronListMenu::Batch)))),
    );
    let more = if open {
        menu(
            more,
            column![
                menu_item(
                    Icon::Check,
                    tr.__("启用"),
                    false,
                    Some(Message::Settings(S::CronSelectedJobsEnable))
                ),
                menu_item(
                    Icon::X,
                    tr.__("禁用"),
                    false,
                    Some(Message::Settings(S::CronSelectedJobsDisable))
                ),
            ]
            .spacing(2),
        )
    } else {
        more
    };
    let select = checkbox(all_selected)
        .label(tr.__("全选"))
        .on_toggle_maybe(
            (visible > 0).then_some(|value| Message::Settings(S::CronJobsSelectAllToggled(value))),
        )
        .style(settings_checkbox_style);
    let actions = row![
        more,
        button(container(text(tr.__("删除")).size(13)).center_y(Length::Fill))
            .height(34)
            .padding([0, 15])
            .style(danger_action_btn_style)
            .on_press_maybe(enabled.then_some(Message::Settings(S::CronSelectedJobsDelete))),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    let status =
        desktop_i18n::__!(tr, "已选中 %u 项", count).expect("valid cron selection template");
    let selection = row![select, text(status).size(12).style(settings_muted_text_style)]
        .spacing(10)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .wrap();
    container(row![selection, actions].spacing(10).align_y(Alignment::Center))
        .padding([9, 12])
        .width(Length::Fill)
        .style(|theme| surface(theme, false))
        .into()
}

pub(super) fn job_has_ended(job: &CronJobDto, now: chrono::DateTime<chrono::Utc>) -> bool {
    job.ends_at
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|end| end <= now)
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;
