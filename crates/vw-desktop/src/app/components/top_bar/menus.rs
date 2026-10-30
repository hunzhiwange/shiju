//! 桌面应用顶部栏的按钮、菜单与窗口交互控件。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::widgets::{menu_btn, menu_container, menu_item_btn, menu_separator};
use crate::app::components::overlays::BelowOverlay;
use crate::app::components::top_bar::menu_actions::NativeMenuAction;
use crate::app::message::view::MenuType;
use crate::app::state::{WorkResourceMode, WorkspaceMode};
use crate::app::{App, Message, Screen, message};
use iced::Element;
use iced::widget::Column;

pub(crate) enum MenuEntry {
    Item {
        #[cfg(any(target_os = "macos", test))]
        action: Option<NativeMenuAction>,
        label: &'static str,
        shortcut: Option<&'static str>,
        message: Box<Message>,
    },
    Separator,
}

impl MenuEntry {
    fn item(
        _action: NativeMenuAction,
        label: &'static str,
        shortcut: Option<&'static str>,
        message: Message,
    ) -> Self {
        Self::Item {
            #[cfg(any(target_os = "macos", test))]
            action: Some(_action),
            label,
            shortcut,
            message: Box::new(message),
        }
    }

    fn into_element(self) -> Element<'static, Message> {
        match self {
            Self::Item { label, shortcut, message, .. } => {
                menu_item_btn(label, shortcut, Some(*message))
            }
            Self::Separator => menu_separator(),
        }
    }
}

fn menu_entries(entries: Vec<MenuEntry>) -> Element<'static, Message> {
    Column::with_children(entries.into_iter().map(MenuEntry::into_element)).into()
}

pub(crate) fn file_menu_entries(is_project: bool) -> Vec<MenuEntry> {
    if is_project {
        vec![
            MenuEntry::item(
                NativeMenuAction::FileNewSession,
                "新建会话",
                None,
                Message::View(message::ViewMessage::ProjectFileNewSession),
            ),
            MenuEntry::item(
                NativeMenuAction::FileNewProject,
                "新建项目",
                None,
                Message::View(message::ViewMessage::ProjectFileNewProject),
            ),
            MenuEntry::item(
                NativeMenuAction::FileShowSessions,
                "查看会话",
                None,
                Message::View(message::ViewMessage::ProjectFileShowSessions),
            ),
            MenuEntry::item(
                NativeMenuAction::FileShowProjects,
                "查看项目",
                None,
                Message::View(message::ViewMessage::ProjectFileShowProjects),
            ),
            MenuEntry::Separator,
            MenuEntry::item(
                NativeMenuAction::FileSave,
                "保存文件",
                Some("⌘S"),
                Message::Preview(crate::app::message::PreviewMessage::SaveFile),
            ),
            MenuEntry::item(
                NativeMenuAction::FileSaveAll,
                "保存所有文件",
                Some("⌥⌘S"),
                Message::View(message::ViewMessage::ProjectFileSaveAll),
            ),
        ]
    } else {
        vec![
            MenuEntry::item(
                NativeMenuAction::FileSave,
                "保存代码",
                Some("⌘S"),
                Message::Preview(crate::app::message::PreviewMessage::SaveFile),
            ),
            MenuEntry::item(
                NativeMenuAction::FileGoHome,
                "返回首页",
                None,
                Message::View(message::ViewMessage::GoHome),
            ),
        ]
    }
}

pub(crate) fn edit_menu_entries(is_project: bool) -> Vec<MenuEntry> {
    if !is_project {
        return Vec::new();
    }

    vec![
        MenuEntry::item(
            NativeMenuAction::EditUndo,
            "撤销",
            Some("⌘Z"),
            Message::Editor(crate::app::message::editor::EditorMessage::Undo),
        ),
        MenuEntry::item(
            NativeMenuAction::EditRedo,
            "重做",
            Some("⇧⌘Z"),
            Message::Editor(crate::app::message::editor::EditorMessage::Redo),
        ),
        MenuEntry::Separator,
        MenuEntry::item(NativeMenuAction::EditCut, "剪切", Some("⌘X"), Message::CutShortcut),
        MenuEntry::item(NativeMenuAction::EditCopy, "复制", Some("⌘C"), Message::CopyShortcut),
        MenuEntry::item(NativeMenuAction::EditPaste, "粘贴", Some("⌘V"), Message::PasteShortcut),
        MenuEntry::Separator,
        MenuEntry::item(
            NativeMenuAction::EditOpenSearch,
            "搜索",
            Some("⌘F"),
            Message::Editor(crate::app::message::editor::EditorMessage::OpenSearch),
        ),
        MenuEntry::item(
            NativeMenuAction::EditOpenReplace,
            "替换",
            Some("⌥⌘F"),
            Message::Editor(crate::app::message::editor::EditorMessage::OpenReplace),
        ),
        MenuEntry::Separator,
        MenuEntry::item(
            NativeMenuAction::EditFindInProject,
            "在文件中搜索",
            None,
            Message::Project(message::ProjectMessage::FileTreeFindInProject),
        ),
        MenuEntry::item(
            NativeMenuAction::EditReplaceInProject,
            "在文件中替换",
            None,
            Message::Project(message::ProjectMessage::FileTreeReplaceInProject),
        ),
    ]
}

pub(crate) fn view_menu_entries(
    workspace_mode: WorkspaceMode,
    show_file_manager: bool,
    file_manager_show_changes: bool,
) -> Vec<MenuEntry> {
    if workspace_mode == WorkspaceMode::Work {
        return vec![
            MenuEntry::item(
                NativeMenuAction::ViewToggleWorkSidebar,
                "切换工作侧栏",
                None,
                Message::View(message::ViewMessage::WorkSidebarToggled),
            ),
            MenuEntry::item(
                NativeMenuAction::ViewWorkOverview,
                "概览",
                None,
                Message::View(message::ViewMessage::WorkResourceModeSelected(
                    WorkResourceMode::Overview,
                )),
            ),
            MenuEntry::item(
                NativeMenuAction::ViewWorkWorkspaceFiles,
                "工作空间文件",
                None,
                Message::View(message::ViewMessage::WorkResourceModeSelected(
                    WorkResourceMode::WorkspaceFiles,
                )),
            ),
            MenuEntry::item(
                NativeMenuAction::ViewWorkBrowser,
                "浏览器",
                None,
                Message::View(message::ViewMessage::WorkResourceModeSelected(
                    WorkResourceMode::Browser,
                )),
            ),
        ];
    }

    let file_manager_panel_toggle_label =
        if show_file_manager { "隐藏右侧面板" } else { "显示右侧面板" };
    let file_manager_mode_toggle_label =
        if file_manager_show_changes { "切换到全部文件" } else { "切换到更改" };

    vec![
        MenuEntry::item(
            NativeMenuAction::ViewToggleSettingsPanel,
            "切换左侧面板",
            None,
            Message::View(message::ViewMessage::ToggleSettingsPanel),
        ),
        MenuEntry::item(
            NativeMenuAction::ViewToggleFileManager,
            file_manager_panel_toggle_label,
            None,
            Message::View(message::ViewMessage::FileManagerPanelVisible(!show_file_manager)),
        ),
        MenuEntry::item(
            NativeMenuAction::ViewToggleFileManagerChanges,
            file_manager_mode_toggle_label,
            None,
            Message::Project(message::ProjectMessage::FileManagerShowChanges(
                !file_manager_show_changes,
            )),
        ),
        MenuEntry::item(
            NativeMenuAction::ViewToggleDiffPanel,
            "切换 Diff 面板",
            None,
            Message::View(message::ViewMessage::ToggleDiffPanel),
        ),
        MenuEntry::item(
            NativeMenuAction::ViewToggleTerminalPanel,
            "切换终端面板",
            None,
            Message::View(message::ViewMessage::ToggleTerminalPanel),
        ),
    ]
}

pub(crate) fn help_menu_entries() -> Vec<MenuEntry> {
    vec![
        MenuEntry::item(
            NativeMenuAction::HelpAbout,
            "关于",
            None,
            Message::View(message::ViewMessage::ToggleAboutModal),
        ),
        MenuEntry::Separator,
        MenuEntry::item(
            NativeMenuAction::HelpRestart,
            "重启",
            None,
            Message::View(message::ViewMessage::RestartApp),
        ),
        MenuEntry::item(
            NativeMenuAction::HelpInstallCli,
            "安装 CLI 命令工具",
            None,
            Message::View(message::ViewMessage::InstallCliTool),
        ),
        MenuEntry::item(
            NativeMenuAction::HelpCheckUpdate,
            "检测更新",
            None,
            Message::View(message::ViewMessage::OpenAppUpdateModal),
        ),
    ]
}

/// 构建或处理 `file_menu` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn file_menu(app: &App) -> Element<'_, Message> {
    let is_project = matches!(app.screen, Screen::Project);
    let file_btn = menu_btn("文件", MenuType::File, app.active_menu);

    let file_content = menu_container(menu_entries(file_menu_entries(is_project)));

    BelowOverlay::new(file_btn, file_content)
        .show(app.active_menu == Some(MenuType::File))
        .on_close(Message::View(message::ViewMessage::ToggleMenu(None)))
        .into()
}

/// 构建或处理 `edit_menu` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn edit_menu(app: &App) -> Element<'_, Message> {
    let is_project = matches!(app.screen, Screen::Project);
    let edit_btn = menu_btn("编辑", MenuType::Edit, app.active_menu);

    let edit_content = menu_container(menu_entries(edit_menu_entries(is_project)));

    BelowOverlay::new(edit_btn, edit_content)
        .show(app.active_menu == Some(MenuType::Edit))
        .on_close(Message::View(message::ViewMessage::ToggleMenu(None)))
        .into()
}

/// 构建或处理 `view_menu` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn view_menu(app: &App) -> Element<'_, Message> {
    let view_btn = menu_btn("视图", MenuType::View, app.active_menu);
    let view_content = menu_container(menu_entries(view_menu_entries(
        app.workspace_mode,
        app.show_file_manager,
        app.file_manager_show_changes,
    )));

    BelowOverlay::new(view_btn, view_content)
        .show(app.active_menu == Some(MenuType::View))
        .on_close(Message::View(message::ViewMessage::ToggleMenu(None)))
        .into()
}

/// 构建或处理 `help_menu` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub(super) fn help_menu(app: &App) -> Element<'_, Message> {
    let help_btn = menu_btn("帮助", MenuType::Help, app.active_menu);
    let help_btn = if let Some(release) = app.desktop_update.notification_release() {
        use iced::widget::{Space, container, row, text, tooltip};
        let dot = container(Space::new().width(6).height(6)).style(|theme: &iced::Theme| {
            container::Style {
                background: Some(theme.extended_palette().danger.strong.color.into()),
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }
        });
        let hint = format!("当前版本 {}，可升级至 {}", env!("CARGO_PKG_VERSION"), release.version);
        tooltip(
            row![help_btn, dot].spacing(2).align_y(iced::Alignment::Center),
            container(text(hint).size(12))
                .padding([6, 10])
                .style(super::widgets::icon_tooltip_style),
            tooltip::Position::Bottom,
        )
        .into()
    } else {
        help_btn
    };
    let help_content = menu_container(menu_entries(help_menu_entries()));

    BelowOverlay::new(help_btn, help_content)
        .show(app.active_menu == Some(MenuType::Help))
        .on_close(Message::View(message::ViewMessage::ToggleMenu(None)))
        .into()
}
