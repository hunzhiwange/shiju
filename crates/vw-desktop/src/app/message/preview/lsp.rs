//! LSP 领域调用与桌面通知、打开文件任务映射。
#![cfg(not(target_arch = "wasm32"))]
use crate::app::{App, Message, PreviewTab};
use super::PreviewMessage;
use iced::Task;
use desktop_editor::preview::lsp as domain;
pub(crate) fn detach_lsp_for_tab(tab: &mut PreviewTab) { domain::detach_lsp_for_tab(tab) }
pub(crate) fn sync_lsp_for_path(app: &mut App, path: &str) -> bool {
    let mut notifications = Vec::new();
    let result = domain::sync_lsp_for_path(&mut app.lsp_disabled, &mut app.lsp_manager, &mut app.lsp_status, &mut app.lsp_status_message, &mut notifications, &mut app.navigation_tabs.preview_tabs, &mut app.navigation_tabs.project_path, path);
    for (source, arguments) in notifications { app.push_localized_notification(source, arguments); }
    result
}
pub(crate) fn disable_lsp(app: &mut App) {
    domain::disable_lsp(&mut app.lsp_disabled, &mut app.lsp_hover_anchor, &mut app.lsp_hover_hide_deadline, &mut app.lsp_hover_pending, &mut app.lsp_manager, &mut app.lsp_overlay, &mut app.lsp_overlay_path, &mut app.lsp_progress, &mut app.lsp_status, &mut app.lsp_status_message, &mut app.navigation_tabs.preview_tabs);
}
pub(crate) fn enable_lsp(app: &mut App) {
    let mut notifications = Vec::new();
    domain::enable_lsp(&mut app.navigation_tabs.active_preview_path, &mut app.lsp_disabled, &mut app.lsp_manager, &mut app.lsp_status, &mut app.lsp_status_message, &mut notifications, &mut app.navigation_tabs.preview_tabs, &mut app.navigation_tabs.project_path);
    for (source, arguments) in notifications { app.push_localized_notification(source, arguments); }
}
pub(crate) fn clear_lsp_hover(app: &mut App) {
    domain::clear_lsp_hover(&mut app.lsp_hover_anchor, &mut app.lsp_hover_hide_deadline, &mut app.lsp_hover_pending, &mut app.lsp_overlay, &mut app.lsp_overlay_path);
}
pub(crate) fn clear_lsp_completion(app: &mut App, suppress: bool) {
    domain::clear_lsp_completion(&mut app.lsp_overlay, &mut app.lsp_overlay_path, suppress);
}
pub(crate) fn process_lsp_hover_timers(app: &mut App) {
    domain::process_lsp_hover_timers(&mut app.lsp_hover_anchor, &mut app.lsp_hover_hide_deadline, &mut app.lsp_hover_pending, &mut app.lsp_overlay, &mut app.lsp_overlay_path, &mut app.navigation_tabs.preview_tabs);
}
pub(crate) fn drain_lsp_events(app: &mut App) -> Task<Message> {
    let mut notifications = Vec::new();
    let result = domain::drain_lsp_events(&mut app.navigation_tabs.active_preview_path, &mut app.lsp_events, &mut app.lsp_hover_anchor, &mut app.lsp_hover_hide_deadline, &mut app.lsp_hover_pending, &mut app.lsp_overlay, &mut app.lsp_overlay_path, &mut app.lsp_progress, &mut app.lsp_status, &mut app.lsp_status_message, &mut notifications, &mut app.pending_preview_goto, &mut app.navigation_tabs.preview_tabs);
    for (source, arguments) in notifications { app.push_localized_notification(source, arguments); }
    Task::batch(result.into_iter().map(|path| Task::done(Message::Preview(PreviewMessage::Open(path)))))
}
#[cfg(test)]
pub(crate) fn focus_hover_overlay() -> Task<Message> { domain::focus_hover_overlay() }
#[cfg(test)]
pub(crate) fn completion_scroll_task(selected: usize) -> Task<Message> { domain::completion_scroll_task(selected) }
pub(crate) fn tick(app: &mut App) -> Task<Message> {
    // 更新加载动画的帧计数器（用于显示旋转动画）
    app.spinner_frame = app.spinner_frame.wrapping_add(1);

    if app.lsp_disabled {
        return Task::none();
    }

    // 处理悬停提示的定时器逻辑（延迟显示和自动隐藏）
    process_lsp_hover_timers(app);

    // 处理所有待处理的 LSP 事件并返回产生的任务
    drain_lsp_events(app)
}
