use crate::app::{App, Message, Screen, message};
use crate::app::state::{WorkspaceMode, WorkResourceMode};
use crate::app::message::view::MenuType;
use desktop_navigation::components::top_bar::menus as domain;
use crate::app::components::top_bar::menu_actions::NativeMenuAction;
pub(crate) enum MenuEntry {
 Item { #[cfg(any(target_os="macos",test))] action: Option<NativeMenuAction>, label: &'static str, shortcut: Option<&'static str>, message: Box<Message> }, Separator,
}
fn adapt(entry: domain::MenuEntry) -> MenuEntry {
 match entry {
 domain::MenuEntry::Separator => MenuEntry::Separator,
 domain::MenuEntry::Item { action: _action, label, shortcut, message } => MenuEntry::Item { #[cfg(any(target_os="macos",test))] action: _action, label, shortcut, message: Box::new(map(*message)) },
 }
}
pub(crate) fn file_menu_entries(is_project: bool) -> Vec<MenuEntry> { domain::file_menu_entries(is_project).into_iter().map(adapt).collect() }
pub(crate) fn edit_menu_entries(is_project: bool) -> Vec<MenuEntry> { domain::edit_menu_entries(is_project).into_iter().map(adapt).collect() }
pub(crate) fn view_menu_entries(workspace_mode: WorkspaceMode, show_file_manager: bool, file_manager_show_changes: bool) -> Vec<MenuEntry> { domain::view_menu_entries(workspace_mode, show_file_manager, file_manager_show_changes).into_iter().map(adapt).collect() }
pub(crate) fn help_menu_entries() -> Vec<MenuEntry> { domain::help_menu_entries().into_iter().map(adapt).collect() }
fn to_domain(value: MenuType) -> domain::MenuType { match value {MenuType::File => domain::MenuType::File,MenuType::Edit => domain::MenuType::Edit,MenuType::View => domain::MenuType::View,MenuType::Help => domain::MenuType::Help,MenuType::OpenExternal => domain::MenuType::OpenExternal,MenuType::GatewayServices => domain::MenuType::GatewayServices,} }
fn from_domain(value: domain::MenuType) -> MenuType { match value {domain::MenuType::File => MenuType::File,domain::MenuType::Edit => MenuType::Edit,domain::MenuType::View => MenuType::View,domain::MenuType::Help => MenuType::Help,domain::MenuType::OpenExternal => MenuType::OpenExternal,domain::MenuType::GatewayServices => MenuType::GatewayServices,} }
fn inputs(app: &App) -> domain::Inputs<'_> { domain::Inputs { i18n: crate::app::i18n::I18n::new(app.desktop_locale), is_project: matches!(app.screen, Screen::Project), active_menu: app.active_menu.map(to_domain), workspace_mode: app.workspace_mode, show_file_manager: app.file_manager.show_file_manager, file_manager_show_changes: app.file_manager.file_manager_show_changes, update_version: app.desktop_update.notification_release().map(|r| r.version.as_str()), } }
pub(super) fn file_menu(app: &App) -> iced::Element<'_, Message> { domain::file_menu(&inputs(app)).map(map) }
pub(super) fn edit_menu(app: &App) -> iced::Element<'_, Message> { domain::edit_menu(&inputs(app)).map(map) }
pub(super) fn view_menu(app: &App) -> iced::Element<'_, Message> { domain::view_menu(&inputs(app)).map(map) }
pub(super) fn help_menu(app: &App) -> iced::Element<'_, Message> { domain::help_menu(&inputs(app)).map(map) }
fn map(event: domain::Message) -> Message { match event {
domain::Message::ProjectFileNewSession => Message::View(message::ViewMessage::ProjectFileNewSession),
domain::Message::ProjectFileNewProject => Message::View(message::ViewMessage::ProjectFileNewProject),
domain::Message::ProjectFileShowSessions => Message::View(message::ViewMessage::ProjectFileShowSessions),
domain::Message::ProjectFileShowProjects => Message::View(message::ViewMessage::ProjectFileShowProjects),
domain::Message::ProjectFileSaveAll => Message::View(message::ViewMessage::ProjectFileSaveAll),
domain::Message::GoHome => Message::View(message::ViewMessage::GoHome),
domain::Message::WorkSidebarToggled => Message::View(message::ViewMessage::WorkSidebarToggled),
domain::Message::WorkResourceModeSelected(value) => Message::View(message::ViewMessage::WorkResourceModeSelected(value)),
domain::Message::ToggleSettingsPanel => Message::View(message::ViewMessage::ToggleSettingsPanel),
domain::Message::FileManagerPanelVisible(value) => Message::View(message::ViewMessage::FileManagerPanelVisible(value)),
domain::Message::ToggleDiffPanel => Message::View(message::ViewMessage::ToggleDiffPanel),
domain::Message::ToggleTerminalPanel => Message::View(message::ViewMessage::ToggleTerminalPanel),
domain::Message::ToggleAboutModal => Message::View(message::ViewMessage::ToggleAboutModal),
domain::Message::RestartApp => Message::View(message::ViewMessage::RestartApp),
domain::Message::InstallCliTool => Message::View(message::ViewMessage::InstallCliTool),
#[cfg(not(target_arch = "wasm32"))]
domain::Message::InstallWeb => Message::View(message::ViewMessage::WebInstall(crate::app::web_install::Event::Open)),
domain::Message::OpenAppUpdateModal => Message::View(message::ViewMessage::OpenAppUpdateModal),
domain::Message::ToggleMenu(value) => Message::View(message::ViewMessage::ToggleMenu(value.map(from_domain))),
domain::Message::Undo => Message::Editor(message::editor::EditorMessage::Undo),
domain::Message::Redo => Message::Editor(message::editor::EditorMessage::Redo),
domain::Message::OpenSearch => Message::Editor(message::editor::EditorMessage::OpenSearch),
domain::Message::OpenReplace => Message::Editor(message::editor::EditorMessage::OpenReplace),
domain::Message::FileTreeFindInProject => Message::Project(message::ProjectMessage::FileTreeFindInProject),
domain::Message::FileTreeReplaceInProject => Message::Project(message::ProjectMessage::FileTreeReplaceInProject),
domain::Message::FileManagerShowChanges(value) => Message::Project(message::ProjectMessage::FileManagerShowChanges(value)),
domain::Message::SaveFile => Message::Preview(message::PreviewMessage::SaveFile),
domain::Message::CutShortcut => Message::CutShortcut,
domain::Message::CopyShortcut => Message::CopyShortcut,
domain::Message::PasteShortcut => Message::PasteShortcut,
domain::Message::MenuHovered(menu) => Message::View(message::ViewMessage::MenuHovered(from_domain(menu))),
domain::Message::MenuAction(event) => Message::View(message::ViewMessage::MenuAction(Box::new(map(*event)))),
} }
