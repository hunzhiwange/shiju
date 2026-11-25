//! 原生工作台视图，沿用桌面主题、分栏和选择控件。
use super::{Event, Workbench, files, git_busy, message};
use crate::app::components::workbench_sidebar::{
    project_row, sidebar_header, sidebar_note, sidebar_section,
};
use crate::app::message::git::GitMessage;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_git_ui::page;
use desktop_i18n::I18n;
use desktop_ui_controls::icons::icon_svg;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, icon_btn, rounded_action_btn_style,
    settings_divider_style, settings_muted_text_style, settings_select, settings_text_input_style,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::text::Wrapping;
use iced::widget::{
    button, column, container, keyed_column, pane_grid, responsive, row, scrollable, text,
    text_input, tooltip,
};
use iced::{Alignment, Element, Length, Theme};

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.git_workbench;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    responsive(move |size| {
        if size.width < 680.0 {
            return column![
                container(projects(i18n, state, tooltip::Position::Bottom)).height(154),
                workspace(app, i18n),
            ]
            .spacing(2)
            .height(Length::Fill)
            .into();
        }
        let grid = pane_grid::PaneGrid::new(&state.layout, |_, main, _| {
            pane_grid::Content::new(if *main {
                workspace(app, i18n)
            } else {
                projects(i18n, state, tooltip::Position::Right)
            })
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(1.0)
        .on_resize(8.0, |event| message(Event::Resize(event)))
        .style(divider);
        container(grid).style(settings_divider_style).into()
    })
    .into()
}

fn divider(theme: &Theme) -> pane_grid::Style {
    let mut style = pane_grid::default(theme);
    style.picked_split.width = 2.0;
    style.hovered_split.width = 2.0;
    style
}

fn projects<'a>(
    i18n: I18n,
    state: &'a Workbench,
    tooltip_position: tooltip::Position,
) -> Element<'a, Message> {
    let heading = sidebar_header(
        i18n.__("项目"),
        icon_btn(
            Icon::ArrowClockwise,
            i18n.__("刷新"),
            (!state.busy() && !state.projects_loading).then_some(message(Event::RefreshProjects)),
        ),
    );
    let search = text_input(i18n.__("搜索项目"), &state.query)
        .on_input(|query| message(Event::SearchProjects(query)))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style);
    let query = state.query.trim().to_lowercase();
    let mut rows = column![].spacing(3);
    let mut visible = state
        .projects
        .iter()
        .filter(|project| {
            query.is_empty()
                || project.name.to_lowercase().contains(&query)
                || project.directory.to_lowercase().contains(&query)
        })
        .peekable();
    let empty = visible.peek().is_none();
    for project in visible {
        let id = project.id.as_ref();
        let selected = state.selected.as_deref() == Some(id);
        rows = rows.push(project_row(
            &project.name,
            &project.directory,
            selected,
            (!state.busy()).then(|| message(Event::SelectProject(id.to_owned()))),
            tooltip_position,
        ));
    }
    if empty {
        let label = if state.projects_loading {
            i18n.__("加载中")
        } else if state.projects_error.is_some() {
            i18n.__("读取项目失败")
        } else if state.projects.is_empty() {
            i18n.__("暂无项目")
        } else {
            i18n.__("未找到匹配的项目")
        };
        rows = rows.push(sidebar_note(label));
    }
    if let Some(error) = &state.projects_error {
        rows = rows.push(sidebar_note(error).wrapping(Wrapping::Glyph));
    }
    sidebar_section(column![heading, container(search).padding([2, 7])].into(), rows)
}

fn workspace<'a>(app: &'a App, i18n: I18n) -> Element<'a, Message> {
    let state = &app.git_workbench;
    let Some(project) = state.project() else {
        let label =
            if state.projects_loading { i18n.__("加载中") } else { i18n.__("请选择项目") };
        return panel(centered(label), 12);
    };
    responsive(move |size| {
        let narrow = size.width < 590.0;
        let modes = row![
            tab(Icon::GitBranch, "Git", !state.show_files, Event::ShowFiles(false)),
            tab(Icon::FolderOpen, i18n.__("文件"), state.show_files, Event::ShowFiles(true)),
        ]
        .spacing(4);
        let identity = row![
            icon_svg(Icon::FolderClosed, 16.0),
            text(project.name.as_str()).size(17).width(Length::Fill).wrapping(Wrapping::Glyph),
            modes,
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let worktrees = &state.git.git_worktree_options;
        let selected = worktrees
            .iter()
            .find(|item| Some(item.directory.as_str()) == state.checkout_directory())
            .cloned();
        let worktree = settings_select(worktrees.iter().cloned(), selected, move |option| {
            state.source().message(GitMessage::SelectGitWorktree(option))
        })
        .format_labels(move |option| {
            if Some(option.directory.as_str()) == state.project_directory() {
                i18n.__("主工作区").to_owned()
            } else {
                option.label.split('·').next().unwrap_or(&option.label).trim().to_owned()
            }
        })
        .placeholder(i18n.__("主工作区"))
        .disabled(state.busy() || worktrees.len() < 2)
        .height(29.0)
        .width(Length::FillPortion(1));
        let mut controls =
            row![selector(i18n.__("工作树"), worktree.into()),].align_y(Alignment::End).spacing(8);
        if !state.show_files {
            let branch = settings_select(
                state.git.branches.iter().cloned(),
                state.git.selected_branch.clone(),
                |branch| message(Event::SwitchBranch(branch)),
            )
            .placeholder(i18n.__("当前分支"))
            .disabled(state.busy())
            .height(29.0)
            .width(Length::Fill);
            controls = controls.push(selector(i18n.__("当前分支"), branch.into()));
        }
        controls = controls.push(action(
            Icon::ArrowRepeat,
            i18n.__("刷新"),
            (!state.busy()).then_some(Event::RefreshCheckout),
        ));
        let body = if state.switching_branch {
            centered(i18n.__("正在切换分支…"))
        } else if state.show_files {
            file_preview(i18n, state)
        } else if state.repo.is_some() {
            page::workbench_view(&git_input(app)).map(move |event| map_git(state, event))
        } else {
            centered(if !state.local() {
                i18n.__("Git 变更管理需要本机网关")
            } else {
                i18n.__("未检测到 Git 仓库")
            })
        };
        let files = file_manager(app);
        let workspace = if narrow {
            Element::from(
                column![
                    container(files).height(Length::FillPortion(1)),
                    container(body).height(Length::FillPortion(1)),
                ]
                .spacing(6)
                .height(Length::Fill),
            )
        } else {
            row![
                container(files).width(270),
                container(iced::widget::Space::new())
                    .width(1)
                    .height(Length::Fill)
                    .style(settings_divider_style),
                body,
            ]
            .spacing(6)
            .height(Length::Fill)
            .into()
        };
        panel(column![identity, controls, workspace].spacing(10), 10)
    })
    .into()
}

fn selector<'a>(title: &'a str, control: Element<'a, Message>) -> Element<'a, Message> {
    column![text(title).size(11).style(settings_muted_text_style), control]
        .spacing(4)
        .width(Length::FillPortion(1))
        .max_width(236)
        .into()
}

fn file_manager(app: &App) -> Element<'_, Message> {
    let state = &app.git_workbench;
    let files = &state.files;
    let input = desktop_file_tree::FileTreeView {
        browse_only: true,
        file_tree_drop_target: &None,
        file_tree_operation_in_progress: false,
        file_tree_selection: &files.selection,
        file_tree_expanded_set: &files.expanded,
        git_changed_files: &state.git.git_changed_files,
        git_changed_files_loading: state.git.git_changed_files_loading,
        git_focused_file: &state.git.git_focused_file,
        file_manager_refresh_frame: files.refresh_frame,
        file_manager_changes_refreshing: files.changes_refreshing,
        file_manager_file_tree_refreshing: files.files_refreshing,
        file_manager_show_changes: !files.show_all_files,
        file_index: &[],
        file_tree_model: Some(&files.tree),
        desktop_locale: app.general_settings.desktop_locale,
        project_path: &files.root,
        find_results_tabs: &files.find.find_results_tabs,
        active_find_results_tab_id: &files.find.active_find_results_tab_id,
        find_results_tab_menu_id: &files.find.find_results_tab_menu_id,
        find_results_tab_menu_pos: files.find.find_results_tab_menu_pos,
        file_tree_rename_value: "",
        file_tree_rename_path: &None,
        file_tree_menu_path: &files.menu_path,
        file_tree_menu_source: &files.menu_source,
        file_tree_menu_anchor: files.menu_anchor,
        can_paste: false,
    };
    let filtered = page::filtered_workbench_changes(&git_input(app));
    let changes = filtered.as_deref().unwrap_or(input.git_changed_files);
    desktop_file_tree::view_file_manager_with_changes(input, changes).map(
        move |event| match event {
            desktop_file_tree::Message::Project(event) => {
                state.file_message(files::Event::Tree(event))
            }
            desktop_file_tree::Message::None => Message::None,
        },
    )
}

fn file_preview<'a>(i18n: I18n, state: &'a Workbench) -> Element<'a, Message> {
    let Some(file) = &state.files.preview else {
        return centered(i18n.__("未选择文件"));
    };
    let title = row![
        text(file.title.as_str()).size(13).width(Length::Fill).wrapping(Wrapping::Glyph),
        file.location.map(|(line, column)| text(format!("{line}:{column}"))
            .size(11)
            .style(settings_muted_text_style)),
        file.truncated.then(|| text(i18n.__("已截断")).size(11).style(settings_muted_text_style)),
        (state.local() && file.path.is_some() && matches!(&file.content, Some(Ok(_)))).then(|| {
            action(
                Icon::Pencil,
                i18n.__("在编辑器中打开"),
                Some(Event::Files(state.epoch, files::Event::OpenInEditor)),
            )
        }),
    ]
    .spacing(7)
    .align_y(Alignment::Center);
    let body = match &file.content {
        Some(Ok(content)) => {
            let font = if content.is_ascii() { iced::Font::MONOSPACE } else { iced::Font::DEFAULT };
            vertical(text(content.as_str()).size(13).font(font).wrapping(Wrapping::Glyph))
        }
        Some(Err(error)) => centered(error),
        None => centered(i18n.__("加载中")),
    };
    panel(
        column![title, keyed_column([(state.files.preview_request, body)]).height(Length::Fill)]
            .spacing(8),
        9,
    )
}

fn git_input(app: &App) -> page::ViewContext<'_> {
    let state = &app.git_workbench;
    page::ViewContext {
        git: &state.git,
        app_theme: &app.appearance.app_theme,
        desktop_locale: app.general_settings.desktop_locale,
        project_path: &state.git.git_worktree_options_project_path,
        repo_path: state.repo_path(),
        context_path: state.repo_path(),
        code_mode: false,
        refresh_frame: 0,
        last_copied_code_hash: app.clipboard_state.last_copied_code_hash,
    }
}

fn map_git(state: &Workbench, event: page::Message) -> Message {
    match event {
        page::Message::Git(event) => state.source().message(event),
        page::Message::CopyCode(value) => Message::CopyCode(value),
        page::Message::ToggleSummary => message(Event::ToggleSummary),
        page::Message::AppendChat(value) => {
            Message::Chat(crate::app::message::ChatMessage::AppendText(value))
        }
        page::Message::OpenUrl(value) => {
            Message::View(crate::app::message::ViewMessage::OpenUrlExternal(value))
        }
        page::Message::OpenTerminal | page::Message::None | page::Message::SelectProject(_) => {
            Message::None
        }
    }
}

pub(crate) fn with_overlays<'a>(app: &'a App, root: Element<'a, Message>) -> Element<'a, Message> {
    let state = &app.git_workbench;
    let input = git_input(app);
    desktop_git_ui::modal_overlays::with_git_diff_overlays(
        page::diff_comment_modal(&input).map(|modal| modal.map(move |event| map_git(state, event))),
        page::discard_file_modal(&input).map(|modal| modal.map(move |event| map_git(state, event))),
        root,
    )
}

pub(crate) fn inline_project_picker(app: &App) -> Option<Element<'static, page::Message>> {
    let current = app.navigation_tabs.project_path.clone()?;
    let paths = &app.workspace_state.recent_projects;
    if !paths.iter().any(|path| path != &current) {
        return None;
    }
    let label = |path: &str| {
        desktop_workspace::basic::display_name_for_path(
            &app.workspace_state.recent_projects_meta,
            path,
        )
    };
    let mut options = paths.clone();
    if !options.contains(&current) {
        options.insert(0, current.clone());
    }
    Some(
        settings_select(options, Some(current.clone()), page::Message::SelectProject)
            .format_labels(|path| format!("{} · {path}", label(path)))
            .trigger_label(label(&current))
            .disabled(git_busy(&app.git))
            .width(115)
            .height(26.0)
            .into(),
    )
}

fn panel<'a>(content: impl Into<Element<'a, Message>>, padding: u16) -> Element<'a, Message> {
    container(content)
        .padding(padding)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme| container::Style {
            background: Some(theme.extended_palette().background.base.color.into()),
            ..Default::default()
        })
        .into()
}

fn centered(label: &str) -> Element<'_, Message> {
    container(text(label).size(13).style(settings_muted_text_style).wrapping(Wrapping::Glyph))
        .center(Length::Fill)
        .padding(10)
        .into()
}

fn vertical<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    scrollable(container(content).width(Length::Fill).padding(iced::Padding::ZERO.right(7)))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn action<'a>(icon: Icon, label: &'static str, event: Option<Event>) -> Element<'a, Message> {
    tooltip(
        button(icon_svg(icon, 14.0))
            .padding([6, 7])
            .style(rounded_action_btn_style)
            .on_press_maybe(event.map(message)),
        container(text(label).size(12)).padding(6).style(container::bordered_box),
        tooltip::Position::Bottom,
    )
    .into()
}

fn tab<'a>(icon: Icon, label: &'a str, selected: bool, event: Event) -> Element<'a, Message> {
    button(row![icon_svg(icon, 14.0), text(label).size(13)].spacing(5).align_y(Alignment::Center))
        .padding([6, 9])
        .style(move |theme, status| selected_style(theme, status, selected))
        .on_press(message(event))
        .into()
}

fn selected_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let mut style = rounded_action_btn_style(theme, status);
    if selected && matches!(status, button::Status::Active | button::Status::Disabled) {
        style.background =
            Some(theme.extended_palette().primary.base.color.scale_alpha(0.14).into());
    }
    style
}
