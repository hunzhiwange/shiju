pub mod options;
pub mod editor_actions;
pub mod editor_options;
pub mod editor_help;
pub mod editor_popovers;
pub mod settings;
#[derive(Debug, Clone)]
pub enum Message {
    EditorEvent(iced_code_editor::Message),
    SetOption(options::EditorOption, bool),
    SetWrapColumn(String),
    SetIndentStyle(iced_code_editor::IndentStyle),
    ToggleSettings,
    ClosePopover,
    OpenSearch,
    OpenReplace,
    TogglePopover(options::EditorPopover),
}
pub mod controls;
pub mod header_tabs;
#[derive(Debug, Clone)]
pub enum HeaderMessage {
    DragStart(String), DragEnd, OpenFile, Close(String), Select(String),
    RightClick(String, f32, f32), MenuClose, TabMenu(super::tabs::Message),
}
pub mod menus;
#[derive(Debug, Clone)]
pub enum MenuMessage {
    ContextMenuCopy, ContextMenuCut, ContextMenuPaste, ContextMenuDelete,
    InsertSelected, InsertSelectionPositions, Navigate { full_path: String, is_dir: bool },
}
pub mod content;
#[derive(Debug, Clone)]
pub enum ContentMessage<External> {
    External(External), EditorEvent(iced_code_editor::Message),
    #[cfg(not(target_arch = "wasm32"))] MouseEntered,
    #[cfg(not(target_arch = "wasm32"))] MouseExited,
}

#[cfg(test)] mod header_tabs_tests;

#[cfg(test)] mod menus_tests;

#[cfg(test)] mod editor_help_tests;

mod presentation;
pub mod layout;

#[cfg(test)] mod editor_tools_acceptance_tests;
