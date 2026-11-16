use iced::widget::column;
use super::*;

pub(super) fn space_tools_menu_action(
    i18n: desktop_i18n::I18n,
    label: &'static str,
    icon: Icon,
    message: Message,
) -> Element<'static, Message> {
    const MENU_ICON_SIZE: f32 = 15.0;
    button(
        row![
            container(
                navigation_icon(icon)
                    .width(Length::Fixed(MENU_ICON_SIZE))
                    .height(Length::Fixed(MENU_ICON_SIZE)),
            )
            .width(Length::Fixed(28.0)),
            text(i18n.__(label)).size(15),
            Space::new().width(Length::Fill),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .width(Length::Fill),
    )
    .on_press(Message::Batch(vec![
        Message::Project(ProjectMessage::ProjectToolsMenuClosed),
        message,
    ]))
    .padding([11, 12])
    .width(Length::Fill)
    .style(navigation_button_style)
    .into()
}

pub(super) fn space_tools_menu(i18n: desktop_i18n::I18n, path: String) -> Element<'static, Message> {
    container(
        column![
            space_tools_menu_action(
                i18n,
                i18n.__("刷新任务"),
                Icon::ArrowRepeat,
                Message::Project(ProjectMessage::ProjectLoadSessions(path.clone())),
            ),
            space_tools_menu_action(
                i18n,
                i18n.__("编辑项目"),
                Icon::Sliders,
                Message::Project(ProjectMessage::ProjectEditOpened(path.clone())),
            ),
            more_menu_divider(),
            space_tools_menu_action(
                i18n,
                i18n.__("打开文件夹"),
                Icon::FolderOpen,
                Message::Project(ProjectMessage::RecentRevealPressed(path.clone())),
            ),
            space_tools_menu_action(
                i18n,
                i18n.__("从列表中移除"),
                Icon::Trash,
                Message::Project(ProjectMessage::RecentRemovePressed(path)),
            ),
        ]
        .spacing(3),
    )
    .padding([10, 12])
    .width(Length::Fixed(220.0))
    .style(more_menu_style)
    .into()
}

pub(super) fn space_row_style(selected: bool) -> impl Fn(&Theme) -> container::Style + Copy {
    move |theme| container::Style {
        background: selected.then(|| {
            Background::Color(theme.extended_palette().background.strong.color.scale_alpha(0.72))
        }),
        border: Border { radius: 9.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub(super) fn space_button(i18n: desktop_i18n::I18n, space: &WorkSpaceItem) -> Element<'static, Message> {
    const SPACE_FOLDER_ICON_SIZE: f32 = 16.0;
    const SPACE_ACTION_ICON_SIZE: f32 = 14.0;
    let action_icon = |icon| {
        header_icon(icon)
            .width(Length::Fixed(SPACE_ACTION_ICON_SIZE))
            .height(Length::Fixed(SPACE_ACTION_ICON_SIZE))
    };

    let switch = button(
        row![
            navigation_icon(Icon::FolderOpen)
                .width(Length::Fixed(SPACE_FOLDER_ICON_SIZE))
                .height(Length::Fixed(SPACE_FOLDER_ICON_SIZE)),
            text(space.label.clone()).size(14).wrapping(text::Wrapping::None),
        ]
        .spacing(9)
        .align_y(iced::Alignment::Center),
    )
    .on_press(space.message.clone())
    .padding([7, 5])
    .width(Length::Shrink)
    .style(navigation_button_style);

    let toggle = header_icon_button_sized(
        i18n,
        action_icon(if space.expanded { Icon::ChevronUp } else { Icon::ChevronDown }),
        if space.expanded { i18n.__("收起任务") } else { i18n.__("展开任务") },
        space.toggle_message.clone(),
        false,
        28.0,
    );

    let actions: Element<'static, Message> = if space.hovered || space.tools_open {
        let tools_trigger = header_icon_button_sized(
            i18n,
            action_icon(Icon::DotsThreeVertical),
            i18n.__("空间操作"),
            space.tools_message.clone(),
            space.tools_open,
            30.0,
        );
        BelowOverlay::new(tools_trigger, space_tools_menu(i18n, space.path.clone()))
            .show(space.tools_open)
            .gap(6.0)
            .on_close(Message::Project(ProjectMessage::ProjectToolsMenuClosed))
            .into()
    } else {
        Space::new().width(Length::Fixed(30.0)).into()
    };

    let path = space.path.clone();
    mouse_area(
        container(
            row![switch, toggle, Space::new().width(Length::Fill), actions,]
                .spacing(2)
                .align_y(iced::Alignment::Center)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .style(space_row_style(space.selected)),
    )
    .on_enter(Message::Project(ProjectMessage::RecentHovered(Some(path))))
    .on_exit(Message::Project(ProjectMessage::RecentHovered(None)))
    .into()
}

pub(super) fn new_task_button(app: &ViewInput<'_>) -> Element<'static, Message> {
    let (label, icon, _) = navigation_specs()[0];
    navigation_button_maybe(
        desktop_i18n::I18n::new(app.desktop_locale),
        if app.new_session_creating { "创建中..." } else { label },
        icon,
        (!app.new_session_creating).then_some(Message::View(ViewMessage::ProjectFileNewSession)),
        None,
    )
}

pub(super) fn navigation_section(app: &ViewInput<'_>) -> Element<'static, Message> {
    let specs = navigation_specs();
    let navigation = column![new_task_button(app)].spacing(2);

    #[cfg(not(target_arch = "wasm32"))]
    let navigation = navigation
        .push(navigation_button(
            desktop_i18n::I18n::new(app.desktop_locale),
            specs[1].0,
            specs[1].1,
            expert_hub_message(ExpertHubSection::Experts),
            specs[1].2,
        ))
        .push(project_navigation_control(app))
        .push(shiju_apps_button(desktop_i18n::I18n::new(app.desktop_locale)))
        .push(shiju_skills_button(desktop_i18n::I18n::new(app.desktop_locale)))
        .push(expert_hub_button(desktop_i18n::I18n::new(app.desktop_locale)));

    #[cfg(target_arch = "wasm32")]
    let navigation = navigation.push(project_navigation_control(app));

    let navigation = navigation.push(navigation_button(
        desktop_i18n::I18n::new(app.desktop_locale),
        specs[6].0,
        specs[6].1,
        Message::View(ViewMessage::OpenSystemSettingsTab(SystemTab::Cron)),
        specs[6].2,
    ));
    #[cfg(not(target_arch = "wasm32"))]
    let navigation = navigation.push(navigation_button(
        desktop_i18n::I18n::new(app.desktop_locale),
        specs[7].0,
        specs[7].1,
        expert_hub_message(ExpertHubSection::Knowledge),
        specs[7].2,
    ));
    navigation.push(more_control(app)).into()
}

pub fn project_navigation_message() -> Message {
    Message::View(ViewMessage::OpenSystemSettingsTab(SystemTab::Projects))
}

pub fn project_create_message() -> Message {
    Message::View(ViewMessage::ProjectFileNewProject)
}

pub(super) fn project_navigation_control(app: &ViewInput<'_>) -> Element<'static, Message> {
    const CREATE_CONTROL_SIZE: f32 = 30.0;
    let (_, icon, _) = navigation_specs()[2];
    let switch = button(
        row![
            container(navigation_icon(icon))
                .width(Length::Fixed(WORK_NAVIGATION_ICON_COLUMN_WIDTH)),
            text(desktop_i18n::I18n::new(app.desktop_locale).__("项目")).size(14),
            Space::new().width(Length::Fill),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .width(Length::Fill),
    )
    .on_press(project_navigation_message())
    .padding([9, 11])
    .width(Length::Fill)
    .style(navigation_button_style);

    let create = header_icon_button_sized(
        desktop_i18n::I18n::new(app.desktop_locale),
        header_icon(Icon::Plus),
        desktop_i18n::I18n::new(app.desktop_locale).__("新建项目"),
        project_create_message(),
        false,
        CREATE_CONTROL_SIZE,
    );

    let content: Element<'static, Message> = if app.work_sidebar_project_nav_hovered {
        Stack::new()
            .push(switch.width(Length::Fill))
            .push(
                container(create)
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Right)
                    .align_y(iced::alignment::Vertical::Center)
                    .padding([9, 12]),
            )
            .width(Length::Fill)
            .into()
    } else {
        switch.width(Length::Fill).into()
    };

    mouse_area(content)
        .on_enter(Message::View(ViewMessage::WorkSidebarProjectNavHovered(true)))
        .on_move(|_| Message::View(ViewMessage::WorkSidebarProjectNavHovered(true)))
        .on_exit(Message::View(ViewMessage::WorkSidebarProjectNavHovered(false)))
        .into()
}

pub(super) fn load_more_button(
    i18n: desktop_i18n::I18n,
    remaining: usize,
    message: Message,
) -> Element<'static, Message> {
    button(
        text(
            i18n.format("加载更多 · 还有 %u 条", &[remaining.into()])
                .expect("valid pagination template"),
        )
        .size(12)
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center),
    )
    .on_press(message)
    .padding([8, 9])
    .width(Length::Fill)
    .style(navigation_button_style)
    .into()
}

pub(super) fn tasks_section(app: &ViewInput<'_>, model: &WorkSidebarModel, render: &dyn Fn(&WorkTaskItem) -> TaskPresentation) -> Element<'static, Message> {
    let mut tasks = column![].spacing(3);
    if model.tasks.is_empty() {
        tasks = tasks.push(
            container(
                text(desktop_i18n::I18n::new(app.desktop_locale).__("暂无任务")).size(12),
            )
            .padding([7, 9])
            .width(Length::Fill),
        );
    } else {
        for task in &model.tasks {
            tasks = tasks.push(task_button(task, render));
        }
        if model.tasks.len() < model.task_total {
            tasks = tasks.push(load_more_button(
                desktop_i18n::I18n::new(app.desktop_locale),
                model.task_total - model.tasks.len(),
                Message::View(ViewMessage::WorkSidebarLoadMorePressed),
            ));
        }
    }
    tasks.into()
}

pub(super) fn spaces_section(app: &ViewInput<'_>, model: &WorkSidebarModel, render: &dyn Fn(&WorkTaskItem) -> TaskPresentation) -> Element<'static, Message> {
    let mut spaces = column![].spacing(2);
    if model.spaces.is_empty() {
        spaces = spaces.push(
            container(
                text(desktop_i18n::I18n::new(app.desktop_locale).__("暂无空间")).size(12),
            )
            .padding([7, 9])
            .width(Length::Fill),
        );
    } else {
        for space in &model.spaces {
            spaces =
                spaces.push(space_button(desktop_i18n::I18n::new(app.desktop_locale), space));
            for session in &space.sessions {
                spaces = spaces.push(task_button(session, render));
            }
            if space.expanded && space.sessions.len() < space.session_total {
                spaces = spaces.push(load_more_button(
                    desktop_i18n::I18n::new(app.desktop_locale),
                    space.session_total - space.sessions.len(),
                    Message::View(ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(
                        space.path.clone(),
                    )),
                ));
            }
        }
        if model.spaces.len() < model.space_total {
            spaces = spaces.push(load_more_button(
                desktop_i18n::I18n::new(app.desktop_locale),
                model.space_total - model.spaces.len(),
                Message::View(ViewMessage::WorkSidebarLoadMorePressed),
            ));
        }
    }
    spaces.into()
}
