//! 预览 LSP 的局部生命周期、悬停、补全及事件处理。
use super::{PreviewTab, LspHoverPending};
use iced::Task;
use iced::widget::operation::{focus, scroll_to};
use iced::widget::{Id, scrollable};
use iced_code_editor::{LspDocument, LspEvent, lsp_language_for_path};
use preview_content::path_to_file_uri;
use std::path::Path;
use std::time::Instant;

pub type Notifications = Vec<(&'static str, Vec<desktop_i18n::format::Argument>)>;
fn set_lsp_status(
    status: &mut Option<String>,
    localized: &mut Option<(String, desktop_i18n::format::Message)>,
    source: &str,
    arguments: Vec<desktop_i18n::format::Argument>,
) {
    let message = desktop_i18n::format::Message::new(source, arguments);
    let raw = message.render(desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese)).expect("valid LSP template");
    *status = Some(raw.clone());
    *localized = Some((raw, message));
}
pub fn lsp_root_uri_for_preview_path(
    project_path: Option<&str>,
    file_path: &Path,
) -> Option<String> {
    if let Some(project_root) =
        project_path.filter(|project_root| !project_root.trim().is_empty()).map(Path::new)
    {
        return Some(path_to_file_uri(project_root));
    }

    let standalone_root = if file_path.is_dir() { file_path } else { file_path.parent()? };
    Some(path_to_file_uri(standalone_root))
}

pub fn detach_lsp_for_tab(tab: &mut PreviewTab) {
    // 从编辑器中分离 LSP 客户端
    tab.editor.detach_lsp();

    // 清除 LSP 服务器标识
    tab.lsp_server_key = None;

    // 清除 LSP URI 标识
    tab.lsp_uri = None;

    // 清除语言 ID
    tab.lsp_language_id = None;
}

pub fn sync_lsp_for_path(
    lsp_disabled: &mut bool,
    lsp_manager: &mut Option<desktop_lsp::LspServiceManager>,
    lsp_status: &mut Option<String>,
    lsp_status_message: &mut Option<(String, desktop_i18n::format::Message)>,
    notifications: &mut Notifications,
    preview_tabs: &mut Vec<PreviewTab>,
    project_path: &mut Option<String>,
    path: &str,
) -> bool {
    if (*lsp_disabled) || preview_tabs.iter().any(|tab| tab.path == path && tab.read_only) {
        if let Some(tab) = preview_tabs.iter_mut().find(|tab| tab.path == path) {
            detach_lsp_for_tab(tab);
        }
        return false;
    }

    // 将路径字符串转换为 Path 对象
    let path_obj = Path::new(path);

    // 查找文件对应的预览标签页索引，不存在则返回失败
    let Some(tab_index) = preview_tabs.iter().position(|t| t.path == path) else {
        return false;
    };

    // 检测文件语言类型，无法识别则断开 LSP 并返回失败
    let Some(language) = lsp_language_for_path(path_obj) else {
        let tab = &mut (*preview_tabs)[tab_index];
        detach_lsp_for_tab(tab);
        return false;
    };

    // 确定项目根目录 URI，无法确定则断开 LSP 并返回失败
    let Some(root_uri) = lsp_root_uri_for_preview_path(project_path.as_deref(), path_obj)
    else {
        let tab = &mut (*preview_tabs)[tab_index];
        detach_lsp_for_tab(tab);
        return false;
    };

    // 将文件路径转换为 file:// URI 格式
    let uri = path_to_file_uri(path_obj);

    // 检查是否可以复用现有的 LSP 服务器连接
    let res = {
        let tab = &mut (*preview_tabs)[tab_index];
        // 如果 LSP 服务器标识匹配且编辑器已附加 LSP，则复用连接
        if tab.lsp_server_key == Some(language.server_key) && tab.editor.has_lsp_attached() {
            // 打开文档并更新相关状态
            tab.editor.lsp_open_document(LspDocument::new(uri.clone(), language.language_id));
            tab.lsp_uri = Some(uri.clone());
            tab.lsp_language_id = Some(language.language_id.to_string());
            true
        } else {
            false
        }
    };
    if res {
        return true;
    }

    // 无法复用现有连接，需要启动新的 LSP 服务器
    let (notify, status, success) = {
        let tab = &mut (*preview_tabs)[tab_index];
        // 先断开旧的 LSP 连接
        detach_lsp_for_tab(tab);

        let Some(manager) = lsp_manager.as_mut() else {
            return false;
        };
        if let Some(project_path) = project_path.as_deref() {
            manager.prestart_for_project(project_path);
        }

        // 尝试附加或复用共享 LSP 服务
        match manager.get_or_create(language.server_key, &root_uri) {
            Ok(client) => {
                // 成功启动，附加 LSP 客户端到编辑器
                tab.editor.attach_lsp(client, LspDocument::new(uri.clone(), language.language_id));
                // 启用自动刷新，确保编辑内容及时同步到 LSP
                tab.editor.set_lsp_auto_flush(true);
                // 更新标签页的 LSP 相关状态
                tab.lsp_server_key = Some(language.server_key);
                tab.lsp_uri = Some(uri.clone());
                tab.lsp_language_id = Some(language.language_id.to_string());
                // 返回：无通知消息、服务器就绪状态、成功标志
                (None, Some(("%s 就绪", vec![language.server_key.into()])), true)
            }
            Err(err) => {
                // 启动失败，清除服务器标识
                tab.lsp_server_key = None;
                // 返回：错误通知、失败状态、失败标志
                (Some(err.clone()), Some(("LSP 失败: %s", vec![err.into()])), false)
            }
        }
    };

    // 如果有通知消息，推送到应用通知队列
    if let Some(message) = notify {
        notifications.push(("LSP 启动失败: %s", vec![message.into()]));
    }
    // 更新全局 LSP 状态显示
    if let Some((source, arguments)) = status {
        set_lsp_status(lsp_status, lsp_status_message, source, arguments);
    } else {
        (*lsp_status) = None;
        (*lsp_status_message) = None;
    }
    success
}

pub fn disable_lsp(
    lsp_disabled: &mut bool,
    lsp_hover_anchor: &mut Option<(String, iced_code_editor::LspPosition)>,
    lsp_hover_hide_deadline: &mut Option<Instant>,
    lsp_hover_pending: &mut Option<super::LspHoverPending>,
    lsp_manager: &mut Option<desktop_lsp::LspServiceManager>,
    lsp_overlay: &mut iced_code_editor::LspOverlayState,
    lsp_overlay_path: &mut Option<String>,
    lsp_progress: &mut std::collections::HashMap<String, std::collections::HashMap<String, super::LspProgress>>,
    lsp_status: &mut Option<String>,
    lsp_status_message: &mut Option<(String, desktop_i18n::format::Message)>,
    preview_tabs: &mut Vec<PreviewTab>,
) {
    (*lsp_disabled) = true;
    clear_lsp_hover(lsp_hover_anchor, lsp_hover_hide_deadline, lsp_hover_pending, lsp_overlay, lsp_overlay_path);
    clear_lsp_completion(lsp_overlay, lsp_overlay_path, false);
    for tab in &mut (*preview_tabs) {
        detach_lsp_for_tab(tab);
    }
    lsp_progress.clear();
    set_lsp_status(lsp_status, lsp_status_message, "LSP 已禁用", vec![]);
    if let Some(manager) = lsp_manager.as_mut() {
        manager.shutdown_all();
    }
}

pub fn enable_lsp(
    active_preview_path: &mut Option<String>,
    lsp_disabled: &mut bool,
    lsp_manager: &mut Option<desktop_lsp::LspServiceManager>,
    lsp_status: &mut Option<String>,
    lsp_status_message: &mut Option<(String, desktop_i18n::format::Message)>,
    notifications: &mut Notifications,
    preview_tabs: &mut Vec<PreviewTab>,
    project_path: &mut Option<String>,
) {
    (*lsp_disabled) = false;
    set_lsp_status(lsp_status, lsp_status_message, "LSP 已启用", vec![]);
    if let (Some(manager), Some(project_path)) =
        (lsp_manager.as_mut(), project_path.as_deref())
    {
        manager.prestart_for_project(project_path);
    }
    if let Some(path) = active_preview_path.clone() {
        let _ = sync_lsp_for_path(lsp_disabled, lsp_manager, lsp_status, lsp_status_message, notifications, preview_tabs, project_path, &path);
    }
}

pub fn clear_lsp_completion(lsp_overlay: &mut iced_code_editor::LspOverlayState, lsp_overlay_path: &mut Option<String>, suppress: bool) {
    // 清空所有补全项（包括原始列表）
    lsp_overlay.all_completions.clear();

    // 清空已过滤的补全项
    lsp_overlay.completion_items.clear();

    // 清空补全过滤器
    lsp_overlay.completion_filter.clear();

    // 隐藏补全窗口
    lsp_overlay.completion_visible = false;

    // 设置抑制标志，防止补全窗口在短期内反复弹出
    lsp_overlay.completion_suppressed = suppress;

    // 重置选中索引为第一项
    lsp_overlay.completion_selected = 0;

    // 如果没有显示悬停提示，同时清除 LSP 路径
    // 这样可以避免在没有 LSP 功能时保持不必要的状态
    if !lsp_overlay.hover_visible {
        (*lsp_overlay_path) = None;
    }
}

pub fn filter_completions(lsp_overlay: &mut iced_code_editor::LspOverlayState) {
    // 获取过滤文本并转换为小写（用于不区分大小写的匹配）
    let filter = lsp_overlay.completion_filter.to_lowercase();
    // 更新过滤器为小写版本
    lsp_overlay.completion_filter = filter.to_lowercase();

    // 根据过滤文本筛选补全项
    if filter.is_empty() {
        // 如果没有过滤文本，显示所有补全项
        lsp_overlay.completion_items = lsp_overlay.all_completions.clone();
    } else {
        // 否则只显示包含过滤文本的补全项（不区分大小写）
        lsp_overlay.completion_items = (*lsp_overlay)
            .all_completions
            .iter()
            .filter(|item| item.to_lowercase().contains(&filter))
            .cloned()
            .collect();
    }

    // 根据是否有匹配项更新补全窗口的可见性
    lsp_overlay.completion_visible = !lsp_overlay.completion_items.is_empty();

    // 确保选中索引不越界（如果过滤后列表变短）
    if lsp_overlay.completion_selected >= lsp_overlay.completion_items.len() {
        lsp_overlay.completion_selected =
            lsp_overlay.completion_items.len().saturating_sub(1);
    }
}

pub fn find_word_start(line: &str, cursor_col: usize) -> usize {
    // 将行内容转换为字符数组，便于索引访问
    let chars: Vec<char> = line.chars().collect();

    // 从光标位置开始
    let mut word_start = cursor_col;

    // 向前扫描，查找单词的起始位置
    while word_start > 0 {
        // 获取光标前一个字符
        let ch = chars.get(word_start - 1).copied().unwrap_or(' ');

        // 如果字符不是字母数字或下划线，说明到达单词边界
        if !ch.is_alphanumeric() && ch != '_' {
            break;
        }

        // 继续向前扫描
        word_start -= 1;
    }

    word_start
}

pub fn apply_completion(active_preview_path: &mut Option<String>, preview_tabs: &mut Vec<PreviewTab>, completion_text: &str) {
    // 获取当前活动的预览路径，没有则直接返回
    let Some(path) = active_preview_path.clone() else {
        return;
    };

    // 查找对应的预览标签页
    if let Some(tab) = preview_tabs.iter_mut().find(|t| t.path.as_str() == path.as_str()) {
        // 获取编辑器内容和光标位置
        let content = tab.editor.content();
        let (line, col) = tab.editor.cursor_position();

        // 获取当前行的内容
        let line_content = content.lines().nth(line).unwrap_or("");

        // 查找光标所在单词的起始位置
        let word_start_col = find_word_start(line_content, col);

        // 计算需要删除的字符数（从单词起始位置到光标位置）
        let chars_to_delete = col.saturating_sub(word_start_col);

        // 删除光标前的单词部分
        for _ in 0..chars_to_delete {
            let _ = tab.editor.inner.update(&iced_code_editor::Message::Backspace);
        }

        // 逐字符插入补全文本
        for ch in completion_text.chars() {
            let _ = tab.editor.inner.update(&iced_code_editor::Message::CharacterInput(ch));
        }
    }
}

pub fn file_uri_to_path(uri: &str) -> Option<String> {
    // 移除 "file://" 前缀，并将 URI 转义字符替换为实际字符
    uri.strip_prefix("file://").map(|path| path.replace("%20", " "))
}

pub fn drain_lsp_events(
    active_preview_path: &mut Option<String>,
    lsp_events: &mut Option<std::sync::mpsc::Receiver<LspEvent>>,
    lsp_hover_anchor: &mut Option<(String, iced_code_editor::LspPosition)>,
    lsp_hover_hide_deadline: &mut Option<Instant>,
    lsp_hover_pending: &mut Option<super::LspHoverPending>,
    lsp_overlay: &mut iced_code_editor::LspOverlayState,
    lsp_overlay_path: &mut Option<String>,
    lsp_progress: &mut std::collections::HashMap<String, std::collections::HashMap<String, super::LspProgress>>,
    lsp_status: &mut Option<String>,
    lsp_status_message: &mut Option<(String, desktop_i18n::format::Message)>,
    notifications: &mut Notifications,
    pending_preview_goto: &mut Option<(String, usize, usize)>,
    preview_tabs: &mut Vec<PreviewTab>,
) -> Vec<String> {
    // 尝试获取 LSP 事件接收器，没有则不请求打开文件
    let Some(receiver) = lsp_events.take() else {
        return Vec::new();
    };

    // 收集定义跳转需要打开的路径，交由桌面映射任务
    let mut jump_tasks = Vec::new();

    // 循环处理所有待处理的事件
    loop {
        match receiver.try_recv() {
            Ok(event) => match event {
                // 处理悬停提示事件
                LspEvent::Hover { text } => {
                    tracing::debug!(
                        "[LSP HOVER] received Hover event, text.len={}, hover_visible={}",
                        text.len(),
                        lsp_overlay.hover_visible
                    );
                    // 如果文本为空，清除悬停提示
                    if text.trim().is_empty() {
                        clear_lsp_hover(lsp_hover_anchor, lsp_hover_hide_deadline, lsp_hover_pending, lsp_overlay, lsp_overlay_path);
                    } else {
                        lsp_overlay.show_hover(text);
                        (*lsp_hover_hide_deadline) = None;
                        (*lsp_overlay_path) = active_preview_path.clone();
                        tracing::debug!(
                            "[LSP HOVER] show_hover called, hover_visible={}, hover_position={:?}",
                            lsp_overlay.hover_visible,
                            lsp_overlay.hover_position
                        );
                    }
                }
                // 处理自动补全事件
                LspEvent::Completion { items } => {
                    // 计算补全窗口的显示位置（基于光标的屏幕位置）
                    let position = (*active_preview_path)
                        .as_ref()
                        .and_then(|path| {
                            (*preview_tabs)
                                .iter()
                                .find(|t| &t.path == path)
                                .and_then(|tab| tab.editor.cursor_screen_position())
                        })
                        .unwrap_or(iced::Point::new(4.0, 4.0));

                    // 设置补全项列表和显示位置
                    lsp_overlay.set_completions(items, position);

                    // 更新补全窗口的精确位置（用于滚动和布局）
                    if let Some(path) = active_preview_path.clone()
                        && let Some(tab) =
                            preview_tabs.iter().find(|t| t.path.as_str() == path.as_str())
                    {
                        lsp_overlay.completion_position = tab.editor.cursor_screen_position();
                    }

                    // 补全结果属于当前活动预览，覆盖切换标签前遗留的浮层路径。
                    if lsp_overlay.completion_visible {
                        (*lsp_overlay_path) = active_preview_path.clone();
                    }
                }
                // 处理跳转到定义事件
                LspEvent::Definition { uri, range } => {
                    // 将 URI 转换为本地路径
                    if let Some(path) = file_uri_to_path(&uri) {
                        // 记录待跳转的位置信息
                        (*pending_preview_goto) = Some((
                            path.clone(),
                            range.start.line as usize,
                            range.start.character as usize,
                        ));
                        // 返回打开文件的局部效果
                        jump_tasks
                            .push(path.clone());
                    }
                }
                LspEvent::Formatting { .. } => {
                    // 预览编辑器尚未接入 LSP 格式化请求与编辑应用流程。
                    notifications.push(("暂不支持 LSP 格式化", vec![]));
                }
                // 处理 LSP 服务器进度事件
                LspEvent::Progress { token, server_key, title, message, percentage, done } => {
                    if done {
                        // 进度完成，从进度映射中移除
                        if let Some(map) = lsp_progress.get_mut(&server_key) {
                            map.remove(&token);
                            // 如果映射为空，移除整个服务器的进度记录
                            if map.is_empty() {
                                lsp_progress.remove(&server_key);
                            }
                        }
                        // 更新状态为就绪
                        set_lsp_status(
                            lsp_status, lsp_status_message,
                            "%s 就绪",
                            vec![server_key.into()],
                        );
                    } else {
                        // 进度进行中，更新状态和进度映射
                        (*lsp_status_message) = None;
                        (*lsp_status) = Some(format!("{} {}", server_key, title));
                        (*lsp_progress)
                            .entry(server_key)
                            .or_default()
                            .insert(token, super::LspProgress { title, message, percentage });
                    }
                }
                // 处理 LSP 日志事件
                LspEvent::Log { server_key: _, message } => {
                    // 将日志显示为应用通知
                    notifications.push(("LSP 日志: %s", vec![message.into()]));
                }
            },
            // 事件队列为空，将接收器放回并退出循环
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                (*lsp_events) = Some(receiver);
                break;
            }
            // 通道已断开，清除接收器并退出循环
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                (*lsp_events) = None;
                break;
            }
        }
    }

    // 保留定义跳转顺序，由桌面创建批处理任务
    jump_tasks
}

pub fn focus_hover_overlay<Message: Send + 'static>() -> Task<Message> {
    focus(Id::new("preview_hover_overlay"))
}

pub fn completion_scroll_task<Message: Send + 'static>(selected: usize) -> Task<Message> {
    // 计算滚动位置：选中索引 × 每项高度（20 像素）
    let scroll_y = selected as f32 * 20.0;

    // 创建滚动到指定位置的任务
    scroll_to(
        Id::new("preview_completion_scrollable"),
        scrollable::AbsoluteOffset { x: 0.0, y: scroll_y },
    )
}

#[cfg(test)] #[path = "lsp_tests.rs"] mod lsp_tests;

mod hover;
pub use hover::{clear_lsp_hover, process_lsp_hover_timers, handle_lsp_hover_from_mouse};
