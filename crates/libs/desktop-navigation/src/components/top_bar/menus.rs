//! 桌面应用顶部栏的按钮、菜单与窗口交互控件。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use super::widgets::{menu_container, menu_separator};
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_platform::menu_actions::NativeMenuAction;

use crate::presentation::WorkResourceMode;
use super::types::WorkspaceMode;

use iced::Element;
use iced::widget::Column;

pub enum MenuEntry {
    Item {
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
                action: Some(_action),
            label,
            shortcut,
            message: Box::new(message),
        }
    }

    pub fn into_element(self, i18n: desktop_i18n::I18n) -> Element<'static, Message> {
        match self {
            Self::Item { label, shortcut, message, .. } => {
                menu_item_btn(i18n.__(label), shortcut, Some(*message))
            }
            Self::Separator => menu_separator(),
        }
    }
}

fn menu_entries(
    i18n: desktop_i18n::I18n,
    entries: Vec<MenuEntry>,
) -> Element<'static, Message> {
    Column::with_children(entries.into_iter().map(|entry| entry.into_element(i18n))).into()
}

pub fn file_menu_entries(is_project: bool) -> Vec<MenuEntry> {
    if is_project {
        vec![
            MenuEntry::item(
                NativeMenuAction::FileNewSession,
                "新建会话",
                None,
                Message::ProjectFileNewSession,
            ),
            MenuEntry::item(
                NativeMenuAction::FileNewProject,
                "新建项目",
                None,
                Message::ProjectFileNewProject,
            ),
            MenuEntry::item(
                NativeMenuAction::FileShowSessions,
                "查看会话",
                None,
                Message::ProjectFileShowSessions,
            ),
            MenuEntry::item(
                NativeMenuAction::FileShowProjects,
                "查看项目",
                None,
                Message::ProjectFileShowProjects,
            ),
            MenuEntry::Separator,
            MenuEntry::item(
                NativeMenuAction::FileSave,
                "保存文件",
                Some("⌘S"),
                Message::SaveFile,
            ),
            MenuEntry::item(
                NativeMenuAction::FileSaveAll,
                "保存所有文件",
                Some("⌥⌘S"),
                Message::ProjectFileSaveAll,
            ),
        ]
    } else {
        vec![
            MenuEntry::item(
                NativeMenuAction::FileSave,
                "保存代码",
                Some("⌘S"),
                Message::SaveFile,
            ),
            MenuEntry::item(
                NativeMenuAction::FileGoHome,
                "返回首页",
                None,
                Message::GoHome,
            ),
        ]
    }
}

pub fn edit_menu_entries(is_project: bool) -> Vec<MenuEntry> {
    if !is_project {
        return Vec::new();
    }

    vec![
        MenuEntry::item(
            NativeMenuAction::EditUndo,
            "撤销",
            Some("⌘Z"),
            Message::Undo,
        ),
        MenuEntry::item(
            NativeMenuAction::EditRedo,
            "重做",
            Some("⇧⌘Z"),
            Message::Redo,
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
            Message::OpenSearch,
        ),
        MenuEntry::item(
            NativeMenuAction::EditOpenReplace,
            "替换",
            Some("⌥⌘F"),
            Message::OpenReplace,
        ),
        MenuEntry::Separator,
        MenuEntry::item(
            NativeMenuAction::EditFindInProject,
            "在文件中搜索",
            None,
            Message::FileTreeFindInProject,
        ),
        MenuEntry::item(
            NativeMenuAction::EditReplaceInProject,
            "在文件中替换",
            None,
            Message::FileTreeReplaceInProject,
        ),
    ]
}

pub fn view_menu_entries(
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
                Message::WorkSidebarToggled,
            ),
            MenuEntry::item(
                NativeMenuAction::ViewWorkOverview,
                "概览",
                None,
                Message::WorkResourceModeSelected(
                    WorkResourceMode::Overview,
                ),
            ),
            MenuEntry::item(
                NativeMenuAction::ViewWorkWorkspaceFiles,
                "工作空间文件",
                None,
                Message::WorkResourceModeSelected(
                    WorkResourceMode::WorkspaceFiles,
                ),
            ),
            MenuEntry::item(
                NativeMenuAction::ViewWorkBrowser,
                "浏览器",
                None,
                Message::WorkResourceModeSelected(
                    WorkResourceMode::Browser,
                ),
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
            Message::ToggleSettingsPanel,
        ),
        MenuEntry::item(
            NativeMenuAction::ViewToggleFileManager,
            file_manager_panel_toggle_label,
            None,
            Message::FileManagerPanelVisible(!show_file_manager),
        ),
        MenuEntry::item(
            NativeMenuAction::ViewToggleFileManagerChanges,
            file_manager_mode_toggle_label,
            None,
            Message::FileManagerShowChanges(
                !file_manager_show_changes,
            ),
        ),
        MenuEntry::item(
            NativeMenuAction::ViewToggleDiffPanel,
            "切换 Diff 面板",
            None,
            Message::ToggleDiffPanel,
        ),
        #[cfg(not(target_arch = "wasm32"))]
        MenuEntry::item(
            NativeMenuAction::ViewToggleTerminalPanel,
            "切换终端面板",
            None,
            Message::ToggleTerminalPanel,
        ),
    ]
}

pub fn help_menu_entries() -> Vec<MenuEntry> {
    vec![
        MenuEntry::item(
            NativeMenuAction::HelpAbout,
            "关于",
            None,
            Message::ToggleAboutModal,
        ),
        MenuEntry::Separator,
        MenuEntry::item(
            NativeMenuAction::HelpRestart,
            "重启",
            None,
            Message::RestartApp,
        ),
        MenuEntry::item(
            NativeMenuAction::HelpInstallCli,
            "安装 CLI 命令工具",
            None,
            Message::InstallCliTool,
        ),
        #[cfg(not(target_arch = "wasm32"))]
        MenuEntry::item(
            NativeMenuAction::HelpInstallWeb,
            "安装 Web 版本",
            None,
            Message::InstallWeb,
        ),
        MenuEntry::item(
            NativeMenuAction::HelpCheckUpdate,
            "检测更新",
            None,
            Message::OpenAppUpdateModal,
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
pub fn file_menu(app: &Inputs<'_>) -> Element<'static, Message> {
    let is_project = app.is_project;
    let file_btn = menu_btn(
        app.i18n.__("文件"),
        MenuType::File,
        app.active_menu,
    );

    let file_content = menu_container(menu_entries(
        app.i18n,
        file_menu_entries(is_project),
    ));

    BelowOverlay::new(file_btn, file_content)
        .show(app.active_menu == Some(MenuType::File))
        .on_close(Message::ToggleMenu(None))
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
pub fn edit_menu(app: &Inputs<'_>) -> Element<'static, Message> {
    let is_project = app.is_project;
    let edit_btn = menu_btn(
        app.i18n.__("编辑"),
        MenuType::Edit,
        app.active_menu,
    );

    let edit_content = menu_container(menu_entries(
        app.i18n,
        edit_menu_entries(is_project),
    ));

    BelowOverlay::new(edit_btn, edit_content)
        .show(app.active_menu == Some(MenuType::Edit))
        .on_close(Message::ToggleMenu(None))
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
pub fn view_menu(app: &Inputs<'_>) -> Element<'static, Message> {
    let view_btn = menu_btn(
        app.i18n.__("视图"),
        MenuType::View,
        app.active_menu,
    );
    let view_content = menu_container(menu_entries(
        app.i18n,
        view_menu_entries(app.workspace_mode, app.show_file_manager, app.file_manager_show_changes),
    ));

    BelowOverlay::new(view_btn, view_content)
        .show(app.active_menu == Some(MenuType::View))
        .on_close(Message::ToggleMenu(None))
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
pub fn help_menu(app: &Inputs<'_>) -> Element<'static, Message> {
    let help_btn = menu_btn(
        app.i18n.__("帮助"),
        MenuType::Help,
        app.active_menu,
    );
    let help_btn = if let Some(release) = app.update_version {
        use iced::widget::{Space, container, row, text, tooltip};
        let dot = container(Space::new().width(6).height(6)).style(|theme: &iced::Theme| {
            container::Style {
                background: Some(theme.extended_palette().danger.strong.color.into()),
                border: iced::Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }
        });
        let hint = desktop_i18n::__!(
            app.i18n,
            "当前版本 %s，可升级至 %s",
            env!("CARGO_PKG_VERSION"),
            release
        )
        .expect("valid update hint");
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
    let help_content = menu_container(menu_entries(
        app.i18n,
        help_menu_entries(),
    ));

    BelowOverlay::new(help_btn, help_content)
        .show(app.active_menu == Some(MenuType::Help))
        .on_close(Message::ToggleMenu(None))
        .into()
}

fn menu_btn<'a>(label: &'a str, menu_type: MenuType, active_menu: Option<MenuType>) -> Element<'a, Message> {
 super::widgets::menu_btn_with_actions(label, active_menu == Some(menu_type),
 Message::ToggleMenu(Some(menu_type)), Message::MenuHovered(menu_type))
}
fn menu_item_btn<'a>(label: &'a str, shortcut: Option<&'a str>, msg: Option<Message>) -> Element<'a, Message> {
 super::widgets::menu_item_btn_with_action(label, shortcut, msg, |m| Message::MenuAction(Box::new(m)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuType { File, Edit, View, Help, OpenExternal, GatewayServices }
pub struct Inputs<'a> {
 pub i18n: desktop_i18n::I18n,
 pub is_project: bool,
 pub active_menu: Option<MenuType>,
 pub workspace_mode: WorkspaceMode,
 pub show_file_manager: bool,
 pub file_manager_show_changes: bool,
 pub update_version: Option<&'a str>,
}
#[derive(Debug, Clone)]
pub enum Message {
 ProjectFileNewSession,
 ProjectFileNewProject,
 ProjectFileShowSessions,
 ProjectFileShowProjects,
 ProjectFileSaveAll,
 GoHome,
 WorkSidebarToggled,
 WorkResourceModeSelected(WorkResourceMode),
 ToggleSettingsPanel,
 FileManagerPanelVisible(bool),
 ToggleDiffPanel,
 ToggleTerminalPanel,
 ToggleAboutModal,
 RestartApp,
 InstallCliTool,
 #[cfg(not(target_arch = "wasm32"))]
 InstallWeb,
 OpenAppUpdateModal,
 ToggleMenu(Option<MenuType>),
 Undo,
 Redo,
 OpenSearch,
 OpenReplace,
 FileTreeFindInProject,
 FileTreeReplaceInProject,
 FileManagerShowChanges(bool),
 SaveFile,
 CutShortcut,
 CopyShortcut,
 PasteShortcut,
 MenuHovered(MenuType),
 MenuAction(Box<Message>),
}
