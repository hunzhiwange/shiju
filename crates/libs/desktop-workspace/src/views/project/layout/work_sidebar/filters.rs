use iced::widget::column;
use super::*;

pub(super) fn filter_active(app: &ViewInput<'_>) -> bool {
    app.work_sidebar_filter_scope != WorkSidebarFilterScope::All
        || app.work_sidebar_filter_status != WorkSidebarFilterStatus::All
        || app.work_sidebar_filter_time != WorkSidebarFilterTime::All
}

pub(super) fn filter_option_style(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style + Copy {
    move |theme, status| {
        let palette = theme.extended_palette();
        let background = if selected {
            Some(palette.background.strong.color.scale_alpha(0.86))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(palette.background.strong.color.scale_alpha(0.54))
        } else {
            None
        };
        button::Style {
            background: background.map(Background::Color),
            text_color: palette.background.base.text,
            border: Border { radius: 9.0.into(), ..Default::default() },
            ..Default::default()
        }
    }
}

pub(super) fn filter_option(
    i18n: desktop_i18n::I18n,
    label: &'static str,
    selected: bool,
    message: Message,
) -> Element<'static, Message> {
    let mut content = row![text(i18n.__(label)).size(14), Space::new().width(Length::Fill)]
        .align_y(iced::Alignment::Center);
    if selected {
        content = content
            .push(header_icon(Icon::Check).width(Length::Fixed(15.0)).height(Length::Fixed(15.0)));
    }
    button(content)
        .on_press(message)
        .padding([9, 11])
        .width(Length::Fill)
        .style(filter_option_style(selected))
        .into()
}

pub(super) fn filter_divider() -> Element<'static, Message> {
    container(Space::new().height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(
                theme.extended_palette().background.strong.color.scale_alpha(0.52),
            )),
            ..Default::default()
        })
        .into()
}

pub(super) fn filter_popover_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        text_color: Some(palette.background.weak.text),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.86),
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: iced::Color::BLACK.scale_alpha(0.28),
            offset: Vector::new(0.0, 10.0),
            blur_radius: 28.0,
        },
        ..Default::default()
    }
}

pub(super) fn filter_popover(app: &ViewInput<'_>) -> Element<'static, Message> {
    let scope = app.work_sidebar_filter_scope;
    let status = app.work_sidebar_filter_status;
    let time = app.work_sidebar_filter_time;
    let content = column![
        text(desktop_i18n::I18n::new(app.desktop_locale).__("筛选状态")).size(12).style(
            |theme: &Theme| text::Style { color: Some(theme.palette().text.scale_alpha(0.52)) }
        ),
        filter_option(
            desktop_i18n::I18n::new(app.desktop_locale),
            desktop_i18n::I18n::new(app.desktop_locale).__("全部状态"),
            status == WorkSidebarFilterStatus::All,
            Message::View(ViewMessage::WorkSidebarFilterStatusSelected(
                WorkSidebarFilterStatus::All,
            )),
        ),
        filter_option(
            desktop_i18n::I18n::new(app.desktop_locale),
            desktop_i18n::I18n::new(app.desktop_locale).__("进行中"),
            status == WorkSidebarFilterStatus::InProgress,
            Message::View(ViewMessage::WorkSidebarFilterStatusSelected(
                WorkSidebarFilterStatus::InProgress,
            )),
        ),
        filter_option(
            desktop_i18n::I18n::new(app.desktop_locale),
            desktop_i18n::I18n::new(app.desktop_locale).__("已完成"),
            status == WorkSidebarFilterStatus::Completed,
            Message::View(ViewMessage::WorkSidebarFilterStatusSelected(
                WorkSidebarFilterStatus::Completed,
            )),
        ),
        filter_option(
            desktop_i18n::I18n::new(app.desktop_locale),
            desktop_i18n::I18n::new(app.desktop_locale).__("待处理"),
            status == WorkSidebarFilterStatus::Pending,
            Message::View(ViewMessage::WorkSidebarFilterStatusSelected(
                WorkSidebarFilterStatus::Pending,
            )),
        ),
        filter_divider(),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("筛选范围")).size(12).style(
            |theme: &Theme| text::Style { color: Some(theme.palette().text.scale_alpha(0.52)) }
        ),
        filter_option(
            desktop_i18n::I18n::new(app.desktop_locale),
            desktop_i18n::I18n::new(app.desktop_locale).__("全部项目"),
            scope == WorkSidebarFilterScope::All,
            Message::View(
                ViewMessage::WorkSidebarFilterScopeSelected(WorkSidebarFilterScope::All,)
            ),
        ),
        filter_option(
            desktop_i18n::I18n::new(app.desktop_locale),
            desktop_i18n::I18n::new(app.desktop_locale).__("当前项目"),
            scope == WorkSidebarFilterScope::CurrentProject,
            Message::View(ViewMessage::WorkSidebarFilterScopeSelected(
                WorkSidebarFilterScope::CurrentProject,
            )),
        ),
        filter_divider(),
        text(desktop_i18n::I18n::new(app.desktop_locale).__("筛选时间")).size(12).style(
            |theme: &Theme| text::Style { color: Some(theme.palette().text.scale_alpha(0.52)) }
        ),
        filter_option(
            desktop_i18n::I18n::new(app.desktop_locale),
            desktop_i18n::I18n::new(app.desktop_locale).__("全部时间"),
            time == WorkSidebarFilterTime::All,
            Message::View(ViewMessage::WorkSidebarFilterTimeSelected(WorkSidebarFilterTime::All,)),
        ),
        filter_option(
            desktop_i18n::I18n::new(app.desktop_locale),
            desktop_i18n::I18n::new(app.desktop_locale).__("最近 24 小时"),
            time == WorkSidebarFilterTime::LastDay,
            Message::View(ViewMessage::WorkSidebarFilterTimeSelected(
                WorkSidebarFilterTime::LastDay,
            )),
        ),
        filter_option(
            desktop_i18n::I18n::new(app.desktop_locale),
            desktop_i18n::I18n::new(app.desktop_locale).__("最近 7 天"),
            time == WorkSidebarFilterTime::LastSevenDays,
            Message::View(ViewMessage::WorkSidebarFilterTimeSelected(
                WorkSidebarFilterTime::LastSevenDays,
            )),
        ),
        filter_option(
            desktop_i18n::I18n::new(app.desktop_locale),
            desktop_i18n::I18n::new(app.desktop_locale).__("最近 30 天"),
            time == WorkSidebarFilterTime::LastThirtyDays,
            Message::View(ViewMessage::WorkSidebarFilterTimeSelected(
                WorkSidebarFilterTime::LastThirtyDays,
            )),
        ),
        filter_divider(),
        filter_option(
            desktop_i18n::I18n::new(app.desktop_locale),
            desktop_i18n::I18n::new(app.desktop_locale).__("重置筛选条件"),
            false,
            Message::View(ViewMessage::WorkSidebarFilterReset),
        ),
    ]
    .spacing(4);

    container(content)
        .padding([12, 14])
        .width(Length::Fixed(264.0))
        .style(filter_popover_style)
        .into()
}

pub(super) fn filter_control(app: &ViewInput<'_>) -> Element<'static, Message> {
    let (icon, label) = header_icon_specs(app.work_sidebar_collapsed)[4];
    let trigger = header_icon_button_sized(
        desktop_i18n::I18n::new(app.desktop_locale),
        header_icon(icon),
        label,
        Message::View(ViewMessage::WorkSidebarFilterToggled),
        app.work_sidebar_filter_open || filter_active(app),
        WORK_FILTER_CONTROL_SIZE,
    );
    BelowOverlay::new(trigger, filter_popover(app))
        .show(app.work_sidebar_filter_open)
        .gap(6.0)
        .on_close(Message::View(ViewMessage::WorkSidebarFilterClosed))
        .into()
}

pub(super) fn search_control(i18n: desktop_i18n::I18n) -> Element<'static, Message> {
    let (icon, label) = header_icon_specs(false)[3];
    header_icon_button(
        i18n,
        icon,
        label,
        Message::Batch(vec![
            Message::View(ViewMessage::WorkSidebarFilterClosed),
            Message::View(ViewMessage::WorkSidebarMoreClosed),
            Message::Search(SearchMessage::Open),
        ]),
        false,
    )
}

pub(super) fn sidebar_toggle_control(app: &ViewInput<'_>) -> Element<'static, Message> {
    let (icon, label) = header_icon_specs(app.work_sidebar_collapsed)[0];
    header_icon_button(
        desktop_i18n::I18n::new(app.desktop_locale),
        icon,
        label,
        Message::View(ViewMessage::WorkSidebarToggled),
        false,
    )
}

pub fn refresh_all_message(app: &ViewInput<'_>) -> Message {
    let mut seen = std::collections::HashSet::new();
    Message::Batch(
        app.recent_projects
            .iter()
            .filter(|path| seen.insert((*path).clone()))
            .cloned()
            .map(|path| Message::Project(ProjectMessage::ProjectLoadSessions(path)))
            .collect(),
    )
}

pub fn refresh_all_control(app: &ViewInput<'_>) -> Element<'static, Message> {
    let (icon, label) = header_icon_specs(app.work_sidebar_collapsed)[1];
    let is_refreshing =
        app.recent_projects.iter().any(|path| app.project_sessions_loading.contains(path));
    header_icon_button(
        desktop_i18n::I18n::new(app.desktop_locale),
        icon,
        label,
        refresh_all_message(app),
        is_refreshing,
    )
}

pub fn task_pool_control(app: &ViewInput<'_>) -> Element<'static, Message> {
    let (icon, label) = header_icon_specs(app.work_sidebar_collapsed)[2];
    header_icon_button(
        desktop_i18n::I18n::new(app.desktop_locale),
        icon,
        label,
        Message::TaskBoard(TaskBoardMessage::ToggleBoard),
        false,
    )
}

pub(super) fn expanded_header(app: &ViewInput<'_>) -> Element<'static, Message> {
    row![
        container(text(desktop_i18n::I18n::new(app.desktop_locale).__("识矩")).size(17))
            .width(Length::Fill)
            .align_y(iced::alignment::Vertical::Center),
        sidebar_toggle_control(app),
        task_pool_control(app),
        search_control(desktop_i18n::I18n::new(app.desktop_locale)),
        filter_control(app),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill)
    .into()
}
