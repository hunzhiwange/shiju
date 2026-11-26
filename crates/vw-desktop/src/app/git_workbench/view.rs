//! 原生工作台视图，沿用桌面主题、分栏和选择控件。
use super::{Event, Workbench, files, git_busy, message};
use crate::app::components::workbench_sidebar::{
    matches_project, project_header, project_query, project_row, sidebar_note, sidebar_section,
    with_project_menu,
};
use crate::app::message::git::GitMessage;
use crate::app::{App, Message, Screen};
use desktop_assets::assets::Icon;
use desktop_editor::components::preview_panel::view::{editor_popovers, header_tabs};
use desktop_git_ui::git_panel::types::GitWorktreeOption;
use desktop_git_ui::page;
use desktop_i18n::I18n;
use desktop_ui_controls::icons::icon_svg;
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_divider_style, settings_modal_card, settings_muted_text_style,
    settings_optional_modal_overlay, settings_select,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::text::Wrapping;
use iced::widget::{
    button, column, container, pane_grid, responsive, row, scrollable, text, tooltip,
};
use iced::{Alignment, Element, Length, Theme};

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.git_workbench;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let base = responsive(move |size| {
        if size.width < 680.0 {
            return column![
                container(projects(app, i18n, state, tooltip::Position::Bottom)).height(154),
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
                projects(app, i18n, state, tooltip::Position::Right)
            })
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(1.0)
        .on_resize(8.0, |event| message(Event::Resize(event)))
        .style(divider);
        container(grid).style(settings_divider_style).into()
    });
    with_project_menu(app, Screen::GitWorkbench, base.into())
}

fn divider(theme: &Theme) -> pane_grid::Style {
    let mut style = pane_grid::default(theme);
    style.picked_split.width = 2.0;
    style.hovered_split.width = 2.0;
    style
}

fn projects<'a>(
    app: &'a App,
    i18n: I18n,
    state: &'a Workbench,
    tooltip_position: tooltip::Position,
) -> Element<'a, Message> {
    let heading = project_header(app,
        (!state.busy() && !state.projects_loading).then_some(message(Event::RefreshProjects)),
    );
    let query = project_query(app);
    let mut matches = state.projects.iter().filter(|project| {
        matches_project(&query, &project.name, &project.directory)
    }).peekable();
    let mut rows = column![].spacing(3);
    if matches.peek().is_none() {
        let label = if state.projects_loading {
            i18n.__("加载中")
        } else if state.projects_error.is_some() {
            i18n.__("读取项目失败")
        } else if !query.is_empty() {
            i18n.__("未找到匹配的项目")
        } else {
            i18n.__("暂无项目")
        };
        rows = rows.push(sidebar_note(label));
    }
    for project in matches {
        let id = project.id.as_str();
        let selected = state.selected.as_deref() == Some(id);
        rows = rows.push(project_row(
            project.name.clone(),
            &project.directory,
            Some(id),
            selected,
            (!state.busy()).then(|| message(Event::SelectProject(id.to_owned()))),
            tooltip_position,
        ));
    }
    if let Some(error) = &state.projects_error {
        rows = rows.push(sidebar_note(error).wrapping(Wrapping::Glyph));
    }
    sidebar_section(heading, rows)
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
            tab(Icon::GitBranch, "Git", !state.show_editor, Event::ShowEditor(false)),
            tab(Icon::Pencil, i18n.__("编辑器"), state.show_editor, Event::ShowEditor(true)),
        ]
        .spacing(4);
        let identity = row![
            icon_svg(Icon::FolderClosed, 16.0),
            SingleLineLabel::new(project.name.clone(), 17.0),
            checkout_select(i18n, state, if size.width < 390.0 { 100.0 } else { 132.0 }),
            action(
                Icon::ArrowRepeat,
                i18n.__("刷新"),
                (!state.busy() && !state.repo_loading).then_some(Event::RefreshCheckout),
            ),
            modes,
        ]
        .spacing(5)
        .align_y(Alignment::Center);
        let workspace = if state.switching_branch {
            centered(i18n.__("正在切换分支…"))
        } else if state.show_editor {
            file_preview(i18n, state)
        } else if state.repo.is_some() {
            page::workbench_view(&git_input(app)).map(move |event| map_git(state, event))
        } else if state.repo_loading {
            centered(i18n.__("加载中"))
        } else if !state.local() {
            centered(i18n.__("Git 变更管理需要本机网关"))
        } else {
            centered(i18n.__("未检测到 Git 仓库"))
        };
        let files = file_manager(app);
        let workspace: Element<'_, Message> = if narrow {
            column![
                container(workspace).height(Length::FillPortion(2)),
                container(files).height(Length::FillPortion(1)),
            ]
            .spacing(6)
            .height(Length::Fill)
            .into()
        } else {
            row![
                container(workspace).width(Length::Fill),
                container(iced::widget::Space::new())
                    .width(1)
                    .height(Length::Fill)
                    .style(settings_divider_style),
                container(files).width(270),
            ]
            .spacing(6)
            .height(Length::Fill)
            .into()
        };
        panel(column![identity, workspace].spacing(10), 10)
    })
    .into()
}

#[derive(Clone, PartialEq)]
enum CheckoutOption {
    Branch(String),
    Worktree(GitWorktreeOption),
}

impl std::fmt::Display for CheckoutOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Branch(branch) => f.write_str(branch),
            Self::Worktree(worktree) => f.write_str(&worktree.label),
        }
    }
}

fn checkout_select(i18n: I18n, state: &Workbench, width: f32) -> Element<'_, Message> {
    let worktrees = &state.git.git_worktree_options;
    let current_worktree =
        worktrees.iter().find(|option| Some(option.directory.as_str()) == state.checkout_directory());
    let current_branch = state
        .git
        .selected_branch
        .as_ref()
        .or_else(|| current_worktree.and_then(|option| option.branch.as_ref()));
    let mut options = Vec::new();
    options.extend(state.git.branches.iter().cloned().map(CheckoutOption::Branch));
    if worktrees.len() > 1 {
        options.extend(worktrees.iter().cloned().map(CheckoutOption::Worktree));
    }
    let selected = current_worktree
        .filter(|option| {
            state.git.branches.is_empty()
                || current_branch.is_none()
                || Some(option.directory.as_str()) != state.project_directory()
        })
        .cloned()
        .map(CheckoutOption::Worktree)
        .or_else(|| current_branch.cloned().map(CheckoutOption::Branch));
    let label = |worktree: &GitWorktreeOption| {
        if Some(worktree.directory.as_str()) == state.project_directory() {
            i18n.__("主工作区").to_owned()
        } else {
            worktree.label.split('·').next().unwrap_or(&worktree.label).trim().to_owned()
        }
    };
    let trigger = current_branch
        .cloned()
        .or_else(|| current_worktree.map(&label))
        .unwrap_or_else(|| i18n.__("当前分支").to_owned());
    settings_select(options, selected, move |option| match option {
        CheckoutOption::Branch(branch) => message(Event::SwitchBranch(branch)),
        CheckoutOption::Worktree(worktree) => {
            state.source().message(GitMessage::SelectGitWorktree(worktree))
        }
    })
    .format_labels(|option| match option {
        CheckoutOption::Branch(branch) => branch.clone(),
        CheckoutOption::Worktree(worktree) => {
            format!("{} · {}", i18n.__("工作树"), label(worktree))
        }
    })
    .trigger_label(trigger)
    .disabled(state.busy() || state.repo_loading)
    .height(28.0)
    .width(width)
    .menu_min_width(250.0)
    .into()
}

fn file_manager(app: &App) -> Element<'_, Message> {
    let state = &app.git_workbench;
    let files = &state.files;
    let interaction = &files.interaction;
    let input = desktop_file_tree::FileTreeView {
        drag_enabled: false,
        file_tree_drop_target: &None,
        file_tree_operation_in_progress: interaction.file_tree_operation_in_progress,
        file_tree_selection: &interaction.file_tree_selection,
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
        file_tree_rename_value: &interaction.file_tree_rename_value,
        file_tree_rename_path: &interaction.file_tree_rename_path,
        file_tree_menu_path: &interaction.file_tree_menu_path,
        file_tree_menu_source: &interaction.file_tree_menu_source,
        file_tree_menu_anchor: interaction.file_tree_menu_anchor,
        can_paste: interaction.file_tree_clipboard.is_some(),
    };
    desktop_file_tree::view_file_manager(input).map(move |event| match event {
        desktop_file_tree::Message::Project(event) => state.file_message(files::Event::Tree(event)),
        desktop_file_tree::Message::None => Message::None,
    })
}

fn file_preview<'a>(i18n: I18n, state: &'a Workbench) -> Element<'a, Message> {
    let Some(file) = state.files.active_preview() else {
        return centered(i18n.__("未选择文件"));
    };
    let tabs = state.files.previews.iter().map(|tab| header_tabs::FileTab {
        key: tab.id,
        title: tab.path().and_then(|path| path.rsplit('/').next()).unwrap_or(&tab.title),
        path: &tab.title,
        selected: tab.id == file.id,
        select: state.file_message(files::Event::SelectPreview(tab.id)),
        close: state.file_message(files::Event::ClosePreview(tab.id)),
    });
    let tabs = header_tabs::file_tab_strip(tabs.collect(), None, |_, tab| tab);
    let editor = file.editor().map(|document| &document.editor.inner);
    let open_standalone =
        state.local() && file.path().is_some() && matches!(&file.content, Some(Ok(_)));
    let tools = editor_popovers::tools(
        editor,
        editor.is_none(),
        i18n,
        state.files.editor_popover,
        open_standalone,
    )
    .map(move |action| state.file_message(files::Event::EditorTool(file.id, action)));
    let title = row![
        text(file.title.as_str()).size(13).width(Length::Fill).wrapping(Wrapping::Glyph),
        file.location.map(|(line, column)| text(format!("{line}:{column}"))
            .size(11)
            .style(settings_muted_text_style)),
        matches!(&file.content, Some(Ok(files::PreviewContent::ReadOnly { truncated: true, .. })))
            .then(|| text(i18n.__("已截断")).size(11).style(settings_muted_text_style)),
        tools,
    ]
    .spacing(7)
    .align_y(Alignment::Center);
    let body = match &file.content {
        Some(Ok(files::PreviewContent::Editor(document))) => {
            let id = file.id;
            let epoch = state.epoch;
            document.editor.content_view(move |event| {
                message(Event::Files(epoch, files::Event::Editor(id, event)))
            })
        }
        Some(Ok(files::PreviewContent::ReadOnly { text: content, .. })) => {
            let font = if content.is_ascii() { iced::Font::MONOSPACE } else { iced::Font::DEFAULT };
            vertical(text(content.as_str()).size(13).font(font).wrapping(Wrapping::Glyph))
        }
        Some(Err(error)) => centered(error),
        None => centered(i18n.__("加载中")),
    };
    // 普通 Column 会检查子控件类型，避免加载提示与滚动区共用状态。
    panel(column![tabs, title, body].spacing(8), 9)
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
    let root = desktop_git_ui::modal_overlays::with_git_diff_overlays(
        page::diff_comment_modal(&input).map(|modal| modal.map(move |event| map_git(state, event))),
        page::discard_file_modal(&input).map(|modal| modal.map(move |event| map_git(state, event))),
        root,
    );
    let confirm_close = state.files.closing_preview_id.and_then(|id| {
        let file = state.files.previews.iter().find(|tab| tab.id == id)?;
        let i18n = I18n::new(app.general_settings.desktop_locale);
        let cancel = state.file_message(files::Event::ConfirmClosePreview(false));
        let confirm = state.file_message(files::Event::ConfirmClosePreview(true));
        let card = settings_modal_card(
            column![
                row![
                    text(i18n.__("有未保存的修改")).size(17).width(Length::Fill),
                    settings_close_button(cancel.clone()),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                text(file.title.as_str()).wrapping(Wrapping::Glyph),
                row![
                    iced::widget::Space::new().width(Length::Fill),
                    button(text(i18n.__("取消")))
                        .padding([6, 12])
                        .style(rounded_action_btn_style)
                        .on_press(cancel.clone()),
                    button(text(i18n.__("丢弃更改")))
                        .padding([6, 12])
                        .style(primary_action_btn_style)
                        .on_press(confirm),
                ]
                .spacing(8),
            ]
            .spacing(12),
        )
        .max_width(400);
        Some((cancel, card.into()))
    });
    let root = settings_optional_modal_overlay(root, confirm_close);
    let pending_delete =
        state.files.interaction.file_tree_pending_delete.as_ref().map(|(_, paths)| paths.as_slice());
    desktop_workspace::views::modals::file_tree_delete::with_file_tree_delete_confirm(
        pending_delete,
        app.general_settings.desktop_locale,
        |event| {
            use desktop_workspace::views::modals::file_tree_delete::Event::FileTreeDeleteConfirmed;
            state.file_message(files::Event::ConfirmDelete(matches!(event, FileTreeDeleteConfirmed)))
        },
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
        .id(files::PREVIEW_SCROLL_ID)
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

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
