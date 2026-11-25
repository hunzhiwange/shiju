//! 原生工作台视图，沿用桌面主题、分栏和选择控件。
use super::{Event, Workbench, files, git_busy, message};
use crate::app::message::git::GitMessage;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_git_ui::page;
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, icon_svg, rounded_action_btn_style,
    settings_divider_style, settings_muted_text_style, settings_select, settings_text_input_style,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::text::Wrapping;
use iced::widget::{
    button, column, container, pane_grid, responsive, row, scrollable, text, text_input, tooltip,
};
use iced::{Alignment, Element, Length, Theme};
use vw_api_file::file::FileNodeKind;

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.git_workbench;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    responsive(move |size| {
        if size.width < 680.0 {
            return column![container(projects(i18n, state)).height(154), workspace(app, i18n),]
                .spacing(2)
                .height(Length::Fill)
                .into();
        }
        let grid = pane_grid::PaneGrid::new(&state.layout, |_, main, _| {
            pane_grid::Content::new(if *main {
                workspace(app, i18n)
            } else {
                projects(i18n, state)
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

fn projects<'a>(i18n: I18n, state: &'a Workbench) -> Element<'a, Message> {
    let heading = row![
        text(i18n.__("项目")).size(14).width(Length::Fill),
        action(
            Icon::ArrowRepeat,
            i18n.__("刷新"),
            Some(Event::RefreshProjects).filter(|_| !state.busy() && !state.projects_loading)
        ),
    ]
    .align_y(Alignment::Center);
    let search = text_input(i18n.__("搜索项目"), &state.query)
        .on_input(|query| message(Event::SearchProjects(query)))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style);
    let query = state.query.trim().to_lowercase();
    let mut rows = column![].spacing(2).width(Length::Fill);
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
        let labels = column![
            text(project.name.as_str()).size(13).wrapping(Wrapping::Glyph),
            text(project.display_path.as_str())
                .size(11)
                .style(settings_muted_text_style)
                .wrapping(Wrapping::Glyph),
        ]
        .spacing(3)
        .width(Length::Fill);
        rows = rows.push(select_row(
            Icon::FolderClosed,
            labels.into(),
            (!state.busy()).then(|| Event::SelectProject(id.to_owned())),
            selected,
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
        rows = rows.push(text(label).size(12).style(settings_muted_text_style));
    }
    if let Some(error) = &state.projects_error {
        rows = rows.push(text(error.as_str()).size(12).wrapping(Wrapping::Glyph));
    }
    panel(column![heading, search, vertical(rows)].spacing(8), 9)
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
            file_workspace(i18n, state, narrow)
        } else if state.repo.is_some() {
            page::workbench_view(&git_input(app), size.width >= 740.0)
                .map(move |event| map_git(state, event))
        } else {
            centered(if !state.local() {
                i18n.__("Git 变更管理需要本机网关")
            } else {
                i18n.__("未检测到 Git 仓库")
            })
        };
        panel(column![identity, controls, body].spacing(10), 10)
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

fn file_workspace<'a>(i18n: I18n, state: &'a Workbench, narrow: bool) -> Element<'a, Message> {
    let files = &state.files;
    let current = files.directory.as_str();
    let parent = current.rsplit_once('/').map_or("", |(parent, _)| parent);
    let title = if current.is_empty() { i18n.__("根目录") } else { current };
    let heading = row![
        text(title).size(12).width(Length::Fill).wrapping(Wrapping::Glyph),
        action(
            Icon::ArrowLeft,
            i18n.__("上一级"),
            (!current.is_empty()).then(|| Event::Files(files::Event::Folder(parent.to_owned()))),
        ),
    ]
    .spacing(4)
    .align_y(Alignment::Center);
    let mut rows = column![].spacing(2);
    match &files.listing {
        Some(Ok(entries)) if !entries.is_empty() => {
            for file in entries {
                let folder = file.kind == FileNodeKind::Directory;
                let selected = files.preview.as_ref().and_then(|preview| preview.path.as_deref())
                    == Some(file.path.as_str());
                rows = rows.push(select_row(
                    if folder { Icon::FolderClosed } else { Icon::FileText },
                    text(file.name.as_str()).size(13).wrapping(Wrapping::Glyph).into(),
                    Some(Event::Files(if folder {
                        files::Event::Folder(file.path.clone())
                    } else {
                        files::Event::Open(file.path.clone())
                    })),
                    selected,
                ));
            }
        }
        Some(Ok(_)) => {
            rows = rows.push(text(i18n.__("目录为空")).size(12).style(settings_muted_text_style))
        }
        Some(Err(error)) => {
            rows = rows.push(text(error.as_str()).size(12).wrapping(Wrapping::Glyph))
        }
        None => rows = rows.push(text(i18n.__("加载中")).size(12).style(settings_muted_text_style)),
    }
    let file_list = panel(column![heading, vertical(rows)].spacing(5), 7);
    let preview = file_preview(i18n, state);
    if narrow {
        column![container(file_list).height(145), preview].spacing(5).height(Length::Fill).into()
    } else {
        row![container(file_list).width(225), preview].spacing(6).height(Length::Fill).into()
    }
}

fn file_preview<'a>(i18n: I18n, state: &'a Workbench) -> Element<'a, Message> {
    let Some(file) = &state.files.preview else {
        return centered(i18n.__("未选择文件"));
    };
    let title = row![
        text(file.title.as_str()).size(13).width(Length::Fill).wrapping(Wrapping::Glyph),
        file.truncated.then(|| text(i18n.__("已截断")).size(11).style(settings_muted_text_style)),
        (state.local() && file.path.is_some() && matches!(&file.content, Some(Ok(_)))).then(|| {
            action(
                Icon::Pencil,
                i18n.__("在编辑器中打开"),
                Some(Event::Files(files::Event::OpenInEditor)),
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
    panel(column![title, body].spacing(8), 9)
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

fn select_row<'a>(
    icon: Icon,
    label: Element<'a, Message>,
    event: Option<Event>,
    selected: bool,
) -> Element<'a, Message> {
    button(
        row![icon_svg(icon, 14.0), container(label).width(Length::Fill)]
            .spacing(7)
            .align_y(Alignment::Center),
    )
    .padding([6, 7])
    .width(Length::Fill)
    .style(move |theme, status| selected_style(theme, status, selected))
    .on_press_maybe(event.map(message))
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
