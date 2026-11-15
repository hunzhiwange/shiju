use crate::Shell;
use super::TerminalMessage;
type TerminalState = crate::TerminalState<Shell>;
// 终端标签页重命名模块
//
// 本模块提供终端标签页标题编辑功能的更新逻辑，包括：
// - 启动重命名编辑模式
// - 处理编辑过程中的文本变化
// - 保存或取消重命名操作
//
// 重命名流程采用内联编辑模式，用户在原标签标题位置直接编辑，
// 支持提交保存或取消操作回滚到原标题。

use iced::Task;

fn start_edit(title: &str, edit_title: &mut Option<String>) {
    *edit_title = Some(title.to_string());
}

fn change_edit(edit_title: &mut Option<String>, value: String) {
    *edit_title = Some(value);
}

fn save_edit(title: &mut String, edit_title: &mut Option<String>) {
    if let Some(new) = edit_title.take() {
        let name = new.trim().to_string();
        if !name.is_empty() {
            *title = name;
        }
    }
}

fn cancel_edit(edit_title: &mut Option<String>) {
    *edit_title = None;
}

/// Apply terminal-local state changes and return explicit host effects.
pub(super) fn update(terminal: &mut TerminalState, locale: config_ui::DesktopLocale, message: TerminalMessage) -> Task<TerminalMessage> {
    let start_id = if let TerminalMessage::RenameStart(id) = &message { Some(*id) } else { None };
    let task = update_terminal(&mut *terminal, message);
    if let Some(id) = start_id {
        if let Some(tab) =
            terminal.tabs.iter_mut().find(|tab| tab.id == id && tab.default_title)
        {
            tab.edit_title = Some(
                desktop_i18n::__!(desktop_i18n::I18n::new(locale), "终端 %u", id)
                    .expect("terminal title template"),
            );
        }
    }
    task
}

fn update_terminal(terminal: &mut TerminalState, message: TerminalMessage) -> Task<TerminalMessage> {
    match message {
        // 启动重命名编辑模式
        TerminalMessage::RenameStart(id) => {
            // 关闭上下文菜单，避免与编辑界面冲突
            terminal.tab_context_menu_id = None;
            terminal.tab_context_menu_pos = None;

            // 选中目标标签页，确保其处于活动状态
            terminal.select_terminal(id);

            // 初始化编辑状态，将当前标题作为编辑初始值
            if let Some(tab) = terminal.tabs.iter_mut().find(|t| t.id == id) {
                start_edit(&tab.title, &mut tab.edit_title);
            }
            Task::none()
        }

        // 处理编辑过程中的文本变化
        TerminalMessage::RenameChanged(id, v) => {
            // 更新临时编辑缓冲区的内容
            if let Some(tab) = terminal.tabs.iter_mut().find(|t| t.id == id) {
                change_edit(&mut tab.edit_title, v);
            }
            Task::none()
        }

        // 保存编辑后的标题
        TerminalMessage::RenameSave(id) => {
            // 关闭上下文菜单
            terminal.tab_context_menu_id = None;
            terminal.tab_context_menu_pos = None;

            // 提取编辑缓冲区内容并应用
            if let Some(tab) = terminal.tabs.iter_mut().find(|t| t.id == id) {
                if tab.edit_title.as_ref().is_some_and(|title| !title.trim().is_empty()) {
                    tab.default_title = false;
                }
                save_edit(&mut tab.title, &mut tab.edit_title);
            }
            Task::none()
        }

        // 取消编辑操作
        TerminalMessage::RenameCancel(id) => {
            // 关闭上下文菜单
            terminal.tab_context_menu_id = None;
            terminal.tab_context_menu_pos = None;

            // 清除编辑状态，丢弃所有修改，保留原标题
            if let Some(tab) = terminal.tabs.iter_mut().find(|t| t.id == id) {
                cancel_edit(&mut tab.edit_title);
            }
            Task::none()
        }

        // 其他消息类型在本模块不处理，返回空任务
        _ => Task::none(),
    }
}


#[cfg(test)]
#[path = "rename_tests.rs"]
mod rename_tests;
